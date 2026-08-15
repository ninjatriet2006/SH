# Operations: Local I/O (Thao tác đĩa cứng cục bộ)

File: `app/operations.rs`

## 1. Trách nhiệm (Responsibility)
Thay vì bọc các câu lệnh gọi CLI như `remote_api`, nhóm hàm Local I/O sử dụng trực tiếp thư viện tiêu chuẩn của Rust (`std::fs`, `std::io`) để đọc/ghi và duyệt tệp trên máy tính cục bộ của người dùng. Tối ưu hóa hiệu năng và tránh phải gọi tiến trình phụ.

## 2. Danh mục Giao diện lập trình (API Surface)
- `list_local(path)`: 
  - Gọi `std::fs::read_dir(path)`.
  - Phân tích Metadata (thư mục/file, kích thước, thời gian chỉnh sửa).
  - Trả về danh sách `Vec<FileItem>`. Bỏ qua các file lỗi quyền truy cập.
- `mkdir_local(path)`: Gọi `std::fs::create_dir_all`.
- Khối dọn dẹp không dùng tới (Dead Code / Future use):
  - `move_local`, `copy_local`, `copy_dir_recursive`, `delete_local`.
  - (Đã được đánh dấu `#[allow(dead_code)]` để tránh cảnh báo trình biên dịch, giữ lại để làm cầu nối Sync sau này).

## 3. Kiến trúc Đồng bộ (Synchronous Execution)
Khác với Remote API, các hàm Local hiện tại đang chạy **Đồng bộ (Synchronous)** (hàm `pub fn` thường thay vì `async fn`) do I/O cục bộ trên Rust cực kỳ nhanh. Tuy nhiên, nếu thao tác trên thư mục có hàng chục nghìn tệp, giao diện vẫn có thể bị đứng khựng lại một khoảnh khắc.

## 4. Định hướng Refactor
- Phân tách riêng các hàm local này sang một file `ops/local_io.rs`.
- Cân nhắc nâng cấp `std::fs` lên `tokio::fs` nếu muốn biến các tác vụ này thành bất đồng bộ hoàn toàn, nhằm giữ cho vòng lặp UI chạy ở 60fps mượt mà kể cả khi duyệt thư mục khủng.
