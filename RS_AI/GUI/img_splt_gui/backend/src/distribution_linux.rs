use crate::distribution::{
    DistributionMode, DistributionOptions, DistributionProgress, DistributionReport, distribution_shape, folder_name,
};
use crate::path_security::file_name;
use crate::{BackendError, CancellationToken, ErrorKind, Result, RollbackFailure};
use rustix::fd::{AsFd, OwnedFd};
use rustix::fs::{
    AtFlags, Dir, FileType, Mode, OFlags, RenameFlags, fstat, mkdirat, open, openat, renameat_with, statat, unlinkat,
};
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

const OPEN_DIR: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);
const OPEN_SOURCE: OFlags = OFlags::RDONLY.union(OFlags::NOFOLLOW).union(OFlags::CLOEXEC);

#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
}

struct Folder {
    name: OsString,
    path: PathBuf,
    fd: OwnedFd,
    identity: Identity,
    created: bool,
}

struct PlannedMove {
    source_name: OsString,
    source_fd: OwnedFd,
    source_identity: Identity,
    folder: usize,
}

struct CompletedMove {
    source_name: OsString,
    destination_name: OsString,
    folder: usize,
}

pub(crate) fn distribute_validated(
    options: &DistributionOptions,
    input_directory: &Path,
    files: Vec<PathBuf>,
    output_directory: &Path,
    cancellation: &CancellationToken,
    progress: &mut impl FnMut(DistributionProgress),
) -> Result<DistributionReport> {
    distribute_validated_with(
        options,
        input_directory,
        files,
        output_directory,
        cancellation,
        progress,
        &mut validate_distributed_source,
    )
}

fn distribute_validated_with(
    options: &DistributionOptions,
    input_directory: &Path,
    files: Vec<PathBuf>,
    output_directory: &Path,
    cancellation: &CancellationToken,
    progress: &mut impl FnMut(DistributionProgress),
    destination_validator: &mut impl FnMut(&Folder, &OsStr, Identity, &Path) -> Result<()>,
) -> Result<DistributionReport> {
    let input_fd = open_directory(input_directory)?;
    let output_fd = open_directory(output_directory)?;
    let total = files.len() as u64;
    let (folder_count, files_per_folder) = distribution_shape(total, options)?;
    let chapter = options.chapter.filter(|chapter| *chapter != 0);
    let next = chapter
        .map(|chapter| next_subchapter(&output_fd, output_directory, chapter))
        .transpose()?;

    let mut folder_names = Vec::new();
    let mut moves = Vec::with_capacity(files.len());
    for (index, source) in files.iter().enumerate() {
        let folder_index = match options.mode {
            DistributionMode::Fixed => ((index as u128 * folder_count as u128) / total as u128) as u64,
            DistributionMode::Balanced | DistributionMode::Greedy => index as u64 / files_per_folder,
        }
        .min(folder_count.saturating_sub(1));
        let name = OsString::from(folder_name(chapter, next, folder_count, folder_index));
        let folder = if let Some(position) = folder_names.iter().position(|existing| existing == &name) {
            position
        } else {
            folder_names.push(name);
            folder_names.len() - 1
        };
        let source_name = file_name(source)?.to_owned();
        let source_fd = openat(&input_fd, &source_name, OPEN_SOURCE, Mode::empty())
            .map_err(|error| io_error("cannot securely bind distribution source", source, error))?;
        let source_stat =
            fstat(&source_fd).map_err(|error| io_error("cannot inspect bound distribution source", source, error))?;
        if FileType::from_raw_mode(source_stat.st_mode) != FileType::RegularFile {
            return Err(BackendError::at_path(
                ErrorKind::Conflict,
                "distribution source is not a regular file",
                input_directory.join(&source_name),
            ));
        }
        moves.push(PlannedMove {
            source_name,
            source_fd,
            source_identity: identity(&source_stat),
            folder,
        });
    }

    cancellation.check()?;
    let mut folders = Vec::with_capacity(folder_names.len());
    for name in folder_names {
        match open_or_create_folder(&output_fd, output_directory, name) {
            Ok(folder) => folders.push(folder),
            Err(error) => {
                rollback_created_folders(&output_fd, &folders);
                return Err(error);
            }
        }
    }

    let mut completed = Vec::new();
    for (index, planned) in moves.into_iter().enumerate() {
        let operation: Result<()> = (|| {
            cancellation.check()?;
            verify_attached(&output_fd, &folders[planned.folder])?;
            let destination_name = move_no_replace(
                &input_fd,
                &planned.source_name,
                &planned.source_fd,
                planned.source_identity,
                &folders[planned.folder],
                output_directory,
            )?;
            completed.push(CompletedMove {
                source_name: planned.source_name.clone(),
                destination_name: destination_name.clone(),
                folder: planned.folder,
            });
            destination_validator(
                &folders[planned.folder],
                &destination_name,
                planned.source_identity,
                Path::new(&planned.source_name),
            )?;
            Ok(())
        })();
        if let Err(error) = operation {
            let rollback_failures = rollback_moves(&input_fd, input_directory, &folders, &completed);
            rollback_created_folders(&output_fd, &folders);
            return Err(error.with_partial_rollback(rollback_failures));
        }
        progress(DistributionProgress {
            completed: index as u64 + 1,
            total,
        });
    }

    Ok(DistributionReport {
        output_directory: output_directory.to_owned(),
        folders: folders.into_iter().map(|folder| folder.path).collect(),
        distributed: total,
    })
}

