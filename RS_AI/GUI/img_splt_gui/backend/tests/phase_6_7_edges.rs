use img_splt_backend::{
    BackendError, CancellationToken, DistributionMode, DistributionOptions, ErrorKind, ImageSettings, MediaToolRunner,
    ProcessOptions, Result, distribute_images, process_images, scan_images,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

fn image_files(directory: &Path, count: usize) -> Vec<PathBuf> {
    (1..=count)
        .map(|index| {
            let path = directory.join(format!("page{index}.png"));
            fs::write(&path, format!("fixture-{index}")).expect("write image fixture");
            path
        })
        .collect()
}

fn process_options(input: &Path, files: Vec<PathBuf>, output: &Path, retries: u64) -> ProcessOptions {
    ProcessOptions {
        input_directory: input.to_owned(),
        files,
        output_directory: output.to_owned(),
        output_format: Some("webp".to_owned()),
        upscale: false,
        settings: ImageSettings {
            max_retries: retries,
            ..ImageSettings::default()
        },
    }
}

fn distribution_options(
    input: &Path,
    files: Vec<PathBuf>,
    output: &Path,
    mode: DistributionMode,
    max_files: u64,
    fixed_folders: u64,
) -> DistributionOptions {
    DistributionOptions {
        input_directory: input.to_owned(),
        files,
        output_directory: output.to_owned(),
        chapter: None,
        mode,
        max_files_per_folder: max_files,
        fixed_folder_count: fixed_folders,
    }
}

fn entry_names(directory: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(directory)
        .expect("read directory")
        .map(|entry| entry.expect("read entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

struct NeverRunner {
    calls: AtomicUsize,
}

impl MediaToolRunner for NeverRunner {
    fn probe_width(&self, _input: &Path) -> Result<u32> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(320)
    }

    fn convert(&self, _input: &Path, _output: &Path, _upscale_width: Option<u32>) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn scan_cancellation_stops_after_first_progress_event() {
    let directory = TempDir::new().expect("create scan directory");
    image_files(directory.path(), 3);
    let cancellation = CancellationToken::new();
    let mut progress_events = 0;

    let error = scan_images(directory.path(), &cancellation, |_| {
        progress_events += 1;
        cancellation.cancel();
    })
    .expect_err("scan should be cancelled");

    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert_eq!(progress_events, 1);
}

#[test]
fn process_pre_cancelled_does_not_call_runner_or_write_output() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let options = process_options(input.path(), files, output.path(), 2);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let runner = NeverRunner {
        calls: AtomicUsize::new(0),
    };

    let error = process_images(&options, &runner, &cancellation, |_| {}).expect_err("processing should be cancelled");

    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
    assert!(entry_names(output.path()).is_empty());
}

struct CancellingRunner {
    cancellation: CancellationToken,
}

impl MediaToolRunner for CancellingRunner {
    fn probe_width(&self, _input: &Path) -> Result<u32> {
        Ok(320)
    }

    fn convert(&self, _input: &Path, output: &Path, _upscale_width: Option<u32>) -> Result<()> {
        fs::write(output, b"converted").expect("write fake conversion");
        self.cancellation.cancel();
        Ok(())
    }
}

#[test]
fn process_mid_conversion_cancellation_removes_temporary_output() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let options = process_options(input.path(), files, output.path(), 2);
    let cancellation = CancellationToken::new();
    let runner = CancellingRunner {
        cancellation: cancellation.clone(),
    };

    let error = process_images(&options, &runner, &cancellation, |_| {}).expect_err("processing should be cancelled");

    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(entry_names(output.path()).is_empty());
}

#[test]
fn distribution_mid_operation_cancellation_rolls_back_moves_and_folders() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 3);
    let options = distribution_options(
        input.path(),
        files.clone(),
        output.path(),
        DistributionMode::Greedy,
        2,
        1,
    );
    let cancellation = CancellationToken::new();

    let error = distribute_images(&options, &cancellation, |progress| {
        if progress.completed == 1 {
            cancellation.cancel();
        }
    })
    .expect_err("distribution should be cancelled");

    assert_eq!(error.kind(), ErrorKind::Cancelled);
    assert!(files.iter().all(|file| file.exists()));
    assert!(entry_names(output.path()).is_empty());
}

