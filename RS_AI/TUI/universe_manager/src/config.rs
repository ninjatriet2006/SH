use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum InstallType {
    #[default]
    InPlace,
    Moved,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub install_type: InstallType,
    pub source_path: Option<String>,
    pub install_path: String,
    pub exec_path: String,
    pub icon_path: Option<String>,
    pub desktop_file: String,
    pub symlink_file: Option<String>,
    pub added_at: String,
    pub is_custom: Option<bool>,
    pub start_cmd: Option<String>,
    pub stop_cmd: Option<String>,
    pub category: Option<String>,
    pub package_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uninstall_cmd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub managed_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub settings: Settings,
    pub apps: Vec<AppEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Healthy,
    Degraded(Vec<String>),
    Broken(Vec<String>),
}

fn extract_exec_path(cmd: &str) -> Option<PathBuf> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return None;
    }

    let path_str = if let Some(rest) = trimmed.strip_prefix('"') {
        if let Some(end_idx) = rest.find('"') {
            rest[..end_idx].to_string()
        } else {
            rest.to_string()
        }
    } else {
        let cmd_lower = trimmed.to_lowercase();
        let mut found_ext = None;
        for ext in &[".exe", ".bat", ".cmd", ".msi"] {
            if let Some(idx) = cmd_lower.find(ext) {
                found_ext = Some(idx + ext.len());
                break;
            }
        }
        if let Some(end_idx) = found_ext {
            trimmed[..end_idx].to_string()
        } else {
            trimmed.split_whitespace().next().unwrap_or("").to_string()
        }
    };

    let clean_path = path_str.trim().to_string();
    if clean_path.is_empty() {
        return None;
    }

    let name_lower = clean_path.to_lowercase();
    if name_lower == "msiexec" || name_lower == "msiexec.exe" {
        return Some(PathBuf::from("C:\\Windows\\System32\\msiexec.exe"));
    }

    Some(PathBuf::from(clean_path))
}

impl AppEntry {
    /// Checks the health status of this integrated application.
    /// - Broken (Red): Install folder or main executable missing.
    /// - Degraded (Yellow): Executable exists, but desktop file, symlink, or icon is missing.
    /// - Healthy (Green): All paths are intact.
    pub fn check_status(&self) -> AppStatus {
        if let Some(ref ptype) = self.package_type {
            if ptype == "Registry" {
                let mut broken_issues = Vec::new();
                if let Some(ref cmd) = self.uninstall_cmd {
                    if cmd.trim().is_empty() {
                        broken_issues.push("Không có lệnh gỡ cài đặt (UninstallString)".to_string());
                    } else if let Some(exec_path) = extract_exec_path(cmd) {
                        if !exec_path.exists() {
                            broken_issues.push(format!(
                                "File chạy gỡ cài đặt không tồn tại: {}",
                                exec_path.to_string_lossy()
                            ));
                        }
                    } else {
                        broken_issues.push("Không thể phân tích file chạy từ lệnh gỡ cài đặt".to_string());
                    }
                } else {
                    broken_issues.push("Không có thông tin lệnh gỡ cài đặt".to_string());
                }

                if !broken_issues.is_empty() {
                    return AppStatus::Broken(broken_issues);
                }
                return AppStatus::Healthy;
            } else if ptype != "Local" {
                return AppStatus::Healthy;
            }
        }

        let mut broken_issues = Vec::new();
        let mut degraded_issues = Vec::new();

        // 1. Check install path (folder)
        let install_path = Path::new(&self.install_path);
        if !install_path.exists() {
            broken_issues.push(format!("Thư mục cài đặt không tồn tại: {}", self.install_path));
        } else if !install_path.is_dir() {
            broken_issues.push(format!("Đường dẫn cài đặt không phải thư mục: {}", self.install_path));
        }

        // 2. Check main executable
        let exec_path = Path::new(&self.exec_path);
        if !exec_path.exists() {
            broken_issues.push(format!("File chạy (executable) không tồn tại: {}", self.exec_path));
        } else if !exec_path.is_file() {
            broken_issues.push(format!("Đường dẫn file chạy không phải là file: {}", self.exec_path));
        } else {
            // Check if executable permission is set
            #[cfg(unix)]
            if let Ok(metadata) = fs::metadata(exec_path) {
                let mode = metadata.permissions().mode();
                if mode & 0o111 == 0 {
                    broken_issues.push(format!("File chạy thiếu quyền thực thi (+x): {}", self.exec_path));
                }
            }
        }

        // If there are broken issues, it's immediately Broken (Red)
        if !broken_issues.is_empty() {
            return AppStatus::Broken(broken_issues);
        }

        // 3. Check desktop shortcut
        let desktop_path = Path::new(&self.desktop_file);
        if !desktop_path.exists() {
            degraded_issues.push(format!("File launcher (.desktop) bị thiếu: {}", self.desktop_file));
        }

        // 4. Check symlink in local bin
        if let Some(ref symlink_str) = self.symlink_file {
            let symlink_path = Path::new(symlink_str);
            if !symlink_path.exists() {
                degraded_issues.push(format!(
                    "Đường dẫn command-line (symlink) bị hỏng hoặc thiếu: {}",
                    symlink_str
                ));
            } else {
                // Check if it actually points to the executable
                if let Ok(target) = fs::read_link(symlink_path)
                    && target != exec_path
                {
                    degraded_issues.push(format!("Symlink trỏ sai đích: {:?} -> {:?}", symlink_path, target));
                }
            }
        }

        // 5. Check icon
        if let Some(ref icon_str) = self.icon_path {
            // Desktop entries can use system icons (just names like 'firefox') or absolute paths.
            // If it starts with / we check its existence.
            if icon_str.starts_with('/') {
                let icon_path = Path::new(icon_str);
                if !icon_path.exists() {
                    degraded_issues.push(format!("File ảnh icon không tồn tại: {}", icon_str));
                }
            }
        }

        if !degraded_issues.is_empty() {
            AppStatus::Degraded(degraded_issues)
        } else {
            AppStatus::Healthy
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        let managed_dir = home.join("Applications").to_string_lossy().to_string();
        Config {
            settings: Settings { managed_dir },
            apps: Vec::new(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".config").join("universe-manager").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();

        // Migration logic: If new path doesn't exist, check old path
        if !path.exists() {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
            let old_path = home.join(".config").join("port-integrator").join("config.json");
            if old_path.exists()
                && let Ok(content) = fs::read_to_string(&old_path)
                && let Ok(mut config) = serde_json::from_str::<Config>(&content)
            {
                // Ensure managed_dir is absolute
                if config.settings.managed_dir.is_empty() {
                    config.settings.managed_dir = home.join("Applications").to_string_lossy().to_string();
                }
                // Save immediately to the new location to complete migration
                let _ = config.save();
                return config;
            }
            return Config::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                match serde_json::from_str::<Config>(&content) {
                    Ok(mut config) => {
                        // Ensure managed_dir is absolute
                        if config.settings.managed_dir.is_empty() {
                            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
                            config.settings.managed_dir = home.join("Applications").to_string_lossy().to_string();
                        }
                        config
                    }
                    Err(_) => Config::default(),
                }
            }
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn add_app(&mut self, entry: AppEntry) {
        // Remove existing app with the same ID if exists
        self.apps.retain(|a| a.id != entry.id);
        self.apps.push(entry);
    }

    pub fn remove_app(&mut self, id: &str) {
        self.apps.retain(|a| a.id != id);
    }
}
