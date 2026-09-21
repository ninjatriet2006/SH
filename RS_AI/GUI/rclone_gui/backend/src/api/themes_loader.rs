use crate::actions::types::ThemeInfo;
use crate::core::resources::resource_dir;
use std::fs;

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn scan_themes() -> Vec<ThemeInfo> {
    let mut themes = vec![ThemeInfo {
        id: "default".to_string(),
        name: "Neon default".to_string(),
        variables: std::collections::HashMap::new(),
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

#[tauri::command]
pub fn get_available_themes() -> Result<Vec<ThemeInfo>, String> {
    Ok(scan_themes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_is_always_first() {
        assert_eq!(scan_themes()[0].id, "default");
    }

    #[test]
    fn theme_list_is_stable() {
        let theme_ids: Vec<_> = scan_themes().into_iter().skip(1).map(|item| item.id).collect();
        assert!(theme_ids.windows(2).all(|pair| pair[0] <= pair[1]));
    }
}
