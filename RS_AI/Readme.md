# Image Spliter (Rust Edition)

> Một công cụ Command Line (CLI) cực nhanh, an toàn và thông minh dùng để xử lý hàng loạt, chuẩn hóa định dạng, tăng độ phân giải và phân loại ảnh một cách có tổ chức.

## 🚀 Tính năng Nổi bật
- **Hiệu năng Đa luồng (Multi-threading):** Tận dụng tối đa sức mạnh của CPU thông qua `rayon` để xử lý hàng nghìn file ảnh cùng lúc.
- **Xử lý An toàn tuyệt đối (Out-of-place):** Thao tác `ffmpeg` được thực hiện trên một thư mục ảo `_process`. Không bao giờ gây hỏng hóc hay làm mất dữ liệu gốc dù có bị mất điện giữa chừng.
- **Tự động Khắc phục Lỗi (Auto Retry):** Tự động phát hiện file rỗng/lỗi và chạy lại (retry) tiến trình xử lý ảnh.
- **Cấu hình Thông minh (YAML):** Không cần chỉnh sửa code! Người dùng có thể tùy biến toàn bộ thông số qua file `settings.yaml` (tự động tạo ra nếu chưa có).
- **Phân chia Thư mục Đa thuật toán:** Hỗ trợ chia file theo 3 chuẩn: Balanced (Cân bằng tuyệt đối), Greedy (Lấp đầy) và Fixed Count (Cố định thư mục).
- **Xử lý Trơn tru với GUI:** Tự động gọi hệ điều hành sinh ra một cửa sổ Terminal để chứa tiến trình nếu bạn double-click trực tiếp từ màn hình Window/Linux.
- **Auto Sudo:** Phát hiện file bị khóa quyền (Permission denied) và kích hoạt cơ chế xin quyền root để cưỡng ép xử lý.

## ⚙️ Hướng dẫn Sử dụng
Công cụ này được đóng gói thành một file thực thi duy nhất (standalone binary).
1. Copy file `img_splt` vào trong thư mục chứa đống ảnh lộn xộn của bạn.
2. Double click để mở nó lên, hoặc chạy lệnh trên terminal: `./img_splt`
3. Tận hưởng tốc độ và sự tiện lợi!