#[cfg(unix)]
#[test]
fn process_rejects_input_symlink_escaping_selected_root() {
    use std::os::unix::fs::symlink;

    let input = TempDir::new().expect("create input directory");
    let outside = TempDir::new().expect("create outside directory");
    let output = TempDir::new().expect("create output directory");
    let outside_file = outside.path().join("secret.png");
    fs::write(&outside_file, b"outside").expect("write outside fixture");
    let escaped = input.path().join("escaped.png");
    symlink(&outside_file, &escaped).expect("create escaping symlink");
    let options = process_options(input.path(), vec![escaped], output.path(), 1);
    let runner = NeverRunner {
        calls: AtomicUsize::new(0),
    };

    let error = process_images(&options, &runner, &CancellationToken::new(), |_| {})
        .expect_err("escaping symlink should be rejected");

    assert_eq!(error.kind(), ErrorKind::InvalidArgument);
    assert_eq!(runner.calls.load(Ordering::SeqCst), 0);
    assert_eq!(fs::read(outside_file).expect("read outside fixture"), b"outside");
}

#[cfg(unix)]
#[test]
fn distribution_rejects_destination_symlink_escape() {
    use std::os::unix::fs::symlink;

    let input = TempDir::new().expect("create input directory");
    let outside = TempDir::new().expect("create outside directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    symlink(outside.path(), output.path().join("Oneshot")).expect("create destination symlink");
    let options = distribution_options(
        input.path(),
        files.clone(),
        output.path(),
        DistributionMode::Greedy,
        2,
        1,
    );

    let error = distribute_images(&options, &CancellationToken::new(), |_| {})
        .expect_err("destination symlink should be rejected");

    assert_eq!(error.kind(), ErrorKind::Conflict);
    assert!(files[0].exists());
    assert!(entry_names(outside.path()).is_empty());
}

#[test]
fn greedy_distribution_fills_each_folder_before_next() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 5);
    let options = distribution_options(input.path(), files, output.path(), DistributionMode::Greedy, 2, 1);

    let report = distribute_images(&options, &CancellationToken::new(), |_| {}).expect("distribute greedily");

    assert_eq!(report.folders.len(), 3);
    assert_eq!(entry_names(&report.folders[0]).len(), 2);
    assert_eq!(entry_names(&report.folders[1]).len(), 2);
    assert_eq!(entry_names(&report.folders[2]).len(), 1);
}

#[test]
fn fixed_distribution_uses_requested_folder_count_for_non_divisible_total() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 5);
    let options = distribution_options(input.path(), files, output.path(), DistributionMode::Fixed, 80, 4);

    let report = distribute_images(&options, &CancellationToken::new(), |_| {}).expect("fixed distribution");
    let folder_sizes: Vec<_> = report.folders.iter().map(|folder| entry_names(folder).len()).collect();

    assert_eq!(report.folders.len(), 4);
    assert_eq!(folder_sizes, [2, 1, 1, 1]);
}

#[test]
fn distribution_preserves_collision_and_uses_numbered_suffix() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let folder = output.path().join("Oneshot");
    fs::create_dir(&folder).expect("create existing folder");
    fs::write(folder.join("page1.png"), b"existing").expect("write collision fixture");
    let options = distribution_options(input.path(), files, output.path(), DistributionMode::Greedy, 80, 1);

    distribute_images(&options, &CancellationToken::new(), |_| {}).expect("distribute collision");

    assert_eq!(fs::read(folder.join("page1.png")).expect("read original"), b"existing");
    assert_eq!(
        fs::read(folder.join("page1_001.png")).expect("read moved"),
        b"fixture-1"
    );
}

struct RetryRunner {
    calls: AtomicUsize,
    failures_before_success: usize,
}

impl MediaToolRunner for RetryRunner {
    fn probe_width(&self, _input: &Path) -> Result<u32> {
        Ok(320)
    }

    fn convert(&self, _input: &Path, output: &Path, _upscale_width: Option<u32>) -> Result<()> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call < self.failures_before_success {
            Err(BackendError::new(ErrorKind::Io, "injected conversion failure"))
        } else {
            fs::write(output, b"converted").map_err(|error| BackendError::new(ErrorKind::Io, error.to_string()))
        }
    }
}

#[test]
fn processing_retries_failures_then_succeeds() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let options = process_options(input.path(), files, output.path(), 3);
    let runner = RetryRunner {
        calls: AtomicUsize::new(0),
        failures_before_success: 2,
    };

    let report = process_images(&options, &runner, &CancellationToken::new(), |_| {}).expect("retry processing");

    assert_eq!(runner.calls.load(Ordering::SeqCst), 3);
    assert_eq!(report.processed, [output.path().join("page1.webp")]);
    assert!(report.failed.is_empty());
}

