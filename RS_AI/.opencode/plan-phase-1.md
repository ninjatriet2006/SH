# plan-phase-1.md — Phase 1: Tách independence TUI/GUI

Mục tiêu: gỡ dep filen_tui khỏi filen_gui, copy code xử lý sang GUI, giữ TUI nguyên vẹn.

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 1.1 | Khảo sát Cargo.toml + main.rs + source TUI | - | 1 | lead (đã làm) | [x] |
| 1.2 | Gỡ dep filen_tui khỏi Cargo.toml GUI | 1.1 | 1 | rust-dev | [ ] |
| 1.3 | Copy operations.rs (FileItem/Operations/helpers/tests) sang GUI | 1.1 | 1 | rust-dev | [ ] |
| 1.4 | Copy get_default_data_dir sang GUI | 1.1 | 1 | rust-dev | [ ] |
| 1.5 | Thêm deps tokio/serde/serde_json/chrono/dirs/which | 1.1 | 1 | rust-dev | [ ] |
| 1.6 | Đăng ký module + sửa imports trong main.rs | 1.3,1.4,1.5 | 2 | rust-dev | [ ] |
| 1.7 | Verify check/clippy/test -p filen_gui | 1.2,1.5,1.6 | 2 | tester | [ ] |

Lưu ý: main.rs hiện dùng `filen_tui::app::operations::{FileItem, Operations}` → đổi sang module local.
