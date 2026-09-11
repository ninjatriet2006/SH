use crate::path_security::{CanonicalRoots, outside_roots};
use crate::{CancellationToken, FileType, classify_file};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static STAGING_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchConvertRequest {
    pub files: Vec<PathBuf>,
    pub output_directory: PathBuf,
    pub output_format: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchConvertReport {
    pub output_directory: PathBuf,
    pub converted: Vec<PathBuf>,
    pub failed: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionProgress {
    pub completed: u64,
    pub total: u64,
    pub current_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ConversionRoots {
    inputs: CanonicalRoots,
    outputs: CanonicalRoots,
}

impl ConversionRoots {
    pub fn new(input_roots: Vec<PathBuf>, output_roots: Vec<PathBuf>) -> io::Result<Self> {
        Ok(Self {
            inputs: CanonicalRoots::new(input_roots, "input")?,
            outputs: CanonicalRoots::new(output_roots, "output")?,
        })
    }
}

/// Validates every batch field and filesystem boundary without starting tools or
/// creating output. Bridges can call this before reserving a request ID.
pub fn preflight_batch_conversion(request: &BatchConvertRequest, roots: &ConversionRoots) -> io::Result<()> {
    validate_request(request)?;
    let output_directory = roots.outputs.resolve_existing(&request.output_directory)?;
    if !output_directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output path must be a directory",
        ));
    }
    normalize_format(&request.output_format)?;
    for input in &request.files {
        let input = roots.inputs.resolve_existing(input)?;
        recheck_input(&input, roots)?;
        if input.file_name().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "input must have a file name",
            ));
        }
    }
    Ok(())
}

pub trait ToolRunner {
    fn run(&self, program: &str, arguments: &[OsString], cancellation: &CancellationToken) -> io::Result<bool>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemToolRunner;

impl ToolRunner for SystemToolRunner {
    fn run(&self, program: &str, arguments: &[OsString], cancellation: &CancellationToken) -> io::Result<bool> {
        cancellation.check()?;
        let mut child = Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        loop {
            if cancellation.is_cancelled() {
                return cancel_child(&mut child);
            }
            if let Some(status) = child.try_wait()? {
                return Ok(status.success());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

fn cancel_child(child: &mut Child) -> io::Result<bool> {
    let _ = child.kill();
    let _ = child.wait();
    Err(io::Error::new(io::ErrorKind::Interrupted, "operation cancelled"))
}

#[derive(Debug, Clone)]
pub struct Converter<R = SystemToolRunner> {
    runner: R,
}

impl Default for Converter<SystemToolRunner> {
    fn default() -> Self {
        Self::new(SystemToolRunner)
    }
}

impl<R: ToolRunner> Converter<R> {
    pub const fn new(runner: R) -> Self {
        Self { runner }
    }

    pub fn convert_batch(
        &self,
        request: &BatchConvertRequest,
        roots: &ConversionRoots,
        cancellation: &CancellationToken,
        mut progress: impl FnMut(ConversionProgress),
    ) -> io::Result<BatchConvertReport> {
        validate_request(request)?;
        let output_directory = roots.outputs.resolve_existing(&request.output_directory)?;
        if !output_directory.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output path must be a directory",
            ));
        }
        let format = normalize_format(&request.output_format)?;
        let total = request.files.len() as u64;
        progress(ConversionProgress {
            completed: 0,
            total,
            current_file: None,
        });
        let mut report = BatchConvertReport {
            output_directory: output_directory.clone(),
            converted: Vec::new(),
            failed: Vec::new(),
        };

        for (index, requested_input) in request.files.iter().enumerate() {
            cancellation.check()?;
            let result = roots.inputs.resolve_existing(requested_input).and_then(|input| {
                self.convert_one(
                    &input,
                    &output_directory,
                    &format,
                    request.overwrite,
                    roots,
                    cancellation,
                )
            });
            match result {
                Ok(output) => report.converted.push(output),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
                Err(_) => report.failed.push(requested_input.clone()),
            }
            progress(ConversionProgress {
                completed: index as u64 + 1,
                total,
                current_file: Some(requested_input.clone()),
            });
        }
        Ok(report)
    }

    fn convert_one(
        &self,
        input: &Path,
        output_directory: &Path,
        format: &str,
        overwrite: bool,
        roots: &ConversionRoots,
        cancellation: &CancellationToken,
    ) -> io::Result<PathBuf> {
        recheck_input(input, roots)?;
        let classification = classify_file(input)?;
        let stem = input
            .file_stem()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "input must have a file name"))?;
        let mut output = output_directory.join(stem);
        output.set_extension(format);
        if input == output || (output.exists() && !overwrite) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "conversion output already exists",
            ));
        }

        let staging = create_staging_directory(output_directory)?;
        let result = (|| {
            let mut staged_output = staging.join("converted");
            staged_output.set_extension(format);
            recheck_input(input, roots)?;
            run_conversion(
                &self.runner,
                classification.file_type,
                input,
                &staged_output,
                &staging,
                format,
                cancellation,
            )?;
            cancellation.check()?;
            recheck_input(input, roots)?;
            publish_output(&staged_output, &output, output_directory, roots, overwrite)?;
            Ok(output)
        })();
        let _ = fs::remove_dir_all(&staging);
        result
    }
}

