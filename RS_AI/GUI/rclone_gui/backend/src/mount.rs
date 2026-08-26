/*
[INTEGRITY NOTES]
Mục đích: Cung cấp API backend cho việc mount rclone và quản lý Systemd service.
Trách nhiệm:
 - Kiểm tra fuse/fuse3.
 - Khởi tạo, Dừng, Quản lý rclone mount process.
 - Tạo và quản lý file .service cho User và System level.
Các module tương tác: lib.rs, frontend (qua Tauri command), bridge/mount_api.ts
*/

use std::process::Command;
use std::fs;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct MountConfig {
    pub service_name: String,
    pub is_user_level: bool,
    pub remote: String,
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

#[derive(serde::Serialize)]
pub struct SystemdServiceInfo {
    pub name: String,
    pub is_user: bool,
    pub status: String,
    pub enabled: bool,
}

/// Kiểm tra hệ thống đã cài đặt FUSE chưa
#[tauri::command]
pub async fn check_fuse_installed() -> Result<bool, String> {
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
}

/// Helper để lấy đường dẫn systemd service
fn get_service_path(service_name: &str, is_user: bool) -> PathBuf {
    if is_user {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let dir = PathBuf::from(home).join(".config/systemd/user");
        let _ = fs::create_dir_all(&dir);
        dir.join(format!("{}.service", service_name))
    } else {
        PathBuf::from(format!("/etc/systemd/system/{}.service", service_name))
    }
}

/// Tạo file Systemd Service cho rclone mount
#[tauri::command]
pub async fn create_mount_service(config: MountConfig) -> Result<String, String> {
    let service_path = get_service_path(&config.service_name, config.is_user_level);
    
    // Lấy đường dẫn thực tế của rclone
    let rclone_path = String::from_utf8_lossy(
        &Command::new("which").arg("rclone").output().map_err(|e| e.to_string())?.stdout
    ).trim().to_string();
    
    if rclone_path.is_empty() {
        return Err("Không tìm thấy lệnh rclone trong hệ thống!".to_string());
    }

    let mut exec_start = format!("{} mount \"{}\" \"{}\"", rclone_path, config.remote, config.mount_path);
    
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
            return Err(format!("Lỗi cấp quyền root: {}", String::from_utf8_lossy(&pkexec.stderr)));
        }
        
        let _ = Command::new("pkexec").arg("systemctl").arg("daemon-reload").output();
    } else {
        fs::write(&service_path, &service_content).map_err(|e| e.to_string())?;
        let _ = Command::new("systemctl").arg("--user").arg("daemon-reload").output();
    }

    Ok("Tạo systemd service thành công!".to_string())
}

/// Xoá systemd service
#[tauri::command]
pub async fn delete_mount_service(service_name: String, is_user: bool) -> Result<String, String> {
    let service_path = get_service_path(&service_name, is_user);
    
    // Stop service first
    manage_mount_service(service_name.clone(), is_user, "stop".to_string()).await.ok();
    manage_mount_service(service_name.clone(), is_user, "disable".to_string()).await.ok();
    
    if !is_user {
        let pkexec = Command::new("pkexec")
            .arg("rm")
            .arg("-f")
            .arg(service_path.to_string_lossy().as_ref())
            .output()
            .map_err(|e| e.to_string())?;
            
        if !pkexec.status.success() {
            return Err(format!("Lỗi cấp quyền root: {}", String::from_utf8_lossy(&pkexec.stderr)));
        }
        let _ = Command::new("pkexec").arg("systemctl").arg("daemon-reload").output();
    } else {
        let _ = fs::remove_file(&service_path);
        let _ = Command::new("systemctl").arg("--user").arg("daemon-reload").output();
    }
    
    Ok("Đã xoá systemd service.".to_string())
}

/// Gửi lệnh start/stop/enable/disable cho systemd
#[tauri::command]
pub async fn manage_mount_service(service_name: String, is_user: bool, action: String) -> Result<String, String> {
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
}

/// Lấy danh sách các file .service từ user và system
#[tauri::command]
pub async fn get_mount_service_config(service_name: String, is_user: bool) -> Result<MountConfig, String> {
    let service_path = get_service_path(&service_name, is_user);
    if !service_path.exists() {
        return Err(format!("Service file not found at: {:?}", service_path));
    }
    
    let content = fs::read_to_string(&service_path)
        .map_err(|e| format!("Failed to read service file: {}", e))?;
        
    let mut config = MountConfig {
        service_name: service_name.clone(),
        is_user_level: is_user,
        remote: String::new(),
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
                    let mut remote = parts[i+1].to_string();
                    if remote.ends_with(':') {
                        remote.pop();
                    }
                    config.remote = remote;
                    config.mount_path = parts[i+2].to_string();
                    i += 2;
                } else if parts[i].starts_with("--vfs-cache-mode") {
                    if let Some(val) = parts[i].split('=').nth(1) {
                        config.vfs_cache_mode = val.to_string();
                    } else if i + 1 < parts.len() {
                        config.vfs_cache_mode = parts[i+1].to_string();
                        i += 1;
                    }
                } else if parts[i].starts_with("--vfs-cache-max-size") {
                    if let Some(val) = parts[i].split('=').nth(1) {
                        config.vfs_cache_max_size = val.to_string();
                    } else if i + 1 < parts.len() {
                        config.vfs_cache_max_size = parts[i+1].to_string();
                        i += 1;
                    }
                } else if parts[i].starts_with("--vfs-cache-max-age") {
                    if let Some(val) = parts[i].split('=').nth(1) {
                        config.vfs_cache_max_age = val.to_string();
                    } else if i + 1 < parts.len() {
                        config.vfs_cache_max_age = parts[i+1].to_string();
                        i += 1;
                    }
                } else if parts[i].starts_with("--dir-cache-time") {
                    if let Some(val) = parts[i].split('=').nth(1) {
                        config.dir_cache_time = val.to_string();
                    } else if i + 1 < parts.len() {
                        config.dir_cache_time = parts[i+1].to_string();
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
}

#[tauri::command]
pub async fn list_mount_services() -> Result<Vec<SystemdServiceInfo>, String> {
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
                        status: if is_active { "running".to_string() } else { "stopped".to_string() },
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
                    let status_out = Command::new("systemctl")
                        .arg("is-active")
                        .arg(&name)
                        .output();
                    let is_active = status_out.map(|o| o.status.success()).unwrap_or(false);
                    
                    let enable_out = Command::new("systemctl")
                        .arg("is-enabled")
                        .arg(&name)
                        .output();
                    let is_enabled = enable_out.map(|o| o.status.success()).unwrap_or(false);
                    
                    services.push(SystemdServiceInfo {
                        name: service_name,
                        is_user: false,
                        status: if is_active { "running".to_string() } else { "stopped".to_string() },
                        enabled: is_enabled,
                    });
                }
            }
        }
    }
    Ok(services)
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
