<!-- INTEGRITY NOTES: i18n (lang_id) + custom_themes spec for filen_gui v3.0.0 (Tauri v2). -->
<!-- Purpose: Define JSON dictionary schema for localization and JSON theme file schema for custom themes. -->
<!-- Source of truth: docs/design-tokens.md (master tokens) + docs/neon_ui_design.md §7 (lang_id table, nguồn kế thừa v2.0.0). -->
<!-- Framework: Tauri v2 (Rust backend + WebView frontend). CSS custom properties map 1-1 to design tokens. -->
<!-- Sub-doc #5 of docs v3.0.0 set — hub: docs/neon_ui_design.md. -->

# `filen_gui` — i18n & Custom Themes (Tauri v2)

**Version**: 3.0.0
**Status**: chốt v3.0.0 — spec cho `src/i18n/*.json` (dictionary) và `themes/*.json` (theme file)
**Nguồn kế thừa**: `docs/design-tokens.md` (master tokens) + `docs/neon_ui_design.md` §7 (bảng `lang_id` cũ)

---

## 1. Tổng quan kiến trúc

- **Frontend**: WebView (HTML/CSS/JS) — mọi text hiển thị qua `t(key, lang)`; mọi màu/size qua CSS custom property `--token-name`.
- **Backend**: Rust (Tauri commands) — đọc/validate theme file, quản lý fs watcher, lưu cấu hình.
- **Hai hệ thống độc lập**: i18n (text) và themes (visual). Không trộn lẫn.

---

## 2. i18n — `lang_id` dictionary

### 2.1 File & schema

- Vị trí: `src/i18n/vi.json`, `src/i18n/en.json` (mỗi ngôn ngữ 1 file).
- Schema: phẳng `{ "lang_id": "translated string" }` — không lồng nhau, không metadata trong file.

```json
{
  "app_title": "FILEN NEON WORKSTATION",
  "nav_explorer": "📁 Trình duyệt",
  "btn_login": "ĐĂNG NHẬP"
}
```

- `lang_type` (ui/modal/server/error/transfer) là **metadata phụ** — không bắt buộc trong file; dùng để phân đoạn khi cần, không ảnh hưởng lookup.

### 2.2 Lookup function

```js
// dicts: { "vi": {...}, "en": {...} } — load 1 lần lúc khởi động
function t(key, lang) {
  const d = dicts[lang] || dicts.en || {};
  if (key in d) return d[key];
  if (lang !== "en" && key in dicts.en) return dicts.en[key]; // fallback en
  return key; // fallback cuối: trả về chính key (dễ phát hiện thiếu)
}
```

- **Fallback chain**: `lang` → `en` → `key` (raw). Không bao giờ trả về chuỗi rỗng.
- **Default language**: `en`. Nếu `lang` không tồn tại trong `dicts`, dùng `en`.

### 2.3 Hot-switch (không reload app)

- Khi user đổi ngôn ngữ (top bar `🌐 VI/EN`):
  1. Cập nhật `current_language` trong state.
  2. Gọi `applyLanguage(lang)` — duyệt toàn bộ DOM node có `data-lang-id` attribute.
  3. Với mỗi node: `node.textContent = t(node.dataset.langId, lang)`.
  4. Không reload WebView, không mất state (pane, selection, transfer queue giữ nguyên).

```js
function applyLanguage(lang) {
  document.querySelectorAll("[data-lang-id]").forEach((el) => {
    el.textContent = t(el.dataset.langId, lang);
  });
}
```

### 2.4 Danh sách `lang_id` đầy đủ

Kế thừa bảng 7.3 (26 keys) + bổ sung keys mới cho **servers dashboard**, **themes settings**, **transfer drawer**.

#### 2.4.1 Kế thừa từ bảng 7.3 (26 keys)

