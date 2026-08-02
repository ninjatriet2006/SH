# Phase 10 — Redesign UI kiểu Nemo Files (bỏ phong cách terminal)

## Yêu cầu user
1. **Bỏ hẳn nút Web Drive** (và modal WebDrive).
2. **Thiết kế lại giao diện**: hiện tại phong cách terminal (nút text, TextEdit đường dẫn, layout trần) → làm theo chuẩn **Nemo Files** (file manager): toolbar icon + breadcrumb, sidebar Places, list view bảng đẹp, status bar, hover/selected highlight.

## Spec chi tiết (file duy nhất: `apps_gui/filen_gui/src/main.rs`)

### A. Bỏ Web Drive
- Xóa nút "🌐 Web Drive" trong `ui_top`; xóa `fn open_web_drive`; xóa `Modal::WebDrive` (enum + nhánh render trong `ui_modal`); bỏ import `web_drive_url` nếu không dùng nơi khác.

### B. Toolbar mỗi pane kiểu Nemo (thay `ui_pane_header` + phần "Đường dẫn: [TextEdit] [Đi]")
- Hàng 1: nút icon nhỏ: `←` Back, `→` Forward, `⬆` Up, `🏠` Home, `⟳` Reload, `🔄` Đổi chế độ (Cục bộ/Cloud) | **breadcrumb** đường dẫn: tách path theo `/`, mỗi segment là nút click → navigate tới path đó; nếu quá hẹp hiện "…" phía trước (tham khảo cách xử lý hẹp: chỉ vẽ các segment cuối).
- Thêm lịch sử per-pane: `back: Vec<String>`, `fwd: Vec<String>` (field mới trong PaneState); navigate/up/home/back/forward cập nhật stack đúng; back/forward button disabled khi stack rỗng.
- Hàng 2 (giữ nguyên chức năng Phase 7/9): 📁 Tạo thư mục, ✏️ Đổi tên, 🗑️ Xóa, ⭐ Yêu thích, 👁️ Xem, 🔗 Link + ô lọc (`ui_pane_filter`) — style lại: `egui::Button::new` bình thường, có hover text.

### C. Sidebar "Places" kiểu Nemo (thay `ui_sidebar`)
- Section **Địa điểm** (cục bộ): 🏠 Trang chủ (home), 🖥️ Máy tính (Desktop), 📁 Tài liệu (Documents), ⬇️ Tải xuống (Downloads), 🖼️ Hình ảnh (Pictures), 🎵 Nhạc (Music) — dùng crate `dirs` (đã có dep) lấy path XDG; click → pane active (hoặc pane trái nếu cả 2 local) list_pane tới path đó; row có hover highlight (Frame fill nhẹ khi hovered) + nền accent khi pane đang ở đúng path đó.
- Section **Filen Cloud**: ☁️ Cloud (root /), 🕘 Gần đây, 🔄 Đồng bộ, 🖥️ Servers (giữ view switch như cũ).
- Section **Tài khoản**: giữ nguyên logic Phase 9 (active + stored + phiên CLI + login/logout).
- Sidebar `.resizable(true)` mặc định rộng ~210px.

### D. List view Nemo (trong `ui_pane_items`)
- Cột header: Tên | Kích thước | Loại | Ngày sửa — nền header xám đậm hơn, chữ đậm, padding đều.
- Row: icon theo loại file (📁 dir, 📄 txt/md/rs/json, 🖼️ png/jpg/gif/webp, 🎵 mp3/wav, 🎬 mp4/mkv, 📦 zip/tar/gz, 📕 pdf), tên chữ màu theo trạng thái; **hover → fill nền nhạt**, **selected → fill accent (94,156,255) + chữ trắng**; row full-width (cả hàng là 1 Response có sense click + double click — dùng `ui.horizontal` bọc `Frame::default().fill(...)`, hoặc `egui::SelectableLabel` + `row_resp.clicked()`/`double_clicked()` như Phase 9).
- Giữ nguyên logic multi-select Phase 9 (Ctrl/Shift, "..", status "Đã chọn N mục"), context menu (Đi vào/Xem/Đổi tên/Xóa/Yêu thích/Copy link/Copy path).
- Status bar dưới mỗi pane: `N mục — đã chọn M (tổng SZ)`.

### E. Top bar + bottom bar tinh gọn
- `ui_top`: bỏ tiêu đề to; 1 hàng gọn: nút "→ Copy / ← Copy / → Move / ← Move" (giữ) + "📝 Xuất Notes" + bên phải version; background tối hơn một chút (Frame fill).
- `ui_bottom`: 1 dòng log cuối cùng (thay vì toàn bộ log) + "N transfer đang chạy" nếu có.

## Ràng buộc
- KHÔNG đổi logic backend: operations.rs, transfer.rs, async threads, modal login 2FA, Recents/Sync/Servers views (chỉ đổi visual nhẹ nếu cần đồng bộ).
- Giữ 98 tests pass, `cargo clippy -- -D warnings` sạch.
- Nếu cần helper mới (ví dụ `segment_paths`, `file_icon`) đặt private fn trong main.rs.

## Verify
- `cargo check -p filen_gui`; `cargo test -p filen_gui` (98 pass); `cargo clippy -p filen_gui -- -D warnings`.
- Build release + chạy 8s xác nhận GUI mở.

## Trạng thái
- [x] Đọc hiện trạng layout
- [x] Dev triển khai (rust-dev: bỏ Web Drive hoàn toàn, toolbar breadcrumb + back/fwd history, sidebar Places XDG, list view Nemo hover/selected, top/bottom gọn)
- [x] Verify: check + 98 tests pass + clippy sạch; build release + chạy 8s OK
- [ ] User test GUI
