# models.rs
Tài liệu mô tả các cấu trúc và hàm helper dùng chung.

- **Tên struct**: `FileItem`
- **Mô tả**: Cấu trúc đại diện cho một file/thư mục. Gồm các field: `name`, `is_dir`, `size`, `mod_time`, `owner`, `group`, `permissions`.

- **Tên struct**: `TrashItemLocal` / `SyncPair` / `TransferItem`
- **Mô tả**: Các model lưu trữ dữ liệu luồng ứng dụng và tiến trình transfer.

- **Tên hàm**: `get_default_data_dir`
- **Mô tả**: Trả về thư mục lưu trữ cấu hình mặc định (tùy HĐH).
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Option<PathBuf>`

- **Tên hàm**: `resolve_filen_bin`
- **Mô tả**: Tìm đường dẫn binary `filen` (chạy qua biến môi trường hoặc thư mục cài đặt).
- **Tham số đầu vào**: Không có
- **Đầu ra**: `PathBuf`

*(Và nhiều hàm parser tiện ích khác như `parse_size_bytes`, `parse_sync_pairs_json`)*
