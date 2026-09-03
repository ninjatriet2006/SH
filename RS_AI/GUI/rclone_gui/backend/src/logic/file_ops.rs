/*
[INTEGRITY NOTES]
- Mục đích: Xử lý logic nghiệp vụ cao cấp cho thao tác File (Bóc tách chuỗi, kiểm tra quyền, sao chép hàng loạt).
- Trách nhiệm: Rút gọn dữ liệu mà Frontend gửi xuống. Thực thi sudo fallback tự động nếu thiếu quyền Local.
- Tương tác: Gọi `core::rclone`, `core::sys`. Gọi từ `api::files`.
*/

use crate::api::files::ConflictInfo;
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
            if remote == "Local" && (err_lower.contains("permission denied") || err_lower.contains("access is denied"))
            {
                #[cfg(target_os = "linux")]
                {
                    // Tự động gọi sudo qua pkexec
                    let mut cmd_args = Vec::new();
                    match action {
                        "rm" => {
                            cmd_args.push("rm".to_string());
                            cmd_args.push("-rf".to_string());
                        }
                        "mkdir" => {
                            cmd_args.push("mkdir".to_string());
                            cmd_args.push("-p".to_string());
                        }
                        "mv" => {
                            cmd_args.push("mv".to_string());
                        }
                        "cp" => {
                            cmd_args.push("cp".to_string());
                            cmd_args.push("-r".to_string());
                        }
                        // chmod nhận tham số dạng: <octal> <path>
                        "chmod" => {
                            cmd_args.push("chmod".to_string());
                        }
                        // Ghi nội dung cần quyền root: dùng `tee` để nhận stdin.
                        // (Không dùng ở nhánh này vì pkexec không chuyển tiếp stdin;
                        //  chỉ báo lỗi rõ ràng cho người dùng.)
                        "write" => {
                            return Err(
                                "Không đủ quyền ghi tệp này. Hãy đổi quyền hoặc chọn vị trí khác.".into()
                            );
                        }
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
pub async fn check_conflicts(
    _app_handle: tauri::AppHandle,
    srcs: Vec<String>,
    dest_path: String,
) -> Result<Vec<ConflictInfo>, String> {
    crate::core::task::blocking(move || {
        let mut conflicts = Vec::new();

        let (dest_remote, dest_real) = parse_remote_path(&dest_path);
        let dest_target = crate::core::rclone::build_target(&dest_remote, &dest_real);

        // Lấy danh sách các file/thư mục hiện có ở cấp 1 của thư mục đích
        let output = crate::core::rclone::run_cmd(&["lsjson", &dest_target])?;
        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            if err_msg.contains("directory not found") || err_msg.contains("failed to read directory") {
                return Ok(conflicts);
            }
            return Err(format!("Lỗi kiểm tra trùng lặp: {}", err_msg));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
            let existing_items: std::collections::HashMap<String, bool> = items
                .into_iter()
                .filter_map(|item| {
                    let name = item.get("Name").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let is_dir = item.get("IsDir").and_then(|v| v.as_bool()).unwrap_or(false);
                    name.map(|n| (n, is_dir))
                })
                .collect();

            for src_item in srcs {
                let (src_remote, src_real_path) = parse_remote_path(&src_item);
                let base_name = if let Some(idx) = src_real_path.rfind('/') {
                    &src_real_path[idx + 1..]
                } else {
                    src_real_path.as_str()
                };

                let src_target = crate::core::rclone::build_target(&src_remote, &src_real_path);

                // Xây dựng đường dẫn đích tuyệt đối cho mục này
                let dest_item_real = if dest_real.is_empty() || dest_real == "/" {
                    base_name.to_string()
                } else {
                    if dest_real.ends_with('/') {
                        format!("{}{}", dest_real, base_name)
                    } else {
                        format!("{}/{}", dest_real, base_name)
                    }
                };
                let dest_item_target = crate::core::rclone::build_target(&dest_remote, &dest_item_real);

                if let Some(&is_dest_dir) = existing_items.get(base_name) {
                    let src_is_dir = crate::core::rclone::is_dir(&src_target).unwrap_or(false);

                    if src_is_dir && is_dest_dir {
                        // Cả 2 đều là thư mục -> Quét đệ quy các file con
                        let src_files_out =
                            crate::core::rclone::run_cmd(&["lsjson", "-R", "--files-only", &src_target]);
                        let dest_files_out =
                            crate::core::rclone::run_cmd(&["lsjson", "-R", "--files-only", &dest_item_target]);

                        if let (Ok(s_out), Ok(d_out)) = (src_files_out, dest_files_out) {
                            if s_out.status.success() && d_out.status.success() {
                                let s_json = String::from_utf8_lossy(&s_out.stdout);
                                let d_json = String::from_utf8_lossy(&d_out.stdout);

                                if let (Ok(s_items), Ok(d_items)) = (
                                    serde_json::from_str::<Vec<serde_json::Value>>(&s_json),
                                    serde_json::from_str::<Vec<serde_json::Value>>(&d_json),
                                ) {
                                    let d_names: std::collections::HashSet<String> = d_items
                                        .into_iter()
                                        .filter_map(|i| i.get("Path").and_then(|v| v.as_str()).map(|s| s.to_string()))
                                        .collect();

                                    for s_item in s_items {
                                        if let Some(s_path) = s_item.get("Path").and_then(|v| v.as_str()) {
                                            if d_names.contains(s_path) {
                                                // Xung đột file con!
                                                conflicts.push(ConflictInfo {
                                                    relative_path: format!("{}/{}", base_name, s_path),
                                                    src_full_path: format!("{}/{}", src_target, s_path),
                                                    dest_full_path: format!("{}/{}", dest_item_target, s_path),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Xung đột trực tiếp (file vs file, file vs dir, hoặc dir vs file)
                        conflicts.push(ConflictInfo {
                            relative_path: base_name.to_string(),
                            src_full_path: src_target,
                            dest_full_path: dest_item_target,
                        });
                    }
                }
            }
        }

        Ok(conflicts)
    })
    .await
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
