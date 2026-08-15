# Layout Main (Bố cục Giao diện chính)

File: `ui/layout.rs`

## 1. Trách nhiệm (Responsibility)
Biến trạng thái tĩnh (App) thành giao diện lưới `ratatui` (Blocks, List, Paragraph) và kết xuất (render) lên Terminal. Đây là hàm gốc đệ quy vẽ toàn bộ màn hình.

## 2. Trạng thái phụ thuộc (State Dependencies)
- `App.current_screen`: Trạng thái quyết định màn hình nào (MainMenu, Explorer, Account, Servers) sẽ được vẽ.
- `App.popup_state`: Nếu khác `None`, hàm sẽ gọi các block UI của Pop-up đè lên lớp cao nhất.

## 3. Định hướng Refactor
- **Vấn đề**: File `ui/layout.rs` quá dài (hơn 1000 dòng) chứa mã nguồn vẽ cho tất cả mọi màn hình và popup. Khó maintain.
- **Giải pháp**: 
  - Khởi tạo thư mục `ui/screens/` để chứa giao diện màn hình chính (`explorer.rs`, `account.rs`).
  - Khởi tạo thư mục `ui/popups/` để chứa code vẽ từng popup (`delete.rs`, `login.rs`).
  - `layout.rs` chỉ còn đóng vai trò là một Router để gọi các hàm vẽ từ các module trên.
