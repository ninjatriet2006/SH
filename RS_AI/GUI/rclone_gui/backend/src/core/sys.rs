/*
[INTEGRITY NOTES]
- Mục đích: Xử lý các thao tác tương tác hệ thống (OS/System) như mở file, quản lý clipboard, danh sách ứng dụng và custom actions.
- Trách nhiệm: Đóng vai trò là cầu nối giữa Tauri frontend và các lệnh shell/hệ điều hành gốc.
- Tương tác: Gọi từ OpenWithModal.ts, clipboard.ts, actionStore.ts thông qua Tauri invoke. Không tương tác trực tiếp rclone.
*/

use crate::core::task::blocking;
use serde::{Deserialize, Serialize};
use std::env;
use std::process::Command;

#[derive(Deserialize)]
pub struct SimpleFileItem {
    pub name: String,
    pub is_dir: bool,
}

/// Khai báo cấu trúc DesktopApp để trả về cho giao diện khi chọn "Open With"
#[derive(Serialize)]
pub struct DesktopApp {
    // Tên hiển thị của ứng dụng
    pub name: String,
    // Lệnh thực thi của ứng dụng
    pub exec: String,
    // Đường dẫn hoặc tên icon của ứng dụng
    pub icon: String,
}

/// Khai báo cấu trúc CustomAction cho các lệnh tự tạo trên menu chuột phải
#[derive(Serialize)]
pub struct CustomAction {
    // ID duy nhất của action
    pub id: String,
    // Tên hiển thị trên menu
    pub name: String,
    // Mẫu lệnh thực thi (VD: echo %f)
    pub exec: String,
    // Icon hiển thị trên menu
    pub icon: String,
    // Điều kiện số lượng file được chọn (s: single, m: multiple, any)
    pub selection: String,
    // Danh sách phần mở rộng hỗ trợ (VD: ["txt", "md"])
    pub extensions: Vec<String>,
}

