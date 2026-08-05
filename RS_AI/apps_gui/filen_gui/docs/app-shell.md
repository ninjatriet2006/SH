<!-- INTEGRITY NOTES: App-shell spec for Tauri v2 migration of filen_gui v3.0.0. -->
<!-- Purpose: Map the framework-agnostic boundary API (docs/architecture.md) onto a Tauri v2 shell + web frontend. -->
<!-- Scope: src-tauri/ layout, command mapping table, event bus, capabilities, window/font/dnd/clipboard, migration checklist. -->
<!-- Verified against: src/operations.rs (2,782 dòng), src/transfer.rs (873 dòng), src/main.rs (3,624 dòng). -->
<!-- Sub-doc #3 of docs v3.0.0 set — hub: docs/neon_ui_design.md. -->

# `filen_gui` App Shell — Tauri v2

**Version**: 3.0.0
**Status**: chốt v3.0.0 — spec cho migration eframe/egui → Tauri v2
**Nguồn**: `docs/architecture.md` (boundary API) · `docs/design-tokens.md` (tokens) · `src/operations.rs` · `src/transfer.rs` · `src/main.rs`
**Nguyên tắc**: core service layer **giữ nguyên 100%** (operations.rs + transfer.rs), chỉ viết lại UI layer + adapter Tauri.

---

## 1. Mục tiêu & phạm vi

- Thay shell eframe/egui bằng **Tauri v2** (webview hệ điều hành + Rust backend), giữ nguyên boundary API ở `docs/architecture.md`.
- **Điểm mấu chốt**: filen_gui không cần gọi JS-side CLI — mọi lệnh `filen-cli` đều chạy trong Rust core (`tokio::process::Command`). Webview chỉ là view mỏng.

---

## 2. Cấu trúc project Tauri v2

```
filen_gui/
├── Cargo.toml                    # [thay] bỏ eframe/egui, thêm tauri
├── src/                          # [giữ nguyên] core (không sửa)
│   ├── operations.rs             # 2,782 dòng — KEEP 100%
│   ├── transfer.rs               #   873 dòng — KEEP 100%
│   └── main.rs                   # 3,624 dòng — [xoá bỏ] thay bằng src-tauri
├── src-tauri/                    # [mới]
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json           # window config, bundle
│   ├── capabilities/default.json # permissions tối thiểu
│   └── src/
│       ├── main.rs               # thin: Builder → run (gọi lib)
│       ├── lib.rs                # Builder, state manage, setup, command registration
│       ├── state.rs              # AppState (TransferManager+ServersState bọc Mutex)
│       └── commands/
│           ├── mod.rs
│           ├── auth.rs           # login/logout/whoami/statfs/accounts
│           ├── fs.rs             # list/mkdir/rm/mv/cp/upload/download/cat/link/write
│           ├── misc.rs           # recents, sync, favorites/trash (phase 2)
│           ├── servers.rs        # webdav/s3/mount start/stop/logs
│           └── transfer.rs       # enqueue/cancel/cancel_all/remove_finished
└── frontend/                     # [mới] — stack đã chốt: vanilla TS + Vite
    ├── package.json
    ├── vite.config.ts
    ├── index.html
    └── src/
        ├── main.ts               # bootstrap: listen events, mount app
        ├── store.ts              # view state (PaneState, AccountState, TransferList…)
        ├── api.ts                # invoke wrapper (1 hàm/command)
        ├── events.ts             # listen map: event → store dispatch
        ├── views/                # Explorer, Recents, Sync, Servers, Login, TransferDrawer
        ├── styles/tokens.css     # CSS custom properties ← design-tokens.md
        └── assets/fonts/         # woff2 tiếng Việt (embedded)
```

### 2.1 Frontend stack đã chốt: **vanilla TypeScript + Vite**

| Lý do | Chi tiết |
|---|---|
| **1. Tối giản dependency** | Không runtime framework, không compiler pipeline riêng (Svelte 5 cần runes + preprocess). Team Rust-first: JS chỉ nên là lớp view mỏng ~vài nghìn dòng TS, dễ đọc/dễ bỏ. |
| **2. Vite là đường sẵn của Tauri v2** | `npm create tauri-app` mặc định hỗ trợ Vite; dev server + HMR + build tĩnh không cần config adapter thêm. |
| **3. Đúng boundary API** | architecture.md §7.5: "nếu framework cần thêm trường chỉ để render, phải bọc ở view layer" — vanilla TS giữ ranh giới sạch, sau này nâng lên React/Svelte chỉ viết lại view, không đụng core/commands. |

