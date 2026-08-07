# plan.md — filen_gui: Build mới + phân tách code + Nemo-like features

## Mục tiêu
1. **Đổi cách build**: không AppImage/deb nữa → build vào `release/filen_gui/` gồm file chạy riêng + data + các file liên quan.
2. **Phân tách code theo tính năng**: tách rõ UI / backend / từng feature (file map trực quan).
3. **Fix cột Name|Size|Modified**: thêm nhiều cột phân loại (như Nemo), kéo giãn tùy ý, nhiều cách sắp xếp.
4. **Nemo-like interactions**: copy, kéo thả, chọn (rubber-band), context menu đầy đủ.
5. **Fix Panel 2**: đang rỗng/không hiển thị.

## Ràng buộc
- Tauri v2, frontend Vite+TS thuần (không framework), Rust core trong `src/`.
- Hiện tại: `DualPaneExplorer.ts` (210 dòng) chứa mọi thứ — cần tách module.
- Build hiện tại: `cargo tauri build --bundles appimage/deb` → đổi sang copy binary + frontend dist + data vào `release/filen_gui/`.
- Verify: `npm run build` (frontend) + `cargo check` + chạy thử binary.
- Data runtime: `frontend/dist` (chứa assets+themes). Không có data dir runtime cần copy (data ở `~/.filen-cli` home user).
- File ops nằm ở Rust command (`src-tauri/src/lib.rs`: fs_cp, fs_mv, fs_rm, fs_mkdir...).
- Không có CI/CD workflow trong repo → không cần sửa CI.

## Plan chi tiết (đã duyệt bởi Plan Reviewer + Cross Checker)

### Phase 1 — Khảo sát & checkpoint
| id | mô tả | phụ thuộc | ưu tiên | agent |
|----|-------|-----------|---------|-------|
| 1.1 | Khảo sát cấu trúc src/ + DualPaneExplorer.ts | - | 1 | explorer |
| 1.2 | Checkpoint plan chi tiết ra file | 1.1 | 1 | docs |

### Phase 2 — Refactor theo feature
| id | mô tả | phụ thuộc | ưu tiên | agent |
|----|-------|-----------|---------|-------|
| 2.1 | Tách UI components (pane, toolbar, statusbar) | 1.2 | 1 | rust-dev |
| 2.2 | Tách backend/state; file ops trong Rust command | 1.2 | 1 | rust-dev |
| 2.3 | Tách feature modules (map, sort, context) | 2.1,2.2 | 1 | rust-dev |
| 2.4 | Smoke build (npm run build + cargo check) | 2.3 | 1 | rust-dev |

### Phase 3 — Build mới
| id | mô tả | phụ thuộc | ưu tiên | agent |
|----|-------|-----------|---------|-------|
| 3.1 | Script build copy binary+frontend/dist vào release | 2.4 | 1 | rust-dev |
| 3.2 | Cập nhật tauri.conf/scripts bỏ appimage/deb | 3.1 | 1 | rust-dev |

### Phase 4 — Bảng cột Nemo-like
| id | mô tả | phụ thuộc | ưu tiên | agent |
|----|-------|-----------|---------|-------|
| 4.1 | Thêm cột phân loại (type, size, date) | 2.4 | 2 | rust-dev |
| 4.2 | Kéo giãn cột tùy ý (resizable) | 4.1 | 2 | rust-dev |
| 4.3 | Nhiều cách sắp xếp (name/size/date/type) | 4.1 | 2 | rust-dev |

### Phase 5 — Interactions Nemo-like
| id | mô tả | phụ thuộc | ưu tiên | agent |
|----|-------|-----------|---------|-------|
| 5.5 | Backend Rust command copy/paste (fs_cp...) | 2.4 | 2 | rust-dev |
| 5.1 | Copy file/folder (clipboard + paste) | 4.3,5.5 | 2 | rust-dev |
| 5.3 | Rubber-band chọn nhiều item | 5.1 | 2 | rust-dev |
| 5.4 | Context menu đầy đủ (open/copy/rename/delete) | 5.1 | 2 | rust-dev |
| 5.2 | Kéo thả file giữa các pane | 5.1,6.3 | 2 | rust-dev |

### Phase 6 — Fix Panel 2
| id | mô tả | phụ thuộc | ưu tiên | agent |
|----|-------|-----------|---------|-------|
| 6.1 | Root-cause Panel 2 rỗng (do refactor 2.x) | 2.4 | 1 | rust-dev |
| 6.2 | Fix render Panel 2 | 6.1 | 1 | rust-dev |
| 6.3 | Đồng bộ state 2 pane (selection/path) | 6.2 | 1 | rust-dev |

### Phase 7 — Verify & docs
| id | mô tả | phụ thuộc | ưu tiên | agent |
|----|-------|-----------|---------|-------|
| 7.1 | npm run build + cargo check + chạy thử | 3.2,4.3,5.2,5.3,5.4,6.3 | 3 | tester |
| 7.2 | Test interactions + fix lỗi | 7.1 | 3 | tester |
| 7.3 | Cập nhật docs + skill | 7.2 | 3 | docs |

## Ghi chú từ review
- 6.1: deliverable rõ "xác định nguyên nhân + đề xuất fix" (không mở rộng phạm vi).
- 5.2: rủi ro cao — cân nhắc HTML5 DnD vs plugin Tauri trước khi commit hướng.
- 5.5: có smoke test đơn lẻ (không chờ tới 7.x).

## Phát hiện khảo sát (bổ sung vào phase 2)
- **BUG hiện hữu**: DualPaneExplorer.ts gọi `fs_rename`, `fs_delete`, `fs_copy`, `fs_move`, `fs_open` — KHÔNG tồn tại trong lib.rs (chỉ có fs_mv, fs_cp, fs_rm, fs_mkdir, fs_cat, fs_write, fs_upload, fs_download, fs_link_create, fs_links_list, fs_list_local, fs_list_remote). Context menu hiện hỏng → cần thêm alias command Rust hoặc sửa frontend dùng tên đúng. Xử lý trong 2.2.
- `loadPane('right')` gọi `fs_list_remote` với `account: undefined` → xác nhận signature command.

## Trạng thái
| phase | mô tả | trạng thái |
|-------|-------|------------|
| 1 | Khảo sát + checkpoint | [x] |
| 2 | Refactor theo feature | [x] |
| 3 | Build mới | [x] |
| 4 | Bảng cột Nemo-like | [x] |
| 5 | Interactions Nemo-like | [x] |
| 6 | Fix Panel 2 | [x] |
| 7 | Verify + docs | [x] |

## Kết quả cuối (2026-08-07)
- Build: `apps_gui/filen_gui/build_release.sh` → `release/filen_gui/` (binary + dist + icons), `bundle.active=false`.
- Refactor: components/pane/ + services/ + features/; DualPaneExplorer.ts chỉ orchestration.
- Cột: Name|Type|Size|Modified + resize handle + sort click th (dirs-first).
- Interactions: clipboard (Ctrl+C/X/V), drag-drop giữa pane, rubber-band multi-select, context menu đầy đủ.
- Panel 2: guard auth → placeholder khi chưa đăng nhập.
- Verify: npm run build PASS, vitest 34/34 PASS, cargo test 100/100 PASS, binary chạy OK.
- Cross-check (Mercury-2): CLEAN.