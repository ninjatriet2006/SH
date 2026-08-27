/*
[INTEGRITY NOTES]
- Mục đích: Xử lý logic nghiệp vụ cao cấp cho thao tác File (Bóc tách chuỗi, kiểm tra quyền, sao chép hàng loạt).
- Trách nhiệm: Rút gọn dữ liệu mà Frontend gửi xuống. Thực thi sudo fallback tự động nếu thiếu quyền Local.
- Tương tác: Gọi `core::rclone`, `core::sys`. Gọi từ `api::files`.
*/

use std::process::Command;


/// Tên hàm: parse_remote_path
/// Mô tả: Bóc tách chuỗi "GDrive::/Documents" thành remote ("GDrive") và đường dẫn ("/Documents").
pub fn parse_remote_path(full_path: &str) -> (String, String) {
    if let Some(idx) = full_path.find("::") {
        let remote = full_path[..idx].to_string();
        let path = full_path[idx + 2..].to_string();
        (remote, path)
    } else {
        ("Local".to_string(), full_path.to_string())
    }
}

/// Tên hàm: run_with_sudo_fallback
/// Mô tả: Bọc lệnh rclone/os. Nếu chạy thất bại do Permission Denied và đây là ổ Local, tự động gọi pkexec (sudo).
pub fn run_with_sudo_fallback<F>(remote: &str, action: &str, args: &[String], fallback_cmd: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    // Gọi hàm gốc trước
    let result = fallback_cmd();
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            let err_lower = e.to_lowercase();
            // Kiểm tra lỗi phân quyền trên Local
            if remote == "Local" && (err_lower.contains("permission denied") || err_lower.contains("access is denied")) {
                #[cfg(target_os = "linux")]
                {
                    // Tự động gọi sudo qua pkexec
                    let mut cmd_args = Vec::new();
                    match action {
                        "rm" => {
                            cmd_args.push("rm".to_string());
                            cmd_args.push("-rf".to_string());
                        },
                        "mkdir" => {
                            cmd_args.push("mkdir".to_string());
                            cmd_args.push("-p".to_string());
                        },
                        "mv" => {
                            cmd_args.push("mv".to_string());
                        },
                        "cp" => {
                            cmd_args.push("cp".to_string());
                            cmd_args.push("-r".to_string());
                        },
                        _ => return Err("Hành động sudo không được hỗ trợ".into()),
                    }
                    for arg in args {
                        cmd_args.push(arg.clone());
                    }
                    
                    let output = Command::new("pkexec")
                        .args(&cmd_args)
                        .output()
                        .map_err(|e| format!("Lỗi gọi pkexec: {}", e))?;
                        
                    if !output.status.success() {
                        let err = String::from_utf8_lossy(&output.stderr).into_owned();
                        if err.is_empty() {
                            return Err("Thao tác pkexec bị huỷ hoặc lỗi phân quyền.".into());
                        }
                        return Err(err);
                    }
                    return Ok(());
                }
                #[cfg(not(target_os = "linux"))]
                {
                    return Err(e);
                }
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_path_local() {
        let (remote, path) = parse_remote_path("/home/user/Documents");
        assert_eq!(remote, "Local");
        assert_eq!(path, "/home/user/Documents");
    }

    #[test]
    fn test_parse_remote_path_cloud() {
        let (remote, path) = parse_remote_path("GDrive::/Work/Project");
        assert_eq!(remote, "GDrive");
        assert_eq!(path, "/Work/Project");
    }

    #[test]
    fn test_parse_remote_path_empty() {
        let (remote, path) = parse_remote_path("");
        assert_eq!(remote, "Local");
        assert_eq!(path, "");
    }
}