> Svelte 5 bị loại: thêm ~1.500 dòng config/build + runtime reactive không cần thiết cho 2-pane explorer + drawer; HMR của Svelte cũng đắt hơn cho dự án này.

---

## 3. Mapping boundary API → Tauri commands

> **QUY ƯỚC**: tham số Tauri nhận **camelCase** từ JS, tự map sang snake_case Rust.
> `Result<(), String>` ở Rust → `invoke()` trả `{ ok, error }` hoặc promise resolve/reject; bảng dưới ghi kiểu trả về ở Rust.
> Mọi dòng đã verify với hàm thật trong `operations.rs` / `transfer.rs`.

### 3.1 Auth (5 commands — verify: operations.rs)

| Tauri command | Args (JS) | Return (Rust) | async | Backing fn thật |
|---|---|---|---|---|
| `auth_login` | `email, password, twofaCode?, keepLogged` | `Result<(), String>` (lỗi `"2FA_REQUIRED"` khi thiếu mã) | ✅ | `Operations::login_new(email, password, twofa_code, keep_str, tx)` |
| `auth_login_twofa` | `email, password, twofaCode, keepLogged` | `Result<(), String>` | ✅ | `Operations::login_new` (lần 2, có mã) |
| `auth_logout` | `account?` | `Result<(), String>` | ✅ | `Operations::logout(account)` |
| `auth_whoami` | `(none)` | `Result<Option<String>, String>` | ✅ | `Operations::whoami(&None)` + lọc email rác (logic main.rs:3284–3294) |
| `auth_statfs` | `account?` | `Result<[String, String], String>` (used, max) | ✅ | `Operations::statfs(account)` |

> `2FA`: không phải command riêng — `auth_login` không kèm mã sẽ trả `Err("2FA_REQUIRED")`; frontend đổi modal sang bước nhập mã rồi gọi `auth_login_twofa`. (Khớp logic `drain_async_results` main.rs:2829–2845.)
> `accounts_load` / `accounts_save` (sync, không async): `load_stored_accounts()` / `save_stored_accounts(&[StoredAccount])` — cũng trong operations.rs.

### 3.2 FS Cloud + Local (12 commands — verify: operations.rs)

| Tauri command | Args (JS) | Return (Rust) | async | Backing fn thật |
|---|---|---|---|---|
| `fs_list_remote` | `account?, path` | `Result<Vec<FileItem>, String>` | ✅ | `Operations::list_remote(account, path)` |
| `fs_list_local` | `path` | `Result<Vec<FileItem>, String>` | ✅ | `Operations::list_local(path)` — `#[tauri::command] async fn fs_list_local(path: String) -> Result<Vec<FileItem>, String>` |
| `fs_mkdir` | `account?, path` | `Result<(), String>` | ✅ | `Operations::mkdir(account, path)` |
| `fs_rm` | `account?, path, noTrash` | `Result<(), String>` | ✅ | `Operations::rm(account, path, no_trash)` |
| `fs_mv` | `account?, from, to` | `Result<(), String>` | ✅ | `Operations::mv(account, from, to)` |
| `fs_cp` | `account?, from, to` | `Result<(), String>` | ✅ | `Operations::cp(account, from, to)` |
| `fs_upload` | `account?, local, remote` | `Result<(), String>` | ✅ | `Operations::upload(account, local, remote)` |
| `fs_download` | `account?, remote, local` | `Result<(), String>` | ✅ | `Operations::download(account, remote, local)` |
| `fs_cat` | `account?, path` | `Result<String, String>` | ✅ | `Operations::cat(account, path)` |
| `fs_link_create` | `account?, path` | `Result<String, String>` | ✅ | `Operations::create_link(account, path)` |
| `fs_links_list` | `account?` | `Result<Vec<[String, String]>, String>` (path,url) | ✅ | `Operations::list_links(account)` |
| `fs_write` | `account?, path, content` | `Result<(), String>` | ✅ | `Operations::write_file(account, path, content)` (multiline tự qua temp+upload) |