| `lang_id` | `lang_type` | `vi` | `en` |
|---|---|---|---|
| `app_title` | ui | `FILEN NEON WORKSTATION` | `FILEN NEON WORKSTATION` |
| `nav_explorer` | ui | `📁 Trình duyệt` | `📁 Explorer` |
| `nav_recents` | ui | `🕒 Gần đây` | `🕒 Recents` |
| `nav_sync` | ui | `🔄 Đồng bộ` | `🔄 Sync Pairs` |
| `nav_servers` | ui | `🖥️ Máy chủ` | `🖥️ Servers` |
| `pane_mode_local` | ui | `🖥️ Cục bộ` | `🖥️ Local` |
| `pane_mode_cloud` | ui | `☁️ Cloud` | `☁️ Cloud` |
| `hdr_file_name` | ui | `Tên tập tin` | `File Name` |
| `hdr_file_size` | ui | `Kích thước` | `Size` |
| `hdr_file_type` | ui | `Loại` | `Type` |
| `hdr_file_date` | ui | `Ngày sửa` | `Date Modified` |
| `btn_login` | modal | `ĐĂNG NHẬP` | `LOG IN` |
| `btn_cancel` | modal | `HỦY` | `CANCEL` |
| `modal_2fa_title` | modal | `Xác thực 2 yếu tố (2FA)` | `Two-Factor Authentication` |
| `modal_mkdir_title` | modal | `Tạo thư mục mới` | `Create New Directory` |
| `modal_rename_title` | modal | `Đổi tên tập tin` | `Rename File` |
| `modal_delete_title` | modal | `Xác nhận xóa` | `Confirm Delete` |
| `chk_perm_delete` | modal | `Xóa vĩnh viễn (không vào Thùng rác)` | `Permanently delete (skip trash)` |
| `server_webdav` | server | `Máy chủ WebDAV` | `WebDAV Server` |
| `server_s3` | server | `Máy chủ S3 API` | `S3 API Server` |
| `server_mount` | server | `Ổ đĩa Mount FUSE` | `FUSE Mount Drive` |
| `server_status_run` | server | `● ĐANG CHẠY` | `● RUNNING` |
| `server_status_stop` | server | `● ĐÃ DỪNG` | `● STOPPED` |
| `transfer_title` | ui | `⚡ Quản lý Truyền tải` | `⚡ Transfer Manager` |
| `err_same_path` | error | `📍 Đường dẫn nguồn và đích giống nhau` | `📍 Source and destination paths are identical` |
| `err_auth_req` | error | `🔒 Yêu cầu đăng nhập tài khoản Cloud` | `🔒 Cloud authentication required` |

#### 2.4.2 Bổ sung — Servers dashboard (10 keys)

| `lang_id` | `lang_type` | `vi` | `en` |
|---|---|---|---|
| `server_start` | server | `▶ KHỞI ĐỘNG` | `▶ START` |
| `server_stop` | server | `⏹ DỪNG` | `⏹ STOP` |
| `server_port` | server | `Cổng` | `Port` |
| `server_host` | server | `Host` | `Host` |
| `server_username` | server | `Tên đăng nhập` | `Username` |
| `server_password` | server | `Mật khẩu` | `Password` |
| `server_https` | server | `HTTPS` | `HTTPS` |
| `server_console_logs` | server | `Nhật ký` | `Console Logs` |
| `server_status_starting` | server | `● ĐANG KHỞI ĐỘNG` | `● STARTING` |
| `server_status_error` | server | `● LỖI` | `● ERROR` |

#### 2.4.3 Bổ sung — Themes settings (10 keys)

| `lang_id` | `lang_type` | `vi` | `en` |
|---|---|---|---|
| `themes_title` | ui | `⚙️ Giao diện` | `⚙️ Themes` |
| `themes_language` | ui | `Ngôn ngữ` | `Language` |
| `themes_apply` | ui | `Áp dụng` | `Apply` |
| `themes_reset` | ui | `Khôi phục mặc định` | `Reset to Default` |
| `themes_import` | ui | `Nhập theme` | `Import Theme` |
| `themes_export` | ui | `Xuất theme` | `Export Theme` |
| `themes_name` | ui | `Tên theme` | `Theme Name` |
| `themes_version` | ui | `Phiên bản` | `Version` |
| `themes_invalid` | error | `File theme không hợp lệ` | `Invalid theme file` |
| `themes_loaded` | ui | `Đã tải theme` | `Theme loaded` |

