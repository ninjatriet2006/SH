# Operations: Remote API & CLI Wrappers

File: `app/operations.rs`

## 1. Trách nhiệm (Responsibility)
Đóng gói (wrap) toàn bộ các tập lệnh CLI do ứng dụng lõi (backend) cung cấp. Đây là module duy nhất được quyền gọi `tokio::process::Command` và phân tích kết quả chuỗi `stdout`/`stderr` từ máy chủ ảo hoặc hệ thống nhúng.

## 2. Danh mục Giao diện lập trình (API Surface)
- **Xác thực & Bảo mật**:
  - `login_new`: Đăng nhập, trả về log tiến trình hoặc yêu cầu 2FA.
  - `export_auth_config`, `export_api_key`: Trích xuất chuỗi mã hóa phiên bản CLI.
- **Dữ liệu Thám hiểm (Explorer Data)**:
  - `list_remote`: Chạy `filen ls --json` -> Parse mảng JSON thành `Vec<FileItem>`.
  - `statfs`: Lấy phân tích bộ nhớ.
- **CRUD Operations**:
  - `upload`, `download`, `mv` (Move/Cut), `cp` (Copy), `mkdir`, `rm` (Remove/Trash).
- **Tính năng Mở rộng (Extended)**:
  - `create_link`: Sinh Public Link.
  - `favorite` / `unfavorite`: Gắn cờ tệp tin.
  - `trash_list`, `trash_restore`, `trash_empty`.

## 3. Kiến trúc Cầu nối (Async Command Execution)
Toàn bộ các lệnh đều dùng mô hình bất đồng bộ (`async fn`) kèm cơ chế *Timeout* (mặc định 30 giây thông qua `tokio::time::timeout`). Nếu tiến trình backend bị treo do mất mạng hoặc đợi input tương tác, hàm sẽ tự động hủy vòng lặp và trả lỗi `Err(timeout)` lên UI để tránh đơ toàn bộ ứng dụng.

## 4. Định hướng Refactor
- Khối lượng hàm tĩnh (static methods) trong `Operations` quá lớn (gần 30 hàm).
- Có 4 cờ `#[allow(dead_code)]` trên các hàm chưa được kích hoạt (`list_links`, `trash_delete`, v.v...).
- **Giải pháp**: Xé nhỏ struct `Operations` ra thành các module tĩnh riêng biệt: `ops::auth`, `ops::filesystem`, `ops::trash`, `ops::sharing`. Cắt bỏ các cờ dead_code bằng cách tích hợp trực tiếp chúng vào giao diện, hoặc thêm cờ bỏ qua cảnh báo toàn cục.
