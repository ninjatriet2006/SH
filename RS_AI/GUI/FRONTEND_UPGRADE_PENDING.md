# Frontend Standards — PENDING UPGRADE

> Cập nhật ngày 2026-10-01.

## Chuẩn mới

Core: React `^19.2.8` + Vite `^8.2.0` + TS `~6.0.2` + Tauri `^2.11.1` +
`tsc -b` + `engines: node ^20.19.0 || >=22.12.0` +
`lucide-react` + `react-router-dom` + `zustand` + `@tauri-apps/plugin-dialog` + `oxlint`.

| Repo | Ghi chú |
|---|---|
| `account_hub_gui` | đủ chuẩn mới |
| `opencode_manager_gui` | đủ chuẩn mới (thiếu plugin-dialog) |
| `rclone_gui` | đủ chuẩn mới, nhưng `version: 1.0.0` lạc loài |
| `subscription_manager_gui` | đủ chuẩn mới — dùng làm **reference** |
| `universal_api` | đủ chuẩn mới (dialog `^2.7.0`) |
| `gateway_filter` | core mới nhưng thiếu router/zustand/dialog/oxlint/engines, có tailwind |
| `universe_manager_gui` | ✅ **rewrite xong** — React + glassmorphism, xóa egui legacy |

## Chuẩn cũ (CẦN nâng cấp — chưa làm)

| Repo | Hiện trạng | Hướng đề xuất |
|---|---|---|
| `filen_gui` | vanilla TS, 56 file, Tauri `^2.0.0` + drag plugin | chỉ nâng toolchain, giữ code |
| `img_splt_gui` | vanilla TS, 7 file, pin cứng | rewrite React theo chuẩn mới |
| `universal_converter_gui` | vanilla TS, 7 file, pin cứng | rewrite React theo chuẩn mới |

Đặc điểm chung nhóm cũ: không React, `@tauri-apps/api` pin cứng `2.11.1`,
`typescript 6.0.3` + `vite 8.2.0` pin cứng, script build `tsc` (không `-b`), thiếu `engines`.
