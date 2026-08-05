<!-- INTEGRITY NOTES: MASTER design tokens for filen_gui v3.0.0 (framework-agnostic). -->
<!-- Purpose: Replace discrete hex tables in neon_ui_design.md (2.1-2.3) with one semantic token set; every value overridable by custom_themes. -->
<!-- Source of truth for defaults: neon_ui_design.md sections 2.1-2.3 + component states in 2.4. -->
<!-- Sub-doc #2 of docs v3.0.0 set — hub: docs/neon_ui_design.md. -->

# `filen_gui` Design Tokens — MASTER List

**Version**: 3.0.0
**Status**: chốt v3.0.0 — token schema cho `custom_themes` merge/override
**Nguồn kế thừa**: `docs/neon_ui_design.md` (mục 2.1–2.3, 2.4)

---

## 1. Token schema (hợp đồng)

- **Tên token**: chuỗi `group.subgroup.name` — viết `snake_case`, không dấu.
- **Value**: primitive string (`#RRGGBB`, `px`, `s`, v.v.) hoặc **token reference** `@group.name` (tham chiếu chéo tới token khác — giúp override 1 chỗ lan toả).
- **Alpha**: hex kèm hậu tố `#RRGGBB@0.85` cho màu có độ trong suốt.
- **Merge/override**: `custom_themes` chỉ cần cung cấp bản đồ khớp key — các key thiếu sẽ fallback về master. Key lạ bị bỏ qua (cảnh báo, không fail). Xem §6.

---

## 2. Group: `colors.surface.*` — nền, khung, border

| Token | Mô tả | Default |
|---|---|---|
| `colors.surface.canvas` | Nền gốc window (obsidian) | `#080A10` |
| `colors.surface.card` | Panel/container/pane/table | `#121624` |
| `colors.surface.glass` | Popup/modal/dropdown nổi (kính) | `#161C2E@0.85` |
| `colors.surface.header` | Header bar / header row | `#1C2338` |
| `colors.surface.input` | Nền input field | `#0E1422` |
| `colors.surface.input.hover` | Input hover | `#12192B` |
| `colors.surface.input.focus` | Input focus | `#141C30` |
| `colors.surface.input.disabled` | Input disabled | `#0A0D16` |
| `colors.surface.button.primary` | Primary button default | `#121A2E` |
| `colors.surface.button.primary.hover` | Primary button hover | `#1A2845` |
| `colors.surface.button.primary.active` | Primary button active (fill) | `@colors.neon.cyan` |
| `colors.surface.button.ghost.hover` | Ghost button / nav hover | `#162035` |
| `colors.surface.button.ghost.active` | Ghost button active | `#1E2D4A` |
| `colors.surface.button.disabled` | Button disabled | `#0E121E` |
| `colors.surface.navtab.active` | Nav tab selected | `#1C2842` |
| `colors.surface.table.selected` | Explorer row selected | `#1E2D4A` |
| `colors.surface.table.cut` | Explorer row cut (clipboard) | `#2A1628` |
| `colors.surface.code.pill` | Code/path/key pill background | `#0A0D14` |
| `colors.surface.dropzone.copy` | Dropzone overlay (copy) | `rgba(18,31,56,0.75)` |
| `colors.surface.dropzone.move` | Dropzone overlay (move) | `rgba(42,22,40,0.75)` |
| `colors.border.muted` | Border mặc định / divider | `#222C42` |
| `colors.border.input.disabled` | Input border disabled | `#161C2E` |
| `colors.border.error` | Border lỗi | `@colors.neon.coral` |

> \* Giá trị dropzone dùng `rgba(r,g,b,0.75)` hợp lệ CSS (giữ nguyên màu gốc `#121F38`/`#2A1628`); hex kèm alpha hậu tố `#RRGGBB@0.75` không phải CSS hợp lệ nên không dùng cho token này — xem ghi chú §7.

## 3. Group: `colors.neon.*` — accent phát sáng

| Token | Mô tả | Default |
|---|---|---|
| `colors.neon.cyan` | Primary brand / active / button chính | `#00F3FF` |
| `colors.neon.magenta` | Secondary accent / cut highlight / alert | `#FF007F` |
| `colors.neon.purple` | Cloud indicator / section header / server card | `#9D00FF` |
| `colors.neon.emerald` | Success / active server / transfer done | `#00FF87` |
| `colors.neon.coral` | Delete / error banner / server stopped | `#FF3366` |
| `colors.neon.amber` | Warning / in-progress / sync pending | `#FFB800` |
| `colors.neon.errorText` | Text trên nền error | `#FF809B` |
| `colors.neon.cutText` | Text row đang cut | `#FF80BF` |

