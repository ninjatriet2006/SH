use std::fs;
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, serde::Serialize)]
pub enum FileType {
    Video,
    Audio,
    Image,
    Document,
    Archive,
    Directory,
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub file_type: FileType,
    pub extension: String,
}

pub fn parse_drag_drop_paths(input: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_quotes {
            if c == quote_char {
                in_quotes = false;
                if !current.is_empty() {
                    paths.push(PathBuf::from(current.trim()));
                    current.clear();
                }
            } else {
                current.push(c);
            }
        } else {
            if c == '\'' || c == '"' {
                in_quotes = true;
                quote_char = c;
            } else if c == ' ' {
                if !current.is_empty() {
                    paths.push(PathBuf::from(current.trim()));
                    current.clear();
                }
            } else {
                current.push(c);
            }
        }
        i += 1;
    }
    if !current.is_empty() {
        paths.push(PathBuf::from(current.trim()));
    }

    // Filter paths that exist
    paths.into_iter().filter(|p| p.exists()).collect()
}

pub fn scan_directory(dir: &Path, allowed_types: &[bool]) -> anyhow::Result<Vec<ScannedFile>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // Filter OS junk & hidden files
            if is_hidden(&path) || is_junk_file(&path) {
                continue;
            }

            let sc = classify_file(path);
            let should_include = match sc.file_type {
                FileType::Archive => allowed_types.first().copied().unwrap_or(false),
                FileType::Video => allowed_types.get(1).copied().unwrap_or(false),
                FileType::Image => allowed_types.get(2).copied().unwrap_or(false),
                FileType::Audio => allowed_types.get(3).copied().unwrap_or(false),
                FileType::Document => allowed_types.get(4).copied().unwrap_or(false),
                FileType::Directory => false,
                FileType::Unknown => false,
            };

            if should_include {
                files.push(sc);
            }
        }
    }

    Ok(files)
}

pub fn classify_file(path: PathBuf) -> ScannedFile {
    if path.is_dir() {
        return ScannedFile {
            path,
            file_type: FileType::Directory,
            extension: "".to_string(),
        };
    }

    let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();

    let file_type = match ext.as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" => FileType::Video,
        "mp3" | "wav" | "flac" | "m4a" | "ogg" | "wma" => FileType::Audio,
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "tiff" => FileType::Image,
        "docx" | "doc" | "xlsx" | "xls" | "pdf" | "txt" | "pptx" | "ppt" | "odt" | "ods" => FileType::Document,
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" => FileType::Archive,
        _ => FileType::Unknown,
    };

    ScannedFile {
        path,
        file_type,
        extension: ext,
    }
}

fn is_hidden(path: &Path) -> bool {
    let filename = path.file_name().unwrap_or_default().to_string_lossy();

    // Unix hidden files
    if filename.starts_with('.') {
        return true;
    }

    // Windows hidden files check using metadata attributes
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        if let Ok(meta) = fs::metadata(path) {
            let attrs = meta.file_attributes();
            // FILE_ATTRIBUTE_HIDDEN = 0x2
            if (attrs & 0x2) != 0 {
                return true;
            }
        }
    }

    false
}

fn is_junk_file(path: &Path) -> bool {
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    filename == "thumbs.db" || filename == "desktop.ini" || filename == ".ds_store"
}
