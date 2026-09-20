# Plan: Backend Job Queue cho rclone_gui (S2)

Mục tiêu: hàng chờ việc chuyển xuống backend (Rust), `actions/*` cắm vào worker,
frontend thành lớp xem mỏng. Hết mất hàng khi tải lại, hết tiến trình mồ côi.

## Current status
- Active: HOÀN TẤT P1-P4 (2026-09-20); owner: Lead
- Passed: cargo check pass, test lib 85/85, clippy lib 0 warning,
  IPC cũ nguyên vẹn (66 commands, trừ read/write đã bỏ theo quyết định view-only)
- Open: build frontend chưa chạy được (thiếu node_modules); build release chưa chạy
- Next gate: build frontend + smoke khi có môi trường; lúc đó mới xóa IPC cũ

## Phase
- P1: `core/jobs.rs` — Job{id,kind,params,status,progress}, store RAM+JSON đĩa,
  worker tuần tự gọi actions execute_*, event `job_update`, IPC enqueue/list/cancel.
  IPC cũ giữ nguyên (trial song song).
- P2: actions còn lại cắm worker + bước điểm danh (bóc thư mục → vé từng món).
- P3: transferManager mỏng theo backend (subscribe event), dialog quyền/fallback
  đi qua queue, bỏ snapshot localStorage.
- P4: Check (Lead tự check, model Check DEAD) + FULL test + docs.

## Quy tắc
- 1 chủ/file/batch; implement → test → check tuần tự.
- IPC cũ chỉ xóa khi P3 xong và P4 pass.
