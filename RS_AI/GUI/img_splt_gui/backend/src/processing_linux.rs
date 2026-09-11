use crate::path_security::{canonical_directory, file_name, require_absolute};
use crate::processing::{MediaToolRunner, OUTPUT_FORMATS, ProcessOptions, ProcessProgress, ProcessReport};
use crate::{BackendError, CancellationToken, ErrorKind, Result};
use rustix::fd::{AsRawFd, OwnedFd};
use rustix::fs::{AtFlags, FileType, Mode, OFlags, RenameFlags, fstat, open, openat, renameat_with, statat, unlinkat};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

const OPEN_DIR: OFlags = OFlags::RDONLY.union(OFlags::DIRECTORY).union(OFlags::NOFOLLOW);
const OPEN_INPUT: OFlags = OFlags::RDONLY.union(OFlags::NOFOLLOW);

#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
}

struct BoundDirectory {
    path: PathBuf,
    fd: OwnedFd,
    identity: Identity,
}

struct BoundInput {
    path: PathBuf,
    name: OsString,
    fd: OwnedFd,
    identity: Identity,
}

struct Temporary {
    name: OsString,
    file: File,
    identity: Identity,
}

pub(crate) fn process_validated(
    options: &ProcessOptions,
    runner: &dyn MediaToolRunner,
    cancellation: &CancellationToken,
    progress: &mut impl FnMut(ProcessProgress),
) -> Result<ProcessReport> {
    options.settings.validate()?;
    if options.files.is_empty() {
        return Err(BackendError::new(
            ErrorKind::InvalidArgument,
            "at least one input file is required",
        ));
    }
    let input_path = canonical_directory(&options.input_directory)?;
    let output_path = canonical_directory(&options.output_directory)?;
    let input_directory = bind_directory(input_path)?;
    let output_directory = bind_directory(output_path)?;
    let output_format = normalize_format(options.output_format.as_deref())?;

    let mut inputs = Vec::with_capacity(options.files.len());
    let mut identities = HashSet::with_capacity(options.files.len());
    let mut output_names = HashSet::with_capacity(options.files.len());
    for path in &options.files {
        let input = bind_input(&input_directory, path)?;
        if !crate::is_supported_image(&input.path) {
            return Err(BackendError::at_path(
                ErrorKind::InvalidArgument,
                "unsupported image format",
                &input.path,
            ));
        }
        if !identities.insert((input.identity.dev, input.identity.ino)) {
            return Err(BackendError::at_path(
                ErrorKind::InvalidArgument,
                "duplicate input file",
                &input.path,
            ));
        }
        let output_name = final_output_name(&input.name, output_format.as_deref())?;
        if !output_names.insert(output_name.clone()) || entry_exists(&output_directory, &output_name)? {
            return Err(BackendError::at_path(
                ErrorKind::Conflict,
                "output already exists or is duplicated",
                output_directory.path.join(output_name),
            ));
        }
        inputs.push(input);
    }

    let total = inputs.len() as u64;
    let mut report = ProcessReport {
        output_directory: output_directory.path.clone(),
        processed: Vec::new(),
        failed: Vec::new(),
    };
    for (index, input) in inputs.into_iter().enumerate() {
        cancellation.check()?;
        match process_one(
            &input,
            &input_directory,
            &output_directory,
            output_format.as_deref(),
            options,
            runner,
            cancellation,
        ) {
            Ok(output) => report.processed.push(output),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::Unavailable | ErrorKind::Cancelled | ErrorKind::Conflict
                ) =>
            {
                return Err(error);
            }
            Err(_) => report.failed.push(input.path),
        }
        progress(ProcessProgress {
            completed: index as u64 + 1,
            total,
        });
    }
    Ok(report)
}

