# Phase 9 — Tương tác kiểu Nemo + Account + Web Drive

## Mục tiêu (theo phản hồi user)
1. **File browser kiểu Nemo**: 1 click = CHỌN, 2 click = MỞ (folder → navigate, file → xem/mở); multi-select (Ctrl+click toggle, Shift+click range); toolbar thao tác trên nhiều mục; nút Up; ".." click = đi lên (không select).
2. **Account**: tự nhận account đang active (whoami) vào danh sách đã lưu nếu chưa có.
3. **Web Drive**: mở trình duyệt thật bằng `open_browser()` (đã có, dòng ~3041).

## Thay đổi chính (chỉ trong `apps_gui/filen_gui/src/main.rs` + 1 helper trong operations.rs)
- `PaneState.selected: Option<String>` → `selected: Vec<String>` (giữ `selected_name` cũ cho rename/delete modal ở mục đầu).
- Click logic trong `ui_pane_items`: click → select; double-click → Navigate (folder) / View (file); ".." click → Navigate.
- Header: thêm nút "⬆" Up (đi lên cha: cắt path).
- Status: "Đã chọn N mục".
- Toolbar Copy/Move/Xóa/Copy link: loop qua từng item đã chọn (ops hiện có nhận path+name).
- `WhoAmIFinished Ok(Some(email))`: nếu chưa có trong `account.stored` → thêm `StoredAccount{email, password:""}` + `save_stored_accounts`; sidebar: password rỗng → nhãn "(phiên CLI)" + disable Đăng nhập nhanh.
- `open_web_drive()`: gọi `open_browser(&url)`; modal giữ nút "Mở trình duyệt".
- Thêm test: helper ghép account (merge) trong operations.rs nếu tách hàm.

## Verify
- `cargo check -p filen_gui` + `cargo test -p filen_gui` (98 cũ + test mới) + `cargo clippy -p filen_gui -- -D warnings`
- Build release + chạy 8s xác nhận GUI mở OK.

## Trạng thái
- [x] Đọc hiện trạng code
- [x] Dev triển khai (rust-dev: multi-select Vec, anchor Shift, double-click Activate, nút Up, toolbar loop selection, Modal::Delete nhiều mục, account merge "(phiên CLI)", web drive mở browser)
- [x] Verify: check + 98 tests pass + clippy sạch; build release + chạy 8s OK
- [ ] User test GUI
