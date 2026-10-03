//! Zed In-place Injection Logic.
//!
//! Ghi trực tiếp credentials vào file cấu hình `credentials.json` của Zed editor.

use std::path::{Path, PathBuf};

pub fn default_zed_credentials_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return Ok(home.join("Library/Application Support/Zed/credentials.json"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            return Ok(PathBuf::from(appdata).join("Zed/credentials.json"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return Ok(home.join(".local/share/zed/credentials.json"));
        }
    }

    Err("Cannot determine Zed credentials path on this platform".to_string())
}

pub fn inject_zed_credentials(target_file: &Path, id: &str, access_token: &str) -> Result<(), String> {
    if let Some(parent) = target_file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create zed dir: {e}"))?;
    }
    let payload = serde_json::json!({
        "type": "zed",
        "id": id,
        "access_token": access_token
    });
    std::fs::write(target_file, payload.to_string()).map_err(|e| format!("write zed credentials: {e}"))?;
    Ok(())
}
