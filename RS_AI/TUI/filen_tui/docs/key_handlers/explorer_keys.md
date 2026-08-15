# Key Handlers: Explorer (app/key_handlers/explorer.rs)

## 1. Trách nhiệm (Responsibility)
Chịu trách nhiệm diễn dịch các phím tắt khi người dùng đang ở màn hình duyệt tệp (Explorer). Nó ánh xạ các thao tác vật lý (nhấn phím) thành các thay đổi trạng thái (State Mutations) hoặc gọi các lệnh CLI.

## 2. Danh sách Phím tắt (Key Bindings)
### Điều hướng cơ bản
- `Up` / `Down` / `k` / `j`: Lên/Xuống danh sách tệp.
- `Enter`: Truy cập thư mục hoặc mở tệp cục bộ.
- `Backspace` / `h`: Trở lại thư mục cha (`..`).
- `Tab`: Chuyển đổi qua lại giữa cửa sổ Local (Trái) và Cloud (Phải).

### Thao tác trên tệp
- `Space`: Chọn/Bỏ chọn tệp (hỗ trợ chọn nhiều tệp).
- `F2` / `r`: Kích hoạt `PopupState::RenameInput`.
- `F7` / `n`: Kích hoạt `PopupState::NewFolderInput` (Tạo thư mục).
- `Delete` / `x`: Kích hoạt `PopupState::ConfirmDelete`.
- `Ctrl + C` / `Ctrl + X`: Sao chép/Cắt tệp đã chọn vào `app.clipboard`.
- `Ctrl + V`: Dán tệp từ clipboard sang cửa sổ đối diện (Hỗ trợ Tải lên, Tải xuống, Sao chép nội bộ).

### Menu tính năng mở rộng
- `Alt + O`: Mở `SpecialActionsMenu` để truy cập các tính năng nâng cao (Tạo link, Yêu thích, Thùng rác, Xem Stat).

## 3. Kiến trúc luồng (Data Flow)
Mỗi nhánh (match arm) thường làm theo quy trình 3 bước:
1. Kiểm tra biến đổi UI (Nếu cần mở Popup, gán `app.popup_state = ...`).
2. Kích hoạt Async (Nếu cần gọi Backend, set `app.is_loading = true` và gọi `Operations::...`).
3. Dọn dẹp (Kết thúc logic, UI tự động cập nhật ở vòng lặp render tiếp theo).

## 4. Định hướng Refactor
- File `explorer.rs` dài gần 1000 dòng. Cần bóc tách khối lệnh xử lý Paste (Ctrl+V) ra thành hàm `handle_paste(&mut app)`. 
- Bóc tách toàn bộ `match PopupState::...` (khi đang ở trong Popup) ra các file riêng biệt nằm trong `key_handlers/popups/`.
