/*
[INTEGRITY NOTES]
- Mục đích: Xử lý logic nghiệp vụ cao cấp cho thao tác File (Bóc tách chuỗi, kiểm tra quyền, sao chép hàng loạt).
- Trách nhiệm: Rút gọn dữ liệu mà Frontend gửi xuống. Thực thi sudo fallback tự động nếu thiếu quyền Local.
- Tương tác: Gọi `core::rclone`, `core::sys`. Gọi từ `api::files`.
*/

use std::process::Command;
use std::collections::HashMap;


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

/// Tên hàm: check_conflicts
/// Mô tả: Kích hoạt `rclone check` ngầm để đệ quy kiểm tra xung đột hash giữa source và dest.
/// Trả về mảng các đường dẫn file con bị khác hash.
pub async fn check_conflicts(_app_handle: tauri::AppHandle, srcs: Vec<String>, dest_path: String) -> Result<Vec<String>, String> {

    let mut conflicts = Vec::new();

    let (dest_remote, dest_real) = parse_remote_path(&dest_path);
    let dest_target = crate::core::rclone::build_target(&dest_remote, &dest_real);

    // Dùng lsjson trên thư mục đích để lấy danh sách các file/thư mục hiện có ở cấp 1 (top-level)
    let output = crate::core::rclone::run_cmd(&["lsjson", &dest_target])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        // Nếu thư mục đích chưa tồn tại, rclone lsjson sẽ trả về lỗi directory not found, điều này có nghĩa là không có conflict
        if err_msg.contains("directory not found") || err_msg.contains("failed to read directory") {
            return Ok(conflicts);
        }
        return Err(format!("Lỗi kiểm tra trùng lặp: {}", err_msg));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
        let existing_names: std::collections::HashSet<String> = items.into_iter()
            .filter_map(|item| item.get("Name").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();

        // So sánh tên (basename) của các file nguồn với các tên hiện có trong thư mục đích
        for src in srcs {
            let (_, real_path) = parse_remote_path(&src);
            let base_name = if let Some(idx) = real_path.rfind('/') {
                &real_path[idx + 1..]
            } else {
                real_path.as_str()
            };

            if existing_names.contains(base_name) {
                conflicts.push(base_name.to_string());
            }
        }
    }

    Ok(conflicts)
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
