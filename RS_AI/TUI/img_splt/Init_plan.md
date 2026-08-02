# Kế hoạch Khởi tạo (Init Plan) - Rust Rewrite

Bản kế hoạch này lưu trữ các quyết định thiết kế và yêu cầu kỹ thuật ban đầu cho quá trình chuyển đổi script `Image_Spliter.sh` sang ngôn ngữ Rust.

## 1. Mục tiêu (Objective)
Xây dựng một phiên bản phần mềm mạnh mẽ, đa luồng, an toàn dữ liệu và dễ dàng bảo trì nhằm thay thế triệt để Bash script truyền thống.

## 2. Các yêu cầu cốt lõi
- **Chuyển dịch 1-1 các tính năng cốt lõi:** Kế thừa toàn bộ logic chia Chapter, Upscale ảnh nhỏ hơn 600px lên 1280px, đổi định dạng và đánh chỉ mục của file Bash gốc.
- **Tốc độ:** Tích hợp Multi-threading (`rayon`) để tăng tốc tiến trình thay vì chạy tuần tự `ffmpeg`.
- **Tương tác CLI (Menu):** Sử dụng `inquire` để tạo Menu tương tác (bấm lên/xuống) thay vì bắt người dùng gõ số mù mờ.
- **Linh hoạt cấu hình:** Không hardcode. Mọi tham số từ `max_retries` tới `max_files_per_folder` đều phải được rút ra một file `settings.yaml`.
- **Bảo mật & Chống mất dữ liệu:**
    1. Cơ chế xin quyền (`sudo`) thông minh nếu gặp file hỏng/bị khóa.
    2. Chế độ xử lý `Out-of-place` (Không sửa trực tiếp trên file gốc mà ghi vào mục tạm `_process` trước).
    3. Bộ lặp an toàn (Retry logic) để xử lý file 0-byte từ ffmpeg.
- **Thuật toán chia file:** Cung cấp 3 chế độ (Balanced, Greedy, Fixed Count).

## 3. Cấu trúc Module
Dự án được phân rã thành các thành phần độc lập:
1. `config.rs`: Cấu hình YAML.
2. `env_check.rs`: Quản lý phụ thuộc hệ thống (ffmpeg, ffprobe) và Terminal.
3. `scanner.rs`: Cỗ máy dò tìm file tự động bỏ qua file thực thi và lọc permission.
4. `renamer.rs`: Tiền xử lý - Cắt ghép tên file, padding số không (0).
5. `processor.rs`: Bộ não chính chạy `rayon` để kích hoạt `ffmpeg` + `ffprobe`.
6. `distributor.rs`: Toán học logic dùng để phân chia số lượng file vào các folder sao cho hợp lý.