fn run_conversion(
    runner: &impl ToolRunner,
    file_type: FileType,
    input: &Path,
    staged_output: &Path,
    staging: &Path,
    format: &str,
    cancellation: &CancellationToken,
) -> io::Result<()> {
    let successful = match file_type {
        FileType::Video | FileType::Audio | FileType::Image => runner.run(
            "ffmpeg",
            &[
                "-n".into(),
                "-i".into(),
                input.as_os_str().to_owned(),
                staged_output.as_os_str().to_owned(),
            ],
            cancellation,
        )?,
        FileType::Document => runner.run(
            "soffice",
            &[
                "--headless".into(),
                "--convert-to".into(),
                format.into(),
                "--outdir".into(),
                staging.as_os_str().to_owned(),
                input.as_os_str().to_owned(),
            ],
            cancellation,
        )?,
        FileType::Directory => runner.run(
            "7z",
            &[
                "a".into(),
                "-y".into(),
                staged_output.as_os_str().to_owned(),
                input.as_os_str().to_owned(),
            ],
            cancellation,
        )?,
        FileType::Archive => {
            let extracted = staging.join("extracted");
            fs::create_dir(&extracted)?;
            let output_arg = prefixed_os_string("-o", &extracted);
            let extracted_ok = runner.run(
                "7z",
                &["x".into(), "-y".into(), output_arg, input.as_os_str().to_owned()],
                cancellation,
            )?;
            if !extracted_ok {
                return Err(io::Error::other("7z extraction failed"));
            }
            runner.run(
                "7z",
                &[
                    "a".into(),
                    "-y".into(),
                    staged_output.as_os_str().to_owned(),
                    extracted.as_os_str().to_owned(),
                ],
                cancellation,
            )?
        }
        FileType::Unknown => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsupported input type"));
        }
    };
    if !successful {
        return Err(io::Error::other("conversion tool failed"));
    }

    // LibreOffice chooses the input stem itself, unlike the other tools.
    if file_type == FileType::Document {
        let mut generated = staging.join(input.file_stem().unwrap_or_else(|| OsStr::new("converted")));
        generated.set_extension(format);
        if generated != staged_output {
            fs::rename(generated, staged_output)?;
        }
    }
    Ok(())
}

fn publish_output(
    staged: &Path,
    output: &Path,
    output_directory: &Path,
    roots: &ConversionRoots,
    overwrite: bool,
) -> io::Result<()> {
    let metadata = fs::symlink_metadata(staged)?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "conversion tool did not produce a regular file",
        ));
    }
    let canonical_parent = fs::canonicalize(output.parent().ok_or_else(outside_roots)?)?;
    if canonical_parent != output_directory || !roots.outputs.contains(&canonical_parent) {
        return Err(outside_roots());
    }
    if !overwrite {
        fs::hard_link(staged, output)?;
        fs::remove_file(staged)?;
    } else if let Ok(existing) = fs::symlink_metadata(output) {
        if existing.file_type().is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "refusing to overwrite a directory",
            ));
        }
        let backup = staged.with_extension("previous");
        fs::rename(output, &backup)?;
        if let Err(error) = fs::rename(staged, output) {
            let _ = fs::rename(backup, output);
            return Err(error);
        }
        let _ = fs::remove_file(backup);
    } else {
        fs::rename(staged, output)?;
    }
    let resolved = fs::canonicalize(output)?;
    if !roots.outputs.contains(&resolved) || fs::canonicalize(output_directory)? != output_directory {
        return Err(outside_roots());
    }
    Ok(())
}

fn recheck_input(input: &Path, roots: &ConversionRoots) -> io::Result<()> {
    let resolved = fs::canonicalize(input)?;
    if resolved == input && roots.inputs.contains(&resolved) {
        Ok(())
    } else {
        Err(outside_roots())
    }
}

