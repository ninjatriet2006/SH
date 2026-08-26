# Dead-Model Registry

Registry ghi nhận các model bị coi là "chết" (không phản hồi / lỗi / timeout) để Lead né khi giao việc.

## Quy tắc sử dụng (Lead)
- **Ghi**: Khi agent fail do model (timeout, lỗi provider, không phản hồi) → thêm 1 dòng `DEAD` (hoặc tăng `fail_count` nếu đã có).
- **Né**: Trước Bước 4 (giao việc), đọc file này; model nào ở trạng thái `DEAD` → KHÔNG giao task cho agent dùng model đó.
- **Gỡ**: Khi model hồi phục (test thành công hoặc user xác nhận) → sửa trạng thái thành `RECOVERED` (giữ 1 dòng lịch sử) hoặc xóa hẳn.
- **Ngưỡng**: 2 lần fail liên tiếp trong cùng phiên → `DEAD`. Dưới ngưỡng → ghi `WATCH` (theo dõi, vẫn dùng được).
- Không tự ghi nếu chỉ là lỗi 1 lần do mạng thoáng qua — ghi `WATCH` trước.

## Registry (mới nhất ở trên)

| model | trạng thái | fail_count | phát hiện lúc | triệu chứng | ghi chú |
|-------|-----------|------------|---------------|-------------|---------|
| opencode/deepseek-v4-flash-free | DEAD | probe | 2026-08-26 | HTTP 400 "Model is unavailable" từ upstream Zen | Đã thay bằng x-preview-f-free toàn team |
| moonshotai/kimi-k3-free (custom_3 + custom_4) | DEAD | probe | 2026-08-26 | 503 model_not_found, biến mất khỏi catalog Tokenrouter | Đã xóa cả 2 provider khỏi global config |
| tokenlb.net claude-opus-4-7 + gpt-5.5 (custom_2) | DEAD | probe | 2026-08-26 | 403 insufficient_user_quota ($0.00) | Provider custom_2 đã xóa |
| tokenlb.net gemini-3-flash-preview (custom_2) | DEAD | probe | 2026-08-26 | 503 no available channel | Provider custom_2 đã xóa |
| opencode/muse-spark-1.2-contributor-free | WATCH | probe | 2026-08-26 | Internal server error khi probe | Thử lại trước khi dùng |

## Lịch sử đã hồi phục
| model | trạng thái | hồi phục lúc | ghi chú |
|-------|-----------|--------------|---------|
| custom_5/mercury-2 | RETIRED | 2026-08-26 | Probe còn sống (HTTP 200) nhưng user quyết định loại bỏ Thread Steal, cross-checker chuyển sang hy3-free |
