//! UNIVERSAL S2 mount micro: MountConfig + validate + render unit thuần.
//! Thuần (dễ test): validate/render không chạm đĩa ngoài `which rclone`.

use serde::{Deserialize, Serialize};

/// UNIVERSAL: cấu hình 1 service rclone mount (JSON giữ nguyên).
#[derive(Serialize, Deserialize, Clone, Debug)]
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

/// UNIVERSAL: tên service là identifier ASCII chặt ([A-Za-z0-9_-], ≤128).
pub fn validate_service_name(service_name: &str) -> Result<(), String> {
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

/// UNIVERSAL: hành động đặc quyền/phá hủy đòi `confirmed=true`.
pub fn require_confirmation(confirmed: bool) -> Result<(), String> {
    if confirmed {
        Ok(())
    } else {
        Err("confirmed=true is required for privileged or destructive mount actions".to_string())
    }
}

/// UNIVERSAL: chỉ cho phép action systemctl trong allowlist.
pub fn validate_action(action: &str) -> Result<(), String> {
    if matches!(action, "start" | "stop" | "enable" | "disable" | "restart") {
        Ok(())
    } else {
        Err("unsupported systemctl action".to_string())
    }
}

/// UNIVERSAL: validate tối thiểu 1 MountConfig (tên + remote + điểm mount tuyệt đối).
pub fn validate_mount_config(config: &MountConfig) -> Result<(), String> {
    validate_service_name(&config.service_name)?;
    if config.remote_name.trim().is_empty() {
        return Err("remote_name must not be empty".to_string());
    }
    if !config.mount_path.starts_with('/') {
        return Err("mount_path must be absolute".to_string());
    }
    Ok(())
}

/// UNIVERSAL: dựng chuỗi remote `name:path` (path rỗng → `name:`).
pub fn build_remote(config: &MountConfig) -> String {
    if config.remote_path.is_empty() {
        format!("{}:", config.remote_name)
    } else {
        format!("{}:{}", config.remote_name, config.remote_path.trim_start_matches('/'))
    }
}

/// UNIVERSAL: dựng dòng ExecStart từ config + đường dẫn rclone đã resolve.
pub fn build_exec_start(config: &MountConfig, rclone_path: &str) -> String {
    let mut cmd = format!("{} mount \"{}\" \"{}\"", rclone_path, build_remote(config), config.mount_path);
    if !config.vfs_cache_mode.is_empty() {
        cmd.push_str(&format!(" --vfs-cache-mode {}", config.vfs_cache_mode));
    }
    if !config.vfs_cache_max_size.is_empty() {
        cmd.push_str(&format!(" --vfs-cache-max-size {}", config.vfs_cache_max_size));
    }
    if !config.vfs_cache_max_age.is_empty() {
        cmd.push_str(&format!(" --vfs-cache-max-age {}", config.vfs_cache_max_age));
    }
    if !config.dir_cache_time.is_empty() {
        cmd.push_str(&format!(" --dir-cache-time {}", config.dir_cache_time));
    }
    if !config.buffer_size.is_empty() {
        cmd.push_str(&format!(" --buffer-size {}", config.buffer_size));
    }
    if config.allow_other {
        cmd.push_str(" --allow-other");
    }
    if config.read_only {
        cmd.push_str(" --read-only");
    }
    cmd
}

/// UNIVERSAL: render toàn bộ nội dung file `.service` (thuần, dễ test).
pub fn render_unit(config: &MountConfig, rclone_path: &str) -> String {
    let exec_start = build_exec_start(config, rclone_path);
    format!(
        "[Unit]\nDescription={}\nAfter=network-online.target\nWants=network-online.target\n\n[Service]\nType=notify\nExecStartPre=/bin/mkdir -p \"{}\"\nExecStart={}\nExecStop=/bin/fusermount -uz \"{}\"\nRestart=on-failure\nRestartSec=10\n\n[Install]\nWantedBy=default.target\n",
        config.description, config.mount_path, exec_start, config.mount_path
    )
}

/// UNIVERSAL: resolve đường dẫn rclone qua `which rclone` (lần chạm đĩa duy nhất).
pub fn resolve_rclone_path() -> Result<String, String> {
    let out = std::process::Command::new("which")
        .arg("rclone")
        .output()
        .map_err(|e| e.to_string())?;
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        return Err("Không tìm thấy lệnh rclone trong hệ thống!".to_string());
    }
    Ok(path)
}

/// UNIVERSAL: tách chuỗi shell (giữ quote) — dùng khi parse lại ExecStart.
pub fn shlex_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape_next = false;
    for c in input.chars() {
        if escape_next {
            current.push(c);
            escape_next = false;
        } else if c == '\\' {
            if in_single {
                current.push(c);
            } else {
                escape_next = true;
            }
        } else if c == '\'' {
            if in_double {
                current.push(c);
            } else {
                in_single = !in_single;
            }
        } else if c == '"' {
            if in_single {
                current.push(c);
            } else {
                in_double = !in_double;
            }
        } else if c.is_whitespace() {
            if in_single || in_double {
                current.push(c);
            } else if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
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

    fn sample() -> MountConfig {
        MountConfig {
            service_name: "rclone-gdrive".to_string(),
            is_user_level: true,
            remote_name: "gdrive".to_string(),
            remote_path: "docs".to_string(),
            mount_path: "/mnt/gdrive".to_string(),
            description: "GDrive mount".to_string(),
            vfs_cache_mode: "writes".to_string(),
            vfs_cache_max_size: "1G".to_string(),
            vfs_cache_max_age: "1h".to_string(),
            dir_cache_time: "5m".to_string(),
            buffer_size: "16M".to_string(),
            allow_other: true,
            read_only: false,
        }
    }

    #[test]
    fn render_unit_contains_exec_and_flags() {
        let unit = render_unit(&sample(), "/usr/bin/rclone");
        assert!(unit.contains("Description=GDrive mount"));
        assert!(unit.contains("/usr/bin/rclone mount \"gdrive:docs\" \"/mnt/gdrive\""));
        for flag in ["--vfs-cache-mode writes", "--vfs-cache-max-size 1G", "--allow-other"] {
            assert!(unit.contains(flag), "{flag}");
        }
        assert!(unit.contains("WantedBy=default.target"));
    }

    #[test]
    fn render_empty_remote_path_keeps_colon() {
        let mut c = sample();
        c.remote_path.clear();
        assert!(build_exec_start(&c, "/usr/bin/rclone").contains("\"gdrive:\""));
    }

    #[test]
    fn validate_rejects_bad_config() {
        let mut c = sample();
        c.service_name = "../evil".to_string();
        assert!(validate_mount_config(&c).is_err());
        let mut c = sample();
        c.remote_name.clear();
        assert!(validate_mount_config(&c).is_err());
        let mut c = sample();
        c.mount_path = "relative/path".to_string();
        assert!(validate_mount_config(&c).is_err());
        assert!(validate_mount_config(&sample()).is_ok());
    }

    #[test]
    fn shlex_keeps_quoted_remote_with_space() {
        let parts = shlex_split("/usr/bin/rclone mount \"gdrive:my docs\" \"/mnt/gdrive\" --allow-other");
        assert_eq!(parts[2], "gdrive:my docs");
        assert_eq!(parts[3], "/mnt/gdrive");
    }
}
