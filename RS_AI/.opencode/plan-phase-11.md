# Phase 11 — Cột kéo thả + Sidebar thu hẹp + File ops vào Context Menu

## Yêu cầu user
1. **Cột bảng kéo thả được** độ rộng (Tên | Kích thước | Loại | Ngày sửa).
2. **Sidebar trái thu nhỏ được hơn**: min width xuống ngang nút "Đăng nhập mới" (~110px).
3. **Bỏ hàng nút thao tác** (Tạo thư mục/Đổi tên/Xóa/Yêu thích/Xem/Link) khỏi pane → chuyển hết vào **Context Menu** (menu chuột phải) + thêm "Tạo thư mục mới"/"Tải lại" khi chuột phải vào vùng trống.

## Thay đổi (chỉ `apps_gui/filen_gui/src/main.rs`)

### 1. Cột kéo thả
- `PaneState` (~43) thêm `col_w: [f32; 3]` (size, kind, date), default `[90.0, 80.0, 120.0]`.
- Header cột trong `ui_pane_items` (~1323): thay hằng TYPE_W/DATE_W bằng `self.panes[idx].col_w`; bố cục: Tên (left, min 140, lấp phần còn lại) | Kích thước (right-aligned, rộng col_w[0]) | Loại (rộng col_w[1]) | Ngày sửa (rộng col_w[2]). Giữa các cột là **drag handle**: `ui.allocate_exact_size(vec2(6.0, 24.0), Sense::drag())` + vẽ vạch dọc mảnh bằng painter; `if resp.dragged() { col_w[i] = (col_w[i] + resp.drag_delta().x).clamp(50.0, 400.0); }`.
- Row item (~1389): thay toàn bộ vị trí painter text dùng col_w (giống header), cột Tên bị cắt bớt khi hẹp.
- 2 pane dùng col_w riêng (mỗi PaneState 1 bản).

### 2. Sidebar min width
- `ui_sidebar` (~778): thêm `.min_width(110.0)` cho SidePanel (giữ default_width 210, resizable true).

### 3. File ops → Context Menu
- **Xóa hàng 2** trong `ui_pane_header` (~1191–1257): bỏ hết nút Tạo thư mục/Đổi tên/Xóa/Yêu thích/Xem/Link; chuyển ô lọc "🔍 Lọc…" lên cuối hàng 1 (right-aligned sau breadcrumb).
- **Menu chuột phải trên row** (~1455): giữ nguyên (Đi vào/Xem/Đổi tên/Xóa/Yêu thích/Bỏ/Copy link/Copy path) + **khi `resp.secondary_clicked()` thì select item đó** (nếu chưa trong selection; giữ Ctrl để toggle) trước khi menu mở — đúng chuẩn Nemo.
- **Menu vùng trống**: sau vòng for items, allocate vùng còn lại `ui.allocate_exact_size(vec2(ui.available_width(), ui.available_height()), Sense::click())` + `.context_menu(...)` với "📁 Tạo thư mục mới" (Modal::Mkdir) và "⟳ Tải lại" (list_pane).

## Ràng buộc
- KHÔNG đổi operations.rs/transfer.rs/backend; giữ multi-select Ctrl/Shift Phase 9, account Phase 9, breadcrumb/back/fwd Phase 10.
- 98 tests pass, clippy `-D warnings` sạch.

## Verify
- `cargo check -p filen_gui`; `cargo test -p filen_gui`; `cargo clippy -p filen_gui -- -D warnings`; build release + chạy 8s.

## Trạng thái
- [x] Đọc hiện trạng code
- [x] Dev triển khai (rust-dev: col_w kéo thả + clamp, sidebar min 110, bỏ hàng nút → context menu row + vùng trống, secondary_clicked select, lọc lên hàng 1)
- [x] Verify: check + 98 tests pass + clippy sạch; build release + chạy 8s OK
- [ ] User test GUI