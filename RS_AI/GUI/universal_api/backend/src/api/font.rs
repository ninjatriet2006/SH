use crate::core::resources::resource_dir;
use crate::ipc::{respond, Empty, IpcResult, Req};
use serde::Serialize;
use std::fs;

pub const DEFAULT_FONT_ID: &str = "default";

#[derive(Debug, Clone, Serialize)]
pub struct FontInfo {
    pub id: String,
    pub name: String,
    pub family: String,
    pub src_path: Option<String>,
}

fn font_id(file_stem: &str) -> String {
    let normalized: String = file_stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    normalized.trim_matches('_').to_string()
}

pub fn scan_fonts() -> Vec<FontInfo> {
    let mut fonts = vec![FontInfo {
        id: DEFAULT_FONT_ID.to_string(),
        name: "System default".to_string(),
        family: "system-ui".to_string(),
        src_path: None,
    }];

    if let Ok(entries) = fs::read_dir(resource_dir("fonts")) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let extension = path.extension().and_then(|v| v.to_str()).unwrap_or_default();
            if !path.is_file()
                || !matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "ttf" | "otf" | "woff" | "woff2"
                )
            {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|v| v.to_str()) else {
                continue;
            };
            fonts.push(FontInfo {
                id: font_id(stem),
                name: stem.to_string(),
                family: stem.to_string(),
                src_path: Some(path.to_string_lossy().into_owned()),
            });
        }
    }

    fonts[1..].sort_by(|a, b| a.id.cmp(&b.id));
    fonts
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_available_fonts(request: Req<Empty>) -> IpcResult<Vec<FontInfo>> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, scan_fonts()))
}
