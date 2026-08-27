/*
[INTEGRITY NOTES]
- Mục đích: Tầng Core (Lõi) - Giao tiếp trực tiếp với CLI rclone.
- Trách nhiệm: Xây dựng lệnh, khởi chạy đồng bộ/bất đồng bộ rclone.
- Tương tác: Được gọi bởi tầng `logic/`.
*/

use std::process::{Command, Output};

/// Tên hàm: build_target
/// Mô tả: Trộn tên remote và đường dẫn thành định dạng chuẩn của rclone.
pub fn build_target(remote: &str, path: &str) -> String {
    if remote == "Local" {
        path.to_string()
    } else {
        format!("{}:{}", remote, path)
    }
}

/// Tên hàm: run_cmd
/// Mô tả: Khởi tạo và chạy lệnh `rclone` đồng bộ.
pub fn run_cmd(args: &[&str]) -> Result<Output, String> {
    Command::new("rclone")
        .args(args)
        .output()
        .map_err(|e| format!("Lỗi hệ thống khi gọi rclone: {}", e))
}

/// Tên hàm: spawn_cmd
/// Mô tả: Khởi tạo lệnh `rclone` ở dạng spawn ngầm.
pub fn spawn_cmd(args: &[&str]) -> Result<(), String> {
    Command::new("rclone")
        .args(args)
        .spawn()
        .map_err(|e| format!("Lỗi hệ thống khi spawn rclone: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_target_local() {
        assert_eq!(build_target("Local", "/home/user"), "/home/user");
    }

    #[test]
    fn test_build_target_remote() {
        assert_eq!(build_target("GDrive", "/Documents"), "GDrive:/Documents");
    }
}
