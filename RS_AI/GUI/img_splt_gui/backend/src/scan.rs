use crate::path_security::canonical_directory;
use crate::{BackendError, CancellationToken, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageScanReport {
    pub directory: PathBuf,
    pub images: Vec<PathBuf>,
    pub total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanProgress {
    pub scanned: u64,
    pub images: u64,
}

pub fn scan_images(
    directory: impl AsRef<Path>,
    cancellation: &CancellationToken,
    mut progress: impl FnMut(ScanProgress),
) -> Result<ImageScanReport> {
    let directory = canonical_directory(directory.as_ref())?;
    let entries =
        fs::read_dir(&directory).map_err(|error| BackendError::from_io("cannot scan directory", &directory, error))?;
    let mut images = Vec::new();
    for (index, entry) in entries.enumerate() {
        cancellation.check()?;
        let entry = entry.map_err(|error| BackendError::from_io("cannot read directory entry", &directory, error))?;
        let scanned = index as u64 + 1;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| BackendError::from_io("cannot inspect directory entry", &path, error))?;
        if file_type.is_file() && !file_type.is_symlink() && !is_hidden(&path) && is_supported_image(&path) {
            images.push(path);
        }
        progress(ScanProgress {
            scanned,
            images: images.len() as u64,
        });
    }
    images.sort_by(|left, right| natural_compare(left, right));
    let total = images.len() as u64;
    Ok(ImageScanReport {
        directory,
        images,
        total,
    })
}

pub fn is_supported_image(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        matches!(
            extension.to_string_lossy().to_ascii_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "webp" | "avif" | "heic" | "bmp" | "tiff"
        )
    })
}

pub fn natural_compare(left: &Path, right: &Path) -> Ordering {
    let left = left.file_name().unwrap_or(left.as_os_str()).to_string_lossy();
    let right = right.file_name().unwrap_or(right.as_os_str()).to_string_lossy();
    natural_str_compare(&left, &right)
}

fn natural_str_compare(left: &str, right: &str) -> Ordering {
    let mut left = left.as_bytes().iter().copied().peekable();
    let mut right = right.as_bytes().iter().copied().peekable();
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (Some(a), Some(b)) if a.is_ascii_digit() && b.is_ascii_digit() => {
                let left_number = take_digits(&mut left);
                let right_number = take_digits(&mut right);
                let ordering = left_number
                    .trim_start_matches('0')
                    .len()
                    .cmp(&right_number.trim_start_matches('0').len())
                    .then_with(|| {
                        left_number
                            .trim_start_matches('0')
                            .cmp(right_number.trim_start_matches('0'))
                    })
                    .then_with(|| left_number.len().cmp(&right_number.len()));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(a), Some(b)) => {
                left.next();
                right.next();
                let ordering = a.cmp(&b);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
        }
    }
}

fn take_digits(iter: &mut std::iter::Peekable<impl Iterator<Item = u8>>) -> String {
    let mut digits = String::new();
    while let Some(value) = iter.peek().copied().filter(u8::is_ascii_digit) {
        digits.push(char::from(value));
        iter.next();
    }
    digits
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("img-splt-scan-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).expect("create test directory");
        path
    }

    #[test]
    fn scans_supported_top_level_images_in_natural_order() {
        let directory = test_dir();
        for name in ["image10.png", "image2.JPG", "notes.txt", ".hidden.png"] {
            fs::write(directory.join(name), b"fixture").expect("write fixture");
        }
        fs::create_dir(directory.join("nested")).expect("nested directory");
        fs::write(directory.join("nested/image1.png"), b"fixture").expect("nested fixture");

        let report = scan_images(&directory, &CancellationToken::new(), |_| {}).expect("scan report");
        let names: Vec<_> = report.images.iter().filter_map(|path| path.file_name()).collect();
        assert_eq!(names, ["image2.JPG", "image10.png"]);
    }

    #[test]
    fn rejects_relative_directory() {
        let error = scan_images("relative", &CancellationToken::new(), |_| {}).expect_err("relative path");
        assert_eq!(error.kind(), crate::ErrorKind::InvalidArgument);
    }
}
