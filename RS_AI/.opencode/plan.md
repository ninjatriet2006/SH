# plan.md — Cải tạo filen_gui theo phong cách Air Explorer

## Mục tiêu
Cải tạo `apps_gui/filen_gui` (egui/eframe) thành trình quản lý cloud kiểu **Air Explorer**:
dual-pane explorer (Local + Filen Cloud), cho phép tra cứu/di chuyển file qua mạng,
sidebar tài khoản, quản lý transfer, tìm kiếm, thông tin dung lượng cloud.

## RÀNG BUỘC (user yêu cầu)
1. **TUI và GUI ĐỘC LẬP hoàn toàn**: filen_gui không dep filen_tui; code xử lý copy/viết lại trong GUI. (Đã xong phase 1)
2. **Các GUI không được phụ thuộc lẫn nhau**: đã verify — không GUI nào trỏ path sang GUI khác; chỉ trỏ TUI. filen_gui đã sạch dep TUI.
3. **Audit backend bằng `help` + bổ sung ops thiếu + test đầy đủ**: filen CLI có sẵn tại ~/.filen-cli/bin/filen.

## AUDIT BACKEND (filen help — đã chạy 2026-07-31)
Nhóm lệnh: auth, fs, sync, mount, webdav, s3, updates.

### Có trong GUI rồi ✅
whoami, statfs, ls --long (list_remote), cat, mkdir, rm --no-trash, mv, cp,
upload, download, favorite, unfavorite, favorites, trash, trash restore/delete/empty,
links, links <path> (create), login_new, logout, export-auth-config, export-api-key.

### THIẾU — cần bổ sung (ưu tiên giảm dần)
| Lệnh backend | Mô tả | Ưu tiên |
|--------------|-------|---------|
| head <file> [-n] | đọc N dòng đầu | 1 |
| tail <file> [-n] | đọc N dòng cuối | 1 |
| stat <item> | thông tin chi tiết file/dir | 1 |
| write <file> <content> | ghi text vào file cloud | 1 |
| recents | danh sách file gần đây | 1 |
| view [path] | mở Web Drive | 2 |
| export-notes [path] | xuất Notes | 2 |
| sync <pairs> [--continuous] | sync local↔cloud (đọc syncPairs.json) | 2 |
| webdav / webdav-proxy | chạy WebDAV server (TUI đã có state mẫu) | 2 |
| s3 | chạy S3 server | 2 |
| mount | network drive (Linux cần FUSE3 — ghi chú) | 3 |

### Lưu ý kỹ thuật
- CLI `cli-progress` ghi progress ra **stderr**, format `{pct}% | {value} / {total}`, render `\r` → transfer runner đọc chunk + split `\r/\n`.
- `--json` flag hỗ trợ cho fs commands (có thể dùng cho stat/recents).

## Quyết định kiến trúc
1. Async model: `std::thread` + `mpsc`; mỗi thread tạo tokio runtime riêng `block_on`.
2. Account model: 1 active account toàn cục; transfer dùng account pane nguồn.
3. Transfer có progress: spawn CLI child trực tiếp + stdout/stderr pipe + `tokio::select!` parse progress + timeout + cancel = kill child.
4. Tìm kiếm: client-side filter trên listing (không có search primitive backend).
5. Data-dir dùng chung với TUI qua get_default_data_dir/resolve_filen_bin.

## Roadmap (cập nhật)
| phase | mô tả | phụ thuộc | ưu tiên | trạng thái |
|-------|-------|-----------|---------|------------|
| 1 | Gỡ dep filen_tui; copy ops sang GUI | - | 1 | [x] |
| 2 | Khung dual-pane explorer (2 pane, sidebar, toolbar, status) | 1 | 1 | [x] |
| 3 | Tích hợp list_local/list_remote async vào 2 pane | 1 | 1 | [x] |
| 4 | Sidebar tài khoản + login/logout + statfs | 2,3 | 2 | [x] |
| 5 | Transfer upload/download progress/timeout/cancel | 3,4 | 2 | [x] |
| 6 | Bổ sung ops thiếu từ audit + tests đầy đủ | 3,4 | 1 | [x] |
| 7 | Tìm kiếm filter + thao tác file (mkdir/rm/mv/cp/fav) | 5 | 2 | [x] |
| 8 | Clippy/test toàn diện + smoke-test + independence check | 5,6,7 | 3 | [x] |

## Kết quả cuối
- Toàn bộ 8 phase hoàn thành. Báo cáo chi tiết: `.opencode/FILEN_GUI_REPORT.md`.
- Verify: filen_gui check/clippy 0 warning, test 97/97; filen_tui 39/39 (không hỏng); cargo tree sạch dep TUI; 5 GUI không phụ thuộc lẫn nhau.
- **Smoke-test cloud THẬT** (account lamminhtriet01@gmail.com): 10/11 PASS ban đầu → phát hiện và fix 3 bug thật (rm --no-trash treo, write multi-line, progress khi pipe) → verify lại PASS + fix thêm trash_empty/delete/export pre-pipe. Links bị chặn do free plan ("You need an active subscription") — không phải bug.
- **Mount kiểm tra xong, HOẠT ĐỘNG** (2026-07-31): FUSE3 OK; CLI nâng cấp 0.0.0 → npm v0.0.39 (PATH ưu tiên npm; binary cũ backup ~/.filen-cli/bin/filen.bak-0.0.0); fix tải rclone 60MB vào ~/.config/@filen/network-drive/ (CDN fail giữa chừng là nguyên nhân mount treo); mount point phải TRONG HOME → GUI default đổi /tmp/filen → ~/.filen-drive (98 tests pass); test full: mount/duyệt/ghi/đọc/xóa/unmount đều OK.
- Cần user: login/logout thật chưa test (cần password, tránh phá session); GUI cài đặt mount point mặc định ~/.filen-drive tự tạo.
- **Dọn CLI cũ (2026-08-01)**: xác nhận bản pkg cũ là DEV BUILD (buildInfo không inject → version placeholder "0.0.0", log "Skipping updates in development environment"); đã xóa ~/.filen-cli/bin/filen (dev build) + filen.bak-0.0.0 (backup) — thư mục bin giờ trống; hệ thống dùng npm @filen/cli v0.0.39 (PATH ưu tiên), whoami/statfs OK.
