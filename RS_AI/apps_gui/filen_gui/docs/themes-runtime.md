<!-- INTEGRITY NOTES: Runtime spec for custom_themes in filen_gui v3.0.0 (Tauri v2). -->
<!-- Purpose: Define ThemeManager (TS), hot-reload fs watcher, runtime validation, fallback, Settings UI, test plan. -->
<!-- Source of truth: docs/i18n-and-themes.md §3 (theme schema + validation) + docs/design-tokens.md (master tokens) + docs/app-shell.md (Tauri v2 shell, vanilla TS + Vite). -->
<!-- Framework: Tauri v2 (Rust backend + WebView frontend). Frontend stack: vanilla TypeScript + Vite (app-shell.md §2.1). -->
<!-- Sub-doc #6 of docs v3.0.0 set — hub: docs/neon_ui_design.md. -->

# `filen_gui` — Custom Themes Runtime (Tauri v2)

**Version**: 3.0.0
**Status**: chốt v3.0.0 — runtime spec cho `ThemeManager` (TS) + fs watcher (Rust) + Settings UI
**Nguồn kế thừa**: `docs/i18n-and-themes.md` §3 (schema/validation/merge) · `docs/design-tokens.md` (master tokens) · `docs/app-shell.md` §2/§4 (stack + event bus)

---

## 1. Phạm vi & nguyên tắc

- Spec này mô tả **runtime** (lúc chạy) của custom themes — bổ sung cho spec **schema** ở `i18n-and-themes.md` §3.
- Frontend: **vanilla TypeScript + Vite** (`frontend/src/`), không runtime framework (app-shell §2.1).
- Backend: Rust (Tauri commands + fs watcher). Core service layer giữ nguyên 100% (app-shell §1).
- Mọi token/API tham chiếu phải khớp `design-tokens.md` (master) và `i18n-and-themes.md` §3.

---

## 2. ThemeManager (TS module)

File: `frontend/src/themes/ThemeManager.ts` (mới).

### 2.1 Trách nhiệm

1. **Load danh sách theme** từ app data dir (`themes/*.json`) — qua Tauri fs plugin hoặc command Rust.
2. **Parse + validate** từng file (xem §4).
3. **Merge 3 tầng**: `default` → `user` → `runtime` (khớp `i18n-and-themes.md` §3.5).
4. **Apply** lên `:root` CSS variables (khớp `i18n-and-themes.md` §4).
5. **Quản lý hot-reload** khi nhận event từ Rust watcher (§3).
6. **Cung cấp state** cho Settings UI (§5).

### 2.2 API (TS)

```ts
interface ThemeFile {
  name: string;        // bắt buộc (i18n §3.2)
  version: string;     // bắt buộc
  tokens: Record<string, string>; // bắt buộc, map { token_name: value }
}

interface ThemeEntry {
  slug: string;        // tên file (không đuôi .json) — i18n §3.1 "tên file = tên theme (slug)"
  file: string;        // path tuyệt đối
  name: string;
  version: string;
  valid: boolean;      // false nếu parse/validate fail
  errors: string[];    // warning/error log
  tokens: Record<string, string>; // tokens đã validate (chỉ key hợp lệ)
}

class ThemeManager {
  constructor(opts: { themesDir: string; defaultTokens: Record<string,string> });
  async loadAll(): Promise<ThemeEntry[]>;          // đọc + validate mọi file trong themes/
  async loadOne(file: string): Promise<ThemeEntry>; // đọc + validate 1 file
  apply(entry: ThemeEntry | null): void;            // merge 3 tầng + setProperty :root
  setRuntimeTweak(token: string, value: string): void; // tầng 3 (không ghi file)
  clearRuntimeTweaks(): void;
  getActive(): string | null;                       // slug theme đang active
  onHotReload(cb: (entry: ThemeEntry | null) => void): void; // đăng ký từ event
}
```

### 2.3 Load danh sách theme

