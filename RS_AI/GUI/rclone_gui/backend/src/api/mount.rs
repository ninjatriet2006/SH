/*
[INTEGRITY NOTES]
Mục đích: Cung cấp API backend cho việc mount rclone và quản lý Systemd service.
Trách nhiệm:
 - Kiểm tra fuse/fuse3.
 - Khởi tạo, Dừng, Quản lý rclone mount process.
 - Tạo và quản lý file .service cho User và System level.
Các module tương tác: lib.rs, frontend (qua Tauri command), bridge/mount_api.ts
*/

use crate::logic::fastlane::fastlane;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct MountConfig {
    pub service_name: String,
    pub is_user_level: bool,
    pub remote_name: String,
    pub remote_path: String,
    pub mount_path: String,
    pub description: String,
    pub vfs_cache_mode: String,
    pub vfs_cache_max_size: String,
    pub vfs_cache_max_age: String,
    pub dir_cache_time: String,
    pub buffer_size: String,
    pub allow_other: bool,
    pub read_only: bool,
}

#[derive(serde::Serialize, Debug)]
pub struct SystemdServiceInfo {
    pub name: String,
    pub is_user: bool,
    pub status: String,
    pub enabled: bool,
}

/// Kiểm tra hệ thống đã cài đặt FUSE chưa
pub async fn check_fuse_installed() -> Result<bool, String> {
    fastlane(|| {
        // Kiểm tra fuse hoặc fuse3 hoặc fusermount
        let fuse3 = Command::new("which").arg("fusermount3").output();
        let fuse = Command::new("which").arg("fusermount").output();

        if let Ok(out) = fuse3 {
            if out.status.success() {
                return Ok(true);
            }
        }
        if let Ok(out) = fuse {
            if out.status.success() {
                return Ok(true);
            }
        }

        Ok(false)
    })
    .await
}

/// Helper để lấy đường dẫn systemd service
fn validate_service_name(service_name: &str) -> Result<(), String> {
    let valid = !service_name.is_empty()
        && service_name.len() <= 128
        && service_name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err("service_name must be a strict ASCII identifier ([A-Za-z0-9_-], max 128)".to_string())
    }
}

fn service_dir(is_user: bool) -> PathBuf {
    if is_user {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        PathBuf::from(home).join(".config/systemd/user")
    } else {
        PathBuf::from("/etc/systemd/system")
    }
}

fn normalized_absolute(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("service path must be absolute".to_string());
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir => normalized.push("/"),
            std::path::Component::Normal(part) => normalized.push(part),
            _ => return Err("service path contains a non-canonical component".to_string()),
        }
    }
    Ok(normalized)
}

fn get_service_path(service_name: &str, is_user: bool) -> Result<PathBuf, String> {
    validate_service_name(service_name)?;
    let dir = normalized_absolute(&service_dir(is_user))?;
    let candidate = normalized_absolute(&dir.join(format!("{service_name}.service")))?;
    if candidate.parent() != Some(dir.as_path()) || !candidate.starts_with(&dir) {
        return Err("service path escapes the systemd service directory".to_string());
    }
    Ok(candidate)
}

fn require_confirmation(confirmed: bool) -> Result<(), String> {
    if confirmed {
        Ok(())
    } else {
        Err("confirmed=true is required for privileged or destructive mount actions".to_string())
    }
}

fn validate_action(action: &str) -> Result<(), String> {
    if matches!(action, "start" | "stop" | "enable" | "disable" | "restart") {
        Ok(())
    } else {
        Err("unsupported systemctl action".to_string())
    }
}

