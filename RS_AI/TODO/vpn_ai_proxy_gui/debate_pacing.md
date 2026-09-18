# Debate: Tích hợp pacing request vào vpn_ai_proxy_gui

## Đề xuất của Spark (vòng 0)
Upstream AI API thường rate-limit theo tần suất. Gateway hiện tại bắn request
dồn dập khi burst → ăn 429 oan → key bị đếm lỗi oan (liên quan max_key_failures).
Đề xuất thêm pacing phía gateway, học từ pace_shim đã chứng minh ngoài thực tế.

**Thiết kế:**
1. Field mới per-tunnel `min_request_interval_ms: u64` (default 0 = tắt, serde default, migration).
2. Thực thi trong `handle_route_request`, TRƯỚC khi lấy semaphore permit (không giữ permit trong lúc sleep).
3. Thuật toán slot-reserve: `Mutex<HashMap<tunnel_id, Instant>>` trong AppState; lock → wait = interval - elapsed → đặt last = now + wait → unlock → `tokio::sleep(wait)` ngoài lock. Request đồng thời xếp hàng đúng thứ tự, không giữ lock khi ngủ.
4. SSE: chỉ giãn lúc START request, stream dài không ảnh hưởng.
5. Request rớt 429-concurrency/503/404 sau pacing: slot đã tiêu — chấp nhận (vẫn giãn được lượt chạm upstream).
6. Đồng hồ `tokio::time::Instant` (monotonic, miễn nhiễm NTP).
7. UI: ô nhập ms trong modal tunnel (0 = Unlimited), giống `max_concurrent_streams`.

## Câu hỏi mở cho phản biện
- Q1: per-tunnel hay global 1 ngưỡng chung? (Spark: per-tunnel — mỗi upstream limit khác nhau.)
- Q2: fixed-interval hay token-bucket cho burst (cho N request đầu miễn giãn)? (Spark: fixed trước, bucket là phức tạp chưa cần.)
- Q3: có nên tự nới interval khi gặp 429 upstream (adaptive backoff)? (Spark: không — 429 upstream đã có key-rotation xử lý; adaptive là state machine mới.)

## Luật (bắt buộc, mọi vòng)
- NOCODE tuyệt đối: chỉ đọc + phân tích, không sửa/tạo file.
- Output: tối đa 10 mục `[ĐỒNG Ý/PHẢN ĐỐI/HỎI] — 1-2 dòng`, tiếng Việt, ngắn.
- Chỉ điểm ĐỒNG THUẬN 100% 2 bên mới được implement.

## Bổ sung sau implement (user yêu cầu per-ENDPOINT, không chỉ per-tunnel)
- `RouteRule.min_request_interval_ms` (default 0); effective = max(route, tunnel);
  slot pacing key theo `route_id`; prune ở delete_route + refresh_clients.
- Ô nhập ms trong modal route (0 = theo tunnel). Verify 27/27 + build sạch.