fn process_one(
    input: &BoundInput,
    input_directory: &BoundDirectory,
    output_directory: &BoundDirectory,
    output_format: Option<&str>,
    options: &ProcessOptions,
    runner: &dyn MediaToolRunner,
    cancellation: &CancellationToken,
) -> Result<PathBuf> {
    verify_input(input_directory, input)?;
    verify_directory(output_directory)?;
    let output_name = final_output_name(&input.name, output_format)?;
    let output_path = output_directory.path.join(&output_name);
    if entry_exists(output_directory, &output_name)? {
        return Err(BackendError::at_path(
            ErrorKind::Conflict,
            "output already exists",
            output_path,
        ));
    }
    let extension = Path::new(&output_name)
        .extension()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "output has no extension", &output_path))?;
    let stem = Path::new(&output_name)
        .file_stem()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "output has no file stem", &output_path))?;
    let stable_input = fd_path(&input.fd);

    let upscale_width = if options.upscale {
        verify_input(input_directory, input)?;
        let result = runner.probe_width(&stable_input);
        verify_input(input_directory, input)?;
        let width = result?;
        (width < options.settings.min_upscale_width).then_some(options.settings.target_upscale_width)
    } else {
        None
    };
    if output_format.is_none() && upscale_width.is_none() {
        let mut temporary = create_temporary(output_directory, stem, extension)?;
        let result = copy_bound(input, &mut temporary, cancellation)
            .and_then(|()| verify_input(input_directory, input))
            .and_then(|()| verify_directory(output_directory))
            .and_then(|()| commit(output_directory, &temporary.name, &output_name, &output_path));
        if result.is_err() {
            remove_temporary(output_directory, &temporary.name)?;
        }
        result?;
        return Ok(output_path);
    }

    let mut last_error = None;
    for _ in 0..options.settings.max_retries {
        cancellation.check()?;
        verify_input(input_directory, input)?;
        verify_directory(output_directory)?;
        let temporary = create_temporary(output_directory, stem, extension)?;
        let stable_output = fd_entry_path(&output_directory.fd, &temporary.name);
        let conversion = runner.convert(&stable_input, &stable_output, upscale_width);
        if let Err(error) = cancellation.check() {
            remove_temporary(output_directory, &temporary.name)?;
            return Err(error);
        }
        if let Err(error) = verify_input(input_directory, input).and_then(|()| verify_directory(output_directory)) {
            remove_temporary(output_directory, &temporary.name)?;
            return Err(error);
        }
        match conversion {
            Ok(()) if temporary_is_non_empty(output_directory, &temporary)? => {
                if let Err(error) = commit(output_directory, &temporary.name, &output_name, &output_path) {
                    remove_temporary(output_directory, &temporary.name)?;
                    return Err(error);
                }
                return Ok(output_path);
            }
            Ok(()) => {
                last_error = Some(BackendError::at_path(
                    ErrorKind::Io,
                    "processor created an empty output",
                    &input.path,
                ));
            }
            Err(error) if error.kind() == ErrorKind::Unavailable => {
                remove_temporary(output_directory, &temporary.name)?;
                return Err(error);
            }
            Err(error) => last_error = Some(error),
        }
        remove_temporary(output_directory, &temporary.name)?;
    }
    Err(last_error.unwrap_or_else(|| BackendError::at_path(ErrorKind::Io, "image processing failed", &input.path)))
}

fn bind_directory(path: PathBuf) -> Result<BoundDirectory> {
    let fd = open_directory(&path)?;
    let stat = fstat(&fd).map_err(|error| io_error("cannot inspect opened directory", &path, error))?;
    Ok(BoundDirectory {
        path,
        fd,
        identity: identity(&stat),
    })
}