- **Nguồn dữ liệu**: thư mục `themes/` trong app data dir (i18n §3.1):
  - Windows: `%APPDATA%/filen_gui/themes/*.json`
  - macOS: `~/Library/Application Support/filen_gui/themes/*.json`
  - Linux: `~/.config/filen_gui/themes/*.json`
- **Cách đọc** (2 lựa chọn, chọn 1):
  - **A. Tauri fs plugin**: `readDir` + `readTextFile` từ `@tauri-apps/plugin-fs` (cần permission `fs:allow-read-dir` + `fs:allow-read-text-file` trong `capabilities/default.json` — app-shell §5).
  - **B. Command Rust** (khuyến nghị, giữ validation ở backend): `#[tauri::command] fn themes_list() -> Result<Vec<ThemeEntry>, String>` — Rust scan dir, parse + validate, trả về entries. Frontend chỉ render.
- **Khuyến nghị B**: validation ở Rust (serde_json) khớp `i18n-and-themes.md` §3.6, giảm bề mặt JS, tái dùng cho watcher.

---

## 3. Hot-reload — fs watcher (Rust side)

### 3.1 Kiến trúc

```
Rust (src-tauri/src/themes.rs — mới)
  notify::RecommendedWatcher ── watch_dir(themes/)
       │  event: create / modify / remove / rename
       ▼
  debounce (300ms) → re-scan dir → re-parse + validate từng file
       ▼
  app.emit("themes:changed", { entries: ThemeEntry[], active: string|null })
       ▼
Frontend (ThemeManager.onHotReload)
  → nếu file active bị sửa: re-merge + setProperty :root (không rebuild DOM, không reload WebView — i18n §3.8)
  → nếu file active bị xóa: fallback default (giữ theme hiện tại? xem §5)
  → cập nhật danh sách trong Settings UI
```

### 3.2 Luồng chi tiết

1. **Khởi tạo**: trong `setup()` (app-shell §2 `lib.rs`), tạo watcher trỏ tới `themes/` dir. Nếu dir chưa tồn tại → tạo trước (không lỗi).
2. **Event → debounce**: watcher phát event liên tục khi file bị ghi nhiều lần. Gom vào bộ đệm, sau **200ms** không có event mới thì xử lý 1 lần (debounce). Tránh re-parse spam khi editor lưu nhiều lần.
3. **Xử lý file bị sửa giữa chừng**: nếu file đang ghi dở (partial write) → `serde_json::from_str` fail → entry `valid: false` + log warning. **Không crash**. Khi file ghi xong, watcher phát event `modify` lần cuối → re-parse thành công → apply.
4. **Rename**: `rename` event → xử lý như remove (file cũ) + create (file mới). Nếu file active bị rename → fallback default.
5. **Emit**: sau khi scan xong, `app.emit("themes:changed", payload)` (dùng `tauri::Emitter`, khớp app-shell §4.1 pattern).
6. **Frontend nhận**: `import { listen } from '@tauri-apps/api/event'` → `listen("themes:changed", cb)` đăng ký 1 lần ở `main.ts` (khớp app-shell §4.1).

### 3.3 Debounce & bảo vệ

- **Debounce 200ms** — gộp burst event.
- **Ignore event không liên quan**: chỉ quan tâm file `.json` trong `themes/`; bỏ qua subdir, file ẩn, file tạm (`*.tmp`, `*.swp`).
- **Không đọc lại file đang active nếu nội dung không đổi** (so hash/`modified` time) — tránh setProperty thừa.

---

## 4. Validation runtime

Khớp `i18n-and-themes.md` §3.6. Chạy ở **Rust backend** (khuyến nghị) hoặc TS (nếu dùng fs plugin).

### 4.1 Các bước

