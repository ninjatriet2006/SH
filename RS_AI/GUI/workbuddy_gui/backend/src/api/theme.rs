use crate::core::resources::resource_dir;
use crate::ipc::{respond, Empty, IpcResult, Req};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub theme_type: String,
    pub colors: HashMap<String, String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_available_themes(request: Req<Empty>) -> IpcResult<Vec<Theme>> {
    let (request_id, _) = request.validate()?;
    let dir = resource_dir("themes");
    let mut themes = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Theme>(&content) {
                    Ok(theme) => themes.push(theme),
                    Err(e) => eprintln!("[theme] skipping bad file {}: {e}", path.display()),
                },
                Err(e) => eprintln!("[theme] cannot read {}: {e}", path.display()),
            }
        }
    }

    themes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(respond(request_id, themes))
}
