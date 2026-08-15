# Popup Key Handlers (TUI)

Tài liệu quy định cách tách biệt mã xử lý phím khi có một Cửa sổ nổi (Popup/Dialog) đang hiển thị đè lên màn hình chính.

## 1. Trách nhiệm (Responsibilities)
Chặn (Intercept) mọi phím bấm của người dùng, không cho lọt xuống Explorer hay Transfer Panel khi một Popup đang mở.

## 2. Các phím cơ bản
- `Esc`: Đóng Popup hiện tại, hủy bỏ thao tác (Cancel).
- `Enter`: Chấp nhận (Confirm) hành động của Popup (vd: Xóa, Đổi tên, Tạo thư mục).
- `y` / `n`: Cho các popup xác nhận nhanh (Yes/No).

## 3. Tiêu chuẩn Phân rã Code (Thực tế)
Thay vì gom chung tất cả logic vào `app.rs`, tạo một submodule `src/app/key_handlers/popups.rs`:
```rust
pub fn handle_popup_keys(app: &mut App, key: KeyEvent) {
    match app.active_popup {
        Popup::Rename => handle_rename_keys(app, key),
        Popup::Mkdir => handle_mkdir_keys(app, key),
        Popup::ConfirmDelete => handle_delete_keys(app, key),
        _ => {}
    }
}
```
Mỗi hàm nhỏ sẽ quản lý state `tui-textarea` tương ứng của Popup đó.
