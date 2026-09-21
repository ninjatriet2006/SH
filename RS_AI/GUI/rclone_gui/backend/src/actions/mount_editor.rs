//! UNIVERSAL S2 mount micro: editor (validate/render + đường dẫn + ghi/xóa file service).
//! Gộp từ `mount_creator` (validate/render thuần) + `mount_files` (đường dẫn/ghi/xóa).
//! Thuần (dễ test): validate/render không chạm đĩa ngoài `which rclone`.
//! Chặn luồng (gọi trong `fastlane` ở tầng api).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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

/// UNIVERSAL: thư mục chứa unit theo level (user `~/.config/systemd/user`, system `/etc/systemd/system`).
pub fn service_dir(is_user: bool) -> PathBuf {
    if is_user {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        PathBuf::from(home).join(".config/systemd/user")
    } else {
        PathBuf::from("/etc/systemd/system")
    }
}

/// UNIVERSAL: đường dẫn file `<name>.service` đã khóa trong thư mục service.
/// `validate_service_name` chỉ cho `[A-Za-z0-9_-]` nên đã chặn `/`, `..`, thoát thư mục.
pub fn service_path(service_name: &str, is_user: bool) -> Result<PathBuf, String> {
    validate_service_name(service_name)?;
    Ok(service_dir(is_user).join(format!("{service_name}.service")))
}

/// UNIVERSAL: hậu tố ngẫu nhiên cho file tạm (không đoán được, không va chạm).
fn random_suffix() -> String {
    // UNIVERSAL: ưu tiên /dev/urandom (đọc đúng 8 byte); rớt xuống pid + thời gian khi lỗi.
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        use std::io::Read;
        let mut buf = [0u8; 8];
        if f.read_exact(&mut buf).is_ok() {
            let mut s = String::with_capacity(16);
            for b in buf {
                s.push_str(&format!("{b:02x}"));
            }
            return s;
        }
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    format!("{:x}", nanos ^ (pid << 64 | 0x9e37_79b9_7f4a_7c15))
}

/// UNIVERSAL: file tạm system tên ngẫu nhiên khó đoán (tránh `/tmp/<tên>.service` cố định).
fn temp_service_path(service_name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/.{service_name}.{}.service", random_suffix()))
}

/// UNIVERSAL: ghi nội dung unit (user: viết trực tiếp; system: qua `pkexec cp` từ file tạm).
pub fn write_service_file(service_name: &str, is_user: bool, content: &str) -> Result<PathBuf, String> {
    let path = service_path(service_name, is_user)?;
    if !is_user {
        // UNIVERSAL: file tạm quyền chặt (0600) + tên ngẫu nhiên, xong xóa kể cả khi lỗi.
        let tmp_path = temp_service_path(service_name);
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut opts = fs::OpenOptions::new();
            opts.create_new(true).write(true).mode(0o600);
            use std::io::Write;
            let mut f = opts.open(&tmp_path).map_err(|e| e.to_string())?;
            f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        }
        let out = Command::new("pkexec")
            .arg("cp")
            .arg(&tmp_path)
            .arg(path.to_string_lossy().as_ref())
            .output()
            .map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&tmp_path);
        if !out.status.success() {
            return Err(format!("Lỗi cấp quyền root: {}", String::from_utf8_lossy(&out.stderr)));
        }
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, content).map_err(|e| e.to_string())?;
    }
    crate::actions::mount_control::daemon_reload(is_user);
    Ok(path)
}

/// UNIVERSAL: xóa file service (system qua `pkexec rm -f`) + daemon-reload.
pub fn remove_service_file(service_name: &str, is_user: bool) -> Result<(), String> {
    let path = service_path(service_name, is_user)?;
    if !is_user {
        let out = Command::new("pkexec")
            .arg("rm")
            .arg("-f")
            .arg(path.to_string_lossy().as_ref())
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("Lỗi cấp quyền root: {}", String::from_utf8_lossy(&out.stderr)));
        }
    } else {
        let _ = fs::remove_file(&path);
    }
    crate::actions::mount_control::daemon_reload(is_user);
    Ok(())
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

    #[test]
    fn service_path_is_contained_and_canonical() {
        let path = service_path("rclone-safe", false).expect("safe path");
        assert_eq!(path, PathBuf::from("/etc/systemd/system/rclone-safe.service"));
        assert!(service_path("../../tmp/owned", false).is_err());
        assert!(service_path("a/b", true).is_err());
        assert!(service_path("", true).is_err());
        assert!(service_path(&"a".repeat(129), true).is_err());
        // UNIVERSAL: user level khóa đúng thư mục user.
        assert_eq!(
            service_path("demo", true).expect("user path"),
            service_dir(true).join("demo.service")
        );
    }

    #[test]
    fn temp_path_is_unpredictable_and_not_fixed_name() {
        let a = temp_service_path("rclone-safe");
        let b = temp_service_path("rclone-safe");
        assert_ne!(a, b);
        assert_ne!(a, PathBuf::from("/tmp/rclone-safe.service"));
        assert!(a.to_string_lossy().starts_with("/tmp/.rclone-safe."));
    }
}
