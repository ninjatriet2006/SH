# sys_cmds.rs
Tài liệu tham chiếu các Tauri commands liên quan đến thao tác hệ thống (System) trong Bridge.

- **Tên hàm**: `os_clipboard_get`
- **Mô tả**: Lấy dữ liệu (path) và định dạng (copy/cut) từ Clipboard của hệ điều hành. Hiện ưu tiên dùng GNOME Clipboard.
- **Tham số đầu vào**: 
  - `app: tauri::AppHandle` (Bắt buộc)
- **Đầu ra**: `Result<Option<OSClipboardData>, String>`

- **Tên hàm**: `os_clipboard_set`
- **Mô tả**: Đưa danh sách file paths và chế độ cut/copy vào OS Clipboard.
- **Tham số đầu vào**:
  - `app: tauri::AppHandle` (Bắt buộc)
  - `paths: Vec<String>` (Bắt buộc)
  - `is_cut: bool` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `sys_list_apps`
- **Mô tả**: Liệt kê các ứng dụng (App) đã cài đặt trên hệ điều hành để làm gợi ý mở (Open With).
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Result<Vec<filen_gui::sys::DesktopApp>, String>`

- **Tên hàm**: `sys_open_with`
- **Mô tả**: Mở một file cụ thể với lệnh thực thi của một ứng dụng bên ngoài.
- **Tham số đầu vào**:
  - `path: String` (Bắt buộc)
  - `exec_cmd: String` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `sys_get_custom_actions`
- **Mô tả**: Trả về danh sách thao tác ngữ cảnh tùy chỉnh (Custom Context Actions) được thiết lập bởi người dùng.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Result<Vec<filen_gui::sys::CustomAction>, String>`

- **Tên hàm**: `sys_execute_custom_action`
- **Mô tả**: Khởi chạy một lệnh tùy chỉnh trên một hoặc nhiều file.
- **Tham số đầu vào**:
  - `exec_template: String` (Bắt buộc)
  - `file_paths: Vec<String>` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`
