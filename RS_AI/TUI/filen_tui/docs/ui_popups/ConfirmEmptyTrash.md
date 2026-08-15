# Popup: Xác nhận dọn dẹp Thùng Rác (ConfirmEmptyTrash)

## 1. Trách nhiệm
Đây là hộp thoại (Modal) màu đỏ nguy hiểm nhất hệ thống, được gọi ra khi người dùng chọn "Dọn Dẹp Thùng Rác" tại mục tùy chọn tệp tin (Alt+O -> Option 3). Nó ngăn chặn hành vi vô tình xóa vĩnh viễn toàn bộ Thùng rác.

## 2. Dòng chảy Logic (Data Flow)
1. Kích hoạt: Đặt `app.popup_state = PopupState::ConfirmEmptyTrash`.
2. UI vẽ ra 1 Paragraph Block màu Đỏ nằm chính giữa màn hình.
3. Luồng sự kiện bàn phím (trong `app/key_handlers/explorer.rs`):
   - Nghe phím `Y` hoặc `Enter`: Gọi hàm lõi `Operations::trash_empty`.
   - Nghe phím `N` hoặc `Esc`: Set State về `None` (Hủy bỏ).

## 3. Định hướng Refactor
- Nên bóc tách logic nghe sự kiện phím cho Popup này vào file `app/key_handlers/popups/confirm_empty_trash.rs`.
- Nên bóc tách logic vẽ Block này vào `ui/popups/confirm_empty_trash.rs`.