/// Khai báo dữ liệu Clipboard để lưu vào bộ nhớ đệm
#[derive(Serialize, Deserialize, Clone)]
pub struct OSClipboardItem {
    pub pane: String,
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct OSClipboardData {
    pub items: Vec<OSClipboardItem>,
    pub is_cut: bool,
}

/// Hàm API: sys_open_with
/// Chức năng: Mở một file cụ thể bằng một ứng dụng hoặc lệnh tùy chỉnh
#[tauri::command]
pub async fn sys_open_with(path: String, exec_cmd: Option<String>, app: Option<String>) -> Result<(), String> {
    blocking(move || {
        // Frontend gửi xuống đường dẫn dạng "Remote::/path" (xem logic::file_ops::parse_remote_path).
        // Chỉ ổ Local mới mở được bằng ứng dụng hệ điều hành.
        let (remote, real_path) = crate::logic::file_ops::parse_remote_path(&path);
        if remote != "Local" {
            return Err(format!(
                "Không thể mở trực tiếp file trên remote '{}'. Hãy copy về Local trước.",
                remote
            ));
        }
        let path = real_path;

        // Ưu tiên sử dụng exec_cmd nếu có, ngược lại dùng xdg-open làm mặc định trên Linux
        let cmd = exec_cmd.or(app).unwrap_or_else(|| "xdg-open".to_string());

        // Lệnh .desktop thường chứa placeholder (%f, %U, ...) và có thể có sẵn tham số.
        // Tách theo cú pháp shell rồi exec trực tiếp — KHÔNG qua `sh -c` — để tên file
        // chứa ký tự đặc biệt (`;`, `$(...)`, dấu nháy) không thể chèn thêm lệnh.
        let mut parts = shell_split(&cmd);
        if parts.is_empty() {
            return Err("Lệnh mở file rỗng.".to_string());
        }

        let program = parts.remove(0);
        let mut args: Vec<String> = Vec::new();
        let mut path_injected = false;

        for part in parts {
            match part.as_str() {
                // Placeholder theo Desktop Entry Spec: thay bằng đường dẫn file.
                "%f" | "%F" | "%u" | "%U" => {
                    args.push(path.clone());
                    path_injected = true;
                }
                // Các placeholder không dùng tới (icon, tên app, ...) thì bỏ qua.
                p if p.len() == 2 && p.starts_with('%') => {}
                other => args.push(other.to_string()),
            }
        }

        if !path_injected {
            args.push(path);
        }

        Command::new(&program)
            .args(&args)
            .spawn()
            .map_err(|e| format!("Lỗi khi chạy '{}': {}", program, e))?;

        Ok(())
    })
    .await
}

/// Tách một chuỗi lệnh theo cú pháp shell tối giản (hỗ trợ nháy đơn/kép và `\`).
/// Dùng để bóc tách Exec= của file .desktop mà không cần gọi shell.
fn shell_split(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for c in input.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' && !in_single {
            escaped = true;
        } else if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == '"' && !in_single {
            in_double = !in_double;
        } else if c.is_whitespace() && !in_single && !in_double {
            if !current.is_empty() {
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

/// Hàm API: sys_list_apps
/// Chức năng: Lấy danh sách ứng dụng trên hệ điều hành để hiển thị mục Open With
#[tauri::command]
pub async fn sys_list_apps() -> Result<Vec<DesktopApp>, String> {
    // Quét Desktop Entry thật (chuẩn FreeDesktop.org) thay vì trả dữ liệu giả.
    blocking(|| Ok(crate::logic::desktop_apps::list())).await
}

/// Hàm API: os_clipboard_set
/// Chức năng: Lưu danh sách file vào clipboard giả lập (thông qua file JSON tạm)
#[tauri::command]
pub async fn os_clipboard_set(items: Vec<OSClipboardItem>, is_cut: bool) -> Result<(), String> {
    blocking(move || {
        let data = OSClipboardData { items, is_cut };
        // Chuyển đối tượng thành chuỗi JSON
        let json = serde_json::to_string(&data).unwrap();
        // Ghi chuỗi JSON vào thư mục tạm của hệ điều hành
        std::fs::write(env::temp_dir().join("rclone_gui_clipboard.json"), json)
            // Bắt lỗi nếu không thể ghi file
            .map_err(|e| e.to_string())?;

        // Trả về thành công
        Ok(())
    })
    .await
}

/// Hàm API: os_clipboard_get
/// Chức năng: Lấy danh sách file từ clipboard giả lập
#[tauri::command]
pub async fn os_clipboard_get() -> Result<Option<OSClipboardData>, String> {
    blocking(|| {
        // Đường dẫn tới file JSON clipboard
        let path = env::temp_dir().join("rclone_gui_clipboard.json");

        // Kiểm tra xem file có tồn tại không
        if path.exists() {
            // Đọc toàn bộ nội dung file JSON
            let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            // Parse chuỗi JSON ngược về đối tượng cấu trúc OSClipboardData
            let data: OSClipboardData = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            // Trả về dữ liệu tìm thấy
            Ok(Some(data))
        } else {
            // Nếu file không tồn tại (chưa copy gì), trả về rỗng
            Ok(None)
        }
    })
    .await
}

/// Hàm API: sys_get_custom_actions
/// Chức năng: Lấy danh sách các lệnh tùy chỉnh của người dùng để hiển thị context menu
#[tauri::command]
pub async fn sys_get_custom_actions() -> Result<Vec<CustomAction>, String> {
    // Tạm thời trả về danh sách rỗng (có thể thêm logic đọc từ file config JSON sau này)
    Ok(vec![])
}

/// Hàm API: sys_get_valid_actions
/// Chức năng: Lọc danh sách action hợp lệ dựa trên file được chọn
#[tauri::command]
pub async fn sys_get_valid_actions(files: Vec<SimpleFileItem>) -> Result<Vec<CustomAction>, String> {
    let actions = sys_get_custom_actions().await?;
    let sel_count = files.len();

    if sel_count == 0 {
        return Ok(vec![]);
    }

    let valid: Vec<CustomAction> = actions
        .into_iter()
        .filter(|a| {
            if a.selection == "s" && sel_count != 1 {
                return false;
            }
            if a.selection == "m" && sel_count < 2 {
                return false;
            }

            if a.extensions.iter().any(|ext| ext == "any") {
                return true;
            }

            files.iter().all(|f| {
                if f.is_dir && a.extensions.iter().any(|ext| ext == "dir") {
                    return true;
                }
                let ext = f.name.split('.').last().unwrap_or("").to_lowercase();
                a.extensions.iter().any(|e| e.to_lowercase() == ext)
            })
        })
        .collect();

    Ok(valid)
}

/// Hàm API: sys_execute_custom_action
/// Chức năng: Thực thi một lệnh tùy chỉnh lên một danh sách file được chọn
#[tauri::command]
pub async fn sys_execute_custom_action(
    exec_template: String,
    base_path: String,
    file_names: Vec<String>,
) -> Result<(), String> {
    // exec_template do người dùng tự định nghĩa nên vẫn chạy qua shell (cho phép
    // pipe, redirect...). Nhưng tên file đến từ dữ liệu ngoài, phải được bọc
    // nháy đơn an toàn để không thể chèn thêm lệnh.
    let paths_str = file_names
        .iter()
        .map(|name| {
            let p = if base_path.starts_with("trash://") || base_path == "/" {
                format!("{}/{}", base_path.trim_end_matches('/'), name)
            } else {
                format!("{}/{}", base_path, name)
            };
            shell_quote(&p)
        })
        .collect::<Vec<String>>()
        .join(" ");

    // Thay thế biến %f trong mẫu lệnh bằng danh sách đường dẫn thực tế
    let cmd = exec_template.replace("%f", &paths_str);

    // Khởi tạo tiến trình thực thi lệnh
    blocking(move || {
        Command::new("sh")
            // Dùng -c để truyền vào chuỗi shell
            .arg("-c")
            // Truyền chuỗi lệnh đã thay thế biến %f
            .arg(cmd)
            // Kích hoạt tiến trình chạy ngầm
            .spawn()
            // Xử lý lỗi nếu lệnh không chạy được
            .map_err(|e| e.to_string())?;

        // Báo thành công
        Ok(())
    })
    .await
}

/// Bọc một chuỗi bất kỳ thành literal an toàn cho shell POSIX bằng nháy đơn.
/// Mỗi dấu `'` trong chuỗi gốc được thoát thành `'\''`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_split_handles_desktop_exec() {
        assert_eq!(shell_split("xdg-open"), vec!["xdg-open"]);
        assert_eq!(shell_split("code --wait %f"), vec!["code", "--wait", "%f"]);
        assert_eq!(shell_split("\"/opt/My App/run\" -a"), vec!["/opt/My App/run", "-a"]);
    }

    #[test]
    fn shell_quote_escapes_injection() {
        assert_eq!(shell_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(shell_quote("/tmp/$(id)"), "'/tmp/$(id)'");
        assert_eq!(shell_quote("/tmp/it's"), r"'/tmp/it'\''s'");
    }
}