1. **Kích thước file**: đọc metadata trước; nếu `> 256 KB` → **reject** (entry invalid, không parse). (i18n §3.6.5)
2. **Parse JSON**: `serde_json::from_str` — fail → invalid + warning. (i18n §3.6)
3. **Kiểu dữ liệu**: `name`/`version` phải string; `tokens` phải object; mỗi value phải string. (i18n §3.6.1)
4. **Hex regex** (cho token màu): `^#([0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$` hoặc `^#RRGGBB@0.\d+$` (alpha hậu tố). Không hợp lệ → **bỏ qua key + warning**. (i18n §3.6.2)
5. **Unknown key**: key không tồn tại trong schema master (`design-tokens.md`) → **bỏ qua + warning**, không làm hỏng theme. (i18n §3.6.3)
6. **Reference `@token`**: phải trỏ tới token tồn tại. Reference có thể trỏ vào reference khác (chuỗi `@token → @token`), nhưng phải **cycle detection**: duyệt chuỗi reference, nếu phát hiện vòng lặp (A→B→A) → **reject** token đó + warning. (i18n §3.6.4)
7. **Range clamp**: value số (px, s, weight, zIndex) nằm ngoài khoảng hợp lý → clamp về biên (vd z-index 0–500, size 0–64px). Không fail, chỉ clamp + warning.
8. **Security**: không `eval`/`exec`; không key bắt đầu `on*`; không chấp nhận `url(...)`, `expression(...)`, `javascript:`; sanitize `name` (escape HTML) trước khi render. (i18n §3.7)

### 4.2 Kết quả

- Entry hợp lệ → `tokens` chỉ chứa key đã validate (key lỗi bị loại).
- Entry lỗi → `valid: false` + `errors[]` (mảng warning/error string).

---

## 5. Fallback

- **Theme lỗi** (parse fail, thiếu name/version, vượt size) → **giữ theme hiện tại** + log error. Không crash app. (i18n §3.6 fallback)
- **Lần khởi động sau vẫn thử lại**: **không blacklist vĩnh viễn**. Mỗi lần `loadAll()` đều đọc lại mọi file — file lỗi lần trước có thể sửa được → lần sau load thành công. Chỉ giữ trạng thái lỗi trong phiên hiện tại (in-memory), không ghi file blacklist.
- **File active bị xóa** → fallback về **default theme** (tầng 1), giữ runtime tweaks (tầng 3). Log warning.
- **Không có file nào hợp lệ** → dùng default theme.

### 5.1 Merge 3 tầng (khớp i18n §3.5)

```
final = default ⊕ user ⊕ runtime
```

- `default`: master tokens từ `design-tokens.md` (đủ ~76 token).
- `user`: tokens từ file theme đang active (chỉ key hợp lệ).
- `runtime`: tweaks tạm thời trong phiên (vd slider brightness) — không ghi file.
- Key thiếu ở tầng sau → giữ giá trị tầng trước.

### 5.2 Apply lên `:root` (khớp i18n §4)

```ts
function applyTheme(tokens: Record<string, string>) {
  const root = document.documentElement.style;
  for (const [token, value] of Object.entries(tokens)) {
    root.setProperty(`--${token.replace(/\./g, "-")}`, value);
  }
}
```
- Map token → CSS var: `colors.surface.canvas` → `--colors-surface-canvas` (dấu `.` → `-`).
- Chỉ đổi CSS vars, **không rebuild DOM**, không reload WebView (i18n §3.8).

---

## 6. Settings UI

File: `frontend/src/views/ThemesSettings.ts` + `frontend/src/views/themes.html` (mới). Dùng `lang_id` từ `i18n-and-themes.md` §2.4.3.

### 6.1 Thành phần

| Thành phần | Mô tả | lang_id |
|---|---|---|
| Tiêu đề | Header panel | `themes_title` |
| Danh sách theme | List các `ThemeEntry` (name + version) | `themes_name`, `themes_version` |
| Preview card | Mini swatch: hiển thị vài màu chủ đạo (canvas, card, neon.cyan, text.primary) từ tokens | — |
| Nút Apply | Áp dụng theme đang chọn | `themes_apply` |
| Nút Delete | Xóa file theme (qua command Rust) | — |
| Nút Import | Mở dialog chọn file `.json` → copy vào `themes/` | `themes_import` |
| Nút Export | Xuất theme hiện tại ra file | `themes_export` |
| Reset | Về default theme | `themes_reset` |
| Trạng thái | Thông báo lỗi/thành công | `themes_invalid`, `themes_loaded` |

