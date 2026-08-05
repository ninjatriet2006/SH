# plan-account-split.md — Tách Xem TK & Quản lý TK CKey (sửa hiểu sai)

## Mục tiêu (theo làm rõ của user)
Tách 2 chức năng thành 2 màn hình riêng biệt:
1. **Xem/Kiểm tra tài khoản [G]** — chỉ hiển thị khi PROVIDER ĐANG CHỌN là CKey (base_url api.xah.io/v1);
   xem thông tin account (profile/balance/usage), không có input endpoint/key.
2. **Quản lý tài khoản CKey [K]** — phím riêng, hiển thị khi có provider CKey; endpoint CKey
   KHÔNG ĐỔI (https://ckey.vn) → KHÔNG hiển thị ô endpoint; chỉ: list account + ô nhập key mới
   (Enter thêm, thêm NHIỀU key lần lượt) + xoá.

## Thay đổi cụ thể
- ckey.rs: khôi phục hằng `CKEY_MANAGE_API_BASE = "https://ckey.vn"` (endpoint cố định, fallback
  khi config endpoint rỗng — vẫn giữ field endpoint trong config cho tương lai đa API).
- app.rs: `has_ckey_support()` đổi ngữ nghĩa → check PROVIDER ĐANG CHỌN (providers_keys[selected_provider_idx]).
  Bỏ `ckey_endpoint_input` + `CkeyFocus::Endpoint` + `save_ckey_endpoint` (không còn UI nhập endpoint);
  `add_ckey_account` dùng endpoint = config.endpoint hoặc CKEY_MANAGE_API_BASE (tự lưu khi khác).
  Screen: đổi CKeyDashboard thành màn hình XEM; thêm Screen::CKeyManage + `open_ckey_manage`.
- ui.rs: footer `[G] Kiểm tra TK` khi provider đang chọn là CKey + `[K] Quản lý TK` khi has_ckey_support;
  draw màn hình XEM (list account chọn + info + R tải lại + I import + U usage, KHÔNG input);
  thêm draw màn hình QUẢN LÝ (list + input key + X xoá, KHÔNG endpoint).
- main.rs: phím G (đk provider đang chọn), phím K (đk has_ckey_support), key handling 2 màn hình.
- Tests: has_ckey_support theo provider đang chọn (CKey true / khác false); add_ckey_account với
  endpoint rỗng dùng CKEY_MANAGE_API_BASE; sửa test cũ dùng ckey_endpoint_input.

## VERIFY
`cargo check -p opencode_manager && cargo test -p opencode_manager && cargo clippy -p opencode_manager --all-targets`
+ `./build_release.sh opencode_manager`

## Hoàn tất 02/08/2026 — 20/20 tests, clippy sạch, cross-check (Mercury-2) CONFIRMED, build_release OK.