/// Tạo file Systemd Service cho rclone mount
pub async fn create_mount_service(config: MountConfig, confirmed: bool) -> Result<String, String> {
    validate_service_name(&config.service_name)?;
    if !config.is_user_level {
        require_confirmation(confirmed)?;
    }
    fastlane(move || {
        let service_path = get_service_path(&config.service_name, config.is_user_level)?;

        // Lấy đường dẫn thực tế của rclone
        let rclone_path = String::from_utf8_lossy(
            &Command::new("which")
                .arg("rclone")
                .output()
                .map_err(|e| e.to_string())?
                .stdout,
        )
        .trim()
        .to_string();

        if rclone_path.is_empty() {
            return Err("Không tìm thấy lệnh rclone trong hệ thống!".to_string());
        }

        let remote = if config.remote_path.is_empty() {
            format!("{}:", config.remote_name)
        } else {
            let path = config.remote_path.trim_start_matches('/');
            format!("{}:{}", config.remote_name, path)
        };
        let mut exec_start = format!("{} mount \"{}\" \"{}\"", rclone_path, remote, config.mount_path);

        if !config.vfs_cache_mode.is_empty() {
            exec_start.push_str(&format!(" --vfs-cache-mode {}", config.vfs_cache_mode));
        }
        if !config.vfs_cache_max_size.is_empty() {
            exec_start.push_str(&format!(" --vfs-cache-max-size {}", config.vfs_cache_max_size));
        }
        if !config.vfs_cache_max_age.is_empty() {
            exec_start.push_str(&format!(" --vfs-cache-max-age {}", config.vfs_cache_max_age));
        }
        if !config.dir_cache_time.is_empty() {
            exec_start.push_str(&format!(" --dir-cache-time {}", config.dir_cache_time));
        }
        if !config.buffer_size.is_empty() {
            exec_start.push_str(&format!(" --buffer-size {}", config.buffer_size));
        }
        if config.allow_other {
            exec_start.push_str(" --allow-other");
        }
        if config.read_only {
            exec_start.push_str(" --read-only");
        }

        let service_content = format!(
            "[Unit]
Description={}
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
ExecStartPre=/bin/mkdir -p \"{}\"
ExecStart={}
ExecStop=/bin/fusermount -uz \"{}\"
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
",
            config.description, config.mount_path, exec_start, config.mount_path
        );

        // Xử lý ghi file
        if !config.is_user_level {
            // Cần quyền root
            let tmp_path = format!("/tmp/{}.service", config.service_name);
            fs::write(&tmp_path, &service_content).map_err(|e| e.to_string())?;

            let pkexec = Command::new("pkexec")
                .arg("cp")
                .arg(&tmp_path)
                .arg(service_path.to_string_lossy().as_ref())
                .output()
                .map_err(|e| e.to_string())?;

            if !pkexec.status.success() {
                return Err(format!(
                    "Lỗi cấp quyền root: {}",
                    String::from_utf8_lossy(&pkexec.stderr)
                ));
            }

            let _ = Command::new("pkexec").arg("systemctl").arg("daemon-reload").output();
        } else {
            fs::write(&service_path, &service_content).map_err(|e| e.to_string())?;
            let _ = Command::new("systemctl").arg("--user").arg("daemon-reload").output();
        }

        Ok("Tạo systemd service thành công!".to_string())
    })
    .await
}

/// Xoá systemd service
pub async fn delete_mount_service(service_name: String, is_user: bool, confirmed: bool) -> Result<String, String> {
    validate_service_name(&service_name)?;
    require_confirmation(confirmed)?;
    // Stop service first
    manage_mount_service(service_name.clone(), is_user, "stop".to_string(), true)
        .await
        .ok();
    manage_mount_service(service_name.clone(), is_user, "disable".to_string(), true)
        .await
        .ok();

    fastlane(move || {
        let service_path = get_service_path(&service_name, is_user)?;

        if !is_user {
            let pkexec = Command::new("pkexec")
                .arg("rm")
                .arg("-f")
                .arg(service_path.to_string_lossy().as_ref())
                .output()
                .map_err(|e| e.to_string())?;

            if !pkexec.status.success() {
                return Err(format!(
                    "Lỗi cấp quyền root: {}",
                    String::from_utf8_lossy(&pkexec.stderr)
                ));
            }
            let _ = Command::new("pkexec").arg("systemctl").arg("daemon-reload").output();
        } else {
            let _ = fs::remove_file(&service_path);
            let _ = Command::new("systemctl").arg("--user").arg("daemon-reload").output();
        }

        Ok("Đã xoá systemd service.".to_string())
    })
    .await
}