#### 2.4.4 Bổ sung — Transfer drawer (12 keys)

| `lang_id` | `lang_type` | `vi` | `en` |
|---|---|---|---|
| `transfer_active` | ui | `Truyền tải đang chạy` | `Active Transfers` |
| `transfer_upload` | ui | `⬆ Tải lên` | `⬆ Upload` |
| `transfer_download` | ui | `⬇ Tải xuống` | `⬇ Download` |
| `transfer_direction` | ui | `Hướng` | `Direction` |
| `transfer_file` | ui | `Tập tin` | `File` |
| `transfer_progress` | ui | `Tiến trình` | `Progress` |
| `transfer_speed` | ui | `Tốc độ` | `Speed` |
| `transfer_eta` | ui | `ETA` | `ETA` |
| `transfer_status` | ui | `Trạng thái` | `Status` |
| `transfer_clear_done` | ui | `Xóa mục đã hoàn thành` | `Clear Done` |
| `transfer_cancel_all` | ui | `Hủy tất cả` | `Cancel All` |
| `transfer_cancel` | ui | `Hủy` | `Cancel` |

#### 2.4.5 Bổ sung — Recents, Sync, Modals, Dropzone, Command palette, Context menu (22 keys)

| `lang_id` | `lang_type` | `vi` | `en` |
|---|---|---|---|
| `btn_view` | ui | `👁️ Xem nhanh` | `👁️ Quick View` |
| `btn_copy_link` | ui | `🔗 Sao chép Link` | `🔗 Copy Link` |
| `btn_download` | ui | `📥 Tải về` | `📥 Download` |
| `sync_run` | ui | `▶ Chạy Đồng bộ` | `▶ Run Sync Now` |
| `sync_settings` | ui | `⚙️ Cài đặt` | `⚙️ Settings` |
| `sync_remove` | ui | `🗑️ Gỡ Đồng bộ` | `🗑️ Remove Pair` |
| `copy_content` | modal | `📋 Sao chép Nội dung` | `📋 Copy Content` |
| `copy_link` | modal | `🔗 Sao chép Link` | `🔗 Copy Link to Clipboard` |
| `copy_to` | ui | `📄 COPY TO [PATH]` | `📄 COPY TO [PATH]` |
| `move_to` | ui | `📦 MOVE TO [PATH]` | `📦 MOVE TO [PATH]` |
| `cmd_mkdir` | ui | `📁 Tạo thư mục` | `📁 Create Directory` |
| `cmd_rename` | ui | `✏️ Đổi tên` | `✏️ Rename` |
| `cmd_delete` | ui | `🗑️ Xóa` | `🗑️ Delete` |
| `cmd_view` | ui | `👁️ Xem nội dung` | `👁️ Preview Content` |
| `cmd_link` | ui | `🔗 Tạo Link chia sẻ` | `🔗 Generate Share Link` |
| `cmd_switch_mode` | ui | `🔄 Đổi chế độ Pane` | `🔄 Switch Pane Mode` |
| `cmd_sync` | ui | `⚡ Đồng bộ thư mục` | `⚡ Trigger Directory Sync` |
| `cmd_server` | ui | `🖥️ Khởi chạy Máy chủ` | `🖥️ Launch WebDAV / S3 / FUSE` |
| `cmd_copy_link` | ui | `🔗 Sao chép Link` | `🔗 Copy Link` |
| `cmd_copy` | ui | `📄 Sao chép` | `📄 Copy` |
| `cmd_cut` | ui | `✂️ Cắt` | `✂️ Cut` |
| `cmd_paste` | ui | `📋 Dán` | `📋 Paste` |

