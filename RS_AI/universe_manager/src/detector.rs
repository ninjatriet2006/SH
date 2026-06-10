use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub is_appimage: bool,
    pub suggested_name: String,
    pub executables: Vec<PathBuf>,
    pub icons: Vec<PathBuf>,
    pub desktop_templates: Vec<PathBuf>,
}

/// Simple parser for desktop entries to extract metadata.
#[derive(Debug, Clone, Default)]
pub struct DesktopMetadata {
    pub name: Option<String>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub categories: Option<String>,
}

impl DesktopMetadata {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Self {
        let mut meta = DesktopMetadata::default();
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return meta,
        };

        let mut in_desktop_entry = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_desktop_entry = false;
                continue;
            }

            if in_desktop_entry {
                if let Some(pos) = trimmed.find('=') {
                    let key = trimmed[..pos].trim();
                    let val = trimmed[pos + 1..].trim();

                    // Strip quotes if any
                    let clean_val = if (val.starts_with('"') && val.ends_with('"')) || 
                                       (val.starts_with('\'') && val.ends_with('\'')) {
                        if val.len() >= 2 { &val[1..val.len() - 1] } else { val }
                    } else {
                        val
                    }.to_string();

                    match key {
                        "Name" => meta.name = Some(clean_val),
                        "Comment" => meta.comment = Some(clean_val),
                        "Exec" => meta.exec = Some(clean_val),
                        "Icon" => meta.icon = Some(clean_val),
                        "Categories" => meta.categories = Some(clean_val),
                        _ => {}
                    }
                }
            }
        }

        meta
    }
}

pub fn suggest_name_from_string(s: &str) -> String {
    // Strip file extensions like .AppImage, .sh, etc.
    let name_without_ext = if let Some(idx) = s.find(".AppImage") {
        &s[..idx]
    } else if let Some(idx) = s.rfind('.') {
        &s[..idx]
    } else {
        s
    };

    // Strip version numbers or architectures (e.g. -x86_64, -1.0.0)
    let parts: Vec<&str> = name_without_ext.split(|c| c == '-' || c == '_').collect();
    let mut words = Vec::new();

    for part in parts {
        // Ignore architecture or common words
        let lower = part.to_lowercase();
        if lower == "x86" || lower == "64" || lower == "x64" || lower == "amd64" || lower == "linux" || lower == "app" || lower == "ide" || lower == "portable" {
            continue;
        }
        // If it starts with a number (like version), we might ignore it or keep it depending on length
        if !part.is_empty() && part.chars().next().unwrap().is_ascii_digit() {
            continue;
        }

        // Capitalize the word
        if !part.is_empty() {
            let mut chars = part.chars();
            let capitalized = match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            };
            words.push(capitalized);
        }
    }

    if words.is_empty() {
        name_without_ext.to_string()
    } else {
        words.join(" ")
    }
}

