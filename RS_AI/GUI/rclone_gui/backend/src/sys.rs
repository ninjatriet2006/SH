/* 
[INTEGRITY NOTES]
- Mục đích: Xử lý các thao tác tương tác hệ thống (OS/System) như mở file, quản lý clipboard, danh sách ứng dụng và custom actions.
- Trách nhiệm: Đóng vai trò là cầu nối giữa Tauri frontend và các lệnh shell/hệ điều hành gốc.
- Tương tác: Gọi từ OpenWithModal.ts, clipboard.ts, actionStore.ts thông qua Tauri invoke. Không tương tác trực tiếp rclone.
*/

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::env;

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
#[derive(Serialize, Deserialize)]
pub struct OSClipboardData {
    // Danh sách các đường dẫn file đã copy/cut
    pub paths: Vec<String>,
    // Cờ đánh dấu là hành động Cắt (true) hay Copy (false)
    pub is_cut: bool,
}

/// Hàm API: sys_open_with
/// Chức năng: Mở một file cụ thể bằng một ứng dụng hoặc lệnh tùy chỉnh
#[tauri::command]
pub async fn sys_open_with(path: String, exec_cmd: Option<String>, app: Option<String>) -> Result<(), String> {
    // Ưu tiên sử dụng exec_cmd nếu có, ngược lại dùng xdg-open làm mặc định trên Linux
    let cmd = if let Some(cmd) = exec_cmd {
        cmd
    } else if let Some(app_name) = app {
        app_name // Nếu chỉ truyền tên app, thử gọi tên app đó như một lệnh
    } else {
        "xdg-open".to_string()
    };
    
    // Gọi lệnh shell để thực thi mở file
    Command::new("sh")
        // Tham số -c để chạy chuỗi lệnh
        .arg("-c")
        // Gắn đường dẫn file vào lệnh (có bọc ngoặc kép để tránh lỗi khoảng trắng)
        .arg(format!("{} \"{}\"", cmd, path))
        // Khởi động tiến trình con chạy độc lập
        .spawn()
        // Nếu lỗi, chuyển sang chuỗi báo lỗi cho frontend
        .map_err(|e| e.to_string())?;

    // Trả về kết quả thành công
    Ok(())
}

/// Hàm API: sys_list_apps
/// Chức năng: Lấy danh sách ứng dụng trên hệ điều hành để hiển thị mục Open With
#[tauri::command]
pub async fn sys_list_apps() -> Result<Vec<DesktopApp>, String> {
    // (Làm giả dữ liệu tạm thời để trả nợ kỹ thuật, việc parse .desktop files phức tạp sẽ tối ưu sau)
    Ok(vec![
        DesktopApp {
            // Tên ứng dụng mặc định
            name: "Default OS App (xdg-open)".to_string(),
            // Lệnh mặc định của Linux để mở file theo file type
            exec: "xdg-open".to_string(),
            // Icon để trống
            icon: "".to_string(),
        }
    ])
}

/// Hàm API: os_clipboard_set
/// Chức năng: Lưu danh sách file vào clipboard giả lập (thông qua file JSON tạm)
#[tauri::command]
pub async fn os_clipboard_set(paths: Vec<String>, is_cut: bool) -> Result<(), String> {
    // Khởi tạo đối tượng dữ liệu clipboard
    let data = OSClipboardData { paths, is_cut };
    // Chuyển đối tượng thành chuỗi JSON
    let json = serde_json::to_string(&data).unwrap();
    // Ghi chuỗi JSON vào thư mục tạm của hệ điều hành
    std::fs::write(env::temp_dir().join("rclone_gui_clipboard.json"), json)
        // Bắt lỗi nếu không thể ghi file
        .map_err(|e| e.to_string())?;
    
    // Trả về thành công
    Ok(())
}

/// Hàm API: os_clipboard_get
/// Chức năng: Lấy danh sách file từ clipboard giả lập
#[tauri::command]
pub async fn os_clipboard_get() -> Result<Option<OSClipboardData>, String> {
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
}

/// Hàm API: sys_get_custom_actions
/// Chức năng: Lấy danh sách các lệnh tùy chỉnh của người dùng để hiển thị context menu
#[tauri::command]
pub async fn sys_get_custom_actions() -> Result<Vec<CustomAction>, String> {
    // Tạm thời trả về danh sách rỗng (có thể thêm logic đọc từ file config JSON sau này)
    Ok(vec![])
}

/// Hàm API: sys_execute_custom_action
/// Chức năng: Thực thi một lệnh tùy chỉnh lên một danh sách file được chọn
#[tauri::command]
pub async fn sys_execute_custom_action(exec_template: String, file_paths: Vec<String>) -> Result<(), String> {
    // Lặp qua mảng file_paths, bọc dấu ngoặc kép cho từng đường dẫn và nối lại bằng dấu cách
    let paths_str = file_paths.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<String>>().join(" ");
    
    // Thay thế biến %f trong mẫu lệnh bằng danh sách đường dẫn thực tế
    let cmd = exec_template.replace("%f", &paths_str);
    
    // Khởi tạo tiến trình thực thi lệnh
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
}
