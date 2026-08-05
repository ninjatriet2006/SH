# plan-neon-docs-v3.md — Hoàn thiện docs Neon UI + lang_id + custom_themes

## Mục tiêu
Hoàn thiện lại bộ docs cho `apps_gui/filen_gui`:
(a) Đánh giá framework (egui vs Tauri v2/iced/Slint) — nếu egui không phù hợp thì đổi framework + thiết kế lại từ đầu;
(b) Bổ sung cơ chế `lang_id` (i18n) và `custom_themes` (theme load từ file config) vào kiến trúc;
(c) Xuất bản docs v3.0.0 hoàn chỉnh, implementable, nhất quán với code.

## Yêu cầu gốc (user)
1. Hoàn thiện lại docs (docs/neon_ui_design.md hiện tại v2.0.0, 888 dòng).
2. Nếu egui không phù hợp → thay đổi framework khác, thiết kế lại từ đầu.
3. Bổ sung lang_id + custom_themes với thông số load từ file.

## Kết quả khảo sát (đã verify 2026-08-06)
- Code hiện tại: egui/eframe 0.31, 7.279 dòng (main.rs 3624, operations.rs 2782, transfer.rs 873).
- Cloud ops 100% qua binary `filen` CLI (tokio process + std process cho servers WebDAV/S3/FUSE).
- Drag&drop nội bộ tự custom (pointer coords); KHÔNG nhận drop file từ OS.
- Font: fallback hệ thống Linux (`/usr/share/fonts`), không embed → không portable Windows/macOS.
- TTY/ANSI parse trong transfer.rs fragile (wrapper `script -qec` + parse ESC[1G).
- Doc cũ viết spec theo ngôn ngữ CSS/web (box-shadow, text-shadow, transition, glassmorphism) — egui immediate mode KHÔNG native hỗ trợ (cần hack painter, hiệu năng kém, kết quả không đạt).
- Localization mục 7.2 doc cũ dùng thuật toán Qt widget-tree (`update_language_ui`) — KHÔNG áp dụng được cho immediate mode.
- Không có AGENTS.md trong repo.

## QUYẾT ĐỊNH FRAMEWORK (user chốt 2026-08-06): **Tauri v2**
- User yêu cầu thiết kế cho **Windows + macOS** (Linux giữ làm dev).
- Lý do: CSS native → neon/glassmorphism chuẩn; pixel-perfect trên Win (WebView2) + macOS (WKWebView); bundle nhỏ; distribution mạnh (.exe/.msi, .dmg); mobile tiềm năng.
- Chấp nhận: viết lại UI egui (~3.6k dòng) sang HTML/CSS/TS; test trên 3 webview khác nhau; WebKitGTK là rủi ro trên Linux (dev).
- Backend: operations.rs + transfer.rs (3.7k dòng) tái dùng → Tauri commands.
- UI mới: HTML/CSS/TS + Vite (chọn stack frontend trong phase 3, cân nhắc vanilla TS vs Svelte).

## ROADMAP (đã qua Plan Reviewer + Cross Checker)
| phase | mô tả | phụ thuộc | ưu tiên | trạng thái |
|-------|-------|-----------|---------|------------|
| 1 | Framework Decision: **CHỐT TAURI v2** | - | 1 | [x] |
| 2 | Core Rust modules framework-agnostic + master design tokens (boundary API cố định, link themes runtime) | - (song song 1) | 1 | [x] |
| 3 | App shell + IPC mapping theo Tauri (commands, events, capabilities) + draft docs incremental | 1,2 | 1 | [x] |
| 4 | UI Neon spec theo Tauri (HTML/CSS): screens + state + interaction (dùng token list) + draft docs | 3 | 1 | [x] |
| 5 | lang_id i18n + custom_themes schema/file format + security/perf risk analysis | 2 | 1 | [x] |
| 6 | custom_themes runtime: hot-reload/merge/fallback + validation test plan (edge cases) | 5 | 2 | [x] |
| 7 | Docs v3.0.0 final: tổng hợp draft + consistency (integrity notes, version, mermaid) + acceptance criteria | 4,5,6 | 1 | [x] |

## Ghi chú quyết định từ review
- Plan Reviewer: phase 1 cần gate Go/No-Go + deliverable; spec framework-agnostic; phase 4 phụ thuộc token master; bỏ critical path kẹt (tách schema/runtime).
- Cross Checker (Mercury-2): thêm fallback path phase 1; incremental docs sau mỗi phase; risk analysis (untrusted theme file, reload latency); link token list ↔ themes runtime; validation plan cho lang_id + themes loading.

## Acceptance criteria (phase 7)
- [ ] Docs v3.0.0 đầy đủ: framework decision + kiến trúc + UI spec + lang_id + custom_themes + migration.
- [ ] Mọi spec implementable không cần hỏi lại (hex/state/schema/diagram đầy đủ).
- [ ] Integrity notes trong docs khớp modules thực tế.
- [ ] lang_id: cơ chế lookup phù hợp framework chốt (không còn Qt widget-tree).
- [ ] custom_themes: file format + schema + merge/override + hot-reload + fallback được định nghĩa đầy đủ.