/// Gửi lệnh start/stop/enable/disable cho systemd
pub async fn manage_mount_service(
    service_name: String,
    is_user: bool,
    action: String,
    confirmed: bool,
) -> Result<String, String> {
    validate_service_name(&service_name)?;
    validate_action(&action)?;
    if !is_user || matches!(action.as_str(), "stop" | "disable" | "restart") {
        require_confirmation(confirmed)?;
    }
    fastlane(move || {
        let mut cmd = if is_user {
            let mut c = Command::new("systemctl");
            c.arg("--user");
            c
        } else {
            Command::new("pkexec") // System level cần pkexec để gọi systemctl
        };

        if !is_user {
            cmd.arg("systemctl");
        }

        cmd.arg(&action);
        cmd.arg(&service_name);

        let output = cmd.output().map_err(|e| e.to_string())?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }
        Ok(format!("Lệnh {} thành công", action))
    })
    .await
}

/// Lấy danh sách các file .service từ user và system
pub async fn get_mount_service_config(service_name: String, is_user: bool) -> Result<MountConfig, String> {
    validate_service_name(&service_name)?;
    fastlane(move || {
        let service_path = get_service_path(&service_name, is_user)?;
        if !service_path.exists() {
            return Err(format!("Service file not found at: {:?}", service_path));
        }

        let content = fs::read_to_string(&service_path).map_err(|e| format!("Failed to read service file: {}", e))?;

        let mut config = MountConfig {
            service_name: service_name.clone(),
            is_user_level: is_user,
            remote_name: String::new(),
            remote_path: String::new(),
            mount_path: String::new(),
            description: String::new(),
            vfs_cache_mode: String::new(),
            vfs_cache_max_size: String::new(),
            vfs_cache_max_age: String::new(),
            dir_cache_time: String::new(),
            buffer_size: String::new(),
            allow_other: false,
            read_only: false,
        };

        for line in content.lines() {
            if line.starts_with("Description=") {
                config.description = line.trim_start_matches("Description=").to_string();
            } else if line.starts_with("ExecStart=") {
                let exec_start_content = line.trim_start_matches("ExecStart=").trim();
                let parts = shlex_split(exec_start_content);
                let mut i = 0;
                while i < parts.len() {
                    if parts[i] == "mount" && i + 2 < parts.len() {
                        let remote_full = parts[i + 1].to_string();
                        let (r_name, r_path) = crate::core::path::cut_remote_path(&remote_full);
                        config.remote_name = r_name;
                        config.remote_path = r_path.trim_start_matches('/').to_string();
                        config.mount_path = parts[i + 2].to_string();
                        i += 2;
                    } else if parts[i].starts_with("--vfs-cache-mode") {
                        if let Some(val) = parts[i].split('=').nth(1) {
                            config.vfs_cache_mode = val.to_string();
                        } else if i + 1 < parts.len() {
                            config.vfs_cache_mode = parts[i + 1].to_string();
                            i += 1;
                        }
                    } else if parts[i].starts_with("--vfs-cache-max-size") {
                        if let Some(val) = parts[i].split('=').nth(1) {
                            config.vfs_cache_max_size = val.to_string();
                        } else if i + 1 < parts.len() {
                            config.vfs_cache_max_size = parts[i + 1].to_string();
                            i += 1;
                        }
                    } else if parts[i].starts_with("--vfs-cache-max-age") {
                        if let Some(val) = parts[i].split('=').nth(1) {
                            config.vfs_cache_max_age = val.to_string();
                        } else if i + 1 < parts.len() {
                            config.vfs_cache_max_age = parts[i + 1].to_string();
                            i += 1;
                        }
                    } else if parts[i].starts_with("--dir-cache-time") {
                        if let Some(val) = parts[i].split('=').nth(1) {
                            config.dir_cache_time = val.to_string();
                        } else if i + 1 < parts.len() {
                            config.dir_cache_time = parts[i + 1].to_string();
                            i += 1;
                        }
                    } else if parts[i] == "--allow-other" {
                        config.allow_other = true;
                    } else if parts[i] == "--read-only" {
                        config.read_only = true;
                    }
                    i += 1;
                }
            }
        }

        Ok(config)
    })
    .await
}

