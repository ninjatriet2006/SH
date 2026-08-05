# plan-account-check.md — Kiểm tra thông tin tài khoản (thay CKey Dashboard)

## Mục tiêu
Biến tính năng CKey Dashboard thành "Kiểm tra thông tin tài khoản" — generic cho các API lớn,
hiện chỉ hỗ trợ CKey. Thay đổi: (1) đổi nhãn + hiển thị thông tin tài khoản (balance, username,
created, usage) — KHÔNG hiện URL/API key; (2) ẩn tính năng khi provider không hỗ trợ (chỉ hiện
khi có provider base_url api.xah.io/v1); (3) nhập NHIỀU tài khoản: 1 endpoint (mặc định TRỐNG,
không preset) + nhiều account key — mỗi key 1 tài khoản, TÊN TỰ ĐỘNG NGẪU NHIÊN (không có phần
nhập tên); account key nhập 1 lần + lưu (đã xác nhận: account key ≠ AI key, không lấy từ config).

## Xác nhận đã chốt với user
- 2 key KHÁC NHAU (docs CKey: management = API key trang Profile ?key=..., LLM = AI key ck-prod Bearer)
  → account key nhập 1 lần, lưu ckey.json, không nhập lại mỗi lần mở.
- Đổi tên: "CKey Dashboard" → "Kiểm tra thông tin tài khoản".
- Ẩn: không có provider hỗ trợ (base_url chứa api.xah.io/v1) → không hiện phím G/mục.
- Endpoint: mặc định TRỐNG (user nhập, vd https://ckey.vn); rỗng → chặn fetch + log nhắc nhập.
- Nhiều account: mỗi account = 1 account key, tên ngẫu nhiên (không nhập tên).

## Tiếp thu reviewer REQUEST_CHANGES
1. Subtask 1.1 bao gồm sửa MỌI call-site + tests cũ dùng CkeyConfig{account_key} (app.rs, config.rs, tests).
2. Endpoint rỗng → KHÔNG fetch, log "Vui lòng nhập endpoint"; CKEY_LLM_BASE_URL (api.xah.io/v1)
   giữ NGUYÊN riêng cho preset/import — không gộp với management endpoint.
3. Điều kiện hỗ trợ (ẩn/hiện): duyệt config.provider + auth_config (built-in sync), so sánh
   base_url bằng normalize_base_url 2 vế (tránh trượt trailing slash).
4. Ẩn phím G hoàn toàn khi không hỗ trợ → CKeyImport/CKeyUsage cũng ẩn theo (chấp nhận: không
   dùng CKey thì import cũng vô nghĩa).
5. Tên ngẫu nhiên: collision policy — nếu trùng tên đã có, append hậu tố -2/-3... (đảm bảo unique).

## Hiện trạng cần sửa (từ code CKey đã merge)
- config.rs: CkeyConfig { account_key: String } → { endpoint: String (trống), accounts: Vec<CkeyAccount{name,key}> } + migration đọc file cũ (account_key → 1 account).
- ckey.rs: CkeyClient hardcode CKEY_API_BASE ("https://ckey.vn") → nhận endpoint từ config; parse_wrapped giữ nguyên.
- app.rs: state ckey (ckey_key_input, ckey_editing_key...) → danh sách accounts + input endpoint/key mới; action nhập key cũ → add/remove/select account; fetch theo từng account.
- ui.rs: modal CKey Dashboard → màn hình "Kiểm tra thông tin tài khoản" (list tài khoản + info từng account), KHÔNG hiển thị URL/API key (masked); footer ẩn [G] nếu không có provider hỗ trợ.
- main.rs: key handling cho màn hình mới.

## VERIFY
`cargo check -p opencode_manager && cargo test -p opencode_manager && cargo clippy -p opencode_manager`
+ `./build_release.sh opencode_manager`

## Hoàn tất 02/08/2026 — verify pass: check OK, 18 tests OK, clippy sạch, build_release OK. Cross-check (Mercury-2): 1 lỗi nhỏ tiêu đề "(API)" — đã sửa.
