# plan-phase-2.md — Phase 2: Khung dual-pane explorer kiểu Air Explorer

Mục tiêu: thay layout 3-tab cũ bằng khung explorer 2 pane (local/cloud), sidebar, toolbar, status bar. Chưa tích hợp ops thật (phase 3). Tất cả trong apps_gui/filen_gui/src/main.rs (+ có thể tách module UI nếu cần).

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 2.1 | Định nghĩa AppState/PaneState (mode local/cloud, active pane) | - | 1 | rust-dev | [ ] |
| 2.2 | Dựng khung egui panels thay layout 3-tab | - | 1 | rust-dev | [ ] |
| 2.3 | Xây sidebar trái (chức năng/tài khoản placeholder) | 2.2 | 2 | rust-dev | [ ] |
| 2.4 | Xây central 2 pane + ScrollArea danh sách file | 2.1,2.2 | 1 | rust-dev | [ ] |
| 2.5 | Xây toolbar chuyển hướng → ← copy/move | 2.4 | 2 | rust-dev | [ ] |
| 2.6 | Xây status bar dưới + giữ log panel | 2.2 | 2 | rust-dev | [ ] |
| 2.7 | Xử lý focus/đổi mode giữa 2 pane | 2.4 | 2 | rust-dev | [ ] |
| 2.8 | Nhãn UI tiếng Việt + kiểm tra font fallback | 2.5,2.6 | 3 | rust-dev | [ ] |
| 2.9 | Verify cargo check + clippy + test | 2.7,2.8 | 3 | tester | [ ] |
