# local_fs.rs
Tài liệu tham chiếu các hàm làm việc với file hệ thống nội bộ (Local Filesystem).

- **Tên hàm**: `list_local`
- **Mô tả**: Quét thư mục trên máy tính cục bộ bằng `std::fs` và trả về danh sách `FileItem`.
- **Tham số đầu vào**:
  - `path: &str` (Bắt buộc) - Đường dẫn thư mục.
- **Đầu ra**: `Result<Vec<FileItem>, String>`

- **Tên hàm**: `copy_local`, `move_local`, `delete_local`
- **Mô tả**: Thao tác file cục bộ. Lưu ý `delete_local` sử dụng crate `trash` để đẩy file vào Thùng rác máy tính.
- **Tham số đầu vào**: 
  - `from: &str` (Bắt buộc)
  - `to: &str` (Bắt buộc)
  - `overwrite: bool` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `list_trash_local`, `trash_restore_local`, `trash_empty_local`
- **Mô tả**: Quản lý Thùng rác cục bộ của hệ điều hành.
- **Tham số đầu vào**: 
  - `item_id: &str` (Bắt buộc đối với restore)
- **Đầu ra**: Tùy hàm.

- **Tên hàm**: `get_thumbnail`
- **Mô tả**: Trích xuất ảnh thu nhỏ dạng Base64 cho ảnh, PDF, video bằng `image` và `ffmpegthumbnailer`/`pdftoppm`.
- **Tham số đầu vào**: 
  - `path: &str` (Bắt buộc)
- **Đầu ra**: `Result<String, String>`