> Phase 2 (có hàm nhưng GUI egui chưa dùng): `fs_favorite`, `fs_unfavorite`, `fs_list_favorites`, `fs_trash_list`, `fs_trash_restore`, `fs_trash_delete`, `fs_trash_empty`, `fs_head`, `fs_tail`, `fs_stat`, `fs_export_notes`, `fs_export_auth_config`, `fs_export_api_key`, `fs_view` — tất cả đều có sẵn trong operations.rs (một số `#[allow(dead_code)]`), map cùng pattern trên.

### 3.3 Recents (1 command — verify: operations.rs)

| Tauri command | Args | Return | async | Backing fn |
|---|---|---|---|---|
| `recents_list` | `account?` | `Result<Vec<FileItem>, String>` | ✅ | `Operations::recents(account)` |

### 3.4 Sync (3 commands — verify: operations.rs)

| Tauri command | Args (JS) | Return (Rust) | async | Backing fn thật |
|---|---|---|---|---|
| `sync_pairs` | `(none)` | `Result<Vec<SyncPair>, String>` | ❌ sync | `Operations::sync_pairs()` |
| `sync_run` | `account?, locations[], continuous` | `Result<(), String>` | ✅ | `Operations::sync(account, locations, continuous)` |
| `sync_once` | `account?, local, remote` | `Result<(), String>` | ✅ | `Operations::sync_once(account, local, remote)` |

> `sync_pair_once` (async) cũng có sẵn nếu cần sync theo alias từ syncPairs.json.

### 3.5 Servers (7 commands — verify: operations.rs + main.rs)

| Tauri command | Args (JS) | Return (Rust) | async | Backing fn thật |
|---|---|---|---|---|
| `server_webdav_start` | `account?, user, pass, port, https` | `Result<(), String>` | ✅ | `WebDavServerState::start(account)` (args từ `webdav_args`) |
| `server_webdav_stop` | `(none)` | `Result<(), String>` | ✅ | `WebDavServerState::stop()` |
| `server_s3_start` | `account?, accessKey, secretKey, port, https` | `Result<(), String>` | ✅ | `S3ServerState::start(account)` |
| `server_s3_stop` | `(none)` | `Result<(), String>` | ✅ | `S3ServerState::stop()` |
| `server_mount_start` | `account?, mountPoint?` | `Result<String, String>` (ghi chú + point) | ✅ | `MountState::start(account, mount_point)` |
| `server_mount_stop` | `(none)` | `Result<(), String>` | ✅ | `MountState::stop()` |
| `server_logs` | `which` ("webdav"/"s3"/"mount") | `Result<Vec<String>, String>` | ❌ sync | trường `logs: Vec<String>` của state |

> Nguồn: egui dùng `spawn_webdav/spawn_s3/spawn_mount` (main.rs:3585–3624) + `WebDavServerState` (operations.rs:1364–1500). Bản Tauri **ưu tiên dùng state struct có sẵn** (giữ child + logs) thay vì spawn riêng như egui. Có thể thêm `server_webdav_start_proxy` (backing: `start_proxy`, operations.rs:1388).

### 3.6 Transfer (4 commands — verify: transfer.rs)

| Tauri command | Args (JS) | Return (Rust) | async | Backing fn thật |
|---|---|---|---|---|
| `transfer_enqueue` | `kind, name, src, dst, srcLocal, dstLocal, cleanupSrc, srcPane, dstPane` | `Result<usize, String>` (id) | ❌ sync | `TransferManager::enqueue(...)` |
| `transfer_cancel` | `id` | `Result<(), String>` | ❌ sync | `TransferManager::cancel(id)` |
| `transfer_cancel_all` | `(none)` | `Result<(), String>` | ❌ sync | `TransferManager::cancel_all()` |
| `transfer_remove_finished` | `(none)` | `Result<(), String>` | ❌ sync | `TransferManager::remove_finished()` |

> Upload/download thật chạy qua `run_cli_transfer(kind, src, dst, timeout, cancelled, on_update)` (transfer.rs:243) trong `tauri::async_runtime::spawn` — progress phát qua event (§4), không qua command.
> Cloud→Cloud dùng `fs_cp`/`fs_mv`; Local→Local dùng `copy_local`/`move_local`/`delete_local_path` (transfer.rs:630–659) — đã verify.

