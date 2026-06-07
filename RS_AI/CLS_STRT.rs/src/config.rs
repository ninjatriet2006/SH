use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AppType {
    Flatpak,
    System,
}

impl std::fmt::Display for AppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppType::Flatpak => write!(f, "Flatpak"),
            AppType::System => write!(f, "Hệ thống"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub name: String,
    pub app_type: AppType,
    pub target: String, // Flatpak ID (ví dụ: org.fcitx.Fcitx5) hoặc tên tiến trình/lệnh (ví dụ: discord)
    pub start_cmd: Option<String>,
    pub kill_cmd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub apps: Vec<AppConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            apps: vec![
                AppConfig {
                    name: "Fcitx5".to_string(),
                    app_type: AppType::Flatpak,
                    target: "org.fcitx.Fcitx5".to_string(),
                    start_cmd: None,
                    kill_cmd: None,
                }
            ]
        }
    }
}

pub fn get_config_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("cls_strt");
        path.push("config.yaml");
        path
    } else {
        PathBuf::from("config.yaml")
    }
}

pub fn load_config() -> Config {
    let path = get_config_path();
    if !path.exists() {
        let default_config = Config::default();
        if let Err(e) = save_config(&default_config) {
            eprintln!("[⚠️] Không thể tạo file cấu hình mặc định tại {}: {}", path.display(), e);
        }
        return default_config;
    }

    match fs::read_to_string(&path) {
        Ok(content) => {
            match serde_yaml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("[⚠️] File cấu hình lỗi ({}), khôi phục mặc định...", e);
                    Config::default()
                }
            }
        }
        Err(_) => Config::default(),
    }
}

pub fn save_config(config: &Config) -> anyhow::Result<()> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml::to_string(config)?;
    fs::write(&path, yaml)?;
    Ok(())
}