#[test]
fn processing_exhausted_failures_are_reported_without_output() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let options = process_options(input.path(), files.clone(), output.path(), 2);
    let runner = RetryRunner {
        calls: AtomicUsize::new(0),
        failures_before_success: usize::MAX,
    };

    let report = process_images(&options, &runner, &CancellationToken::new(), |_| {}).expect("failure report");

    assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
    assert!(report.processed.is_empty());
    assert_eq!(report.failed, files);
    assert!(entry_names(output.path()).is_empty());
}

struct EmptyOutputRunner {
    calls: AtomicUsize,
}

struct RacingOutputRunner {
    final_output: PathBuf,
}

impl MediaToolRunner for RacingOutputRunner {
    fn probe_width(&self, _input: &Path) -> Result<u32> {
        Ok(320)
    }

    fn convert(&self, _input: &Path, output: &Path, _upscale_width: Option<u32>) -> Result<()> {
        fs::write(output, b"converted").expect("write converted temporary output");
        fs::write(&self.final_output, b"attacker-won-race").expect("inject final-output collision");
        Ok(())
    }
}

#[test]
fn processing_never_clobbers_output_created_after_preflight() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let final_output = output.path().join("page1.webp");
    let options = process_options(input.path(), files, output.path(), 1);
    let runner = RacingOutputRunner {
        final_output: final_output.clone(),
    };

    let error = process_images(&options, &runner, &CancellationToken::new(), |_| {})
        .expect_err("racing output must prevent commit");

    assert_eq!(error.kind(), ErrorKind::Conflict);
    assert_eq!(
        fs::read(final_output).expect("read racing output"),
        b"attacker-won-race"
    );
    assert_eq!(entry_names(output.path()), ["page1.webp"]);
}

#[cfg(target_os = "linux")]
struct SwappingInputRunner {
    selected_path: PathBuf,
}

#[cfg(target_os = "linux")]
impl MediaToolRunner for SwappingInputRunner {
    fn probe_width(&self, _input: &Path) -> Result<u32> {
        Ok(320)
    }

    fn convert(&self, input: &Path, output: &Path, _upscale_width: Option<u32>) -> Result<()> {
        let original = self.selected_path.with_extension("original");
        fs::rename(&self.selected_path, &original).expect("move selected source");
        fs::write(&self.selected_path, b"attacker replacement").expect("replace selected source");
        fs::copy(input, output)
            .map(|_| ())
            .map_err(|error| BackendError::new(ErrorKind::Io, error.to_string()))
    }
}

#[cfg(target_os = "linux")]
#[test]
fn processing_binds_input_inode_when_source_is_swapped_during_conversion() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let options = process_options(input.path(), files.clone(), output.path(), 1);
    let runner = SwappingInputRunner {
        selected_path: files[0].clone(),
    };

    let error = process_images(&options, &runner, &CancellationToken::new(), |_| {})
        .expect_err("source replacement must abort processing");

    assert_eq!(error.kind(), ErrorKind::Conflict);
    assert_eq!(fs::read(&files[0]).expect("read replacement"), b"attacker replacement");
    assert!(entry_names(output.path()).is_empty());
}

#[cfg(target_os = "linux")]
struct SwappingOutputDirectoryRunner {
    output_path: PathBuf,
    moved_path: PathBuf,
}

#[cfg(target_os = "linux")]
impl MediaToolRunner for SwappingOutputDirectoryRunner {
    fn probe_width(&self, _input: &Path) -> Result<u32> {
        Ok(320)
    }

    fn convert(&self, input: &Path, output: &Path, _upscale_width: Option<u32>) -> Result<()> {
        fs::rename(&self.output_path, &self.moved_path).expect("move selected output directory");
        fs::create_dir(&self.output_path).expect("replace selected output directory");
        fs::copy(input, output)
            .map(|_| ())
            .map_err(|error| BackendError::new(ErrorKind::Io, error.to_string()))
    }
}