### 3.7 Tổng

| Nhóm | Số command |
|---|---|
| Auth | 5 (+2 accounts sync) |
| FS | 12 (+13 phase-2 sẵn có) |
| Recents | 1 |
| Sync | 3 |
| Servers | 7 |
| Transfer | 4 |
| **Tổng bảng chính** | **32** (45 nếu tính phase-2) |

---

## 4. Event bus: Rust → Frontend

Thay kênh `mpsc::Sender<AsyncResult>` (main.rs) bằng **Tauri Event** — đúng nguyên tắc architecture.md §5 (event bus là kênh duy nhất core → UI).

### 4.1 Bảng event (map từ `AsyncResult` main.rs:338–387)

| Tauri event | Payload (serde JSON) | Từ | AsyncResult nguồn |
|---|---|---|---|
| `core:files-listed` | `{ pane, items: FileItem[] }` | `fs_list_remote` worker | `FilesListed(idx, items)` |
| `core:pane-error` | `{ pane, error }` | mọi op theo pane | `Error(idx, err)` |
| `auth:whoami-finished` | `{ email?: string, error?: string }` | setup worker | `WhoAmIFinished` |
| `auth:statfs-finished` | `{ used, max, error? }` | setup/after login | `StatfsFinished` |
| `auth:login-finished` | `{ email, keepLogged, ok, error? }` | `auth_login` worker | `LoginFinished` |
| `auth:logout-finished` | `{ email, ok, error? }` | `auth_logout` worker | `LogoutFinished` |
| `auth:log` | `{ line }` | `login_new` `tx` → `AppEvent::LoginLog` | *(mới — tách từ login_new)* |
| `core:file-op-finished` | `{ kind, pane, name, ok, error? }` | mkdir/rm/mv/cp/fav… | `FileOpFinished` |
| `core:file-text-finished` | `{ kind, name, ok, content?, error? }` | cat/head/tail/view | `FileTextFinished` |
| `transfer:progress` | `{ id, progress?, bytesDone, totalBytes }` | `run_cli_transfer` `on_update` | `TransferProgress` |
| `transfer:finished` | `{ id, ok, error? }` | transfer worker | `TransferFinished` |
| `recents:finished` | `{ items, error? }` | `recents_list` | `RecentsFinished` |
| `sync:pairs-finished` | `{ pairs, error? }` | `sync_pairs` | `SyncPairsFinished` |
| `sync:pair-finished` | `{ idx, ok, error? }` | `sync_run` worker | `SyncPairFinished` |
| `server:started` | `{ which, ok, error? }` | server start worker | `ServerStarted` |
| `server:log` | `{ which, line }` | log reader task | *(mới — stream logs)* |

Rust: `use tauri::Emitter;` → `app.emit("transfer:progress", payload)`.
Frontend: `import { listen } from '@tauri-apps/api/event';` → `listen("transfer:progress", cb)` (đăng ký 1 lần ở `main.ts`).

### 4.2 Luồng async lifecycle

```
Khởi tạo
  Rust setup() → manage(AppState) → spawn whoami worker
       └─ emit auth:whoami-finished → frontend dispatch store (active? reload panes)

Mỗi lệnh ngắn (list/mkdir/rm/mv/cp/cat/link/recents/statfs/whoami/logout)
  JS invoke() ──► #[tauri::command] async ──► tokio runtime (không block webview)
       └─ kết quả trả thẳng về await (Result<T,String>) — UI cập nhật trong then()

Lệnh dài (transfer, login, server start/stop, sync)
  JS invoke() ──► command spawn worker trên tauri::async_runtime, trả ngay
       └─ worker emit nhiều event (progress/log/finished) → store update từng phần

Tắt ứng dụng (CloseRequested)
  on_window_event → transfer.cancel_all() → stop webdav/s3/mount → save accounts
```

> Nguyên tắc: command **trả trực tiếp** cho op nhanh (UI await), **spawn + emit** cho op dài. Không cần `request_repaint()` như egui — DOM tự cập nhật khi store đổi.

---

## 5. Capabilities / permissions tối thiểu

