//! UNIVERSAL S2 mount micro: truy vấn chỉ đọc (fuse + liệt kê + đọc config).
//! Chặn luồng (tầng api bọc `fastlane`); parse ExecStart giữ hành vi cũ.

use crate::actions::mount_creator::{MountConfig, shlex_split, validate_service_name};
use crate::actions::mount_files::service_path;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// UNIVERSAL: 1 service systemd phát hiện từ đĩa.
#[derive(serde::Serialize, Debug)]
pub struct SystemdServiceInfo {
    pub name: String,
    pub is_user: bool,
    pub status: String,
    pub enabled: bool,
}

/// UNIVERSAL: true khi có `fusermount3` hoặc `fusermount` trong PATH.
pub fn fuse_installed() -> Result<bool, String> {
    for bin in ["fusermount3", "fusermount"] {
        match Command::new("which").arg(bin).output() {
            Ok(out) if out.status.success() => return Ok(true),
            _ => continue,
        }
    }
    Ok(false)
}

/// UNIVERSAL: đọc lại MountConfig từ file unit (parse Description/ExecStart).
pub fn read_service_config(service_name: &str, is_user: bool) -> Result<MountConfig, String> {
    validate_service_name(service_name)?;
    let path = service_path(service_name, is_user)?;
    if !path.exists() {
        return Err(format!("Service file not found at: {path:?}"));
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Failed to read service file: {e}"))?;
    Ok(parse_service_config(service_name, is_user, &content))
}

/// UNIVERSAL: parse thuần nội dung unit → MountConfig (dễ test, không chạm đĩa).
pub fn parse_service_config(service_name: &str, is_user: bool, content: &str) -> MountConfig {
    let mut config = MountConfig {
        service_name: service_name.to_string(),
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
        if let Some(desc) = line.strip_prefix("Description=") {
            config.description = desc.to_string();
        } else if let Some(exec) = line.strip_prefix("ExecStart=") {
            apply_exec_flags(&mut config, exec.trim());
        }
    }
    config
}

/// UNIVERSAL: gắn cờ ExecStart (`--opt val` hoặc `--opt=val`) vào config.
fn apply_exec_flags(config: &mut MountConfig, exec: &str) {
    let parts = shlex_split(exec);
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == "mount" && i + 2 < parts.len() {
            // UNIVERSAL: remote rclone là `name:path` (1 dấu `:`), khác `::` của cut_remote_path.
            let full = parts[i + 1].as_str();
            let (r_name, r_path) = match full.find(':') {
                Some(idx) => (full[..idx].to_string(), full[idx + 1..].to_string()),
                None => (full.to_string(), String::new()),
            };
            config.remote_name = r_name;
            config.remote_path = r_path.trim_start_matches('/').to_string();
            config.mount_path = parts[i + 2].clone();
            i += 2;
        } else if let Some(val) = flag_value(&parts, &mut i, "--vfs-cache-mode") {
            config.vfs_cache_mode = val;
        } else if let Some(val) = flag_value(&parts, &mut i, "--vfs-cache-max-size") {
            config.vfs_cache_max_size = val;
        } else if let Some(val) = flag_value(&parts, &mut i, "--vfs-cache-max-age") {
            config.vfs_cache_max_age = val;
        } else if let Some(val) = flag_value(&parts, &mut i, "--dir-cache-time") {
            config.dir_cache_time = val;
        } else if let Some(val) = flag_value(&parts, &mut i, "--buffer-size") {
            config.buffer_size = val;
        } else if parts[i] == "--allow-other" {
            config.allow_other = true;
        } else if parts[i] == "--read-only" {
            config.read_only = true;
        }
        i += 1;
    }
}

/// UNIVERSAL: lấy giá trị cờ dạng `--opt val` hoặc `--opt=val`.
fn flag_value(parts: &[String], i: &mut usize, flag: &str) -> Option<String> {
    let cur = parts.get(*i)?.as_str();
    if cur == flag {
        let next = parts.get(*i + 1)?.clone();
        *i += 1;
        return Some(next);
    }
    cur.strip_prefix(&format!("{flag}=")).map(|v| v.to_string())
}

/// UNIVERSAL: quét 1 thư mục unit, giữ lại service có `rclone mount`.
fn scan_dir(dir: PathBuf, is_user: bool, out: &mut Vec<SystemdServiceInfo>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".service") {
            continue;
        }
        let content = fs::read_to_string(entry.path()).unwrap_or_default();
        if !(content.contains("ExecStart=") && content.contains("rclone mount")) {
            continue;
        }
        let mut status_cmd = Command::new("systemctl");
        if is_user {
            status_cmd.arg("--user");
        }
        let active = status_cmd
            .arg("is-active")
            .arg(&name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let mut enable_cmd = Command::new("systemctl");
        if is_user {
            enable_cmd.arg("--user");
        }
        let enabled = enable_cmd
            .arg("is-enabled")
            .arg(&name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        out.push(SystemdServiceInfo {
            name: name.strip_suffix(".service").unwrap_or(&name).to_string(),
            is_user,
            status: if active { "running".to_string() } else { "stopped".to_string() },
            enabled,
        });
    }
}

/// UNIVERSAL: liệt kê service rclone mount ở cả user và system.
pub fn scan_mount_services() -> Result<Vec<SystemdServiceInfo>, String> {
    let mut services = Vec::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    scan_dir(PathBuf::from(home).join(".config/systemd/user"), true, &mut services);
    scan_dir(PathBuf::from("/etc/systemd/system"), false, &mut services);
    Ok(services)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::mount_creator::render_unit;

    #[test]
    fn parse_roundtrips_rendered_unit() {
        let cfg = MountConfig {
            service_name: "demo".to_string(),
            is_user_level: true,
            remote_name: "gdrive".to_string(),
            remote_path: "docs".to_string(),
            mount_path: "/mnt/gdrive".to_string(),
            description: "demo mount".to_string(),
            vfs_cache_mode: "writes".to_string(),
            vfs_cache_max_size: "1G".to_string(),
            vfs_cache_max_age: "1h".to_string(),
            dir_cache_time: "5m".to_string(),
            buffer_size: "16M".to_string(),
            allow_other: true,
            read_only: true,
        };
        let parsed = parse_service_config("demo", true, &render_unit(&cfg, "/usr/bin/rclone"));
        assert_eq!(parsed.remote_name, "gdrive");
        assert_eq!(parsed.remote_path, "docs");
        assert_eq!(parsed.mount_path, "/mnt/gdrive");
        assert_eq!(parsed.buffer_size, "16M");
        assert!(parsed.allow_other && parsed.read_only);
    }
}
