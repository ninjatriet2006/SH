# Phase 12 — Drag & drop giữa 2 pane + Clipboard Ctrl+C/X/V

## Mục tiêu (theo yêu cầu user — kiểu Nemo/Air Explorer)
1. **Kéo thả** mục từ pane này sang pane kia để copy (giữ Shift khi thả = move — tùy chọn; bản đầu drop = copy, nếu giữ Shift lúc thả = move).
2. **Phím tắt chuẩn**: Ctrl+C / Ctrl+X (trong pane active) → sang pane khác (hoặc folder khác cùng pane) Ctrl+V để copy/move; **hỗ trợ copy cùng phiên**: cloud→cloud khác folder (không tải về máy), local→local khác folder.

## Thay đổi (chỉ `apps_gui/filen_gui/src/main.rs`)

### A. State mới trong FilenGuiApp
```rust
clipboard: Option<ClipboardContent>,
drag: Option<DragSource>,
drop_target: Option<usize>,      // pane được hover khi drag (visual)
pane_rects: [egui::Rect; 2],     // cập nhật mỗi frame
```
```rust
struct ClipboardContent { src_pane: usize, src_mode: PaneMode, src_path: String, names: Vec<String>, cut: bool }
struct DragSource { src_pane: usize, names: Vec<String> }
```

### B. Refactor start_transfer
Tách logic `start_transfer` (main.rs ~591–672) thành `start_transfer_between(&mut self, src: usize, dst: usize, names: Vec<String>, kind: TransferKind, ctx)` — giữ nguyên phân loại Local→Local/→Cloud/Cloud→/Cloud→Cloud + enqueue. `start_transfer(kind, from_active, ctx)` (nút Copy/Move toolbar) gọi nó với selection hiện tại.

### C. Phím tắt (trong `update`, sau drain async, trước panels)
```rust
if !ctx.wants_keyboard_input() {
    let (cc, cx, cv) = ctx.input(|i| (i.key_pressed(Key::C), i.key_pressed(Key::X), i.key_pressed(Key::V)));
    // + check i.modifiers.ctrl
    if ctrl && cc { self.copy_selection(false); }
    if ctrl && cx { self.copy_selection(true); }
    if ctrl && cv { self.paste_clipboard(ctx); }
}
```
- `copy_selection(cut)`: names = selected (lọc ".."); rỗng → log "⚠️ Chưa chọn mục nào"; lưu clipboard {active_pane, mode, path, names, cut}; log "📋 Đã sao chép N mục — Ctrl+V để dán" / "✂️ Đã cắt N mục".
- `paste_clipboard(ctx)`: clipboard None → log. dst = active_pane. Nếu src_pane==dst && src_path == panes[dst].path → log "📍 Đích trùng nguồn — vào thư mục khác rồi dán". Ngược lại `paste_names(src_pane, dst, names, cut, ctx)`.

### D. Drag & drop
- Row sense: `Sense::click()` → `Sense::click_and_drag()` (giữ nguyên click/double-click/context).
- `if resp.drag_started() && !is_parent`: drag = Some(DragSource{ src_pane: idx, names: nếu name chưa trong selected → vec![name], nếu có → selected.clone() }) + log "🖱️ Đang kéo N mục — thả sang khung kia để sao chép (giữ Shift = di chuyển)".
- `ui_panes` (~1009): sau mỗi SidePanel::show lưu `self.pane_rects[i] = inner.response.rect`.
- Trong `update` cuối (sau panels, trước modal): nếu drag active → `drop_target = pointer.interact_pos() → pane ≠ src && pane_rects[i].contains(pos)`; nếu `pointer.any_released()`: có target → `paste_names(drag.src_pane, target, names, shift_held, ctx)` rồi clear drag/drop_target.
- Visual: trong `ui_pane`: nếu `drop_target == Some(idx)` vẽ `painter.rect_stroke` viền accent 2px quanh pane + hint "Thả để sao chép / di chuyển".

### E. paste_names (dùng chung C + D)
```rust
fn paste_names(&mut self, src: usize, dst: usize, names: Vec<String>, cut: bool, ctx) {
    // src==dst && path trùng → log, return
    let kind = if cut { TransferKind::Move } else { TransferKind::Copy };
    self.start_transfer_between(src, dst, names, kind, ctx);
    // nếu cut → clipboard = None (đã tiêu thụ); cập nhật panes[src].selected.clear() nếu src pane vẫn còn selection
}
```

## Ràng buộc
- KHÔNG đổi operations.rs/transfer.rs (trừ khi bắt buộc); tất cả logic mới trong main.rs.
- Giữ 98 tests pass, `cargo clippy -- -D warnings` sạch.

## Verify
- `cargo check -p filen_gui`; `cargo test -p filen_gui`; `cargo clippy -p filen_gui -- -D warnings`; build release + chạy 8s.

## Trạng thái
- [x] Điều tra + fix `resolve_filen_bin` (2 lỗi: (a) GUI desktop không có PATH nvm → thêm quét nvm/volta/bun/local; (b) **bug scan_node_bins nhặt nhầm `agy` trong ~/.local/bin** → "Usage of agy"; fix lọc đúng tên `filen` + test `test_scan_node_bins_ignores_foreign_binaries`)
- [x] Dev triển khai drag&drop + clipboard (clipboard/drag/drop_target/pane_rects, start_transfer_between, Ctrl+C/X/V, drag thả copy/Shift=move, viền highlight)
- [x] Xóa nút Copy/Move/Xuất Notes + panel top (không còn tác dụng — thay bằng drag&drop + phím tắt); xóa toàn bộ mạch ExportNotes (variant/action/async/field)
- [x] Verify: check + 99 tests pass + clippy sạch; build release + chạy 8s OK
- [ ] User test GUI