# plan-phase-7.md — Phase 7: UI tìm kiếm + thao tác file + tính năng phase 6

Mục tiêu: UI đầy đủ cho filter, file ops, recents, view, export-notes, sync, servers panel.

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 7.1 | UI ô tìm kiếm mỗi pane + lọc client-side | - | 1 | rust-dev | [ ] |
| 7.2 | Popup mkdir: nút toolbar + nhập tên | - | 1 | rust-dev | [ ] |
| 7.3 | Popup rename file/dir | - | 1 | rust-dev | [ ] |
| 7.4 | Confirm popup delete, tùy chọn --no-trash | - | 1 | rust-dev | [ ] |
| 7.5 | Nút favorite/unfavorite gọi ops async | - | 2 | rust-dev | [ ] |
| 7.6 | View file: popup cat nội dung | - | 2 | rust-dev | [ ] |
| 7.7 | Copy link: links create + ghi clipboard | - | 2 | rust-dev | [ ] |
| 7.8 | Context menu chuột phải gom thao tác file | 7.2-7.7 | 2 | rust-dev | [ ] |
| 7.9 | Recents panel hiển thị file gần đây | - | 2 | rust-dev | [ ] |
| 7.10 | View URL: mở clipboard/web | - | 2 | rust-dev | [ ] |
| 7.11 | Export-notes: chọn path + chạy | - | 3 | rust-dev | [ ] |
| 7.12 | Sync: đọc syncPairs.json + nút chạy | - | 2 | rust-dev | [ ] |
| 7.13 | Servers panel: WebDAV/S3/Mount form + Start/Stop | - | 3 | rust-dev | [ ] |
| 7.14 | Servers logs hiển thị | 7.13 | 3 | rust-dev | [ ] |
| 7.15 | Tích hợp thread/mpsc, refresh pane, tiếng Việt | 7.1-7.14 | 1 | rust-dev | [ ] |
