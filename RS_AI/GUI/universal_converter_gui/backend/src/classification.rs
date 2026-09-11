use crate::CancellationToken;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Video,
    Audio,
    Image,
    Document,
    Archive,
    Directory,
    Unknown,
}

impl FileType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Image => "image",
            Self::Document => "document",
            Self::Archive => "archive",
            Self::Directory => "directory",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub path: PathBuf,
    pub file_type: FileType,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub directory: PathBuf,
    pub files: Vec<Classification>,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub current_path: Option<PathBuf>,
}

pub fn classify_file(path: impl AsRef<Path>) -> io::Result<Classification> {
    let path = path.as_ref();
    require_absolute(path)?;
    let metadata = fs::metadata(path)?;
    let file_type = if metadata.is_dir() {
        FileType::Directory
    } else {
        classify_extension(path)
    };

    Ok(Classification {
        path: path.to_path_buf(),
        file_type,
        size_bytes: if metadata.is_file() { metadata.len() } else { 0 },
    })
}

pub fn scan_directory(directory: impl AsRef<Path>, allowed_types: &[FileType]) -> io::Result<ScanReport> {
    scan_directory_cancellable(directory, allowed_types, &CancellationToken::new(), |_| {})
}

pub fn scan_directory_cancellable(
    directory: impl AsRef<Path>,
    allowed_types: &[FileType],
    cancellation: &CancellationToken,
    mut progress: impl FnMut(ScanProgress),
) -> io::Result<ScanReport> {
    let directory = directory.as_ref();
    require_absolute(directory)?;
    if !directory.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scan path must be a directory",
        ));
    }

    let mut files = Vec::new();
    let mut completed = 0;
    progress(ScanProgress {
        completed,
        total: None,
        current_path: None,
    });
    for entry in fs::read_dir(directory)? {
        cancellation.check()?;
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_file() || is_hidden(&path) || is_junk_file(&path) {
            continue;
        }

        let classification = classify_file(path)?;
        if allowed_types.contains(&classification.file_type) {
            files.push(classification);
        }
        completed += 1;
        progress(ScanProgress {
            completed,
            total: None,
            current_path: Some(entry.path()),
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let total = files.len() as u64;

    Ok(ScanReport {
        directory: directory.to_path_buf(),
        files,
        total,
    })
}

fn require_absolute(path: &Path) -> io::Result<()> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "path must be absolute"))
    }
}

fn classify_extension(path: &Path) -> FileType {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" => FileType::Video,
        "mp3" | "wav" | "flac" | "m4a" | "ogg" | "wma" => FileType::Audio,
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff" => FileType::Image,
        "docx" | "doc" | "xlsx" | "xls" | "pdf" | "txt" | "pptx" | "ppt" | "odt" | "ods" => FileType::Document,
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" => FileType::Archive,
        _ => FileType::Unknown,
    }
}

fn is_hidden(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.starts_with('.'))
    {
        return true;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if fs::metadata(path).is_ok_and(|metadata| metadata.file_attributes() & 0x2 != 0) {
            return true;
        }
    }

    false
}

fn is_junk_file(path: &Path) -> bool {
    path.file_name().and_then(|value| value.to_str()).is_some_and(|name| {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "thumbs.db" | "desktop.ini" | ".ds_store"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(path)
    }

    #[test]
    fn classifies_supported_types_and_reports_size() {
        let cases = [
            ("scan/movie.mp4", FileType::Video),
            ("scan/song.mp3", FileType::Audio),
            ("scan/report.pdf", FileType::Document),
            ("scan/archive.zip", FileType::Archive),
            ("scan/unknown.bin", FileType::Unknown),
        ];

        for (path, expected) in cases {
            let classification = classify_file(fixture(path)).expect("fixture must be readable");
            assert_eq!(classification.file_type, expected);
            assert!(classification.size_bytes > 0);
        }
    }

    #[test]
    fn classifies_directories_without_using_cwd() {
        let classification = classify_file(fixture("scan/nested")).expect("fixture must be readable");
        assert_eq!(classification.file_type, FileType::Directory);
        assert_eq!(classification.size_bytes, 0);
    }

    #[test]
    fn scan_is_non_recursive_and_filters_hidden_and_junk_files() {
        let report = scan_directory(fixture("scan"), &[FileType::Video, FileType::Audio, FileType::Document])
            .expect("fixture directory must scan");
        let names: Vec<_> = report.files.iter().filter_map(|file| file.path.file_name()).collect();

        assert_eq!(names, ["movie.mp4", "report.pdf", "song.mp3"]);
        assert_eq!(report.total, 3);
    }

    #[test]
    fn rejects_relative_paths() {
        let error = classify_file("relative.mp4").expect_err("relative paths must be rejected");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn cancellable_scan_stops_and_emits_initial_progress() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let mut progress = Vec::new();
        let error = scan_directory_cancellable(fixture("scan"), &[FileType::Video], &cancellation, |event| {
            progress.push(event)
        })
        .expect_err("cancelled scan must stop");

        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(progress.len(), 1);
        assert_eq!(progress[0].completed, 0);
    }
}
