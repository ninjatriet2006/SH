//! UNIVERSAL S2 mount micro: đường dẫn + ghi/xóa file service + daemon-reload.
//! Chặn luồng (gọi trong `fastlane` ở tầng api).

use crate::actions::mount_creator::validate_service_name;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// UNIVERSAL: thư mục chứa unit theo level (user `~/.config/systemd/user`, system `/etc/systemd/system`).
pub fn service_dir(is_user: bool) -> PathBuf {
    if is_user {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        PathBuf::from(home).join(".config/systemd/user")
    } else {
        PathBuf::from("/etc/systemd/system")
    }
}

/// UNIVERSAL: chuẩn hóa đường tuyệt đối, từ chối `..`/prefix lạ (chống thoát thư mục).
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

/// UNIVERSAL: đường dẫn file `<name>.service` đã khóa trong thư mục service.
pub fn service_path(service_name: &str, is_user: bool) -> Result<PathBuf, String> {
    validate_service_name(service_name)?;
    let dir = normalized_absolute(&service_dir(is_user))?;
    let candidate = normalized_absolute(&dir.join(format!("{service_name}.service")))?;
    if candidate.parent() != Some(dir.as_path()) || !candidate.starts_with(&dir) {
        return Err("service path escapes the systemd service directory".to_string());
    }
    Ok(candidate)
}

/// UNIVERSAL: ghi nội dung unit (user: viết trực tiếp; system: qua `pkexec cp` từ /tmp).
pub fn write_service_file(service_name: &str, is_user: bool, content: &str) -> Result<PathBuf, String> {
    let path = service_path(service_name, is_user)?;
    if !is_user {
        let tmp_path = format!("/tmp/{service_name}.service");
        fs::write(&tmp_path, content).map_err(|e| e.to_string())?;
        let out = Command::new("pkexec")
            .arg("cp")
            .arg(&tmp_path)
            .arg(path.to_string_lossy().as_ref())
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!("Lỗi cấp quyền root: {}", String::from_utf8_lossy(&out.stderr)));
        }
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&path, content).map_err(|e| e.to_string())?;
    }
    daemon_reload(is_user);
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
    daemon_reload(is_user);
    Ok(())
}

/// UNIVERSAL: nạp lại systemd sau khi thêm/xóa unit.
pub fn daemon_reload(is_user: bool) {
    if is_user {
        let _ = Command::new("systemctl").arg("--user").arg("daemon-reload").output();
    } else {
        let _ = Command::new("pkexec").arg("systemctl").arg("daemon-reload").output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_path_is_contained_and_canonical() {
        let path = service_path("rclone-safe", false).expect("safe path");
        assert_eq!(path, PathBuf::from("/etc/systemd/system/rclone-safe.service"));
        assert!(service_path("../../tmp/owned", false).is_err());
        assert!(service_path("a/b", true).is_err());
    }
}