fn create_staging_directory(output_directory: &Path) -> io::Result<PathBuf> {
    for _ in 0..16 {
        let id = STAGING_ID.fetch_add(1, Ordering::Relaxed);
        let path = output_directory.join(format!(".universal-converter-{}-{id}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique conversion staging directory",
    ))
}

fn prefixed_os_string(prefix: &str, path: &Path) -> OsString {
    let mut value = OsString::from(prefix);
    value.push(path);
    value
}

fn validate_request(request: &BatchConvertRequest) -> io::Result<()> {
    if request.files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "at least one input file is required",
        ));
    }
    if !request.output_directory.is_absolute() || request.files.iter().any(|path| !path.is_absolute()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "all input and output paths must be absolute",
        ));
    }
    Ok(())
}

fn normalize_format(format: &str) -> io::Result<String> {
    let format = format.trim().trim_start_matches('.').to_ascii_lowercase();
    if format.is_empty() || format.len() > 16 || !format.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output format must contain 1-16 ASCII letters or digits",
        ));
    }
    Ok(format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRunner {
        calls: Mutex<Vec<(String, Vec<OsString>)>>,
    }

    impl ToolRunner for FakeRunner {
        fn run(&self, program: &str, arguments: &[OsString], cancellation: &CancellationToken) -> io::Result<bool> {
            cancellation.check()?;
            self.calls
                .lock()
                .expect("test mutex must not be poisoned")
                .push((program.to_owned(), arguments.to_vec()));
            if program == "ffmpeg" {
                fs::write(arguments.last().expect("output argument"), b"converted")?;
            } else if program == "soffice" {
                let output_dir = PathBuf::from(&arguments[4]);
                let input = PathBuf::from(arguments.last().expect("input argument"));
                fs::write(
                    output_dir
                        .join(input.file_stem().expect("stem"))
                        .with_extension(&arguments[2]),
                    b"doc",
                )?;
            } else if program == "7z" && arguments.first() == Some(&OsString::from("a")) {
                fs::write(&arguments[2], b"archive")?;
            }
            Ok(true)
        }
    }

    struct TestTree {
        root: PathBuf,
        input: PathBuf,
        output: PathBuf,
    }

    impl TestTree {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "universal-converter-{name}-{}-{}",
                std::process::id(),
                STAGING_ID.fetch_add(1, Ordering::Relaxed)
            ));
            let input = root.join("input");
            let output = root.join("output");
            fs::create_dir_all(&input).expect("input directory");
            fs::create_dir_all(&output).expect("output directory");
            fs::write(input.join("movie.mp4"), b"video").expect("input fixture");
            Self { root, input, output }
        }

        fn roots(&self) -> ConversionRoots {
            ConversionRoots::new(vec![self.input.clone()], vec![self.output.clone()]).expect("valid roots")
        }

        fn request(&self) -> BatchConvertRequest {
            BatchConvertRequest {
                files: vec![self.input.join("movie.mp4")],
                output_directory: self.output.clone(),
                output_format: "mkv".to_owned(),
                overwrite: false,
            }
        }
    }

    impl Drop for TestTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn uses_safe_process_arguments_and_reports_progress() {
        let tree = TestTree::new("args");
        let converter = Converter::new(FakeRunner::default());
        let mut updates = Vec::new();
        let report = converter
            .convert_batch(&tree.request(), &tree.roots(), &CancellationToken::new(), |event| {
                updates.push(event)
            })
            .expect("conversion succeeds");
        assert_eq!(report.converted, [tree.output.join("movie.mkv")]);
        assert_eq!(updates.iter().map(|event| event.completed).collect::<Vec<_>>(), [0, 1]);
        let calls = converter.runner.calls.lock().expect("calls");
        assert_eq!(calls[0].0, "ffmpeg");
        assert_eq!(calls[0].1[0], "-n");
        assert!(!calls[0].1.iter().any(|argument| argument == "--"));
    }

    #[test]
    fn enforces_overwrite_policy_before_running_tool() {
        let tree = TestTree::new("overwrite");
        fs::write(tree.output.join("movie.mkv"), b"original").expect("existing output");
        let converter = Converter::new(FakeRunner::default());
        let report = converter
            .convert_batch(&tree.request(), &tree.roots(), &CancellationToken::new(), |_| {})
            .expect("batch completes with failed item");
        assert_eq!(report.failed, [tree.input.join("movie.mp4")]);
        assert!(converter.runner.calls.lock().expect("calls").is_empty());
        assert_eq!(fs::read(tree.output.join("movie.mkv")).expect("output"), b"original");

        let mut request = tree.request();
        request.overwrite = true;
        converter
            .convert_batch(&request, &tree.roots(), &CancellationToken::new(), |_| {})
            .expect("overwrite succeeds");
        assert_eq!(fs::read(tree.output.join("movie.mkv")).expect("output"), b"converted");
    }

    #[test]
    fn rejects_traversal_and_outside_root_inputs() {
        let tree = TestTree::new("outside");
        let outside = tree.root.join("outside.mp4");
        fs::write(&outside, b"outside").expect("outside fixture");
        let converter = Converter::new(FakeRunner::default());
        for path in [tree.input.join("../outside.mp4"), outside] {
            let mut request = tree.request();
            request.files = vec![path.clone()];
            let report = converter
                .convert_batch(&request, &tree.roots(), &CancellationToken::new(), |_| {})
                .expect("security failure is an item failure");
            assert_eq!(report.failed, [path]);
        }
        assert!(converter.runner.calls.lock().expect("calls").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape_for_input_and_output_directory() {
        use std::os::unix::fs::symlink;

        let tree = TestTree::new("symlink");
        let outside = tree.root.join("outside");
        fs::create_dir(&outside).expect("outside directory");
        fs::write(outside.join("movie.mp4"), b"outside").expect("outside input");
        symlink(outside.join("movie.mp4"), tree.input.join("linked.mp4")).expect("input symlink");
        symlink(&outside, tree.output.join("linked-output")).expect("output symlink");
        let converter = Converter::new(FakeRunner::default());

        let mut input_request = tree.request();
        input_request.files = vec![tree.input.join("linked.mp4")];
        assert_eq!(
            converter
                .convert_batch(&input_request, &tree.roots(), &CancellationToken::new(), |_| {})
                .expect("item failure")
                .failed,
            [tree.input.join("linked.mp4")]
        );

        let mut output_request = tree.request();
        output_request.output_directory = tree.output.join("linked-output");
        let error = converter
            .convert_batch(&output_request, &tree.roots(), &CancellationToken::new(), |_| {})
            .expect_err("output escape must fail command");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn cancellation_stops_before_process_launch() {
        let tree = TestTree::new("cancel");
        let converter = Converter::new(FakeRunner::default());
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let error = converter
            .convert_batch(&tree.request(), &tree.roots(), &cancellation, |_| {})
            .expect_err("cancelled operation must stop");
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert!(converter.runner.calls.lock().expect("calls").is_empty());
    }

    #[test]
    fn preflight_validates_all_paths_format_and_containment_without_running_tools() {
        let tree = TestTree::new("preflight");
        let roots = tree.roots();
        preflight_batch_conversion(&tree.request(), &roots).expect("valid preflight");

        let mut invalid_format = tree.request();
        invalid_format.output_format = "../mkv".to_owned();
        assert_eq!(
            preflight_batch_conversion(&invalid_format, &roots)
                .expect_err("format rejected")
                .kind(),
            io::ErrorKind::InvalidInput
        );

        let outside = tree.root.join("outside.mp4");
        fs::write(&outside, b"outside").expect("outside fixture");
        let mut outside_input = tree.request();
        outside_input.files = vec![outside];
        assert_eq!(
            preflight_batch_conversion(&outside_input, &roots)
                .expect_err("containment rejected")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn cancellation_after_child_exit_is_interrupted() {
        let mut child = Command::new(std::env::current_exe().expect("test executable"))
            .arg("--help")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("child process");
        child.wait().expect("child exits before cancellation");

        let error = cancel_child(&mut child).expect_err("cancellation must remain interrupted");
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    }

    #[test]
    fn archive_conversion_extracts_then_repackages_with_7z() {
        let tree = TestTree::new("7z");
        let archive = tree.input.join("bundle.zip");
        fs::write(&archive, b"archive").expect("archive fixture");
        let converter = Converter::new(FakeRunner::default());
        let request = BatchConvertRequest {
            files: vec![archive],
            output_directory: tree.output.clone(),
            output_format: "7z".to_owned(),
            overwrite: false,
        };
        let report = converter
            .convert_batch(&request, &tree.roots(), &CancellationToken::new(), |_| {})
            .expect("archive conversion");
        assert_eq!(report.converted, [tree.output.join("bundle.7z")]);
        let calls = converter.runner.calls.lock().expect("calls");
        assert_eq!(calls.len(), 2);
        assert_eq!((&calls[0].0, &calls[0].1[0]), (&"7z".to_owned(), &OsString::from("x")));
        assert_eq!((&calls[1].0, &calls[1].1[0]), (&"7z".to_owned(), &OsString::from("a")));
    }
}
