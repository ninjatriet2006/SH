use crate::{BackendError, CancellationToken, ErrorKind, ImageSettings, Result};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) const OUTPUT_FORMATS: &[&str] = &["jpg", "jpeg", "png", "webp", "avif", "heic", "bmp", "tiff"];

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub input_directory: PathBuf,
    pub files: Vec<PathBuf>,
    pub output_directory: PathBuf,
    pub output_format: Option<String>,
    pub upscale: bool,
    pub settings: ImageSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReport {
    pub output_directory: PathBuf,
    pub processed: Vec<PathBuf>,
    pub failed: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessProgress {
    pub completed: u64,
    pub total: u64,
}

pub trait MediaToolRunner: Sync {
    fn probe_width(&self, input: &Path) -> Result<u32>;
    fn convert(&self, input: &Path, output: &Path, upscale_width: Option<u32>) -> Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemMediaToolRunner;

impl MediaToolRunner for SystemMediaToolRunner {
    fn probe_width(&self, input: &Path) -> Result<u32> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width",
                "-of",
                "csv=s=x:p=0",
            ])
            .arg(input)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| tool_start_error("ffprobe", error))?;
        if !output.status.success() {
            return Err(BackendError::at_path(ErrorKind::Io, "ffprobe failed", input));
        }
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .map_err(|_| BackendError::at_path(ErrorKind::Io, "ffprobe returned an invalid width", input))
    }

    fn convert(&self, input: &Path, output: &Path, upscale_width: Option<u32>) -> Result<()> {
        let mut command = Command::new("ffmpeg");
        command.arg("-nostdin").arg("-y").arg("-i").arg(input);
        if let Some(width) = upscale_width {
            command.arg("-vf").arg(format!("scale={width}:-1"));
        }
        let status = command
            .arg(output)
            .args(["-loglevel", "quiet"])
            .stdin(Stdio::null())
            .status()
            .map_err(|error| tool_start_error("ffmpeg", error))?;
        if status.success() {
            Ok(())
        } else {
            Err(BackendError::at_path(ErrorKind::Io, "ffmpeg failed", input))
        }
    }
}

pub fn process_images(
    options: &ProcessOptions,
    runner: &dyn MediaToolRunner,
    cancellation: &CancellationToken,
    mut progress: impl FnMut(ProcessProgress),
) -> Result<ProcessReport> {
    #[cfg(target_os = "linux")]
    {
        crate::processing_linux::process_validated(options, runner, cancellation, &mut progress)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (options, runner, cancellation, &mut progress);
        Err(BackendError::new(
            ErrorKind::Unavailable,
            "secure image processing is unavailable on this platform",
        ))
    }
}

fn tool_start_error(tool: &str, error: io::Error) -> BackendError {
    let kind = if error.kind() == io::ErrorKind::NotFound {
        ErrorKind::Unavailable
    } else {
        ErrorKind::Io
    };
    BackendError::new(kind, format!("cannot start {tool}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct FakeRunner;

    impl MediaToolRunner for FakeRunner {
        fn probe_width(&self, _input: &Path) -> Result<u32> {
            Ok(320)
        }

        fn convert(&self, input: &Path, output: &Path, upscale_width: Option<u32>) -> Result<()> {
            assert_eq!(upscale_width, Some(1280));
            fs::copy(input, output)
                .map(|_| ())
                .map_err(|error| BackendError::from_io("fake conversion failed", output, error))
        }
    }

    fn test_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("img-splt-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).expect("create test directory");
        path
    }

    #[test]
    fn processes_to_explicit_output_without_changing_source() {
        let input = test_dir("process-input");
        let output = test_dir("process-output");
        let source = input.join("page1.png");
        fs::write(&source, b"image fixture").expect("write fixture");
        let options = ProcessOptions {
            input_directory: input,
            files: vec![source.clone()],
            output_directory: output.clone(),
            output_format: Some("webp".to_owned()),
            upscale: true,
            settings: ImageSettings::default(),
        };
        let report = process_images(&options, &FakeRunner, &CancellationToken::new(), |_| {}).expect("process");
        assert_eq!(report.processed, [output.join("page1.webp")]);
        assert!(source.exists());
    }

    #[test]
    fn rejects_file_outside_declared_input_root() {
        let input = test_dir("contained-input");
        let outside = test_dir("outside-input");
        let output = test_dir("contained-output");
        let source = outside.join("page.png");
        fs::write(&source, b"fixture").expect("write fixture");
        let options = ProcessOptions {
            input_directory: input,
            files: vec![source],
            output_directory: output,
            output_format: None,
            upscale: false,
            settings: ImageSettings::default(),
        };
        assert_eq!(
            process_images(&options, &FakeRunner, &CancellationToken::new(), |_| {})
                .expect_err("outside root")
                .kind(),
            ErrorKind::InvalidArgument
        );
    }
}