fn open_or_create_folder(root: &OwnedFd, root_path: &Path, name: OsString) -> Result<Folder> {
    let created = match mkdirat(root, &name, Mode::RWXU) {
        Ok(()) => true,
        Err(error) if error == rustix::io::Errno::EXIST => false,
        Err(error) => {
            return Err(io_error(
                "cannot create distribution folder",
                root_path.join(&name),
                error,
            ));
        }
    };
    let before = stat_no_follow(root, &name)
        .map_err(|error| BackendError::from_io("cannot inspect distribution folder", root_path.join(&name), error))?;
    if FileType::from_raw_mode(before.st_mode) != FileType::Directory {
        return Err(BackendError::at_path(
            ErrorKind::Conflict,
            "distribution folder is not a safe directory",
            root_path.join(&name),
        ));
    }
    let fd = openat(root, &name, OPEN_DIR, Mode::empty())
        .map_err(|error| io_error("cannot securely open distribution folder", root_path.join(&name), error))?;
    let opened = fstat(&fd).map_err(|error| io_error("cannot inspect opened distribution folder", root_path, error))?;
    let opened_identity = identity(&opened);
    if opened_identity != identity(&before) {
        return Err(BackendError::at_path(
            ErrorKind::Conflict,
            "distribution folder changed while opening",
            root_path.join(&name),
        ));
    }
    Ok(Folder {
        path: root_path.join(&name),
        name,
        fd,
        identity: opened_identity,
        created,
    })
}

fn move_no_replace(
    input: &OwnedFd,
    source_name: &OsStr,
    source_fd: &OwnedFd,
    expected: Identity,
    folder: &Folder,
    output_directory: &Path,
) -> Result<OsString> {
    let source_path = Path::new(source_name);
    let stem = source_path
        .file_stem()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "input has no file stem", source_path))?;
    let extension = source_path.extension();
    for counter in 0..=999_999_u64 {
        let destination = if counter == 0 {
            source_name.to_owned()
        } else {
            let mut candidate = PathBuf::from(format!("{}_{counter:03}", stem.to_string_lossy()));
            if let Some(extension) = extension {
                candidate.set_extension(extension);
            }
            candidate.into_os_string()
        };
        let opened = fstat(source_fd)
            .map_err(|error| io_error("cannot revalidate bound distribution source", source_path, error))?;
        let current = stat_no_follow(input, source_name)
            .map_err(|error| BackendError::from_io("cannot revalidate distribution source", source_path, error))?;
        if FileType::from_raw_mode(current.st_mode) != FileType::RegularFile
            || identity(&opened) != expected
            || identity(&current) != expected
        {
            return Err(BackendError::at_path(
                ErrorKind::Conflict,
                "distribution source changed before commit",
                source_path,
            ));
        }
        match renameat_with(input, source_name, &folder.fd, &destination, RenameFlags::NOREPLACE) {
            Ok(()) => return Ok(destination),
            Err(error) if error == rustix::io::Errno::EXIST => continue,
            Err(error) => {
                return Err(io_error(
                    "cannot securely distribute image",
                    output_directory.join(&folder.name).join(&destination),
                    error,
                ));
            }
        }
    }
    Err(BackendError::at_path(
        ErrorKind::Conflict,
        "cannot allocate collision-free output name",
        output_directory.join(&folder.name).join(source_name),
    ))
}

