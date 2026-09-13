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
| cBAjYv/gpt-5.6-sol | DEAD | 3 | 2026-09-12 | Filen lần đầu và cả Filen/OpenCode retry bị từ chối do provider precharge quota còn `$0.019756` | Đã reroute: lead → `Thuan_justworker/gpt-5.6-sol`, 6 agent vai nặng → `JustWoker/gpt-5.6-sol` theo yêu cầu user 2026-09-12 |
| cBAjYv/gpt-5.6-sol | RECOVERED | 2 | 2026-09-10 | Reviewer rồi Rust Dev Phase 6 lỗi protocol `message_start`; phiên sau phản hồi lại và user yêu cầu tiếp tục | Có thể tiếp tục dùng; theo dõi nếu tái phát |
| cBAjYv/gpt-5.6-sol | RECOVERED | 1 | 2026-09-09 | Rust Dev task 5.4 lần đầu trả `<none>`; retry hoàn tất 8/8 tests | Có thể tiếp tục dùng; theo dõi nếu tái phát |
| cBAjYv/gpt-5.6-terra | DEAD | 2 | 2026-09-08 | Docs rồi Explorer lỗi protocol `message_start` khi message trước còn mở; hai Explorer khác bị hủy cùng batch | Không giao agent Terra; chuyển mọi task sang vai dùng Sol cho tới khi probe hồi phục |
| opencode/x-preview-f-free | RETIRED | - | 2026-09-04 | Biến mất khỏi catalog live Zen (`/zen/v1/models` còn 66 models, không có x-preview) | Đã thay: lead/splitter→mimo-v2.5, rust-dev→laguna-s-2.1, teamwork/review/test→model theo vai |
| opencode/hy3-free | RETIRED | - | 2026-09-04 | Biến mất khỏi catalog live Zen | Cross-checker chuyển sang muse-spark-1.3-contributor-free (xa họ model nhất) |
| opencode/deepseek-v4-flash-free | DEAD | probe | 2026-08-26 | HTTP 400 "Model is unavailable" từ upstream Zen | Đã thay bằng x-preview-f-free toàn team |
| moonshotai/kimi-k3-free (custom_3 + custom_4) | DEAD | probe | 2026-08-26 | 503 model_not_found, biến mất khỏi catalog Tokenrouter | Đã xóa cả 2 provider khỏi global config |
| tokenlb.net claude-opus-4-7 + gpt-5.5 (custom_2) | DEAD | probe | 2026-08-26 | 403 insufficient_user_quota ($0.00) | Provider custom_2 đã xóa |
| tokenlb.net gemini-3-flash-preview (custom_2) | DEAD | probe | 2026-08-26 | 503 no available channel | Provider custom_2 đã xóa |
| opencode/muse-spark-1.2-contributor-free | WATCH | probe | 2026-08-26 | Internal server error khi probe | Thử lại trước khi dùng |

## Lịch sử đã hồi phục
| model | trạng thái | hồi phục lúc | ghi chú |
|-------|-----------|--------------|---------|
| opencode/deepseek-v4-flash-free | RECOVERED | 2026-09-04 | Xuất hiện lại trong catalog live Zen → gán tester (vòng lặp test nhanh, code-fluent) |
| custom_5/mercury-2 | RETIRED | 2026-08-26 | Probe còn sống (HTTP 200) nhưng user quyết định loại bỏ Thread Steal, cross-checker chuyển sang hy3-free |