> **Tổng cộng: 26 + 10 + 10 + 12 + 22 = **80 `lang_id`**.

### 2.5 Quy tắc bắt buộc

1. **Không hardcode text** trong UI — mọi chuỗi hiển thị phải qua `t(key, lang)`.
2. **Không gọi `t()` trong lúc render** — gọi 1 lần rồi gán `textContent`; tránh gọi lại mỗi frame.
3. **Extension**: thêm ngôn ngữ = thêm 1 file JSON mới (`src/i18n/xx.json`) + đăng ký trong `dicts`. Không sửa code.
4. Key thiếu trong file → fallback `en` → raw key (dễ phát hiện lỗi khi QA).

---

## 3. Custom Themes

### 3.1 File format & vị trí

- **Format**: JSON thuần. Không chứa code, không eval.
- **Vị trí lưu** (app data dir):
  - Windows: `%APPDATA%/filen_gui/themes/*.json`
  - macOS: `~/Library/Application Support/filen_gui/themes/*.json`
  - Linux: `~/.config/filen_gui/themes/*.json`
- Mỗi file = 1 theme. Tên file = tên theme (slug).

### 3.2 Schema đầy đủ

```json
{
  "name": "My Neon Theme",
  "version": "1.0.0",
  "tokens": {
    "colors.surface.canvas": "#0A0D14",
    "colors.surface.card": "#141A2C",
    "colors.neon.cyan": "#00FFCC",
    "colors.text.primary": "#F0F4FC",
    "typography.size.base": "13px",
    "spacing.md": "8px",
    "radius.md": "4px",
    "effects.glow.active.cyan": "0 0 15px rgba(0,255,204,0.8)",
    "zIndex.drawer": "100"
  }
}
```

- `name` (string, bắt buộc), `version` (string, bắt buộc), `tokens` (object, bắt buộc).
- `tokens` là bản đồ `{ token_name: value }` — **chỉ ghi key muốn đổi**, phần còn lại fallback master.

### 3.3 Nhóm token được phép (khớp `design-tokens.md`)

| Nhóm | Token mẫu | Ghi chú |
|---|---|---|
| `colors.surface.*` | `colors.surface.canvas`, `colors.surface.card`, `colors.surface.glass`, `colors.surface.header`, `colors.surface.input`, `colors.surface.button.primary`, `colors.surface.navtab.active`, `colors.surface.table.selected`, `colors.surface.dropzone.copy`, `colors.border.muted`, `colors.border.error` | nền/khung/border |
| `colors.neon.*` | `colors.neon.cyan`, `colors.neon.magenta`, `colors.neon.purple`, `colors.neon.emerald`, `colors.neon.coral`, `colors.neon.amber`, `colors.neon.errorText`, `colors.neon.cutText` | accent |
| `colors.text.*` | `colors.text.primary`, `colors.text.secondary`, `colors.text.muted`, `colors.text.onNeon`, `colors.text.onNeonHover` | chữ |
| `typography.*` | `typography.family.*`, `typography.size.*`, `typography.weight.*`, `typography.lineHeight.*`, `typography.tracking.*`, `typography.role.*` | font/size/weight |
| `spacing.*` | `spacing.0` … `spacing.3xl` | khoảng cách |
| `radius.*` | `radius.none` … `radius.pill` | bo góc |
| `effects.*` | `effects.shadow.*`, `effects.glow.*`, `effects.transition.*` | glow/shadow |
| `zIndex.*` | `zIndex.base` … `zIndex.tooltip` | xếp lớp |

> Tên token phải **khớp chính xác** tên trong `design-tokens.md` (master). Không tự đặt tên mới.

### 3.5 Merge/override 3 tầng

Thứ tự ưu tiên (tầng sau đè tầng trước):

1. **Default theme** (master tokens từ `design-tokens.md`).
2. **User theme** (file `themes/*.json` đang active).
3. **Runtime tweaks** (thay đổi tạm thời trong phiên, ví dụ user kéo slider brightness — không ghi file).

