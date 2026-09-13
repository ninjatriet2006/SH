use crate::{
    auth_cmds::Empty,
    contract::{backend, success, validate, IpcResult, Req},
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct ThemeResource {
    pub id: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct FontResource {
    pub id: String,
    pub name: String,
    pub path: String,
}

fn resource_path(name: &str) -> PathBuf {
    if let Some(root) = std::env::var_os("FILEN_GUI_RESOURCE_DIR") {
        return PathBuf::from(root).join(name);
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bridge must be inside filen_gui")
        .join(name)
}

fn resource_files(name: &str) -> Result<Vec<PathBuf>, String> {
    let directory = resource_path(name);
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Cannot read {}: {error}", directory.display())),
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    Ok(files)
}

#[tauri::command]
pub fn appearance_list_themes(request: Req<Empty>) -> IpcResult<Vec<ThemeResource>> {
    let (request_id, _) = validate(request)?;
    let data = resource_files("themes")
        .map_err(backend)?
        .into_iter()
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .map(|path| {
            let id = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("Invalid theme filename: {}", path.display()))?
                .to_string();
            let content =
                fs::read_to_string(&path).map_err(|error| format!("Cannot read theme {}: {error}", path.display()))?;
            Ok(ThemeResource { id, content })
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(backend)?;
    Ok(success(request_id, data))
}

fn is_font_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("ttf" | "otf" | "woff" | "woff2")
    )
}

#[tauri::command]
pub fn appearance_list_fonts(request: Req<Empty>) -> IpcResult<Vec<FontResource>> {
    let (request_id, _) = validate(request)?;
    let data = resource_files("fonts")
        .map_err(backend)?
        .into_iter()
        .filter(|path| is_font_file(path))
        .map(|path| {
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("Invalid font filename: {}", path.display()))?
                .to_string();
            let normalized: String = name
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() {
                        ch.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .collect();
            let id = normalized.trim_matches('_').to_string();
            Ok(FontResource {
                id,
                name,
                path: path.to_string_lossy().into_owned(),
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map_err(backend)?;
    Ok(success(request_id, data))
}

#[cfg(test)]
mod tests {
    use super::is_font_file;
    use std::path::Path;

    #[test]
    fn detects_supported_font_extensions_case_insensitively() {
        assert!(is_font_file(Path::new("Neon.TTF")));
        assert!(is_font_file(Path::new("Neon.woff2")));
        assert!(!is_font_file(Path::new("README.md")));
    }
}