fn validate_distributed_source(
    folder: &Folder,
    destination: &OsStr,
    expected: Identity,
    source_path: &Path,
) -> Result<()> {
    let moved = stat_no_follow(&folder.fd, destination)
        .map_err(|error| BackendError::from_io("cannot verify distributed source identity", source_path, error))?;
    if FileType::from_raw_mode(moved.st_mode) == FileType::RegularFile && identity(&moved) == expected {
        Ok(())
    } else {
        Err(BackendError::at_path(
            ErrorKind::Conflict,
            "distribution source changed during commit",
            source_path,
        ))
    }
}

fn verify_attached(root: &OwnedFd, folder: &Folder) -> Result<()> {
    let current = stat_no_follow(root, &folder.name)
        .map_err(|error| BackendError::from_io("cannot revalidate distribution folder", &folder.path, error))?;
    if FileType::from_raw_mode(current.st_mode) == FileType::Directory && identity(&current) == folder.identity {
        Ok(())
    } else {
        Err(BackendError::at_path(
            ErrorKind::Conflict,
            "distribution folder changed before commit",
            &folder.path,
        ))
    }
}

fn rollback_moves(
    input: &OwnedFd,
    input_directory: &Path,
    folders: &[Folder],
    completed: &[CompletedMove],
) -> Vec<RollbackFailure> {
    let mut failures = Vec::new();
    for moved in completed.iter().rev() {
        if let Err(error) = renameat_with(
            &folders[moved.folder].fd,
            &moved.destination_name,
            input,
            &moved.source_name,
            RenameFlags::NOREPLACE,
        ) {
            failures.push(RollbackFailure::new(
                folders[moved.folder].path.join(&moved.destination_name),
                input_directory.join(&moved.source_name),
                errno(error),
            ));
        }
    }
    failures
}

fn rollback_created_folders(root: &OwnedFd, folders: &[Folder]) {
    for folder in folders.iter().rev().filter(|folder| folder.created) {
        if stat_no_follow(root, &folder.name).is_ok_and(|current| identity(&current) == folder.identity) {
            let _ = unlinkat(root, &folder.name, AtFlags::REMOVEDIR);
        }
    }
}

fn next_subchapter(root: &OwnedFd, root_path: &Path, chapter: u32) -> Result<u64> {
    let prefix = format!("Chapter {chapter}.");
    let cloned = root
        .as_fd()
        .try_clone_to_owned()
        .map_err(|error| BackendError::from_io("cannot clone output directory descriptor", root_path, error))?;
    let mut directory =
        Dir::read_from(cloned).map_err(|error| io_error("cannot inspect output directory", root_path, error))?;
    let mut maximum = 0_u64;
    while let Some(entry) = directory.read() {
        let entry = entry.map_err(|error| io_error("cannot read output directory", root_path, error))?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = OsStr::from_bytes(bytes);
        let is_directory =
            stat_no_follow(root, name).is_ok_and(|stat| FileType::from_raw_mode(stat.st_mode) == FileType::Directory);
        if is_directory {
            let name = name.to_string_lossy();
            if let Some(number) = name.strip_prefix(&prefix).and_then(|value| value.parse::<u64>().ok()) {
                maximum = maximum.max(number);
            }
        }
    }
    Ok(maximum + 1)
}

fn open_directory(path: &Path) -> Result<OwnedFd> {
    let mut current = open(Path::new("/"), OPEN_DIR, Mode::empty())
        .map_err(|error| io_error("cannot open filesystem root", path, error))?;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                let stat = stat_no_follow(&current, name)
                    .map_err(|error| BackendError::from_io("cannot inspect directory component", path, error))?;
                if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
                    return Err(BackendError::at_path(
                        ErrorKind::Conflict,
                        "directory path contains a symlink or non-directory component",
                        path,
                    ));
                }
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