Kết quả cuối = `default` ⊕ `user` ⊕ `runtime`. Key thiếu ở tầng sau → giữ giá trị tầng trước.

### 3.6 Validation

Khi load theme file (Rust backend):

1. **Kiểu dữ liệu**: `name`/`version` phải string; `tokens` phải object; mỗi value phải string.
2. **Range hex**: màu phải khớp regex `^#([0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})$` (hỗ trợ alpha 8-digit) hoặc `^#RRGGBB@0.\d+$` (alpha hậu tố). Không hợp lệ → bỏ qua key + warning.
3. **Unknown key**: key không tồn tại trong schema master → **bỏ qua + warning**, không làm hỏng theme.
4. **Reference**: value `@token` phải trỏ tới token tồn tại; không trỏ vào reference khác (tránh vòng lặp).
5. **Kích thước file**: giới hạn ≤ 256 KB — vượt quá → từ chối.

**Fallback**: nếu file lỗi (parse fail, thiếu `name`/`version`, vượt kích thước) → **bỏ qua theme đó, dùng default theme**. Không crash app.

### 3.7 Security risk analysis

File theme **không trusted** (user tự tải về):

- **Không `eval`/`exec`** bất kỳ nội dung nào từ file — chỉ `JSON.parse` (Rust `serde_json`).
- **Chỉ parse JSON**: không cho phép function, không cho phép key bắt đầu bằng `on*` (tránh XSS qua event handler).
- **Giới hạn kích thước file** (256 KB) — chống DoS qua file khổng lồ.
- **Hex regex** — value màu phải khớp regex nghiêm ngặt; không chấp nhận `url(...)`, `expression(...)`, `javascript:` — chống CSS injection.
- **Không ghi đè file hệ thống** — theme chỉ đọc từ thư mục `themes/` riêng.
- **Sanitize `name`** trước khi render (escape HTML) — tránh XSS qua tên theme.

### 3.8 Performance

- **Hot-reload qua fs watcher** (Rust `notify` crate) theo dõi thư mục `themes/`.
- Khi file đổi → re-parse + validate → **chỉ cập nhật CSS variables** trên `:root` (setProperty), **không rebuild DOM**, không reload WebView.
- Chi phí layout thấp: đổi CSS var chỉ trigger repaint/reflow cục bộ, không re-render toàn bộ cây.

---

## 4. Cơ chế kỹ thuật — CSS custom properties

- Mỗi design token map **1-1** với CSS var `--token-name` (dấu `.` → `-`):
  - `colors.surface.canvas` → `--colors-surface-canvas`
  - `colors.neon.cyan` → `--colors-neon-cyan`
  - `typography.size.base` → `--typography-size-base`
- **Theme file → JSON.parse → setProperty trên `:root`**:

```js
function applyTheme(tokens) {
  const root = document.documentElement.style;
  for (const [token, value] of Object.entries(tokens)) {
    root.setProperty(`--${token.replace(/\./g, "-")}`, value);
  }
}
```

- CSS dùng `var(--colors-surface-canvas)` ở mọi nơi — đổi theme chỉ cần đổi var, không sửa CSS.
- Runtime tweaks cũng qua `setProperty` — cùng cơ chế, tầng 3.

---

## 5. Tóm tắt

- **i18n**: 80 `lang_id`, lookup `t(key, lang)` + fallback en → default, hot-switch qua `data-lang-id`, thêm ngôn ngữ = thêm file JSON.
- **Themes**: JSON file `{ name, version, tokens }`, 8 nhóm token, merge 3 tầng (default → user → runtime), validation nghiêm ngặt, fallback default khi lỗi, security (không eval, giới hạn size, hex regex), hot-reload qua fs watcher chỉ đổi CSS vars.
- **Cơ chế**: CSS custom properties map 1-1 token; theme → `JSON.parse` → `setProperty` trên `:root`.

---
*End of i18n & Themes spec.*