# Luồng Hoạt Động (Workflow)

Tài liệu này mô tả chi tiết đường đi của một file ảnh khi được đi qua công cụ `img_splt`.

## Sơ đồ Thực thi (Flow)

### Bước 1: Khởi động & Môi trường
1. Công cụ khởi động. Nếu bạn double-click từ GUI, nó sẽ tự động đánh thức một màn hình Terminal để hứng lấy ứng dụng.
2. Kiểm tra `ffmpeg` và `ffprobe`. Nếu thiếu, ứng dụng sẽ xin quyền `sudo` để cài đặt.
3. Sinh ra (hoặc đọc) file `settings.yaml` trong thư mục hiện tại để tải cấu hình.

### Bước 2: Quét File (Scanner)
- Công cụ tìm kiếm toàn bộ file trong thư mục cấp 1 (Không đụng vào thư mục con).
- Tự động bỏ qua file thực thi (Executable) và file cấu hình (như `settings.yaml`).
- Quét quyền đọc file, nếu phát hiện file bị khóa (Permission Denied), nó sẽ cảnh báo và cho phép nhập Pass Sudo để khôi phục quyền truy cập.

### Bước 3: Tiền xử lý - Đổi tên (Renamer)
Người dùng chọn 1 trong 4 chế độ đổi tên (Bỏ qua / Lấy đầu / Lấy đuôi / Cắt theo index). 
Nếu đổi tên xảy ra trùng lặp, cơ chế `Zero-padding` sẽ đánh số thứ tự đệm đuôi (`_001`, `_002`) để file Explorer trên Windows/Linux luôn có thể Sort đúng trật tự.

### Bước 4: Xử lý Đa luồng (Format & Upscale)
- Các file ảnh lọt qua cửa được đưa vào mảng `image_files`.
- **Rayon Threadpool:** Giao việc cho tất cả các nhân (Cores) của CPU hoạt động hết công suất.
- Các lệnh `ffmpeg` được gọi ngầm để upscale ảnh hoặc đổi đuôi theo chuẩn định dạng đích.
- Kết quả được "tuôn" vào một thư mục tạm thời có tên `_process`.

### Bước 5: Kiểm duyệt & Dọn rác
- Kiểm tra toàn bộ thư mục `_process`. Những file có kích thước `0-byte` (Bị hỏng ngầm) sẽ bị thẳng tay xóa bỏ.
- Cơ chế **Retry Logic:** Nếu có file hỏng, hệ thống tự kích hoạt tiến trình xử lý lại file đó tối đa `5` vòng lặp (max_retries).
- Khi toàn bộ quá trình thành công và an toàn tuyệt đối, hệ thống xóa bỏ hình ảnh gốc và đổi tên (Swap) thư mục `_process` thành thư mục làm việc hiện tại.

### Bước 6: Phân phối Chapter (Distributor)
- Hệ thống đếm lại số lượng file hoàn chỉnh thực tế cuối cùng.
- Áp dụng thuật toán chia thư mục (Cân bằng / Lấp đầy / Cố định số lượng) do người dùng chọn.
- Di chuyển file ảnh vào các Folder con tương ứng theo mẫu `Chapter X.Y`.

### Bước 7: Kết thúc
- Hiển thị thông báo.
- Dừng (Pause) tiến trình để người dùng đọc thông số.
- Đóng Terminal khi người dùng ấn Enter.
