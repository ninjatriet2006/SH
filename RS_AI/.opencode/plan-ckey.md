# plan-ckey.md — Hỗ trợ tính năng CKey cho opencode_manager TUI

## Mục tiêu
Tích hợp CKey (ckey.vn) vào `TUI/opencode_manager`: quản lý tài khoản CKey (profile/balance),
danh sách model AI kèm giá VND, API keys AI, thống kê/lịch sử usage; import model vào
provider config của opencode.json.

## API CKey (đã đọc từ https://ckey.vn/docs — 20 endpoint, nhóm liên quan):
- Base URL API quản lý: `https://ckey.vn` — auth bằng `?key=<account_key>` (GET/POST form)
- LLM OpenAI-compatible: `https://api.xah.io/v1` — AI key dạng `ck-prod-xxx`
- Endpoint dùng: GET /api/profile | GET /api/llm/models (giá VND/million, supported_paths)
  | GET /api/llm/keys | GET /api/llm/usage-stats | GET /api/llm/usage (phân trang)
- KHÔNG làm (ngoài phạm vi opencode manager): GPU, proxy xoay/tĩnh, nạp tiền.

## Kiến trúc (dựa trên code hiện có)
- `api.rs`/module mới `ckey.rs`: CkeyClient với các hàm async fetch; structs deserialize
- `config.rs`: thêm `CkeyConfig` load/save `~/.config/opencode/ckey.json` (account_key)
- `app.rs`: Screen::CKeyDashboard + Screen::CKeyUsage; state ckey (profile, keys, models,
  usage stats, usage history, index); action: fetch account, import models → provider "ckey"
  (tái sử dụng cơ chế scanned_models + stale-removal vừa fix), xem usage history
- `ui.rs`: modal CKey Dashboard + modal Usage History (bảng)
- `main.rs`: key handling cho 2 màn hình mới + phím tắt mở CKey
- Preset fallback: thêm "CKey" (id "ckey", base_url https://api.xah.io/v1, npm openai-compatible)
  + nhận diện built-in trong save_all_config (không ghi vào opencode.json)

## VERIFY
`cargo check -p opencode_manager && cargo test -p opencode_manager && cargo clippy -p opencode_manager`
+ `./build_release.sh opencode_manager`

## Subtasks (plan-splitter + reviewer REQUEST_CHANGES đã tiếp thu)
| id | mô tả | phụ thuộc | ưu tiên | trạng thái |
|----|-------|-----------|---------|------------|
| 1 | CkeyClient async + deserialize structs 5 endpoints (parse thuần test được) | - | 1 | [x] |
| 2 | CkeyConfig load/save ckey.json (account_key) | - | 1 | [x] |
| 3 | Preset CKey fallback + nhận diện built-in (id "ckey" exact) | - | 1 | [x] |
| 4 | Screen enums + state ckey (profile/keys/models/stats/history/import) | 1,2 | 1 | [x] |
| 5 | Actions: nhập/clear account_key, fetch account, import model provider ckey, usage | 3,4 | 2 | [x] |
| 6 | UI modal CKey Dashboard (profile/balance/keys/stats + input key) | 5 | 2 | [x] |
| 7 | UI modal Usage History bảng phân trang | 5 | 2 | [x] |
| 8 | UI modal Import model CKey có cột giá VND + stale-removal | 5 | 2 | [x] |
| 9 | main.rs key handling 3 màn hình + phím tắt mở CKey | 6,7,8 | 3 | [x] |
| 10 | Tests unit: parse fixtures, config round-trip, import logic, built-in "ckey" không ghi vào opencode.json | 1,2,3,5 | 3 | [x] |
| 11 | cargo check/test/clippy + build_release verify | 10 | 3 | [x] |

## Ghi chú reviewer đã tiếp thu
1. Có action nhập/clear account_key ngay trong Dashboard; log lỗi rõ khi key rỗng/sai (401/403).
2. CkeyClient: struct lỗi riêng, map 401/402 → thông báo thân thiện (giống ApiStatus).
3. Nhận diện built-in CKey bằng exact id "ckey" trong save_all_config + test bảo vệ.
4. Màn hình import model có cột giá VND/million.
5. Test parse từ fixture JSON string (hàm parse thuần), không test mạng thật.

## Hoàn tất 02/08/2026 — verify pass: cargo check/test (13 test) OK, clippy chỉ còn warning pre-existing ApiClient (api.rs), build_release.sh OK.
