# Tree View Component (TUI)

Tài liệu quy định cách render cấu trúc cây thư mục ở cột bên trái của Explorer Pane.

## 1. Trách nhiệm (Responsibilities)
- Nhận vào `AppState` và ID của Pane (`left` hoặc `right`).
- Render danh sách thư mục cha/con dưới dạng cây (Tree).
- Hỗ trợ hiển thị trạng thái đang mở/đóng của thư mục (`[+]`, `[-]`).
- Đánh dấu (Highlight) thư mục đang được chọn.

## 2. Tiêu chuẩn Phân rã Code (Thực tế)
- Tách hàm render khỏi `src/ui/layout.rs` và đặt tại `src/ui/components/tree_view.rs`.
- Sử dụng `ratatui::widgets::List` hoặc custom Tree widget.
- Code render không được chứa bất kỳ logic tính toán đường dẫn nào, chỉ lấy dữ liệu đã được tính sẵn từ `app.panes[id].tree_items`.
