# Transfer Panel Component (TUI)

Khu vực (Panel) hiển thị danh sách các tiến trình upload/download đang diễn ra. Có thể bị thu nhỏ hoặc mở rộng.

## 1. Trách nhiệm (Responsibilities)
- Nhận danh sách các `TransferItem` từ State (bao gồm percent, bytes, speed, status).
- Render chúng dưới dạng một bảng hoặc danh sách.
- Hỗ trợ cuộn (nếu danh sách quá dài).
- Đánh dấu trạng thái màu sắc: Đang chạy (Xanh), Hoàn tất (Xanh lá), Lỗi (Đỏ), Hủy (Vàng).
- Vẽ thanh tiến trình (Gauge / Progress Bar) bằng `ratatui::widgets::Gauge` hoặc ký tự `[=======>  ]`.

## 2. Tiêu chuẩn Phân rã Code
- Bắt buộc tạo `src/ui/components/transfer_panel.rs`.
- Logic render thanh Progress Bar phải được đóng gói vào một hàm con `render_progress_bar`. Không để hàm render chính của Transfer Panel bị phình to.
