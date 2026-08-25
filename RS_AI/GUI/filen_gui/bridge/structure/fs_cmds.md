# fs_cmds.rs
Tài liệu tham chiếu các Tauri commands liên quan đến thao tác File System (FS) trong Bridge.

- **Tên hàm**: `fs_list_remote_terminal`
- **Mô tả**: Liệt kê danh sách file/thư mục trên Cloud (chế độ thường).
- **Tham số đầu vào**:
  - `account: Option<String>` (Tùy chọn)
  - `path: String` (Bắt buộc)
- **Đầu ra**: `Result<Vec<FileItem>, String>`

- **Tên hàm**: `fs_list_remote_stream_terminal`
- **Mô tả**: Liệt kê danh sách file/thư mục trên Cloud theo luồng (chunking).
- **Tham số đầu vào**:
  - `account: Option<String>` (Tùy chọn)
  - `path: String` (Bắt buộc)
  - `on_chunk: tauri::ipc::Channel<Vec<FileItem>>` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `fs_get_thumbnail`
- **Mô tả**: Lấy ảnh thu nhỏ (thumbnail) Base64 của file dưới dạng Async.
- **Tham số đầu vào**:
  - `path: String` (Bắt buộc)
- **Đầu ra**: `Result<String, String>`

- **Tên hàm**: `fs_list_local`
- **Mô tả**: Liệt kê danh sách file/thư mục tại Local và đăng ký thư mục này vào trình theo dõi biến động (Inotify Watcher).
- **Tham số đầu vào**:
  - `path: String` (Bắt buộc)
  - `state: tauri::State<'_, AppState>` (Bắt buộc)
- **Đầu ra**: `Result<Vec<FileItem>, String>`

- **Tên hàm**: `fs_mkdir_terminal`, `fs_rm_terminal`, `fs_mv_terminal`, `fs_cp_terminal`
- **Mô tả**: Các thao tác thư mục tạo, xóa, di chuyển, sao chép cơ bản trên Cloud.
- **Tham số đầu vào**: Đều yêu cầu `account: Option<String>` (Tùy chọn) và các đường dẫn `path`, `from`, `to` (Bắt buộc).

- **Tên hàm**: `fs_mkdir_local`, `fs_rm_local`, `fs_mv_local`, `fs_cp_local`, `fs_cp_batch`
- **Mô tả**: Các thao tác thư mục tạo, xóa, di chuyển, sao chép cơ bản dưới Local.
- **Tham số đầu vào**: Đều yêu cầu các đường dẫn `path`, `from`, `to` (Bắt buộc).

- **Tên hàm**: `fs_trash_list_local`, `fs_trash_restore_local`, `fs_trash_empty_local`
- **Mô tả**: Các thao tác với thùng rác hệ điều hành cục bộ.
- **Tham số đầu vào**: `item_id` (Bắt buộc đối với restore).

- **Tên hàm**: `fs_trash_list_remote_terminal`, `fs_trash_restore_remote_terminal`, `fs_trash_delete_remote_terminal`, `fs_trash_empty_remote_terminal`
- **Mô tả**: Các thao tác với thùng rác trên Cloud.
- **Tham số đầu vào**: Đều nhận `account: Option<String>` (Tùy chọn). Restore/delete cần thêm `idx: usize` (Bắt buộc).

- **Tên hàm**: `fs_upload_terminal`, `fs_download_terminal`
- **Mô tả**: Tải file lên và xuống trực tiếp không đưa vào hàng đợi Transfer.
- **Tham số đầu vào**:
  - `account: Option<String>` (Tùy chọn)
  - `local: String` (Bắt buộc)
  - `remote: String` (Bắt buộc)

- **Tên hàm**: `fs_cat_terminal`, `fs_write_terminal`, `fs_write_local`
- **Mô tả**: Đọc / ghi nhanh nội dung text trực tiếp vào file.
- **Tham số đầu vào**: `account: Option<String>` (Tùy chọn với cloud), `path: String` (Bắt buộc), `content: String` (Bắt buộc đối với write).

- **Tên hàm**: `fs_rename_terminal`, `fs_delete_terminal`, `fs_copy_terminal`, `fs_move_terminal`
- **Mô tả**: Các Alias được phía UI DualPaneExplorer dùng để đồng nhất cách gọi (ví dụ đổi tên bản chất là move cùng cha).
- **Tham số đầu vào**: `account: Option<String>` (Tùy chọn) và các đường dẫn (Bắt buộc).

- **Tên hàm**: `fs_stat_advanced`
- **Mô tả**: Tính toán chi tiết tổng dung lượng, số lượng file, thư mục bên trong (đệ quy).
- **Tham số đầu vào**:
  - `path: String` (Bắt buộc)
- **Đầu ra**: `Result<StatInfo, String>`

- **Tên hàm**: `fs_search_local`
- **Mô tả**: Tìm kiếm File/Thư mục cục bộ (hỗ trợ Fuzzy, lọc theo nội dung text, kích thước).
- **Tham số đầu vào**:
  - `path: String` (Bắt buộc)
  - `query: String` (Bắt buộc)
  - `options: Option<SearchOptions>` (Tùy chọn)
- **Đầu ra**: `Result<Vec<SearchResult>, String>`