fn stat_no_follow<Fd: AsFd>(parent: Fd, name: &OsStr) -> io::Result<rustix::fs::Stat> {
    statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(errno)
}

fn identity(stat: &rustix::fs::Stat) -> Identity {
    Identity {
        dev: stat.st_dev,
        ino: stat.st_ino,
    }
}

fn io_error(message: &'static str, path: impl Into<PathBuf>, error: rustix::io::Errno) -> BackendError {
    BackendError::from_io(message, path, errno(error))
}

fn errno(error: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(error.raw_os_error())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distribution::DistributionMode;
    use std::error::Error;
    use std::fs;
    use tempfile::TempDir;

    fn options(input: &Path, source: PathBuf, output: &Path) -> DistributionOptions {
        DistributionOptions {
            input_directory: input.to_owned(),
            files: vec![source],
            output_directory: output.to_owned(),
            chapter: None,
            mode: DistributionMode::Greedy,
            max_files_per_folder: 1,
            fixed_folder_count: 1,
        }
    }

    fn injected_validation_error(source_path: &Path) -> BackendError {
        BackendError::from_io(
            "cannot verify distributed source identity",
            source_path,
            io::Error::new(io::ErrorKind::PermissionDenied, "injected destination stat failure"),
        )
    }

    #[test]
    fn current_move_is_rolled_back_when_destination_validation_fails() {
        let input = TempDir::new().expect("create input directory");
        let output = TempDir::new().expect("create output directory");
        let source = input.path().join("page.png");
        fs::write(&source, b"fixture").expect("write source");
        let options = options(input.path(), source.clone(), output.path());

        let error = distribute_validated_with(
            &options,
            input.path(),
            vec![source.clone()],
            output.path(),
            &CancellationToken::new(),
            &mut |_| {},
            &mut |_, _, _, source_path| Err(injected_validation_error(source_path)),
        )
        .expect_err("destination validation failure must abort distribution");

        assert_eq!(error.kind(), ErrorKind::Io);
        assert_eq!(error.path(), Some(Path::new("page.png")));
        assert!(error.partial_rollback().is_none());
        assert_eq!(
            error
                .source()
                .and_then(|cause| cause.downcast_ref::<io::Error>())
                .map(io::Error::kind),
            Some(io::ErrorKind::PermissionDenied)
        );
        assert_eq!(fs::read(source).expect("read restored source"), b"fixture");
        assert!(output.path().read_dir().expect("read output").next().is_none());
    }

    #[test]
    fn current_move_rollback_failure_is_typed_without_replacing_validation_cause() {
        let input = TempDir::new().expect("create input directory");
        let output = TempDir::new().expect("create output directory");
        let source = input.path().join("page.png");
        fs::write(&source, b"fixture").expect("write source");
        let options = options(input.path(), source.clone(), output.path());
        let occupied_source = source.clone();

        let error = distribute_validated_with(
            &options,
            input.path(),
            vec![source.clone()],
            output.path(),
            &CancellationToken::new(),
            &mut |_| {},
            &mut |_, _, _, source_path| {
                fs::write(&occupied_source, b"occupied").expect("occupy source before rollback");
                Err(injected_validation_error(source_path))
            },
        )
        .expect_err("occupied source must make current rollback partial");

        let rollback = error.partial_rollback().expect("typed partial rollback");
        assert_eq!(error.kind(), ErrorKind::Io);
        assert_eq!(error.path(), Some(Path::new("page.png")));
        assert_eq!(
            error
                .source()
                .and_then(|cause| cause.downcast_ref::<io::Error>())
                .map(io::Error::kind),
            Some(io::ErrorKind::PermissionDenied)
        );
        assert_eq!(rollback.failures().len(), 1);
        assert_eq!(
            rollback.failures()[0].from_path(),
            output.path().join("Oneshot/page.png")
        );
        assert_eq!(rollback.failures()[0].to_path(), source);
        assert_eq!(rollback.failures()[0].io_error().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&occupied_source).expect("read occupied source"), b"occupied");
        assert_eq!(
            fs::read(output.path().join("Oneshot/page.png")).expect("read unrestored move"),
            b"fixture"
        );
    }
}
