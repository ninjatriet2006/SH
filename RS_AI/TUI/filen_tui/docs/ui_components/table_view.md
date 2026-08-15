# Table View Component (TUI)

Tài liệu quy định cách render danh sách chi tiết các file bên trong thư mục (cột bên phải của Explorer Pane).

## 1. Trách nhiệm (Responsibilities)
- Render bảng (Table) với các cột: Name, Size, Modified Date.
- Hỗ trợ đánh dấu (Select) nhiều hàng cùng lúc (hiển thị màu khác hoặc đánh dấu `[*]`).
- Hỗ trợ đánh dấu (Highlight) hàng mà con trỏ đang trỏ tới.
- Cắt gọt (Truncate) tên file nếu quá dài so với chiều rộng màn hình.

## 2. Tiêu chuẩn Phân rã Code
- Bắt buộc tạo file `src/ui/components/table_view.rs`.
- Sử dụng `ratatui::widgets::Table`.
- Phải tách riêng hàm parse dung lượng `size_to_string(bytes)` ra một module tiện ích chung `src/utils/format.rs`.
