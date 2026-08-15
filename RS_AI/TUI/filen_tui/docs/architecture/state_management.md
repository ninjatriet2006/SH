# State Management (app/mod.rs)

## 1. Trách nhiệm (Responsibility)
Quản lý toàn bộ vòng đời và trạng thái (State) của ứng dụng TUI `filen_tui`. Đóng vai trò là Single Source of Truth chứa: danh sách tài khoản, trạng thái cửa sổ (Pane), popup hiện tại và biến cờ cho tiến trình nền.

## 2. Trạng thái phụ thuộc (State Dependencies)
- `PaneState`: Trạng thái độc lập cho 2 cửa sổ trái/phải (`is_local`, `items`, `scroll_offset`, `selected_idx`).
- `PopupState`: Biến Enum mang dữ liệu cụ thể cho từng loại Popup (ví dụ: `ConfirmDelete` chứa tên tệp cần xóa, `LoginInput` chứa email/password).
- `WebDavServerState` / `S3ServerState`: Chứa thông tin cấu hình cổng, tài khoản và buffer log cho máy chủ ảo.
- Channel `msg_tx`: Lưu một đầu của `mpsc::UnboundedSender` để các luồng nền có thể đẩy sự kiện (AppEvent) về cho UI.

## 3. Sự kiện (Events/Actions)
Không trực tiếp lắng nghe phím tắt. Thay vào đó, nó định nghĩa `enum AppEvent` (Key, Tick, AsyncFinished, RemoteLoadFinished,...) để luồng event loop có thể điều hướng.

## 4. Định hướng Refactor
- **Vấn đề**: `app/mod.rs` đang quá lớn (chứa cả struct `App`, `PaneState`, `AppEvent` và các logic mạng, webdav).
- **Giải pháp**: Tách `PaneState` ra thành `pane.rs`. Tách các struct liên quan đến Server ra thành `servers.rs`. Tách `enum AppEvent` ra thành `events.rs`.
