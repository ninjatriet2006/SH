[Pattern Docs]
# KIẾN TRÚC THƯ MỤC BACKEND
- **src/**: Thư mục chứa mã nguồn Rust.
  - `lib.rs`: Tệp logic chính, định nghĩa các lệnh (Tauri Commands) gọi từ Frontend.
  - `utils.rs`: Chứa các hàm tiện ích dùng chung (build_rclone_target, run_rclone_cmd) để chuẩn hoá quá trình thực thi.
  - `remote.rs`: Xử lý tạo, xóa, và liệt kê các dịch vụ đám mây (Remotes).
  - `trash.rs`: Xử lý dọn rác (Empty Trash) trên đám mây.
  - `auth.rs`: Xử lý tính toán dung lượng (Quota/Storage) trên đám mây.

# CẤU TRÚC DỮ LIỆU (DATA STRUCTURES - STRUCTS)

## lib.rs
- **Tên cấu trúc (Struct)**: FileItem
- **Mô tả**: Lưu trữ thông tin một file/folder được phân tích từ kết quả JSON của rclone.
- **Thuộc tính**:
  - `uuid: String` (Bắt buộc)
  - `name: String` (Bắt buộc)
  - `is_dir: bool` (Bắt buộc)
  - `size: i64` (Bắt buộc)
  - `mod_time: String` (Bắt buộc)
  - `file_type: Option<String>` (Tùy chọn)
  - `owner: Option<String>` (Tùy chọn)
  - `group: Option<String>` (Tùy chọn)
  - `permissions: Option<String>` (Tùy chọn)

# TÀI LIỆU HÀM (API DOCS)

# lib.rs
- **Tên hàm**: list_remotes
- **Mô tả**: Trả về danh sách tất cả các ổ đĩa đám mây (Remotes) đã được cấu hình trong rclone, cộng thêm ổ đĩa Local ảo.
- **Tham số đầu vào**: Không có
- **Đầu ra**: Result<Vec<Value>, String>

- **Tên hàm**: list_files
- **Mô tả**: Liệt kê toàn bộ file và thư mục con trực tiếp tại đường dẫn cung cấp.
- **Tham số đầu vào**: `remote: String` (Bắt buộc), `path: String` (Bắt buộc)
- **Đầu ra**: Result<Vec<FileItem>, String>

- **Tên hàm**: fs_mkdir
- **Mô tả**: Tạo thư mục mới trên ổ đĩa.
- **Tham số đầu vào**: `remote: String` (Bắt buộc), `path: String` (Bắt buộc)
- **Đầu ra**: Result<(), String>

- **Tên hàm**: fs_delete
- **Mô tả**: Xóa một file hoặc thư mục.
- **Tham số đầu vào**: `remote: String` (Bắt buộc), `path: String` (Bắt buộc)
- **Đầu ra**: Result<(), String>

- **Tên hàm**: fs_touch
- **Mô tả**: Tạo một file rỗng trên hệ thống.
- **Tham số đầu vào**: `remote: String` (Bắt buộc), `path: String` (Bắt buộc)
- **Đầu ra**: Result<(), String>

- **Tên hàm**: fs_rename
- **Mô tả**: Đổi tên một file hoặc thư mục.
- **Tham số đầu vào**: `remote: String` (Bắt buộc), `old_path: String` (Bắt buộc), `new_path: String` (Bắt buộc)
- **Đầu ra**: Result<(), String>

- **Tên hàm**: fs_copy
- **Mô tả**: Bắt đầu tiến trình copy file/thư mục và báo cáo tiến độ.
- **Tham số đầu vào**: `app_handle: AppHandle` (Bắt buộc), `state: State` (Bắt buộc), `src_remote: String` (Bắt buộc), `src_path: String` (Bắt buộc), `dest_remote: String` (Bắt buộc), `dest_path: String` (Bắt buộc), `task_id: Option<u32>` (Tùy chọn)
- **Đầu ra**: Result<(), String>

- **Tên hàm**: fs_move
- **Mô tả**: Bắt đầu tiến trình di chuyển (move) file/thư mục và báo cáo tiến độ.
- **Tham số đầu vào**: (Tương tự fs_copy)
- **Đầu ra**: Result<(), String>

# utils.rs
- **Tên hàm**: build_rclone_target
- **Mô tả**: Xây dựng đường dẫn chuẩn cho lệnh rclone.
- **Tham số đầu vào**: `remote: &str` (Bắt buộc), `path: &str` (Bắt buộc)
- **Đầu ra**: String

- **Tên hàm**: run_rclone_cmd
- **Mô tả**: Khởi tạo và chạy lệnh rclone đồng bộ.
- **Tham số đầu vào**: `args: &[&str]` (Bắt buộc)
- **Đầu ra**: Result<Output, String>

- **Tên hàm**: spawn_rclone_cmd
- **Mô tả**: Khởi tạo lệnh rclone ngầm.
- **Tham số đầu vào**: `args: &[&str]` (Bắt buộc)
- **Đầu ra**: Result<(), String>

# remote.rs
- **Tên hàm**: get_providers
- **Mô tả**: Lấy danh sách provider (Google Drive, Dropbox...).
- **Tham số đầu vào**: Không có
- **Đầu ra**: Result<String, String>

- **Tên hàm**: create_remote
- **Mô tả**: Tạo remote mới.
- **Tham số đầu vào**: `name: String` (Bắt buộc), `provider: String` (Bắt buộc), `options: HashMap<String, String>` (Tùy chọn)
- **Đầu ra**: Result<String, String>

- **Tên hàm**: delete_remote
- **Mô tả**: Xóa remote khỏi rclone.
- **Tham số đầu vào**: `name: String` (Bắt buộc)
- **Đầu ra**: Result<String, String>

- **Tên hàm**: update_remote
- **Mô tả**: Cập nhật remote.
- **Tham số đầu vào**: `name: String` (Bắt buộc), `options: HashMap<String, String>` (Tùy chọn)
- **Đầu ra**: Result<String, String>

# trash.rs
- **Tên hàm**: fs_trash_empty_remote_terminal
- **Mô tả**: Dọn dẹp thùng rác của ổ đĩa Cloud.
- **Tham số đầu vào**: `account: Option<String>` (Tùy chọn)
- **Đầu ra**: Result<(), String>

# auth.rs
- **Tên hàm**: auth_statfs_terminal
- **Mô tả**: Lấy thông tin dung lượng lưu trữ của một Remote.
- **Tham số đầu vào**: `account: Option<String>` (Tùy chọn)
- **Đầu ra**: Result<(String, String), String>
