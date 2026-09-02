[Pattern Docs]
# KIẾN TRÚC THƯ MỤC FRONTEND

- **index.html**: Khung DOM — menubar, top nav, sidebar (tree), các `.app-view`, transfer drawer.
- **src/main.ts**: Entry point. Mount `DualPaneExplorer`, `MenuBar`, `TreeView`,
  `TransferDrawer`; nạp i18n; điều phối chuyển tab view.
- **src/store.ts**: State toàn cục *không thuộc explorer* (nhật ký, bookmark, cài đặt)
  và cơ chế di trú khoá localStorage.
- **src/services/**: Giao tiếp backend và các store chuyên biệt.
- **src/features/**: Logic tính năng (clipboard, drag-drop, transfer queue, context menu).
- **src/components/**: Thành phần UI; `components/pane/` là các phần của một khung file.
- **../bridge/**: Lớp bọc `invoke()` cho các nhóm API (explorer, remote, mount, config).

# NGUỒN SỰ THẬT (SINGLE SOURCE OF TRUTH)

- **Trạng thái explorer** (path, files, selection, sort, độ rộng cột, lịch sử
  back/forward, cột hiển thị) → **chỉ** nằm trong `services/explorerStore.ts`.
  Truy cập qua accessor (`getPanePath`, `setPaneFiles`, …), không đọc trực tiếp `state`.
- **Trạng thái còn lại** (activityLog, bookmarks, settings) → `store.ts`.
- Đường dẫn luôn ở dạng `Remote::/path`; backend tự bóc tách.

# KHOÁ LOCALSTORAGE

Tiền tố `rclonegui_`. `store.readStored()` tự di trú một lần từ tiền tố `filen_`
(codebase gốc) sang tiền tố mới rồi xoá khoá cũ.

| Khoá | Nội dung |
|---|---|
| `rclonegui_activity_log` | Nhật ký hoạt động (tối đa 200 bản ghi) |
| `rclonegui_bookmarks` | Danh sách ghim |
| `rclonegui_settings` | `AppSettings` |
| `rclonegui_emblems` | Emoji ghim trên file/folder |
| `rclonegui_explorer_state` | Độ rộng cột + cột hiển thị của 2 pane |

# SỰ KIỆN (CUSTOM EVENTS)

| Tên | Phát từ | Lắng nghe ở |
|---|---|---|
| `rclonegui-settings-changed` | `store.saveSettings()` | `PaneView` (nạp lại), `PaneToolbar` (đồng bộ nhãn) |
| `rclonegui-bookmarks-changed` | `store.toggleBookmark()`, `BookmarkManagerModal` | — (dành cho UI ghim tương lai) |
| `rclonegui-emblems-changed` | `emblemStore.save()` | `FileTable` (vẽ lại emblem) |
| `open-transfer-drawer` | `transferManager.enqueue()` | `TransferDrawer` |
| `transfer_progress` (Tauri) | backend `logic/transfer.rs` | `transferManager` |

# CẤU TRÚC DỮ LIỆU (INTERFACES)

## store.ts
- **FileItem** — `uuid`, `name`, `is_dir`, `size`, `mod_time`,
  `file_type?`, `owner?`, `group?`, `permissions?`
- **ActivityItem** — `id`, `timestamp`, `action`, `details`
- **BookmarkItem** — `name`, `path`
- **AppSettings** — `showHiddenFiles`, `language`, `theme`
- **AppState** — `activityLog?`, `bookmarks?`, `settings?`
  (Ghi chú: trước đây có `explorer?` nhân đôi với `explorerStore` — đã gỡ bỏ.)

## services/explorerStore.ts
- **Pane** = `'left' | 'right'`
- **ExplorerSelection** — `pane`, `name`, `path`, `is_dir`
- **ExplorerState** — path/files/selection/sort/colWidths/history/visibleCols cho 2 pane.

## services/undoManager.ts
- **UndoAction** — `type: 'rename'|'copy'|'move'|'delete'`, `src`, `dest`, `isLocal`

## features/transferManager.ts
- **TransferKind** = `'upload'|'download'|'copy'|'move'|'delete'`
- **TransferStatus** = `'queued'|'running'|'done'|'error'|'cancelled'`
- **TransferTask** — `id`, `kind`, `name`, `src`, `dst`, `status`, `progress`,
  `bytesDone`, `totalBytes`, `speed`, `srcLocal`, `dstLocal`, `isFallback?`, `excludes?`, …

# TÀI LIỆU HÀM (API DOCS)

## store.ts
- **readStored**(`key`) → `string | null` — đọc khoá, tự di trú từ `filen_*`.
- **logActivity**(`action`, `details`) — ghi nhật ký, giới hạn 200 bản ghi.
- **isBookmarked**(`path`) → `boolean`
- **toggleBookmark**(`name`, `path`) — ghim/bỏ ghim, phát `rclonegui-bookmarks-changed`.
- **saveSettings**() — lưu `appState.settings`, phát `rclonegui-settings-changed`.

## services/explorerStore.ts
Accessor cho từng pane: `getPanePath`/`setPanePath`, `getPaneFiles`/`setPaneFiles`,
`getPaneSelection`/`setPaneSelection`/`clearPaneSelection`,
`getPaneSortKey`/`getPaneSortDir`/`setPaneSort`, `getPaneColWidths`/`setPaneColWidth`,
`getPaneVisibleCols`/`setPaneVisibleCols`, `getActivePane`/`setActivePane`.
Lịch sử: `pushPaneHistory`, `popPaneBack`, `popPaneForward`, `canPaneGoBack`, `canPaneGoForward`.

## services/fileOps.ts
Lớp bọc mỏng quanh `bridge/explorer_api.ts` — chuyển tiếp nguyên vẹn full path
xuống backend, không tự parse hay xử lý quyền.
- **listLocal**(`path`) → `Promise<FileItem[]>`
- **searchLocal**(`path`, `query`) → `Promise<SearchResult[]>`
- **mkdir** / **remove** / **rename**(`path`, `newName`)
- **copy** / **move**(`src`, `dest`, `taskId?`)
- **cpLocal** / **moveLocal** / **upload** / **download** / **cpBatch**
- **open**(`path`) — mở bằng ứng dụng mặc định của OS.
- **statAdvanced**(`path`) → `Promise<StatInfo>`
- **getFreeSpace** / **getAboutSpace**(`path`)
- **chmod**(`path`, `mode`) / **chown**(`path`, `uid`, `gid`) — chỉ ổ Local; lỗi
  được ném lên để `PropertiesModal` hiển thị cho người dùng.
- Ghi chú: `cat` và `write` (với nội dung khác rỗng) chưa được backend hỗ trợ.

## services/trashOps.ts
Bọc các command `fs_trash_*`. Hiện phần lớn backend chưa triển khai và UI chưa có
đường dẫn `trash://` nào, nên nhánh này chưa hoạt động đầy đủ.

## services/undoManager.ts
- **push**(`action`) — ghi vào ngăn xếp undo (tối đa 50), xoá ngăn xếp redo.
- **undo**() / **redo**() — tính toán và thực thi thao tác đảo ngược.
  Ghi chú: undo `delete` chưa hỗ trợ vì chưa có hệ thống Thùng rác.

## features/transferManager.ts
- **enqueue**(`kind`, `name`, `src`, `dst`, `onSuccess?`, `onFail?`, `isFallback?`, `excludes?`) → `Promise<number>`
  Đẩy task vào hàng đợi, phát `open-transfer-drawer`, kích hoạt `processQueue`.
- **processQueue**() *(private)* — xử lý tuần tự. Với `move`/`copy` sẽ hỏi
  `check_transfer_capability` trước; nếu remote không hỗ trợ thì mở `FallbackModal`
  và tách thành chuỗi task thay thế (server-side copy+delete, hoặc tải về temp rồi upload).
- **cancel**(`id`) / **cancelAll**() / **removeFinished**()

## features/format.ts
- **formatSize**(`bytes`) → `string`
- **formatDate**(`iso`) → `string`
- **escapeHtml**(`value`) → `string` — **bắt buộc** dùng khi ghép dữ liệu ngoài
  (tên file, tên remote, đường dẫn) vào `innerHTML`.

## features/dragDrop.ts
- **serializeDrag** / **parseDrag** — payload kéo thả giữa 2 pane.
- **baseName** / **joinPath** / **generateUniqueName**
- **startOSDrag**(`paths`) — kéo file Local ra desktop qua `tauri-plugin-drag`.

## features/clipboard.ts
- **setClipboard** / **getClipboard** / **hasClipboard** / **clearClipboard**
- **syncFromOSClipboard**() — đọc clipboard giả lập từ backend.
- **pasteTo**(...) — kiểm tra xung đột (`fs_check_conflicts`), hỏi người dùng
  (`ConflictModal`), rồi đẩy vào `transferManager`.

## features/contextMenu.ts
- **showMenu**(`e`, `items`, `onClick`) — hiển thị menu tại vị trí chuột.
- **MenuFile**(`e`, opts) / **MenuEmpty**(`e`, opts) — menu cho file và cho vùng trống.

# THÀNH PHẦN UI

| Component | Vai trò | Điểm mount |
|---|---|---|
| `DualPaneExplorer` | Điều phối 2 pane, hotkey, drag-drop, context menu | `#view-explorer` |
| `pane/PaneContainer` | Quản lý tab của một pane | trong DualPaneExplorer |
| `pane/PaneView` | Toolbar + breadcrumb + bảng file + status bar | trong PaneContainer |
| `pane/PaneToolbar` | Nav, bookmark, tìm kiếm, view mode, toggle file ẩn | trong PaneView |
| `pane/FileTable` | Bảng file, chọn nhiều, rubber-band, thumbnail | trong PaneView |
| `pane/PaneStatusBar` | Tổng số mục, dung lượng ổ | trong PaneView |
| `MenuBar` | File/Edit/View/Go — nhận `ExplorerCommands` qua constructor | `#menubar-container` |
| `TreeView` | Cây thư mục Local ở sidebar, tải con lazy | `#tree-container` |
| `RecentsView` | Hiển thị `activityLog` | `#view-activity` |
| `DebugView` | Log lời gọi API | `#view-debug` |
| `TransferDrawer` | Hàng đợi truyền tải | `#transfer-drawer` |
| `SearchModal` | Tìm kiếm đệ quy theo tên | mở từ nút 🔍 |
| `BookmarkManagerModal` | Sửa/xoá/sắp xếp ghim | mở từ menu 🔖 |
| `PropertiesModal` | Thuộc tính file + emblem | mở từ context menu |
| `OpenWithModal`, `ConflictModal`, `FallbackModal`, `BatchRenameModal`, `OperationModal`, `ContextMenu`, `FloatingStatusBar` | Hộp thoại / tiện ích dùng chung | theo ngữ cảnh |

`OperationModal` là khung hộp thoại dùng chung; nội dung nằm trong `.modal-body`,
lấy qua `getBody()` khi cần thay sau khi tải dữ liệu.

# GIẢI PHÓNG TÀI NGUYÊN (CLEANUP)

Các component có đăng ký listener trên `window`/`document` hoặc
`IntersectionObserver` **phải** có `destroy()` và được gọi khi bị thay thế:
- `FileTable.destroy()` — gọi từ `PaneView.renderBody`/`renderPlaceholder`.
- `PaneToolbar.destroy()` và `PaneView.destroy()` — gọi từ `PaneContainer.closeTab`.

# KIỂM THỬ
- `features/format.test.ts` — `formatSize`, `formatDate`.
- `features/escapeHtml.test.ts` — thoát HTML, chống attribute breakout.
- `store.test.ts` — di trú khoá localStorage.
Chạy: `npm test` (vitest, môi trường jsdom).
