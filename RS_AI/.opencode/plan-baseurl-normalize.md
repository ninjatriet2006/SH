# plan-baseurl-normalize.md — Sửa lỗi base_url nhập thừa path

## Mục tiêu
Người dùng nhập `https://api.inceptionlabs.ai/v1/chat/completions` vào form provider — manager
nhận và ghi NGUYÊN XI vào opencode.json → opencode không hoạt động (base URL chuẩn chỉ đến `/v1`).
Cần kiểm tra vì sao nhận (không validate) + thêm tính năng tự sửa (normalize) base_url + test bảo vệ.

## Kết quả khảo sát (explorer + reviewer)
- KHÔNG có validate URL ở đâu; chỉ trim + check non-empty; mọi path add/edit qua `save_form()` (app.rs:1038).
- get_models_url (api.rs:62) append `/models` vào URL gốc → provider cũ lưu URL hỏng tiếp tục fail.

## Quyết định thiết kế (tiếp thu reviewer REQUEST_CHANGES)
1. Hàm `normalize_base_url` — CHỈ cắt khi có segment `v<digit>` (regex `^v\d+$`, KHÔNG khớp `v1beta`)
   và toàn bộ path sau nó thuộc WHITELIST endpoint (chat/completions, completions, models, embeddings,
   responses, messages, generate, predictions, audio/transcriptions, images/generations, moderations,
   fine-tunes, runs, assistants, threads, search, edits...). Không substring bừa → Groq `/openai/v1`,
   OpenRouter `/api/v1`, Google `/v1beta` GIỮ NGUYÊN. Không scheme (`://`) → giữ nguyên.
2. Behavior: có v<digit> + suffix whitelist → AUTO-FIX (không cần confirm) + log warn "Đã tự sửa base URL";
   khác → giữ nguyên, không cảnh báo.
3. Gắn normalize CÙNG MỘT HÀM cho cả 2 VẾ ở mọi so sánh: save_form (ngay đầu, trước detect_duplicate),
   detect_duplicate, built-in check (save_form + save_all_config), open_edit_provider, get_models_url
   (backward compat: provider cũ URL hỏng vẫn gọi API được).

## VERIFY
`cargo check -p opencode_manager && cargo test -p opencode_manager && cargo clippy -p opencode_manager`
+ `./build_release.sh opencode_manager`

## Subtasks (reviewer REQUEST_CHANGES đã tiếp thu)
| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 1 | normalize_base_url + whitelist + unit tests (10+ case) | - | 1 | rust-dev | [x] |
| 2 | Gắn normalize 2 vế: save_form/detect_duplicate/built-in/save_all_config/open_edit_provider | 1 | 1 | rust-dev | [x] |
| 3 | normalize trong get_models_url (backward compat) | 1 | 1 | rust-dev | [x] |
| 4 | Test regression: URL hỏng cũ → duplicate vẫn bắt, built-in vẫn auth.json, models url đúng | 2,3 | 2 | rust-dev | [x] |
| 5 | cargo check/test/clippy + build_release.sh verify | 4 | 3 | tester | [x] |

## Hoàn tất 02/08/2026 — verify pass: cargo check/test (15 test) OK, clippy chỉ còn warning pre-existing ApiClient, build_release.sh OK.
