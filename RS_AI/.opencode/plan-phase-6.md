# plan-phase-6.md — Phase 6: Bổ sung ops thiếu từ audit backend + tests

Mục tiêu: thêm ops vào apps_gui/filen_gui/src/operations.rs theo audit filen help; test đầy đủ. Chưa cần UI.

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 6.1 | head+tail ops parse -n mặc định 10, tests | - | 1 | rust-dev | [ ] |
| 6.2 | stat op --json parse chi tiết, tests | - | 1 | rust-dev | [ ] |
| 6.3 | write op content + edge case, tests | - | 1 | rust-dev | [ ] |
| 6.4 | recents op format ls --long, tests | - | 1 | rust-dev | [ ] |
| 6.5 | view+export-notes ops log URL, tests | - | 2 | rust-dev | [ ] |
| 6.6 | sync op đọc syncPairs.json data-dir, tests | - | 2 | rust-dev | [ ] |
| 6.7 | Tham khảo TUI WebDavServerState/S3ServerState (CHỈ đọc) | - | 2 | rust-dev | [ ] |
| 6.8 | webdav/webdav-proxy server child state, tests | 6.7 | 2 | rust-dev | [ ] |
| 6.9 | s3 server child state, tests | 6.7 | 2 | rust-dev | [ ] |
| 6.10 | mount op FUSE note/child, tests | - | 3 | rust-dev | [ ] |
| 6.11 | Clippy sạch + tổng hợp test phase 6 | 6.1-6.10 | 3 | rust-dev | [ ] |

Lưu ý: KHÔNG sửa/sao chép trực tiếp từ TUI — chỉ tham khảo cấu trúc state; mọi code viết mới trong filen_gui.
