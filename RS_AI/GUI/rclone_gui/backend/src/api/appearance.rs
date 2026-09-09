use crate::core::resources::resource_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FontInfo {
    pub id: String,
    pub name: String,
    pub family: String,
    pub src_path: Option<String>,
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

pub fn scan_themes() -> Vec<ThemeInfo> {
    let mut themes = vec![ThemeInfo {
        id: "default".to_string(),
        name: "Neon default".to_string(),
        variables: HashMap::new(),
    }];
    if let Ok(entries) = fs::read_dir(resource_dir("themes")) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(theme) = serde_json::from_str::<ThemeInfo>(&content) else {
                continue;
            };
            if safe_id(&theme.id) && theme.id != "default" {
                themes.push(theme);
            }
        }
    }
    themes[1..].sort_by(|a, b| a.id.cmp(&b.id));
    themes
}

fn font_id(stem: &str) -> String {
    stem.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

pub fn scan_fonts() -> Vec<FontInfo> {
    let mut fonts = vec![FontInfo {
        id: "default".to_string(),
        name: "System default".to_string(),
        family: "system-ui".to_string(),
        src_path: None,
    }];
    if let Ok(entries) = fs::read_dir(resource_dir("fonts")) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !path.is_file() || !matches!(ext.as_str(), "ttf" | "otf" | "woff" | "woff2") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
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

#[tauri::command]
pub fn get_available_themes() -> Result<Vec<ThemeInfo>, String> {
    Ok(scan_themes())
}

#[tauri::command]
pub fn get_available_fonts() -> Result<Vec<FontInfo>, String> {
    Ok(scan_fonts())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_always_first() {
        assert_eq!(scan_themes()[0].id, "default");
        assert_eq!(scan_fonts()[0].id, "default");
    }

    #[test]
    fn resource_lists_are_stable() {
        let theme_ids: Vec<_> = scan_themes().into_iter().skip(1).map(|item| item.id).collect();
        let font_ids: Vec<_> = scan_fonts().into_iter().skip(1).map(|item| item.id).collect();
        assert!(theme_ids.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(font_ids.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn bundled_dejavu_font_uses_contract_id() {
        assert_eq!(font_id("DejaVuSans"), "dejavusans");
        assert!(scan_fonts().iter().any(|font| font.id == "dejavusans"));
    }
}
