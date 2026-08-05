<!-- INTEGRITY NOTES: Master Neon UI Design & Technical Specification Document for filen_gui — v3.0.0 (HUB). -->
<!-- Purpose: Single entry point (hub) cho toàn bộ bộ docs filen_gui v3.0.0. Chốt framework Tauri v2 (Windows + macOS production, Linux dev). -->
<!-- Modules Interacted: src-tauri/commands (auth.rs, fs.rs, misc.rs, servers.rs, transfer.rs), frontend/src (main.ts, store.ts, api.ts, events.ts, views/*, styles/tokens.css), core src/operations.rs + src/transfer.rs (giữ nguyên 100%). -->
<!-- Compliance: Direct file operations only; ID Linking localization via lang_id; strict Data Integrity; rule prefix active. -->
<!-- Doc set: 1 hub (file này) + 6 sub-docs (architecture, design-tokens, app-shell, ui-spec, i18n-and-themes, themes-runtime). -->

# `filen_gui` Neon UI Design System & Technical Specification — v3.0.0 (HUB)

**Version**: 3.0.0  
**Date**: 2026-08-06  
**Target Component**: `apps_gui/filen_gui`  
**Framework**: Tauri v2 (Rust backend + WebView frontend)  
**Platforms**: Windows + macOS (production), Linux (dev)  
**Design Aesthetics**: Dark Cyberpunk Neon Glassmorphism  
**Document Location**: `apps_gui/filen_gui/docs/neon_ui_design.md` (hub — sibling to `src/`)

---

## Table of Contents
1. [Executive Summary](#1-executive-summary)
2. [Framework Decision: Tauri v2](#2-framework-decision-tauri-v2)
   - [2.1 Quyết định](#21-quyt-nh)
   - [2.2 Lý do chọn](#22-l-do-chn)
   - [2.3 Rủi ro & biện pháp giảm thiểu](#23-ri-ro--bin-php-gim-thiu)
3. [Doc Set Structure (index)](#3-doc-set-structure-index)
4. [Migration Overview: egui → Tauri](#4-migration-overview-egui--tauri)
5. [INTEGRITY NOTES (modules)](#5-integrity-notes-modules)
6. [Acceptance Criteria v3.0.0](#6-acceptance-criteria-v300)
7. [Version & History](#7-version--history)

---

## 1. Executive Summary

`filen_gui` v3.0.0 là bản chốt **Tauri v2** — Rust backend + WebView frontend (vanilla TS + Vite) — thay thế shell eframe/egui của bản v2.0.0. Sản phẩm giữ nguyên DNA thiết kế Neon (dark glassmorphism, dual-pane explorer, hybrid TUI + GUI workflow, ID Linking localization qua `lang_id`) nhưng chuyển toàn bộ UI sang web frontend và giữ **core service layer bất biến**.

### Key Objectives
1. **Framework chốt (R2)**: Tauri v2 trên Windows + macOS (production) và Linux (dev). Core (`operations.rs` + `transfer.rs`) giữ nguyên 100%, chỉ viết lại UI layer + adapter Tauri.
2. **Aesthetic Excellence (R1)**: Giữ bảng màu neon obsidian (`#00F3FF` Cyan, `#FF007F` Magenta, `#9D00FF` Purple, `#00FF87` Emerald, `#FF3366` Coral) — triển khai qua design tokens (`design-tokens.md`), không còn hardcode hex trong UI.
3. **Workflow Synthesis (R2)**: Dual-pane drag-and-drop + keyboard parity (hotkeys, command palette) kế thừa `filen_tui` — mapping đầy đủ trong `ui-spec.md` §3.
4. **Unambiguous Blueprint (R3)**: Bộ docs 7 file (1 hub + 6 sub-docs) làm hợp đồng kỹ thuật cho mọi dev/AI implement mà không cần hỏi thêm.

---

## 2. Framework Decision: Tauri v2

### 2.1 Quyết định

| Mục | Quyết định |
|---|---|
| **Framework** | **Tauri v2** (Rust backend + WebView hệ điều hành) |
| **Frontend stack** | **vanilla TypeScript + Vite** (không runtime framework — app-shell.md §2.1) |
| **Platforms** | Windows + macOS (production), Linux (dev) |
| **Core service layer** | `src/operations.rs` + `src/transfer.rs` — giữ nguyên 100% |
| **UI shell cũ** | eframe/egui (`src/main.rs`) — xoá bỏ, thay bằng `src-tauri/` + `frontend/` |

### 2.2 Lý do chọn

| # | Lý do | Chi tiết |
|---|---|---|
| 1 | **Kích thước binary & RAM** | Tauri dùng webview hệ điều hành (WebView2 / WKWebView) thay vì bundle engine render riêng → binary nhỏ hơn nhiều so với egui/eframe, phù hợp app desktop nhẹ. |
| 2 | **CSS hiện đại cho hiệu ứng Neon** | Glassmorphism, glow, gradient, backdrop-filter, animation đạt chuẩn hơn trên CSS/WebView so với egui painter API — đạt 100% spec v2.0.0 (§2.2–2.4) với ít code hơn. |
| 3 | **Core giữ nguyên** | Boundary API framework-agnostic (architecture.md §7) cho phép đổi shell không đụng core. `operations.rs`/`transfer.rs` tái dùng toàn bộ — rủi ro regression thấp. |
| 4 | **Hệ sinh thái & tooling** | Vite HMR + TypeScript cho UI, `tauri::async_runtime` cho lệnh dài, event bus (`Emitter`) thay `mpsc` — khớp architecture.md §5. |
| 5 | **Multi-platform** | Một codebase chạy Windows/macOS/Linux; bundle `deb`/`appimage`/`msi`/`dmg` từ Tauri CLI. |

### 2.3 Rủi ro & biện pháp giảm thiểu

| # | Rủi ro | Mức | Giảm thiểu |
|---|---|---|---|
| 1 | WebView khác nhau theo OS (WebView2/WKWebView/WebKitGTK) — CSS hiển thị lệch | Trung bình | Test trên cả 3 nền tảng; dùng CSS fallback (font stack tiếng Việt, `@supports` cho backdrop-filter); pin layout tối giản. |
| 2 | Tauri v2 API (permission, event, plugin) còn mới — tên draft có thể đổi | Trung bình | Verify với schema `gen/schemas` lúc implement; capabilities tối thiểu (app-shell.md §5). |
| 3 | Drag & drop file OS cần đường dẫn tuyệt đối — HTML5 drop không có path | Trung bình | Dùng `WindowEvent::DragDrop` phía Rust (app-shell.md §6.3); HTML5 chỉ cho kéo-thả nội bộ. |
| 4 | `login_new` 2FA — chuỗi `"2FA_REQUIRED"` là hợp đồng với frontend | Thấp | Giữ format lỗi; không đổi (app-shell.md §8.2). |
| 5 | Tiến trình server (WebDAV/S3/Mount) bị kill khi window đóng | Trung bình | Bọc `Mutex<ServersState>`; xử lý `CloseRequested` cancel + stop trước khi thoát (app-shell.md §8.1). |
| 6 | Mật khẩu `StoredAccount.password` plaintext JSON | Trung bình | Giữ như hiện tại (0600); tích hợp keyring ở phase sau — ngoài phạm vi v3.0.0. |

---

## 3. Doc Set Structure (index)

Bộ docs v3.0.0 gồm **1 hub (file này) + 6 sub-docs**. Mỗi sub-doc là 1 hợp đồng độc lập; đọc hub trước để biết thứ tự.

| # | File | Mô tả (1 dòng) | Khi nào đọc |
|---|---|---|---|
| 1 | `architecture.md` | Core architecture framework-agnostic: boundary API (traits), event bus, lifecycle, rules bất biến khi đổi framework | Trước tiên — hiểu ranh giới UI/core; mọi thay đổi core phải theo doc này |
| 2 | `design-tokens.md` | Master list ~76 design tokens (colors/typography/spacing/radius/effects/zIndex) + quy tắc override `custom_themes` | Khi làm bất kỳ UI/theme nào — nguồn chốt token, không tự đặt tên mới |
| 3 | `app-shell.md` | Tauri v2 shell: cấu trúc `src-tauri/` + `frontend/`, bảng 32 commands, event bus, capabilities, window/font/dnd/clipboard, migration checklist | Khi implement backend/commands/events hoặc verify mapping core → Tauri |
| 4 | `ui-spec.md` | UI spec frontend: layout 7 screens, component library, hotkey map, state TS interfaces, mermaid flows | Khi implement view/component/state trên WebView — tham chiếu token + lang_id |
| 5 | `i18n-and-themes.md` | i18n dictionary (`lang_id`, lookup, hot-switch) + theme schema JSON, merge 3 tầng, validation, security | Khi thêm text, thêm ngôn ngữ, hoặc định nghĩa theme file |
| 6 | `themes-runtime.md` | Runtime custom themes: `ThemeManager` (TS), fs watcher hot-reload, validation runtime, fallback, Settings UI, test plan | Khi implement/bảo trì hệ thống theme lúc chạy |

---

## 4. Migration Overview: egui → Tauri

Bản v2.0.0 dùng **eframe/egui** (`src/main.rs`, ~3.624 dòng shell + view). Bản v3.0.0 chuyển sang **Tauri v2** theo nguyên tắc: **core giữ nguyên 100%, chỉ viết lại UI layer + adapter**.

```
egui (v2.0.0)                        Tauri v2 (v3.0.0)
─────────────────────                ─────────────────────────────
src/main.rs (app shell + view)   →   src-tauri/ (main.rs, lib.rs, state.rs, commands/*)
   eframe window + painter            tauri.conf.json + capabilities/default.json
   ui_*() egui widgets            →   frontend/src/views/* (HTML/CSS/TS)
   mpsc::Sender<AsyncResult>      →   Tauri Event (Emitter) — app-shell.md §4.1
   setup_fonts/load_font          →   @font-face woff2 trong frontend/assets/fonts
   egui::Shadow / Color32         →   CSS var(--token) từ design-tokens.md
src/operations.rs (2.782 dòng)    →   GIỮ NGUYÊN 100%
src/transfer.rs   (  873 dòng)    →   GIỮ NGUYÊN 100%
```

### 4.1 Giữ nguyên (core — không sửa)

| File | Dòng | Vai trò trong v3.0.0 |
|---|---|---|
| `src/operations.rs` | 2,782 | Toàn bộ ops cloud/local + server state + account/auth. Backing cho commands (`auth_*`, `fs_*`, `recents_list`, `sync_*`, `server_*`). |
| `src/transfer.rs` | 873 | `TransferManager`, `run_cli_transfer`, `copy_local/move_local/delete_local_path`. Backing cho `transfer_*`. |
| Unit tests | — | Giữ 100% (không phụ thuộc GUI). |

### 4.2 Viết lại (UI + adapter)

| Mục cũ (egui) | Đi đâu (v3.0.0) | Ghi chú |
|---|---|---|
| `main()` eframe + `NativeOptions` | `src-tauri/src/main.rs` + `tauri.conf.json` | ~30 dòng |
| `FilenGuiApp::update` vòng lặp frame | `frontend/src/main.ts` + `store.ts` | Drain event khi nhận, không mỗi frame |
| `PaneState`, `AccountState`, `Modal`, `ClipboardContent` | `frontend/src/store.ts` (TS types) | Giữ field/ý nghĩa (ui-spec.md §4) |
| `ui_sidebar/ui_panes/ui_login_window/ui_modal/…` | `frontend/src/views/*` | Theo design-tokens + lang_id |
| `spawn_transfer_thread`, `spawn_webdav/s3/mount` | `src-tauri/src/commands/{transfer,servers}.rs` | Spawn + emit event |
| `setup_fonts` | `frontend/src/styles/tokens.css` + `@font-face` | Embed webfont tiếng Việt |
| Clipboard nội bộ (Ctrl+C/X/V) | `frontend` store + plugin clipboard-manager | Giữ backup `Operations::copy_to_clipboard` |

### 4.3 Thứ tự migration (phases)

1. **P0 — khung**: `create-tauri-app` (vanilla TS + Vite), window/font/tokens, `auth_*` + `fs_list` + `fs_cat`, store 2-pane Explorer.
2. **P1 — thao tác file**: mkdir/rm/mv/cp/upload/download/link + transfer queue + progress events.
3. **P2 — đầy đủ**: recents, sync, servers (start/stop/logs), favorites/trash/head/tail/stat/write, export.
4. **P3 — hoàn thiện**: drag-drop OS file, clipboard plugin, bundle `deb`/`appimage`/`msi`/`dmg`, kiểm tra tiếng Việt.

---

## 5. INTEGRITY NOTES (modules)

> Cập nhật cho v3.0.0: modules thay đổi từ `main.rs`, `operations.rs`, `transfer.rs` (v2.0.0) sang cấu trúc Tauri. **Core vẫn là `operations.rs` + `transfer.rs` (giữ nguyên).**

| Module | Vị trí | Trách nhiệm | Doc liên quan |
|---|---|---|---|
| Core — `Operations` | `src/operations.rs` | Ops cloud/local + server state + account/auth — **GIỮ NGUYÊN** | architecture.md §4 |
| Core — `TransferManager` | `src/transfer.rs` | Transfer queue + CLI runner — **GIỮ NGUYÊN** | architecture.md §4.2 |
| Commands — Auth | `src-tauri/src/commands/auth.rs` | `auth_login`, `auth_login_twofa`, `auth_logout`, `auth_whoami`, `auth_statfs`, `accounts_load/save` | app-shell.md §3.1 |
| Commands — FS | `src-tauri/src/commands/fs.rs` | `fs_list_remote/local`, `fs_mkdir/rm/mv/cp/upload/download/cat/link/write` (+ phase-2) | app-shell.md §3.2 |
| Commands — Misc | `src-tauri/src/commands/misc.rs` | `recents_list`, `sync_*` | app-shell.md §3.3–3.4 |
| Commands — Servers | `src-tauri/src/commands/servers.rs` | `server_webdav/s3/mount_*`, `server_logs` | app-shell.md §3.5 |
| Commands — Transfer | `src-tauri/src/commands/transfer.rs` | `transfer_enqueue/cancel/cancel_all/remove_finished` | app-shell.md §3.6 |
| Shell — Tauri | `src-tauri/src/{main,lib,state}.rs` | Builder, state manage, setup, command registration | app-shell.md §2 |
| Frontend — App | `frontend/src/{main,store,api,events}.ts` | Bootstrap, view state, invoke wrapper, event dispatch | ui-spec.md §4 |
| Frontend — Views | `frontend/src/views/*` | Explorer, Recents, Sync, Servers, Login, TransferDrawer, ThemesSettings | ui-spec.md §2 |
| Frontend — Tokens | `frontend/src/styles/tokens.css` | CSS custom properties ← design-tokens.md (map 1-1) | design-tokens.md / i18n-and-themes.md §4 |
| Frontend — Fonts | `frontend/src/assets/fonts/` | woff2 tiếng Việt (Noto Sans VN / Roboto / Be Vietnam Pro) | app-shell.md §6.2 |
| Theme Runtime | `src-tauri/src/themes.rs` (mới) | fs watcher hot-reload, validate, emit `themes:changed` | themes-runtime.md §3–4 |
| Theme Manager | `frontend/src/themes/ThemeManager.ts` (mới) | Load/validate/merge 3 tầng/apply `:root` | themes-runtime.md §2 |

---

## 6. Acceptance Criteria v3.0.0

Các tiêu chí sau phải đạt trước khi coi v3.0.0 là hoàn thành:

| # | Tiêu chí | Mức | Doc tham chiếu |
|---|---|---|---|
| 1 | Khởi chạy được trên **Windows + macOS + Linux (dev)** với cùng binary từ `src-tauri/` | Bắt buộc | app-shell.md §2 |
| 2 | `auth_*` (login/2FA/whoami/statfs) chạy đúng, chuỗi `"2FA_REQUIRED"` giữ nguyên | Bắt buộc | app-shell.md §3.1 |
| 3 | 2-pane Explorer (local/cloud) đọc được, điều hướng, chọn nhiều, hotkey map đầy đủ | Bắt buộc | ui-spec.md §2.2, §3 |
| 4 | Transfer queue hoạt động qua event `transfer:progress`/`transfer:finished` (không qua command return) | Bắt buộc | app-shell.md §4 |
| 5 | Mọi màu/size dùng `var(--token)` (không hardcode hex) — khớp design-tokens.md | Bắt buộc | ui-spec.md §6 |
| 6 | Mọi label qua `t(key)` — key đều có trong i18n dictionary (80 keys), hot-switch VI/EN không reload | Bắt buộc | i18n-and-themes.md §2 |
| 7 | Custom theme load/validate/merge 3 tầng + hot-reload qua watcher, không crash khi file lỗi | Bắt buộc | themes-runtime.md §2–5 |
| 8 | Core `operations.rs` + `transfer.rs` không đổi signature — toàn bộ unit test pass | Bắt buộc | architecture.md §7 |
| 9 | Servers (WebDAV/S3/Mount) start/stop/logs qua command + state struct; không bị kill khi đóng window | Bắt buộc | app-shell.md §3.5, §8 |
| 10 | Font tiếng Việt embed woff2 — dấu tiếng Việt hiển thị đúng trên cả 3 OS | Bắt buộc | app-shell.md §6.2 |

---

## 7. Version & History

| Version | Ngày | Thay đổi |
|---|---|---|
| **3.0.0** | 2026-08-06 | Chốt **Tauri v2** (Windows + macOS, Linux dev). Hub doc: Framework Decision, doc set index (6 sub-docs), migration egui → Tauri, INTEGRITY NOTES modules mới (`src-tauri/commands`, `frontend/src`), acceptance criteria. Sub-docs đồng bộ version 3.0.0. |
| 2.0.0 | (trước) | Bản egui: thiết kế neon đầy đủ (palette, glow, typography, state matrices, mermaid flows, Rust state structs, lang_id). Được kế thừa làm nguồn cho design-tokens/ui-spec. |

---

*End of v3.0.0 hub document.*
