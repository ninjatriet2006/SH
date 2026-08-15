# Transfer Panel Key Handlers (TUI)

Tài liệu quy định cách tách biệt mã xử lý phím khi người dùng di chuyển focus sang thanh Transfer Panel ở dưới cùng màn hình (quản lý tiến trình upload/download).

## 1. Trách nhiệm (Responsibilities)
- Cho phép người dùng cuộn lên/xuống danh sách các tác vụ đang truyền tải.
- Hủy (Cancel) hoặc Xóa (Remove) tác vụ.

## 2. Các phím cơ bản
- `Up` / `Down` / `j` / `k`: Di chuyển con trỏ trong danh sách transfer.
- `x` / `Delete`: Hủy một tác vụ đang chạy hoặc xóa một tác vụ đã hoàn thành/lỗi khỏi danh sách.
- `C` (Shift+C): Clear toàn bộ các tác vụ đã hoàn tất.
- `Esc`: Trả lại focus cho Explorer Panel (Left/Right).

## 3. Tiêu chuẩn Phân rã Code
Tạo file `src/app/key_handlers/transfer.rs` chứa hàm `handle_transfer_keys(app, key)`. 
Không viết logic này chung vào `explorer.rs` vì sẽ làm file explorer bị quá tải trách nhiệm.