pub fn detect<P: AsRef<Path>>(target: P) -> Result<DetectionResult, std::io::Error> {
    let path = target.as_ref().canonicalize()?;
    
    // Check if target is a file or a directory
    if path.is_file() {
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        let is_appimage = filename.ends_with(".AppImage") || filename.contains(".appimage");
        let suggested_name = suggest_name_from_string(&filename);

        return Ok(DetectionResult {
            is_appimage,
            suggested_name,
            executables: vec![path.clone()],
            icons: Vec::new(),
            desktop_templates: Vec::new(),
        });
    }

    // It's a directory
    let folder_name = path.file_name().unwrap_or_default().to_string_lossy();
    let suggested_name = suggest_name_from_string(&folder_name);

    let mut executables = Vec::new();
    let mut icons = Vec::new();
    let mut desktop_templates = Vec::new();

    // Walk the directory (maximum depth of 4 to avoid infinite recursion or heavy load)
    for entry in WalkDir::new(&path)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
    {
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }

        let name = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let extension = file_path.extension().map(|s| s.to_ascii_lowercase());

        // Check for desktop files
        if extension.as_deref() == Some(std::ffi::OsStr::new("desktop")) {
            desktop_templates.push(file_path.to_path_buf());
            continue;
        }

        // Check for icons (PNG, SVG, JPG, JPEG)
        if let Some(ext) = &extension {
            if ext == "png" || ext == "svg" || ext == "jpg" || ext == "jpeg" {
                icons.push(file_path.to_path_buf());
                continue;
            }
        }

        // Check for executables:
        // - Must have executable permissions on unix
        // - Exclude common library or data files to reduce noise
        if let Ok(metadata) = fs::metadata(file_path) {
            let mode = metadata.permissions().mode();
            let is_exec = mode & 0o111 != 0;

            if is_exec {
                // Filter out common false positives and helper tools
                let is_lib = name.contains(".so") || name.ends_with(".a") || name.ends_with(".node");
                let is_help_tool = name == "chrome-sandbox" 
                    || name == "chrome_crashpad_handler" 
                    || name == "crashpad_handler"
                    || name == "updater"
                    || name == "Updater"
                    || name == "update"
                    || name == "Update";
                
                if !is_lib && !is_help_tool {
                    executables.push(file_path.to_path_buf());
                }
            }
        }
    }

    // Sort executables:
    // 1. Files in the root directory come first (more likely to be the main launcher).
    // 2. Files matching the suggested name (case-insensitive) come first.
    // 3. Shortest name length (more likely to be `app` rather than `app_helper`).
    // 4. Alphabetical.
    executables.sort_by(|a, b| {
        let a_in_root = a.parent() == Some(&path);
        let b_in_root = b.parent() == Some(&path);

        if a_in_root && !b_in_root {
            std::cmp::Ordering::Less
        } else if !a_in_root && b_in_root {
            std::cmp::Ordering::Greater
        } else {
            let a_name = a.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            let b_name = b.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            let clean_suggested = suggested_name.to_lowercase().replace(' ', "");
            let a_matches = a_name == clean_suggested;
            let b_matches = b_name == clean_suggested;

            if a_matches && !b_matches {
                std::cmp::Ordering::Less
            } else if !a_matches && b_matches {
                std::cmp::Ordering::Greater
            } else {
                let a_len = a.file_name().unwrap_or_default().len();
                let b_len = b.file_name().unwrap_or_default().len();
                a_len.cmp(&b_len).then_with(|| a.cmp(b))
            }
        }
    });

    // Sort icons:
    // 1. Files in root first.
    // 2. SVGs first (better scalability).
    // 3. Filename containing "icon", "logo", "app" first.
    // 4. File size (larger file size usually means higher resolution icon).
    icons.sort_by(|a, b| {
        let a_in_root = a.parent() == Some(&path);
        let b_in_root = b.parent() == Some(&path);

        if a_in_root && !b_in_root {
            return std::cmp::Ordering::Less;
        }
        if !a_in_root && b_in_root {
            return std::cmp::Ordering::Greater;
        }

        let a_ext = a.extension().map(|e| e.to_ascii_lowercase());
        let b_ext = b.extension().map(|e| e.to_ascii_lowercase());
        let a_is_svg = a_ext.as_deref() == Some(std::ffi::OsStr::new("svg"));
        let b_is_svg = b_ext.as_deref() == Some(std::ffi::OsStr::new("svg"));

        if a_is_svg && !b_is_svg {
            return std::cmp::Ordering::Less;
        }
        if !a_is_svg && b_is_svg {
            return std::cmp::Ordering::Greater;
        }

        let a_name = a.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let b_name = b.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let a_has_keyword = a_name.contains("icon") || a_name.contains("logo") || a_name.contains("app");
        let b_has_keyword = b_name.contains("icon") || b_name.contains("logo") || b_name.contains("app");

        if a_has_keyword && !b_has_keyword {
            return std::cmp::Ordering::Less;
        }
        if !a_has_keyword && b_has_keyword {
            return std::cmp::Ordering::Greater;
        }

        // Compare by file size (largest first)
        let a_size = fs::metadata(a).map(|m| m.len()).unwrap_or(0);
        let b_size = fs::metadata(b).map(|m| m.len()).unwrap_or(0);
        b_size.cmp(&a_size)
    });

    Ok(DetectionResult {
        is_appimage: false,
        suggested_name,
        executables,
        icons,
        desktop_templates,
    })
}
