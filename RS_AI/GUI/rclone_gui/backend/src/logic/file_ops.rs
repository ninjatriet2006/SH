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
pub async fn check_conflicts(app_handle: tauri::AppHandle, srcs: Vec<String>, dest_path: String) -> Result<Vec<String>, String> {
    use std::process::{Command, Stdio};
    use std::io::{BufRead, BufReader};
    use tauri::Emitter;

    let mut conflicts = Vec::new();

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for src in srcs {
        let (remote, path) = parse_remote_path(&src);
        let path = path.trim_end_matches('/');
        let (parent, base) = match path.rfind('/') {
            Some(idx) => (&path[..idx], &path[idx+1..]),
            None => ("", path),
        };
        
        let parent_remote = if parent.is_empty() {
            format!("{}::/", remote)
        } else {
            format!("{}::{}", remote, parent)
        };
        
        groups.entry(parent_remote).or_insert_with(Vec::new).push(base.to_string());
    }

    let (dest_remote, dest_real) = parse_remote_path(&dest_path);
    let dest_target = crate::core::rclone::build_target(&dest_remote, &dest_real);

    for (parent_src, bases) in groups {
        let (src_remote, src_real) = parse_remote_path(&parent_src);
        let src_target = crate::core::rclone::build_target(&src_remote, &src_real);
        
        let mut string_args = vec![
            "check".to_string(), 
            src_target.clone(), 
            dest_target.clone(), 
            "--combined".to_string(), 
            "-".to_string(),
            "--use-json-log".to_string(),
            "--stats".to_string(),
            "0.5s".to_string(),
        ];
        
        for base in &bases {
            string_args.push("--include".to_string());
            string_args.push(base.clone());
            string_args.push("--include".to_string());
            string_args.push(format!("{}/**", base));
        }
        
        let mut child = Command::new("rclone")
            .args(&string_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Lỗi khởi chạy rclone check: {}", e))?;

        let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;

        let app_handle_clone = app_handle.clone();
        
        // Luồng đọc stderr để lấy progress json
        let stderr_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line_str) = line {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line_str) {
                        if let Some(stats) = json.get("stats") {
                            let payload = serde_json::json!({
                                "stats": stats
                            });
                            let _ = app_handle_clone.emit("conflict_check_progress", payload);
                        }
                    }
                }
            }
        });

        // Luồng chính đọc stdout để bắt kết quả conflict
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line_str) = line {
                if line_str.starts_with("* ") {
                    let conflict_path = line_str[2..].trim_end().to_string();
                    conflicts.push(conflict_path);
                }
            }
        }

        stderr_thread.join().unwrap();
        let status = child.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            // Rclone check trả về mã lỗi 1 nếu có difference, điều này là bình thường
            // Nên chúng ta không bắt lỗi ở đây trừ khi status != 1 và status != 0
            if status.code() != Some(0) && status.code() != Some(1) {
                return Err(format!("Lỗi kiểm tra trùng lặp, mã lỗi: {}", status));
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
