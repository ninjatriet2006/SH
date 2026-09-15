# Feature: Quản lý Tiến trình VPN (Tích hợp Nút Start/Stop)

## Hiện trạng & Vấn đề từ User
Bức ảnh chỉ ra lỗi `Connection refused (os error 111)` - tức là cổng SOCKS5 1080 chưa được mở vì phần mềm AdGuard VPN chưa được khởi chạy ở chế độ Proxy.
Hiện tại, Proxy của chúng ta có tính năng định tuyến qua các hầm SOCKS/HTTP, nhưng nó lại hoàn toàn thụ động (chỉ biết kết nối). Nếu VPN chưa bật, nó báo lỗi và bắt người dùng phải tự đi tìm cách gõ lệnh bật VPN trên Terminal. 
- Điều này mang lại UX (trải nghiệm người dùng) rất tệ vì không phải ai cũng biết các lệnh CLI bí truyền của AdGuard hay Cloudflare WARP.

## Nhiệm vụ (Task List)
- [ ] **Mở rộng Cấu trúc Dữ liệu (`config.json`):**
  - Thêm 2 trường `start_command` và `stop_command` (kiểu chuỗi String) vào struct `Tunnel` trong backend.
- [ ] **Bổ sung UI Cấu hình (Modal Add/Edit Tunnel):**
  - Thêm ô nhập liệu "Start Command" và "Stop Command".
  - Thêm nút "Gợi ý lệnh" (Templates) cho các VPN phổ biến:
    - *Cloudflare WARP:* `warp-cli connect` / `warp-cli disconnect`
    - *AdGuard VPN:* `adguardvpn-cli connect` / `adguardvpn-cli disconnect`
- [ ] **Cập nhật UI Card (TunnelsTab):**
  - Cạnh nút `Test Exit IP`, thêm một nút bấm **▶️ Run VPN** (Khởi động).
  - Nếu trạng thái đang là ONLINE, nút này đổi thành **⏹️ Stop VPN**.
- [ ] **Xây dựng module Backend (Tauri Process Manager):**
  - Viết 2 hàm `start_tunnel_process` và `stop_tunnel_process`.
  - Sử dụng `std::process::Command` của Rust để kích hoạt lệnh chạy ngầm dưới hệ điều hành (chạy qua Bash trên Linux hoặc CMD/PowerShell trên Windows).

## ⚠️ Yêu cầu bắt buộc: Kế hoạch AI tự kiểm chứng độc lập (Self-Verification)
1. **Kiểm thử Child Process:** Cấu hình lệnh `start_command` là `python3 -m http.server 1080` (giả lập bật 1 dịch vụ ở cổng 1080).
2. **Kiểm thử Luồng UI:** Bấm nút **▶️ Run VPN**. Chờ 2 giây, bấm lại `Test Exit IP` xem có hết lỗi `Connection refused` không. Bấm **⏹️ Stop VPN** xem dịch vụ có thực sự bị tắt hay bị kẹt (zombie process).
3. **Ghi nhận:** Báo cáo chi tiết vào cuối file.