### 6.2 Luồng

- **Apply**: `ThemeManager.applyTheme(entry)` → merge + setProperty. Lưu slug vào `settings.json` (field `active_theme`).
- **Delete**: gọi command Rust `themes_delete(file)` → xóa file → watcher emit `themes:changed` → UI tự cập nhật. Nếu xóa theme đang active → fallback default.
- **Import**: dialog chọn file → copy vào `themes/` (validate trước khi copy) → watcher emit → UI cập nhật.
- **Lưu lựa chọn**: `settings.json` (app data dir) chứa `{ "active_theme": "<slug>" }`. Lúc khởi động, `ThemeManager` đọc `settings.json` → apply theme active.

---

## 7. Test plan

### 7.1 Edge cases

| # | Case | Kỳ vọng |
|---|---|---|
| 1 | File rỗng (0 byte) | invalid + warning, giữ theme hiện tại |
| 2 | JSON sai cú pháp | invalid + warning, không crash |
| 3 | Token sai kiểu (value là number/object thay vì string) | bỏ key + warning |
| 4 | Hex sai format (vd `#GGG`, `red`, `url(...)`) | bỏ key + warning |
| 5 | File quá lớn (> 256 KB) | reject, không đọc |
| 6 | Xóa file đang dùng (active) | fallback default, UI cập nhật |
| 7 | Hot-reload khi đang transfer (file ghi dở) | parse fail → retry khi file xong, không crash |
| 8 | 2 theme cùng tên (2 file cùng slug, khác dir?) | xử lý: file sau đè file trước + warning (hoặc đổi tên) |
| 9 | Unknown key | bỏ + warning, theme vẫn áp dụng |
| 10 | Reference `@token` trỏ tới token không tồn tại | bỏ key + warning |
| 11 | Reference vòng lặp (A→B→A) | phát hiện, bỏ + warning |
| 12 | Runtime tweak sau khi hot-reload | tweak giữ nguyên (tầng 3 đè) |
| 13 | Không có file nào hợp lệ | dùng default theme |
| 14 | File active bị rename | fallback default, UI cập nhật |

### 7.2 Unit test — validator (Rust)

- `validate_theme(json_str)` → `Result<ThemeEntry, Vec<String>>`:
  - parse fail, thiếu name/version, tokens không object, value sai type
  - hex regex đúng/sai (6/8-digit, `@0.85`, `#GG`)
  - unknown key, reference không tồn tại, reference vòng lặp
  - size > 256KB
  - clamp giá trị ngoài biên

### 7.3 Integration test — watcher (Rust)

- Tạo dir tạm `themes/` → watcher → ghi file → chờ debounce → assert event `themes:changed` emit + entry hợp lệ.
- Ghi file dở (partial) → assert không crash, retry.
- Xóa file → assert fallback.
- Rename file → assert create+remove.

---

## 8. Tóm tắt

- **ThemeManager (TS)**: load/parse/validate/merge 3 tầng/apply `:root` CSS vars; nguồn `themes/*.json` trong app data dir.
- **Hot-reload**: Rust `notify` watcher → debounce 200ms → re-parse → `app.emit("themes:changed")` → frontend `listen` → setProperty (không rebuild DOM).
- **Validation**: size ≤256KB, hex regex, unknown key bỏ qua, reference resolve, clamp, security (không eval).
- **Fallback**: theme lỗi → giữ hiện tại + log; không blacklist vĩnh viễn (thử lại mỗi lần khởi động).
- **Settings UI**: list + preview swatch + apply/delete/import/export/reset; lưu `active_theme` vào `settings.json`.
- **Test**: 14 edge cases + unit test validator + integration test watcher.

---
*End of Custom Themes Runtime spec.*