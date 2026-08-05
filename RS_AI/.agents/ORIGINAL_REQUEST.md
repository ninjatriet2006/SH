# Original User Request

## Initial Request — 2026-08-05T23:37:12+07:00

Thiết kế bản vẽ giao diện (UI design document) cho phần mềm `filen_GUI` theo phong cách neon, kết hợp ưu điểm của giao diện cũ và thiết kế mới. Bản thiết kế chỉ nằm ở mức độ tài liệu, lưu vào thư mục `docs` (ngang hàng `src`), yêu cầu mô tả chi tiết từng phần nhỏ và phân nhánh để AI khác có thể đọc hiểu và lập trình không bị sai.

Working directory: /home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui
Integrity mode: demo

## Requirements

### R1. Giao diện Neon & Chi tiết
Tạo tài liệu Markdown mô tả thiết kế giao diện với phong cách Neon. Phải mô tả thật chi tiết từng màn hình, từng component (màu sắc, hiệu ứng glow, bố cục, typography, state hover/active).

### R2. Kết hợp Cũ & Mới
Phân tích sơ lược luồng thao tác cũ và tích hợp các tính năng/luồng thao tác ưu việt vào thiết kế mới, đảm bảo UX mượt mà, nhiều phân nhánh logic.

### R3. Cấu trúc Tài liệu trong `docs`
Tài liệu phải bao gồm sơ đồ phân nhánh màn hình (user flow), và mô tả dữ liệu UI state. Tất cả được lưu trong thư mục `docs/` (ngang hàng thư mục `src`). Không được viết code trực tiếp cho ứng dụng lúc này, chỉ thiết kế.

## Acceptance Criteria

### Tài liệu thiết kế
- [ ] Tồn tại thư mục `docs/` ngang hàng với `src/`.
- [ ] Có file thiết kế Markdown nằm trong `docs/` chứa toàn bộ mô tả.
- [ ] Thiết kế có định nghĩa rõ mã màu hex (neon), kích thước, font chữ và hiệu ứng sáng (glow).
- [ ] Có liệt kê đủ user flow / sơ đồ phân nhánh các màn hình.
- [ ] Độ chi tiết đủ cao để một AI khác không cần hỏi lại mà vẫn code được ra UI.