#[cfg(target_os = "linux")]
#[test]
fn processing_does_not_escape_bound_output_when_directory_is_swapped() {
    let root = TempDir::new().expect("create test root");
    let input = root.path().join("input");
    let output = root.path().join("output");
    let moved = root.path().join("moved-output");
    fs::create_dir(&input).expect("create input directory");
    fs::create_dir(&output).expect("create output directory");
    let files = image_files(&input, 1);
    let options = process_options(&input, files, &output, 1);
    let runner = SwappingOutputDirectoryRunner {
        output_path: output.clone(),
        moved_path: moved.clone(),
    };

    let error = process_images(&options, &runner, &CancellationToken::new(), |_| {})
        .expect_err("output directory replacement must abort processing");

    assert_eq!(error.kind(), ErrorKind::Conflict);
    assert!(entry_names(&output).is_empty());
    assert!(entry_names(&moved).is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn distribution_detects_folder_replaced_by_symlink_mid_operation_and_rolls_back() {
    use std::os::unix::fs::symlink;

    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let outside = TempDir::new().expect("create outside directory");
    let files = image_files(input.path(), 2);
    let options = distribution_options(
        input.path(),
        files.clone(),
        output.path(),
        DistributionMode::Greedy,
        1,
        1,
    );

    let error = distribute_images(&options, &CancellationToken::new(), |progress| {
        if progress.completed == 1 {
            fs::rename(
                output.path().join("Oneshot Part 2"),
                output.path().join("attacker-moved-folder"),
            )
            .expect("move second destination folder");
            symlink(outside.path(), output.path().join("Oneshot Part 2")).expect("replace destination with symlink");
        }
    })
    .expect_err("replaced destination folder must abort distribution");

    assert_eq!(error.kind(), ErrorKind::Conflict);
    assert!(files.iter().all(|file| file.exists()));
    assert!(entry_names(outside.path()).is_empty());
    assert!(
        fs::symlink_metadata(output.path().join("Oneshot Part 2"))
            .expect("inspect attacker symlink")
            .file_type()
            .is_symlink()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn distribution_detects_source_swap_and_rolls_back_completed_moves() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 2);
    let second_original = input.path().join("page2-original.png");
    let options = distribution_options(
        input.path(),
        files.clone(),
        output.path(),
        DistributionMode::Greedy,
        1,
        1,
    );

    let error = distribute_images(&options, &CancellationToken::new(), |progress| {
        if progress.completed == 1 {
            fs::rename(&files[1], &second_original).expect("move second source");
            fs::write(&files[1], b"attacker replacement").expect("replace second source");
        }
    })
    .expect_err("source replacement must abort distribution");

    assert_eq!(error.kind(), ErrorKind::Conflict);
    assert!(files[0].exists());
    assert_eq!(fs::read(&files[1]).expect("read replacement"), b"attacker replacement");
    assert_eq!(fs::read(second_original).expect("read original source"), b"fixture-2");
    assert!(entry_names(output.path()).is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn distribution_reports_partial_rollback_without_overwriting_occupied_source() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 2);
    let second_original = input.path().join("page2-original.png");
    let options = distribution_options(
        input.path(),
        files.clone(),
        output.path(),
        DistributionMode::Greedy,
        1,
        1,
    );

    let error = distribute_images(&options, &CancellationToken::new(), |progress| {
        if progress.completed == 1 {
            fs::write(&files[0], b"occupied source").expect("occupy first source name");
            fs::rename(&files[1], &second_original).expect("move second source");
            fs::write(&files[1], b"attacker replacement").expect("replace second source");
        }
    })
    .expect_err("occupied source must make rollback partial");

    let rollback = error.partial_rollback().expect("partial rollback state");
    assert_eq!(error.kind(), ErrorKind::Conflict);
    assert_eq!(error.path(), Some(Path::new("page2.png")));
    assert!(error.to_string().contains("distribution source changed before commit"));
    assert_eq!(rollback.failures().len(), 1);
    assert_eq!(rollback.failures()[0].to_path(), files[0]);
    assert_eq!(
        rollback.failures()[0].io_error().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(&files[0]).expect("read occupied source"), b"occupied source");
    assert_eq!(
        fs::read(output.path().join("Oneshot Part 1/page1.png")).expect("read unrestored original"),
        b"fixture-1"
    );
}

impl MediaToolRunner for EmptyOutputRunner {
    fn probe_width(&self, _input: &Path) -> Result<u32> {
        Ok(320)
    }

    fn convert(&self, _input: &Path, output: &Path, _upscale_width: Option<u32>) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        fs::write(output, []).map_err(|error| BackendError::new(ErrorKind::Io, error.to_string()))
    }
}

#[test]
fn processing_empty_outputs_are_retried_then_cleaned_and_failed() {
    let input = TempDir::new().expect("create input directory");
    let output = TempDir::new().expect("create output directory");
    let files = image_files(input.path(), 1);
    let options = process_options(input.path(), files.clone(), output.path(), 2);
    let runner = EmptyOutputRunner {
        calls: AtomicUsize::new(0),
    };

    let report = process_images(&options, &runner, &CancellationToken::new(), |_| {}).expect("empty output report");

    assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
    assert_eq!(report.failed, files);
    assert!(entry_names(output.path()).is_empty());
}
