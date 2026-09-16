# Fix & Feature: Fingerprint Pool UI (Giao diện quản lý mảng Hồ sơ ẩn danh) & Vá lỗi Path của Log File

## Hiện trạng & Vấn đề từ User
1. **Fingerprint Pool UI:** `Opencode` đã hoàn thiện rất tốt logic Backend cho Fingerprint Pool (quay vòng `mod len()`). Tuy nhiên, `Opencode` đã **quên hoàn toàn việc xây dựng Giao diện (Frontend UI)** cho tính năng này! Hiện tại tab `FingerprintTab.tsx` vẫn là giao diện cũ kĩ, chỉ cho phép sửa một profile tĩnh duy nhất.
2. **Lỗi Path của JSONL Log:** `Opencode` đã thông minh khi dùng `.jsonl` thay vì `SQLite` để lưu log siêu nhẹ (Disk Log Storage). Rất xuất sắc! Tuy nhiên, hàm `get_logs_dir()` ở `backend/src/monitor/mod.rs` lại đang dùng `std::env::current_exe().parent()`. Việc lưu log chung với thư mục `.exe` sẽ gây lỗi sập App (Permission Denied) trên Linux/macOS khi đóng gói bản Release. Cần chuyển nó về chung nhà với `config.json` ở `~/.config`.

## Nhiệm vụ (Task List) Dành riêng cho Opencode Agent (Mức độ Code chi tiết)

### Bước 1: Đại tu toàn diện File `FingerprintTab.tsx`
- **Vị trí:** `frontend/src/components/FingerprintTab.tsx`
- **Hành động:** 
  - Xây dựng một giao diện dạng **Danh sách (List)**. Mảng dữ liệu nguồn là: `config.fingerprint_pool`.
  - Bổ sung nút **"Add New Fingerprint Profile"** ở góc trên.
  - Mỗi phần tử trong danh sách cần hiển thị:
    - User-Agent (rút gọn nếu quá dài).
    - Các cờ (Flags) dưới dạng các Tag nhỏ.
    - Nút **Edit** và **Delete** bên cạnh mỗi phần tử.

### Bước 2: Tạo Modal Thêm/Sửa Fingerprint Profile
- **Hành động:**
  - Viết một form (Modal) cho phép nhập: `custom_user_agent`, `strip_sdk_headers`, `strip_ide_headers`, `strip_sec_ch_ua`, `remove_empty_headers`, `mask_local_paths_in_body`.
  - Khi lưu, đẩy đối tượng này vào mảng `config.fingerprint_pool` và gọi `onSaveConfig`.
  - Đồng bộ logic Backend: Đảm bảo Backend thực sự dùng mảng `fingerprint_pool` thay vì `fingerprint_profile` tĩnh. Nếu mảng rỗng, Backend có thể fallback về `fingerprint_profile` tĩnh để không bị lỗi.

### Bước 3: Sửa đường dẫn lưu File Log (Fix Permission Denied)
- **Vị trí:** File `backend/src/monitor/mod.rs` (hàm `get_logs_dir`).
- **Hành động:**
  - Sao chép nguyên xi logic tìm thư mục `.config` của hàm `config_path()` trong `backend/src/proxy/mod.rs`.
  - Ghi đè hàm `get_logs_dir()`: Nếu có biến môi trường `VPN_AI_PROXY_CONFIG_DIR`, dùng nó. Nếu không, lấy `HOME/.config/vpn_ai_proxy_gui/logs`.
  - Xóa sạch đoạn code dùng `current_exe()`.

## ⚠️ Yêu cầu bắt buộc: Kế hoạch tự kiểm chứng độc lập (Self-Verification)
1. Bấm vào tab "Fingerprint Cloaking". Add profile mới `TEST_AGENT_1`. Tắt App mở lại vẫn còn nguyên.
2. Mở terminal, gọi proxy. Kiểm tra thư mục `~/.config/vpn_ai_proxy_gui/logs/` có xuất hiện file `traffic_log.jsonl` không. Đảm bảo nó KHÔNG CÒN lưu ở thư mục chứa file `.exe` nữa.