File: `src-tauri/capabilities/default.json` (draft — verify tên permission khi cài plugin).

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:event:default",
    "core:window:default",
    "dialog:default",
    {
      "identifier": "fs:allow-read-dir",
      "allow": [{ "path": "$HOME/**" }]
    }
  ]
}
```

> **Ghi chú quan trọng**: theo boundary API, mọi lệnh `filen-cli` chạy trong **Rust core** (`tokio::process::Command`) nên webview **không cần** `shell` — đã **bỏ `shell:allow-execute`** khỏi capabilities để giảm bề mặt tấn công. Chỉ giữ `core:default + core:event:default + dialog:default` (+ `clipboard-manager` nếu dùng plugin). Nếu feature tương lai cần mở URL/CLI từ JS thì thêm `tauri-plugin-shell` + permission `shell:allow-execute` lúc đó.
> Plugin bổ sung: `tauri-plugin-dialog` (`dialog:allow-open/save`), `tauri-plugin-clipboard-manager` (`clipboard-manager:allow-write-text`), `tauri-plugin-shell` (chỉ khi cần mở URL/CLI từ JS).

---

## 6. Window config, font tiếng Việt, drag & drop, clipboard

### 6.1 Window (`src-tauri/tauri.conf.json`)

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "filen_gui",
  "version": "0.1.0",
  "identifier": "io.filen.gui",
  "build": {
    "beforeDevCommand": "npm run dev --prefix frontend",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "npm run build --prefix frontend",
    "frontendDist": "../frontend/dist"
  },
  "app": {
    "windows": [
      {
        "title": "Filen File Manager — GUI",
        "width": 1100,
        "height": 700,
        "minWidth": 800,
        "minHeight": 520,
        "resizable": true,
        "center": true
      }
    ],
    "security": { "csp": null }
  },
  "bundle": { "active": true, "targets": ["deb", "appimage"] }
}
```

> Khớp window hiện tại của egui (main.rs:506–509): 1100×700, title "Filen File Manager — GUI". Thêm `minWidth/minHeight` để UI 2-pane không vỡ.

### 6.2 Font tiếng Việt

- **Embed webfont woff2** vào `frontend/src/assets/fonts/` (subset Latin Extended + Vietnamese: Noto Sans VN / Roboto / Be Vietnam Pro). Load qua `@font-face` — **không phụ thuộc font hệ thống** (khác egui load font system main.rs:434–498).
- Fallback chain theo `design-tokens.md` §5.1: `Noto Sans, Roboto, system-ui, sans-serif`; mono: `JetBrains Mono, ui-monospace, monospace`.
- Token → CSS variables trong `frontend/src/styles/tokens.css` (đủ ~76 token từ design-tokens.md; dùng `@theme` nếu cần).

### 6.3 Drag & drop file OS

| Cách | Mô tả | Khuyến nghị |
|---|---|---|
| **Native (chính)** | `on_window_event` bắt `WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. })` trong Rust → nhận **đường dẫn tuyệt đối** → emit `core:files-dropped` | ✅ dùng cho drop file từ file-manager vào dropzone |
| HTML5 | `dragover`/`drop` trên webview trả `File` nhưng **không có path tuyệt đối** trong Tauri | chỉ dùng cho kéo-thả nội bộ (sắp xếp/điều hướng) |
| Plugin | `tauri-plugin-drag-drop` (community) cho JS `onDragDropEvent` | tuỳ chọn, tránh dependency |

> Egui hiện dùng pointer events nội bộ (main.rs:852–886) — phần đó chuyển sang HTML5 hoàn toàn (2-pane trong web, không cần path OS).

### 6.4 Clipboard

- **Chính**: `tauri-plugin-clipboard-manager` (`writeText`) — cross-platform, không cần shell out.
- **Dự phòng**: giữ `Operations::copy_to_clipboard` (operations.rs:212) cho nhánh không có plugin.
- Egui dùng clipboard nội bộ (Ctrl+C/X/V) để copy/move giữa 2 pane → giữ nguyên trong **frontend store** (ClipboardContent tương đương main.rs:281–287).

---

## 7. Migration checklist từ egui (7.279 dòng) → Tauri

### 7.1 Giữ nguyên 100% (core — không sửa, không thêm GUI)

