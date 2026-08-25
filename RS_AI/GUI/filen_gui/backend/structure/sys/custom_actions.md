# sys/custom_actions.rs
Tài liệu tham chiếu các hàm quản lý action của người dùng.

- **Tên struct**: `CustomAction`
- **Mô tả**: Lưu trữ siêu dữ liệu (metadata) của các tùy chỉnh Context Menu như id, name, command thực thi (`exec`), icon, loại file áp dụng (`extensions`).

- **Tên hàm**: `get_custom_actions`
- **Mô tả**: Quét thư mục `~/.local/share/filen_gui/actions`, đọc và phân tích tất cả các file có đuôi `.action`.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Vec<CustomAction>` (Đã được sắp xếp theo tên)

- **Tên hàm**: `parse_action_file`
- **Mô tả**: (Hàm nội bộ) Phân tích cú pháp INI từ file `.action` để trích xuất ra struct `CustomAction`.
- **Tham số đầu vào**: `path: &PathBuf`
- **Đầu ra**: `Option<CustomAction>`
