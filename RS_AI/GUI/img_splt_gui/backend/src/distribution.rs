use crate::path_security::{canonical_direct_file, canonical_directory};
use crate::{BackendError, CancellationToken, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionMode {
    Balanced,
    Greedy,
    Fixed,
}

#[derive(Debug, Clone)]
pub struct DistributionOptions {
    pub input_directory: PathBuf,
    pub files: Vec<PathBuf>,
    pub output_directory: PathBuf,
    pub chapter: Option<u32>,
    pub mode: DistributionMode,
    pub max_files_per_folder: u64,
    pub fixed_folder_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributionReport {
    pub output_directory: PathBuf,
    pub folders: Vec<PathBuf>,
    pub distributed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DistributionProgress {
    pub completed: u64,
    pub total: u64,
}

pub fn distribute_images(
    options: &DistributionOptions,
    cancellation: &CancellationToken,
    mut progress: impl FnMut(DistributionProgress),
) -> Result<DistributionReport> {
    if options.files.is_empty() {
        return Err(BackendError::new(
            ErrorKind::InvalidArgument,
            "at least one input file is required",
        ));
    }
    if options.max_files_per_folder == 0 || options.fixed_folder_count == 0 {
        return Err(BackendError::new(
            ErrorKind::InvalidArgument,
            "folder limits must be greater than zero",
        ));
    }
    let input_directory = canonical_directory(&options.input_directory)?;
    let output_directory = canonical_directory(&options.output_directory)?;
    let mut files = Vec::with_capacity(options.files.len());
    let mut unique = HashSet::with_capacity(options.files.len());
    for file in &options.files {
        let file = canonical_direct_file(file, &input_directory)?;
        if !unique.insert(file.clone()) {
            return Err(BackendError::at_path(
                ErrorKind::InvalidArgument,
                "duplicate input file",
                file,
            ));
        }
        files.push(file);
    }

    #[cfg(target_os = "linux")]
    {
        crate::distribution_linux::distribute_validated(
            options,
            &input_directory,
            files,
            &output_directory,
            cancellation,
            &mut progress,
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (input_directory, files, output_directory, cancellation, &mut progress);
        Err(BackendError::new(
            ErrorKind::Unavailable,
            "secure distribution is unavailable on this platform",
        ))
    }
}

pub(crate) fn distribution_shape(total: u64, options: &DistributionOptions) -> Result<(u64, u64)> {
    let shape = match options.mode {
        DistributionMode::Greedy => {
            let count = total.div_ceil(options.max_files_per_folder);
            (count, options.max_files_per_folder)
        }
        DistributionMode::Balanced => {
            let count = total.div_ceil(options.max_files_per_folder);
            (count, total.div_ceil(count))
        }
        DistributionMode::Fixed => {
            let count = options.fixed_folder_count.min(total);
            (count, total.div_ceil(count))
        }
    };
    if shape.0 == 0 || shape.1 == 0 {
        Err(BackendError::new(
            ErrorKind::InvalidArgument,
            "invalid distribution shape",
        ))
    } else {
        Ok(shape)
    }
}

pub(crate) fn folder_name(chapter: Option<u32>, next: Option<u64>, count: u64, index: u64) -> String {
    match (chapter, next) {
        (Some(chapter), Some(next)) => format!("Chapter {chapter}.{}", next + index),
        _ if count > 1 => format!("Oneshot Part {}", index + 1),
        _ => "Oneshot".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn balanced_distribution_moves_files_into_chapter_folders() {
        let input = test_dir("distribution-input");
        let output = test_dir("distribution-output");
        let files: Vec<_> = (1..=5)
            .map(|index| {
                let path = input.join(format!("page{index}.png"));
                fs::write(&path, b"fixture").expect("write fixture");
                path
            })
            .collect();
        let options = DistributionOptions {
            input_directory: input,
            files,
            output_directory: output.clone(),
            chapter: Some(2),
            mode: DistributionMode::Balanced,
            max_files_per_folder: 3,
            fixed_folder_count: 5,
        };
        let report = distribute_images(&options, &CancellationToken::new(), |_| {}).expect("distribution");
        assert_eq!(report.distributed, 5);
        assert_eq!(report.folders, [output.join("Chapter 2.1"), output.join("Chapter 2.2")]);
        assert_eq!(
            fs::read_dir(output.join("Chapter 2.1")).expect("first folder").count(),
            3
        );
        assert_eq!(
            fs::read_dir(output.join("Chapter 2.2")).expect("second folder").count(),
            2
        );
    }

    #[test]
    fn cancelled_distribution_does_not_move_files() {
        let input = test_dir("cancel-input");
        let output = test_dir("cancel-output");
        let source = input.join("page.png");
        fs::write(&source, b"fixture").expect("write fixture");
        let options = DistributionOptions {
            input_directory: input,
            files: vec![source.clone()],
            output_directory: output,
            chapter: None,
            mode: DistributionMode::Greedy,
            max_files_per_folder: 80,
            fixed_folder_count: 5,
        };
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert_eq!(
            distribute_images(&options, &cancellation, |_| {})
                .expect_err("cancelled")
                .kind(),
            ErrorKind::Cancelled
        );
        assert!(source.exists());
    }
}