| File | Dòng | Ghi chú |
|---|---|---|
| `src/operations.rs` | 2,782 | Toàn bộ ops cloud/local + server state + helper args. Chỉ sửa nhỏ: `login_new` đã nhận `tx: Option<UnboundedSender<AppEvent>>` — đổi để worker Tauri emit `auth:log` (giữ signature cũ, truyền tx mới). |
| `src/transfer.rs` | 873 | `TransferManager`, `run_cli_transfer` (nhận `on_update` → wrap emit `transfer:progress`), `copy_local/move_local/delete_local_path`. |
| `Cargo.toml` deps core | — | Giữ `tokio/serde/serde_json/chrono/dirs/which`. |
| Unit tests | — | 100% giữ (không phụ thuộc GUI). |

### 7.2 Viết lại (UI + adapter — ~3.624 dòng main.rs)

| Mục main.rs cũ | Đi đâu | Giữ / Viết lại |
|---|---|---|
| `main()` eframe, `NativeOptions` | `src-tauri/src/main.rs` + `tauri.conf.json` | **Viết lại** (~30 dòng) |
| `FilenGuiApp::update` vòng lặp frame | `frontend/src/main.ts` + `store.ts` | **Viết lại** (drain event mỗi lần nhận thay vì mỗi frame) |
| `PaneState`, `AccountState`, `LoginFormState`, `Modal`, `ClipboardContent`, `DragSource` | `frontend/src/store.ts` (TS types) | **Viết lại** nhưng giữ nguyên field/ý nghĩa |
| `AsyncResult` enum (drain main.rs:2748–3050) | `frontend/src/events.ts` (dispatch map) | **Viết lại** → map §4.1 |
| `start_login/start_logout/switch_account/whoami_async/statfs_async` | `src-tauri/src/commands/auth.rs` | **Viết lại** (worker + emit event) |
| `spawn_transfer_thread/start_transfer_between` | `src-tauri/src/commands/transfer.rs` + `frontend` | **Viết lại** (spawn + event; phân loại pane giữ nguyên) |
| `spawn_webdav/s3/mount` | `src-tauri/src/commands/servers.rs` (dùng state struct) | **Viết lại** |
| `setup_fonts/load_font` | `frontend/src/styles/tokens.css` + `@font-face` woff2 | **Viết lại** (embed thay vì đọc hệ thống) |
| `ui_sidebar/ui_panes/ui_pane/ui_login_window/ui_modal/…` | `frontend/src/views/*` (HTML+TS) | **Viết lại** theo design-tokens.md |
| `copy_to_clipboard` helper (main.rs dùng shell) | Plugin clipboard-manager (giữ core làm backup) | **Thay thế** |
| Drag & drop giữa pane | `frontend` HTML5; drop OS file → Rust `DragDropEvent` | **Viết lại** |

### 7.3 Thứ tự migration đề xuất (phases)

1. **P0 — khung**: `create-tauri-app` (vanilla TS + Vite), cấu hình window/font/tokens, `auth` + `fs_list` + `fs_cat` command, store 2-pane Explorer đọc được. 
2. **P1 — thao tác file**: mkdir/rm/mv/cp/upload/download/link + transfer queue + progress events (thay egui drawer).
3. **P2 — đầy đủ**: recents, sync, servers (start/stop/logs), favorites/trash/head/tail/stat/write, export. 
4. **P3 — hoàn thiện**: drag-drop OS file, clipboard plugin, bundle deb/appimage, kiểm tra dòng chữ tiếng Việt.

---

## 8. Rủi ro / điểm cần chú ý

1. **Tiến trình server**: egui dùng `std::process::Child` giữ trong state; Tauri cần bọc `Mutex<ServersState>` — server chạy nền không bị kill khi window đóng (giữ nguyên hành vi).
2. **`login_new` 2FA**: chuỗi lỗi `"2FA_REQUIRED"` là hợp đồng với frontend — không đổi format.
3. **CLI progress**: `run_cli_transfer` đã parse ESC[1G + `script -qec` — không đổi; chỉ đổi nơi nhận callback (mpsc → emit).
4. **Bảo mật mật khẩu**: `StoredAccount.password` lưu plaintext JSON (0600) như hiện tại — cân nhắc tích hợp keyring trong Tauri phase sau (ngoài phạm vi spec này).
5. **Tauri v2 API**: tên event/permission trong file này là draft — verify chính xác với tài liệu Tauri v2 khi implement (schema `gen/schemas`, `Emitter` trait).