fn bind_input(directory: &BoundDirectory, path: &Path) -> Result<BoundInput> {
    require_absolute(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "input has no parent directory", path))?;
    let parent = canonical_directory(parent)?;
    if parent != directory.path {
        return Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "file resolves outside the input directory",
            path,
        ));
    }
    let name = file_name(path)?.to_owned();
    let fd = openat(&directory.fd, &name, OPEN_INPUT, Mode::empty()).map_err(|error| {
        let kind = if error == rustix::io::Errno::LOOP {
            ErrorKind::InvalidArgument
        } else {
            ErrorKind::Io
        };
        BackendError::at_path(kind, "cannot securely open input file", path)
    })?;
    let stat = fstat(&fd).map_err(|error| io_error("cannot inspect input file", path, error))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "input is not a regular file",
            path,
        ));
    }
    Ok(BoundInput {
        path: directory.path.join(&name),
        name,
        fd,
        identity: identity(&stat),
    })
}

fn verify_input(directory: &BoundDirectory, input: &BoundInput) -> Result<()> {
    let opened = fstat(&input.fd).map_err(|error| io_error("cannot revalidate opened input", &input.path, error))?;
    let attached = statat(&directory.fd, &input.name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|error| io_error("cannot revalidate input path", &input.path, error))?;
    if FileType::from_raw_mode(attached.st_mode) == FileType::RegularFile
        && identity(&opened) == input.identity
        && identity(&attached) == input.identity
    {
        Ok(())
    } else {
        Err(BackendError::at_path(
            ErrorKind::Conflict,
            "input file changed during processing",
            &input.path,
        ))
    }
}

fn verify_directory(directory: &BoundDirectory) -> Result<()> {
    let opened = fstat(&directory.fd)
        .map_err(|error| io_error("cannot revalidate output directory descriptor", &directory.path, error))?;
    let attached = open_directory(&directory.path)?;
    let attached = fstat(&attached)
        .map_err(|error| io_error("cannot revalidate output directory path", &directory.path, error))?;
    if identity(&opened) == directory.identity && identity(&attached) == directory.identity {
        Ok(())
    } else {
        Err(BackendError::at_path(
            ErrorKind::Conflict,
            "output directory changed during processing",
            &directory.path,
        ))
    }
}

fn copy_bound(input: &BoundInput, temporary: &mut Temporary, cancellation: &CancellationToken) -> Result<()> {
    let mut source = File::from(
        input
            .fd
            .try_clone()
            .map_err(|error| BackendError::from_io("cannot clone input descriptor", &input.path, error))?,
    );
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        cancellation.check()?;
        let read = source
            .read(&mut buffer)
            .map_err(|error| BackendError::from_io("cannot read image", &input.path, error))?;
        if read == 0 {
            break;
        }
        temporary
            .file
            .write_all(&buffer[..read])
            .map_err(|error| BackendError::from_io("cannot copy image", PathBuf::from(&temporary.name), error))?;
    }
    temporary
        .file
        .sync_all()
        .map_err(|error| BackendError::from_io("cannot sync copied image", PathBuf::from(&temporary.name), error))
}

fn create_temporary(directory: &BoundDirectory, stem: &OsStr, extension: &OsStr) -> Result<Temporary> {
    for index in 0..1_000_u16 {
        let name = OsString::from(format!(
            ".{}.img-splt-{index}.{}",
            stem.to_string_lossy(),
            extension.to_string_lossy()
        ));
        match openat(
            &directory.fd,
            &name,
            OFlags::WRONLY
                .union(OFlags::CREATE)
                .union(OFlags::EXCL)
                .union(OFlags::NOFOLLOW),
            Mode::RUSR | Mode::WUSR,
        ) {
            Ok(fd) => {
                let stat =
                    fstat(&fd).map_err(|error| io_error("cannot inspect temporary output", &directory.path, error))?;
                return Ok(Temporary {
                    name,
                    file: File::from(fd),
                    identity: identity(&stat),
                });
            }
            Err(error) if error == rustix::io::Errno::EXIST => continue,
            Err(error) => {
                return Err(io_error(
                    "cannot create temporary output",
                    directory.path.join(&name),
                    error,
                ));
            }
        }
    }
    Err(BackendError::new(
        ErrorKind::Conflict,
        "cannot allocate temporary output path",
    ))
}

