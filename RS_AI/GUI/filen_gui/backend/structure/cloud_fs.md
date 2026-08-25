# cloud_fs.rs
Tài liệu tham chiếu các hàm làm việc với Cloud FS (File System).

- **Tên hàm**: `list_remote_terminal`, `list_remote_stream_terminal`
- **Mô tả**: Liệt kê file/thư mục trên cloud (hỗ trợ dạng luồng stream trả về từng cục nhỏ).
- **Tham số đầu vào**:
  - `active_account: &Option<String>` (Tùy chọn)
  - `path: &str` (Bắt buộc)
- **Đầu ra**: `Result<Vec<FileItem>, String>` (hoặc stream)

- **Tên hàm**: `mkdir_terminal`
- **Mô tả**: Tạo thư mục mới trên cloud.
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `path: &str` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `rm_terminal`
- **Mô tả**: Xóa file/thư mục (vào thùng rác hoặc xóa vĩnh viễn).
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `path: &str` (Bắt buộc)
  - `no_trash: bool` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `mv_terminal`, `cp_terminal`
- **Mô tả**: Di chuyển (Move) hoặc sao chép (Copy) file trên Cloud.
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `from: &str` (Bắt buộc)
  - `to: &str` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `upload_terminal`, `download_terminal`
- **Mô tả**: Truyền tải tệp (Upload / Download).
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `local: &str` (Bắt buộc)
  - `remote: &str` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `cat_terminal`, `head_terminal`, `tail_terminal`, `write_file_terminal`
- **Mô tả**: Đọc / ghi nội dung file văn bản trực tiếp trên Cloud.
- **Tham số đầu vào**: Tùy hàm, thường kèm `path` (Bắt buộc) và `content` (Bắt buộc).
- **Đầu ra**: Text nội dung (`cat`) hoặc `Result<(), String>` (`write`).

- **Tên hàm**: `stat_terminal`
- **Mô tả**: Lấy thông tin chi tiết (JSON) của một tệp.
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `item: &str` (Bắt buộc)
- **Đầu ra**: `Result<String, String>`

- **Tên hàm**: `favorite_terminal`, `unfavorite_terminal`, `list_favorites_terminal`
- **Mô tả**: Quản lý mục yêu thích (Favorites).
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `path: &str` (Bắt buộc đối với favorite/unfavorite)
- **Đầu ra**: Tùy hàm (Danh sách yêu thích hoặc kết quả boolean).

- **Tên hàm**: `list_trash_terminal`, `trash_restore_terminal`, `trash_delete_terminal`, `trash_empty_terminal`
- **Mô tả**: Quản lý thùng rác (Trash) trên Cloud.
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `idx_1based: usize` (Bắt buộc đối với restore/delete)
- **Đầu ra**: Danh sách thùng rác hoặc `Result<(), String>`.

- **Tên hàm**: `create_link_terminal`, `list_links_terminal`
- **Mô tả**: Quản lý chia sẻ link công khai.
- **Tham số đầu vào**: 
  - `active_account: &Option<String>` (Tùy chọn)
  - `path: &str` (Bắt buộc khi tạo link)
- **Đầu ra**: URL string, hoặc Danh sách links.