pub async fn list_mount_services() -> Result<Vec<SystemdServiceInfo>, String> {
    fastlane(|| {
        let mut services = Vec::new();

        // Scan user services
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let user_dir = PathBuf::from(home).join(".config/systemd/user");
        if let Ok(entries) = fs::read_dir(user_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".service") {
                    let content = fs::read_to_string(entry.path()).unwrap_or_default();
                    if content.contains("ExecStart=") && content.contains("rclone mount") {
                        let service_name = name.strip_suffix(".service").unwrap_or(&name).to_string();

                        let status_out = Command::new("systemctl")
                            .arg("--user")
                            .arg("is-active")
                            .arg(&name)
                            .output();
                        let is_active = status_out.map(|o| o.status.success()).unwrap_or(false);

                        let enable_out = Command::new("systemctl")
                            .arg("--user")
                            .arg("is-enabled")
                            .arg(&name)
                            .output();
                        let is_enabled = enable_out.map(|o| o.status.success()).unwrap_or(false);

                        services.push(SystemdServiceInfo {
                            name: service_name,
                            is_user: true,
                            status: if is_active {
                                "running".to_string()
                            } else {
                                "stopped".to_string()
                            },
                            enabled: is_enabled,
                        });
                    }
                }
            }
        }

        // Scan system services
        if let Ok(entries) = fs::read_dir("/etc/systemd/system") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".service") {
                    let content = fs::read_to_string(entry.path()).unwrap_or_default();
                    if content.contains("ExecStart=") && content.contains("rclone mount") {
                        let service_name = name.strip_suffix(".service").unwrap_or(&name).to_string();
                        let status_out = Command::new("systemctl").arg("is-active").arg(&name).output();
                        let is_active = status_out.map(|o| o.status.success()).unwrap_or(false);

                        let enable_out = Command::new("systemctl").arg("is-enabled").arg(&name).output();
                        let is_enabled = enable_out.map(|o| o.status.success()).unwrap_or(false);

                        services.push(SystemdServiceInfo {
                            name: service_name,
                            is_user: false,
                            status: if is_active {
                                "running".to_string()
                            } else {
                                "stopped".to_string()
                            },
                            enabled: is_enabled,
                        });
                    }
                }
            }
        }
        Ok(services)
    })
    .await
}

/// Helper function to split a string similar to shell parsing (handles quotes)
fn shlex_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for c in input.chars() {
        if escape_next {
            current.push(c);
            escape_next = false;
        } else if c == '\\' {
            if in_single_quote {
                current.push(c);
            } else {
                escape_next = true;
            }
        } else if c == '\'' {
            if in_double_quote {
                current.push(c);
            } else {
                in_single_quote = !in_single_quote;
            }
        } else if c == '"' {
            if in_single_quote {
                current.push(c);
            } else {
                in_double_quote = !in_double_quote;
            }
        } else if c.is_whitespace() {
            if in_single_quote || in_double_quote {
                current.push(c);
            } else if !current.is_empty() {
                parts.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_name_is_strict_identifier() {
        for valid in ["rclone-drive", "RCLONE_2", "a"] {
            assert!(validate_service_name(valid).is_ok(), "{valid}");
        }
        for malicious in [
            "",
            "../evil",
            "a.service",
            "a/b",
            "a;id",
            "$(id)",
            "a b",
            "a\nExecStart=/bin/id",
        ] {
            assert!(validate_service_name(malicious).is_err(), "{malicious:?}");
        }
    }

    #[test]
    fn service_path_is_contained_and_canonical() {
        let path = get_service_path("rclone-safe", false).expect("safe path");
        assert_eq!(path, PathBuf::from("/etc/systemd/system/rclone-safe.service"));
        assert_eq!(path.parent(), Some(Path::new("/etc/systemd/system")));
        assert!(get_service_path("../../tmp/owned", false).is_err());
    }

    #[test]
    fn privileged_and_destructive_actions_require_confirmation() {
        assert!(require_confirmation(false).is_err());
        assert!(require_confirmation(true).is_ok());
        for action in ["start", "stop", "enable", "disable", "restart"] {
            assert!(validate_action(action).is_ok());
        }
        for action in ["status;id", "daemon-reload", "", "--help"] {
            assert!(validate_action(action).is_err());
        }
    }
}