fn temporary_is_non_empty(directory: &BoundDirectory, temporary: &Temporary) -> Result<bool> {
    let opened = fstat(&temporary.file)
        .map_err(|error| io_error("cannot inspect temporary output descriptor", &directory.path, error))?;
    let attached = statat(&directory.fd, &temporary.name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|error| io_error("cannot inspect temporary output path", &directory.path, error))?;
    Ok(identity(&opened) == temporary.identity
        && identity(&attached) == temporary.identity
        && FileType::from_raw_mode(attached.st_mode) == FileType::RegularFile
        && attached.st_size > 0)
}

fn commit(directory: &BoundDirectory, temporary: &OsStr, output: &OsStr, output_path: &Path) -> Result<()> {
    renameat_with(&directory.fd, temporary, &directory.fd, output, RenameFlags::NOREPLACE).map_err(|error| {
        io_error(
            "cannot finalize processed image without replacing an existing file",
            output_path,
            error,
        )
    })
}

fn remove_temporary(directory: &BoundDirectory, name: &OsStr) -> Result<()> {
    match unlinkat(&directory.fd, name, AtFlags::empty()) {
        Ok(()) => Ok(()),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(()),
        Err(error) => Err(io_error(
            "cannot remove temporary output",
            directory.path.join(name),
            error,
        )),
    }
}

fn entry_exists(directory: &BoundDirectory, name: &OsStr) -> Result<bool> {
    match statat(&directory.fd, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(_) => Ok(true),
        Err(error) if error == rustix::io::Errno::NOENT => Ok(false),
        Err(error) => Err(io_error(
            "cannot inspect output entry",
            directory.path.join(name),
            error,
        )),
    }
}

fn final_output_name(input: &OsStr, output_format: Option<&str>) -> Result<OsString> {
    let input = Path::new(input);
    let extension = output_format
        .map(OsStr::new)
        .or_else(|| input.extension())
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "input has no extension", input))?;
    let stem = input
        .file_stem()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "input has no file stem", input))?;
    Ok(PathBuf::from(stem).with_extension(extension).into_os_string())
}

fn normalize_format(format: Option<&str>) -> Result<Option<String>> {
    let Some(format) = format else {
        return Ok(None);
    };
    let format = format.trim().trim_start_matches('.').to_ascii_lowercase();
    if OUTPUT_FORMATS.contains(&format.as_str()) {
        Ok(Some(format))
    } else {
        Err(BackendError::new(
            ErrorKind::InvalidArgument,
            "unsupported output format",
        ))
    }
}

fn open_directory(path: &Path) -> Result<OwnedFd> {
    let mut current = open(Path::new("/"), OPEN_DIR, Mode::empty())
        .map_err(|error| io_error("cannot open filesystem root", path, error))?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                current = openat(&current, name, OPEN_DIR, Mode::empty())
                    .map_err(|error| io_error("cannot securely open directory", path, error))?;
            }
            _ => {
                return Err(BackendError::at_path(
                    ErrorKind::InvalidArgument,
                    "directory path must be absolute and normalized",
                    path,
                ));
            }
        }
    }
    Ok(current)
}

fn fd_path(fd: &OwnedFd) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", fd.as_raw_fd()))
}

fn fd_entry_path(fd: &OwnedFd, name: &OsStr) -> PathBuf {
    fd_path(fd).join(name)
}

fn identity(stat: &rustix::fs::Stat) -> Identity {
    Identity {
        dev: stat.st_dev,
        ino: stat.st_ino,
    }
}

fn io_error(message: &'static str, path: impl Into<PathBuf>, error: rustix::io::Errno) -> BackendError {
    BackendError::from_io(message, path, io::Error::from_raw_os_error(error.raw_os_error()))
}
