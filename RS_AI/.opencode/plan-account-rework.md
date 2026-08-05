# plan-account-rework.md — Thiết kế lại: G xem TK + K thêm nhanh nhiều provider

## Quyết định đã chốt với user
1. 1 provider CKey = 1 account. KHÔNG có "quản lý tài khoản" (xoá CKeyManage, accounts list, input endpoint, tên account random cũ).
2. `[G]` Kiểm tra TK — chỉ hiển thị khi provider ĐANG CHỌN là CKey (base_url api.xah.io/v1):
   - Khi chưa có account key cho provider → POPUP nhập/chọn account key ngay tại đó (không qua sửa provider).
   - Popup: danh sách account key ĐÃ LƯU từ các provider khác (1 account CKey có thể có nhiều AI key → chọn account sẵn có, hiển thị masked + provider đang dùng) HOẶC nhập account key mới.
   - Tương tác đầy đủ GET docs CKey: /api/profile, /api/llm/models, /api/llm/keys, /api/llm/usage-stats (lọc time/ai_key), /api/llm/usage (phân trang, lọc model/ai_key) + import model + tải lại.
   - Management base CỐ ĐỊNH https://ckey.vn (CKEY_MANAGE_API_BASE) — không lưu endpoint.
3. `[K]` Thêm nhanh nhiều provider — GENERIC, không phụ thuộc CKey:
   - 1 endpoint (URL AI key bất kỳ) + dán NHIỀU AI key (nhiều dòng) → Enter → tạo N provider cùng endpoint.
   - Tên/id provider = NGẪU NHIÊN KÝ TỰ (vd X7K2P9, không liên quan provider), unique với provider có sẵn.
   - Check trùng cặp (endpoint normalize + key) với provider có sẵn → bỏ key trùng + log.
   - Nhận CẢ k lẫn K (xử lý trước list handler — bug trước chỉ nhận hoa).
   - Cơ chế Enter-editing giống form A.
4. Form `[A]`: thêm ô "Account key (trang Profile)" — chỉ hiển thị khi endpoint là CKey; Enter-editing; lưu ckey.json theo provider id; pre-fill khi sửa.
5. ckey.json đổi: {endpoint, accounts[]} → {provider_id → account_key} map; migration cũ (account đầu → "ckey" nếu có).

## Cấu trúc hiện tại cần sửa (đọc kỹ trước)
- ckey.rs: CKEY_MANAGE_API_BASE, CKEY_LLM_BASE_URL, CkeyConfig (config.rs), generate_account_name/ensure_unique_name (bỏ/thay bằng tên provider random).
- app.rs: CkeyConfig {endpoint, accounts: Vec<CkeyAccount{name,key}>}; Screen::CKeyManage; CkeyFocus (Endpoint/Key/List); ckey_key_input/ckey_endpoint_input; add_ckey_account/remove_ckey_account/save_ckey_endpoint/open_ckey_manage; ckey_fetch_all theo accounts; has_ckey_support (provider đang chọn — GIỮ).
- ui.rs: footer G (đk) + K (luôn); draw_ckey_dashboard_modal (input endpoint/key + list accounts); draw_ckey_manage_modal.
- main.rs: phím G (đk), phím K (luôn, k|K), key handling CKeyDashboard/CKeyManage.
- form A: ProviderForm focus 0-6 (0 Preset,1 Name,2 URL,3 Key,4 Test,5 Save,6 Cancel) — chèn Account key.

## VERIFY
`cargo check -p opencode_manager && cargo test -p opencode_manager && cargo clippy -p opencode_manager --all-targets`
+ `./build_release.sh opencode_manager`

## ✅ HOÀN THÀNH (02/08/2026)
- Dev đã implement đầy đủ 5 mục thiết kế: CkeyConfig map + migration, Screen::BulkAddProviders + execute_bulk_add, popup need-key (pick/save), form A account key, k|K mở bulk add.
- Verify: cargo check 0 lỗi, 22/22 tests pass, clippy sạch (chỉ warning pre-existing ApiClient::Default), release build 2.7M OK.
- Cross-check Mercury-2 FOUND_ISSUES 5 điểm → đã xử lý:
  - config.rs: load() không còn im lặng nuốt lỗi parse accounts (trả Err + App::new log cảnh báo).
  - app.rs execute_bulk_add: thêm validate URL (bắt buộc scheme://host).
  - 2 điểm nhầm/bỏ qua: normalize_base_url KHÔNG cắt URL lạ (code line 144-146 giữ nguyên khi suffix ngoài whitelist); log ckey_save_new_account_key chỉ in provider_id (không lộ key). ApiClient::Default pre-existing — giữ.
- Cập nhật lần cuối: 22/22 tests, clippy 0 warning mới, release rebuilt 02/08 22:21.

## 🐛 HOTFIX 02/08 23:36 — /api/llm/usage trả 400 "Vui lòng nhập API key!"
- Test thực tế (curl) với account key thật của user: profile/models/keys/usage-stats đều 200, NHƯNG `/api/llm/usage?key=...&page=&limit=` → 400 vì API **bắt buộc có param `api_key`** (chỉ cần hiện diện, rỗng vẫn 200; `ai_key=` sai → 403).
- Fix: `CkeyClient::fetch_usage` thêm param `api_key` (giá trị hint = key_prefix của AI key đầu tiên từ /api/llm/keys); `ckey_fetch_usage_page` truyền hint.
- Verify: 22/22 tests, clippy sạch, release rebuilt.

## 🐛 HOTFIX 2 03/08 00:15 — /api/profile lỗi "missing field `username`"
- Test thực tế: response `/api/profile` = `data.profile = {...}` (nested), nhưng `fetch_profile` parse `data` trực tiếp thành `CkeyProfile` → thiếu field `username`.
- Fix: `fetch_profile` parse qua wrapper `ProfileWrap { profile: CkeyProfile }` (giống fixture test đã có từ trước).
- Verify: 22/22 tests, clippy 0 warning mới, release rebuilt.