## 4. Group: `colors.text.*` — chữ

| Token | Mô tả | Default |
|---|---|---|
| `colors.text.primary` | Text chính / filename | `#F0F4FC` |
| `colors.text.secondary` | Metadata / subtext / icon inactive | `#94A3B8` |
| `colors.text.muted` | Placeholder / disabled / hotkey hint | `#475569` |
| `colors.text.onNeon` | Text trên fill neon (active button) | `#080A10` |
| `colors.text.onNeonHover` | Text trên neon khi hover | `#FFFFFF` |

## 5. Group: `typography.*`

### 5.1 `typography.family.*`
| Token | Mô tả | Default |
|---|---|---|
| `typography.family.proportional` | Font UI (sans; fallback tiếng Việt) | `Noto Sans, Roboto, DejaVu Sans` |
| `typography.family.monospace` | Font code/path/hash/hotkey | `JetBrains Mono, DejaVu Sans Mono, monospace` |

### 5.2 `typography.size.*`
| Token | Default |
|---|---|
| `typography.size.xs` | `10px` |
| `typography.size.sm` | `11px` |
| `typography.size.md` | `12px` |
| `typography.size.base` | `13px` |
| `typography.size.lg` | `14px` |
| `typography.size.xl` | `18px` |

### 5.3 `typography.weight.*`
| Token | Default |
|---|---|
| `typography.weight.regular` | `400` |
| `typography.weight.semibold` | `600` |
| `typography.weight.bold` | `700` |

### 5.4 `typography.lineHeight.*`
| Token | Default |
|---|---|
| `typography.lineHeight.tight` | `14px` |
| `typography.lineHeight.normal` | `16px` |
| `typography.lineHeight.body` | `18px` |
| `typography.lineHeight.header` | `20px` |
| `typography.lineHeight.title` | `24px` |

### 5.5 `typography.tracking.*`
| Token | Default |
|---|---|
| `typography.tracking.none` | `0px` |
| `typography.tracking.section` | `0.5px` |
| `typography.tracking.otp` | `4px` |

### 5.6 `typography.role.*` — cấu hình soạn sẵn theo hierarchy (từ 2.3)
| Token | Cấu hình (tham chiếu) |
|---|---|
| `typography.role.appTitle` | `family: proportional, size: xl, weight: bold, lineHeight: title, color: neon.cyan, shadow: shadow.text.title` |
| `typography.role.sectionHeader` | `family: proportional, size: lg, weight: semibold, lineHeight: header, color: neon.purple, uppercase: true, tracking: section` |
| `typography.role.body` | `family: proportional, size: base, weight: regular, lineHeight: body, color: text.primary` |
| `typography.role.metadata` | `family: proportional, size: sm, weight: regular, lineHeight: normal, color: text.secondary` |
| `typography.role.code` | `family: monospace, size: md, weight: regular, lineHeight: normal, color: neon.cyan, bg: surface.code.pill` |
| `typography.role.badge` | `family: monospace, size: xs, weight: bold, lineHeight: tight, color: neon.emerald, glow: glow.emerald.capsule` |

## 6. Group: `spacing.*`
| Token | Default |
|---|---|
| `spacing.0` | `0px` |
| `spacing.xxs` | `2px` |
| `spacing.xs` | `4px` |
| `spacing.sm` | `6px` |
| `spacing.md` | `8px` |
| `spacing.lg` | `12px` |
| `spacing.xl` | `16px` |
| `spacing.xxl` | `24px` |
| `spacing.3xl` | `32px` |

## 7. Group: `radius.*`
| Token | Mô tả | Default |
|---|---|---|
| `radius.none` | Không bo | `0px` |
| `radius.sm` | Input, ô nhỏ | `2px` |
| `radius.md` | Card / table | `4px` |
| `radius.lg` | Sync pair card, modal | `6px` |
| `radius.xl` | Panel lớn | `8px` |
| `radius.pill` | Capsule / badge | `999px` |

## 8. Group: `effects.*` — glow, shadow, transition

