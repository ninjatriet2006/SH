# Báo cáo Phase 8 — filen_gui (Docs, 2026-07-31)

## 1. Tổng quan kiến trúc
- `apps_gui/filen_gui` — GUI egui/eframe 0.31, **độc lập hoàn toàn khỏi filen_tui** (Cargo.toml không dep TUI; mọi ops được copy/viết lại trong GUI).
- 3 module: `main.rs` (UI dual-pane Air Explorer + sidebar + transfer panel + servers), `operations.rs` (ops CLI + helpers + server state), `transfer.rs` (transfer runner).
- Mô hình async: `std::thread` + `mpsc`; mỗi thread tự tạo tokio runtime `block_on`. Transfer spawn CLI child, pipe stdout/stderr parse progress, timeout + cancel (kill child). Data-dir dùng chung với TUI (`get_default_data_dir`/`resolve_filen_bin`).
- UI: 2 pane (Local/Cloud), toolbar copy/move, sidebar tài khoản + statfs, tìm kiếm client-side, modal thao tác file, context menu chuột phải, log panel.

## 2. Đối chiếu tính năng với audit backend (plan.md)
**Đã có ops + UI:** ls (list_local/list_remote), cat (Xem), mkdir, rm (Xóa + tùy chọn --no-trash), mv (Đổi tên / di chuyển), cp, upload/download (transfer panel progress/cancel/timeout), favorite/unfavorite, create_link (Copy link + clipboard), recents (panel Gần đây), view (Web Drive + mở browser), export-notes (Xuất Notes), sync (panel Đồng bộ, đọc syncPairs.json), login_new (2FA 2 bước) / logout / whoami / statfs, webdav / s3 / mount (panel Servers + start/stop + logs).

**Ops có trong `operations.rs` nhưng CHƯA có UI** (phase 6 chỉ yêu cầu ops + tests, chưa wire UI):
- `head`, `tail`, `stat`, `write_file` — có unit test, chưa có nút/modal UI.
- `trash` (list/restore/delete/empty), `favorites` (list), `links` (list), `export-auth-config`, `export-api-key` — có ops nhưng chưa có panel/UI riêng.
- `webdav-proxy`: có `webdav_proxy_args` + `WebDavServerState::start_proxy` nhưng panel Servers chưa có nút chạy chế độ proxy.

**Thiếu so với roadmap phase 7:** không phát hiện mục nào thiếu — toàn bộ 7.1–7.15 đã có trong code.

## 3. Kết quả verify
| Kiểm tra | Kết quả |
|----------|---------|
| `cargo check -p filen_gui` | ✅ pass |
| `cargo clippy -p filen_gui --all-targets -- -D warnings` | ✅ pass |
| `cargo test -p filen_gui` | ✅ 74 passed, 0 failed |
| `cargo check/clippy -p filen_tui` | ✅ pass (không hỏng) |
| `cargo tree -p filen_gui` | ✅ không dep TUI |
| `cargo tree` 5 GUI | ✅ GUI không dep lẫn nhau |

## 4. Smoke-test
- Đã chạy: verify build/clippy/test (mục 3). Unit test phủ ops mới phase 6 (args, parse stat/ls-long/sync-pairs, server state).
- Chưa chạy: smoke-test thủ công qua GUI (local list/mkdir/rm; cloud upload/download roundtrip; cancel/timeout) — cần user chạy trên máy.

## 5. Phần cần user xác nhận
1. **Đăng nhập lại session để test cloud:** chạy `filen logout` (gỡ session cũ) rồi đăng nhập lại qua GUI (form hỗ trợ 2FA 2 bước) → test list cloud, upload/download, copy/move, recents, sync.
2. **Mount cần FUSE3** trên Linux (ghi chú hiển thị sẵn trong panel Servers; cần cài `fuse3` nếu chưa có).
3. Có cần thêm UI cho head/tail/stat/write + panel Thùng rác/Yêu thích/Links không (ops đã sẵn sàng, chưa wire UI).

## 6. Ghi chú independence
- `filen_gui` đã sạch dep TUI.
- 4 GUI khác (`img_splt_gui`, `opencode_manager_gui`, `universal_converter_gui`, `universe_manager_gui`) **vẫn dep vào TUI tương ứng** (chưa tách) — ngoài phạm vi phase này, cần phase riêng nếu user yêu cầu.
