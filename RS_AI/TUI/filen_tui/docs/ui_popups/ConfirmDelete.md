# Popup: Xác nhận Xóa (ConfirmDelete)

## 1. Trách nhiệm (Responsibility)
Hiển thị hộp thoại cảnh báo khi người dùng nhấn phím Xóa (Delete/x) trên một hoặc nhiều tệp/thư mục. Chặn thao tác xóa nhầm bằng cách buộc người dùng xác nhận rõ ràng.

## 2. Giao diện (UI/UX)
- Khối văn bản (`Paragraph`) được bọc trong một `Block` viền đỏ rực (`Color::Red`) kèm chữ in đậm.
- Hiển thị tên tệp sắp bị xóa (`> tên_tệp`).
- Hiển thị hai nút giả lập: `[ Có (Y) ]` và `[ Không (N) ]`.

## 3. Luồng dữ liệu (Data Flow)
1. Kích hoạt: `app.popup_state = PopupState::ConfirmDelete { name: file_name }`.
2. Rendering: `layout.rs` pattern match nhánh `ConfirmDelete` và vẽ đè lên giữa màn hình `centered_rect(50, 30)`.
3. Xử lý sự kiện (`key_handlers/explorer.rs`):
   - Nhấn `Y` / `Enter`: Đặt `app.is_loading = true`, gọi `Operations::rm()` (nếu ở Cloud) hoặc `fs::remove_*` (nếu ở Local).
   - Nhấn `N` / `Esc`: Hủy bỏ, gán `popup_state = PopupState::None`.

## 4. Định hướng Refactor
- Phân tách khối code vẽ giao diện (khoảng 30 dòng) ra file `ui/popups/confirm_delete.rs`.
- Khối xử lý phím nằm trong `explorer.rs` nên chuyển sang `key_handlers/popups/confirm_delete.rs`.
