use crate::core::resources::resource_dir;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

fn langs_dir() -> PathBuf {
    resource_dir("langs")
}

pub fn scan_lang_codes() -> Vec<String> {
    let mut codes = Vec::new();
    if let Ok(entries) = fs::read_dir(langs_dir()) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    codes.push(stem.to_string());
                }
            }
        }
    }
    codes.sort();
    codes
}

pub fn first_available_lang() -> Option<String> {
    scan_lang_codes().into_iter().next()
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_available_langs(request: Req<Empty>) -> IpcResult<Vec<String>> {
    let (request_id, _) = request.validate()?;
    Ok(respond(request_id, scan_lang_codes()))
}

#[derive(Deserialize)]
pub struct LangContentRequest {
    pub lang_code: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_lang_content(request: Req<LangContentRequest>) -> IpcResult<serde_json::Value> {
    let (request_id, payload) = request.validate()?;
    get_lang_content_inner(payload.lang_code)
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

fn get_lang_content_inner(lang_code: String) -> Result<serde_json::Value, String> {
    if lang_code.is_empty()
        || !lang_code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid language code: {}", lang_code));
    }

    let path = langs_dir().join(format!("{}.json", lang_code));
    if !path.is_file() {
        return Err(format!("Language not found: {}", lang_code));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Error reading language file {}: {}", lang_code, e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("Error parsing language JSON {}: {}", lang_code, e))
}