### 8.1 `effects.shadow.*` (outer drop shadow, từ 2.2)
| Token | Mô tả | Default |
|---|---|---|
| `effects.shadow.glow.cyan` | Primary active glow | `0 0 12px @colors.neon.cyan@0.45` |
| `effects.shadow.glow.cyan.hover` | Hover glow pulse | `0 0 8px @colors.neon.cyan@0.30` |
| `effects.shadow.glow.magenta` | Cut/alert glow | `0 0 14px @colors.neon.magenta@0.50` |
| `effects.shadow.glow.emerald` | Success/active server | `0 0 10px @colors.neon.emerald@0.40` |
| `effects.shadow.glow.coral` | Danger/delete | `0 0 15px @colors.neon.coral@0.60` |
| `effects.shadow.text.title` | Text shadow app title | `0 0 8px @colors.neon.cyan@0.60` |

### 8.2 `effects.glow.*` (inner + focus/active trạng thái)
| Token | Mô tả | Default |
|---|---|---|
| `effects.glow.inner.cyan` | Inner ambient glow | `inset 0 0 6px @colors.neon.cyan@0.20` |
| `effects.glow.active.cyan` | Active button glow | `0 0 15px @colors.neon.cyan@0.80` |
| `effects.glow.focus.cyan` | Focused control | `0 0 12px @colors.neon.cyan@0.60` |
| `effects.glow.selected.cyan` | Selected row | `0 0 15px @colors.neon.cyan@0.70` |
| `effects.glow.ghost.cyan` | Ghost button active | `0 0 10px @colors.neon.cyan@0.50` |
| `effects.glow.ghost.hover` | Ghost button hover | `0 0 6px @colors.neon.cyan@0.20` |
| `effects.glow.ghost.selected` | Ghost selected (purple) | `0 0 8px @colors.neon.purple@0.40` |
| `effects.glow.emerald.capsule` | Badge capsule glow | `0 0 20px @colors.neon.emerald@0.60` |
| `effects.glow.magenta.cut` | Cut row glow | `0 0 8px @colors.neon.magenta@0.35` |
| `effects.glow.emerald.anchor` | Focus anchor row | `0 0 6px @colors.neon.emerald@0.30` |
| `effects.glow.input.error` | Input error glow | `0 0 10px @colors.neon.coral@0.50` |

### 8.3 `effects.transition.*`
| Token | Default |
|---|---|
| `effects.transition.duration.fast` | `0.10s` |
| `effects.transition.duration.hover` | `0.15s` |
| `effects.transition.easing.linear` | `linear` |

## 9. Group: `zIndex.*`
| Token | Mô tả | Default |
|---|---|---|
| `zIndex.base` | Nội dung thường | `0` |
| `zIndex.pane` | Pane/panel | `1` |
| `zIndex.drawer` | Transfer drawer | `100` |
| `zIndex.modal` | Modal/command palette | `200` |
| `zIndex.overlay` | Overlay mờ phía sau modal | `300` |
| `zIndex.dropzone` | Dropzone highlight | `400` |
| `zIndex.tooltip` | Tooltip | `500` |

---

## 10. Quy tắc override (`custom_themes`)

1. Theme override là bản đồ `{ token_name: value }`; **chỉ ghi key muốn đổi** — phần còn lại dùng master.
2. Value hợp lệ: literal (`#RRGGBB`, `13px`, `400`) hoặc reference `@token` (chuỗi token cùng schema).
3. Theme **bắt buộc** định nghĩa đủ các token hiển thị nền + chữ để tránh mất tương phản:
   - `colors.surface.canvas`, `colors.surface.card`, `colors.surface.glass`, `colors.surface.header`
   - `colors.text.primary`, `colors.text.secondary`, `colors.text.muted`
   - toàn bộ `typography.role.*`
   - Các token còn lại tùy chọn, fallback master.
4. Key không tồn tại trong schema → cảnh báo lúc load, không làm hỏng theme.
5. Reference phải trỏ tới token tồn tại (không trỏ vào reference khác để tránh vòng lặp).

## 11. Tổng token

**~76 token** (23 `colors.surface.*`/border, 8 `colors.neon.*`, 5 `colors.text.*`, 32 `typography.*`, 9 `spacing.*`, 6 `radius.*`, 17 `effects.*`, 7 `zIndex.*`). Chi tiết theo bảng trên là nguồn chốt cho `custom_themes` sau này.
