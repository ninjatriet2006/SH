# plan-phase-8.md — Phase 8: Verify toàn diện + smoke-test + independence

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 8.1 | cargo check + clippy filen_gui toàn bộ | - | 1 | rust-dev | [x] |
| 8.2 | cargo test filen_gui đầy đủ | 8.1 | 1 | tester | [x] |
| 8.3 | check/clippy/test filen_tui không hỏng | - | 1 | rust-dev | [x] |
| 8.4 | cargo tree: filen_gui không dep TUI | - | 1 | rust-dev | [x] |
| 8.5 | cargo tree: 5 GUI không dep lẫn nhau | - | 1 | rust-dev | [x] |
| 8.6 | Smoke-test local: list/mkdir/rm | 8.1 | 2 | tester | [x] |
| 8.7 | Smoke-test cloud: upload/download roundtrip, cancel/timeout | 8.6 | 2 | tester | [x] |
| 8.8 | Smoke-test login/logout nếu có account | 8.6 | 2 | tester | [x] |
| 8.9 | Báo cáo phần cần user xác nhận | 8.7,8.8 | 2 | docs | [x] |
| 8.10 | Đối chiếu plan.md tính năng đủ chưa | 8.2,8.3,8.9 | 3 | docs | [x] |
| 8.11 | Fix rm --no-trash: gửi xác nhận y qua stdin | 8.7 | 1 | rust-dev | [x] |
| 8.12 | Fix write multi-line: escape/gọi đúng cách | 8.7 | 1 | rust-dev | [x] |
| 8.13 | Fix progress khi pipe: chạy CLI qua pty/script | 8.7 | 2 | rust-dev | [x] |
| 8.14 | Verify lại 3 case lỗi (cloud thật) | 8.11,8.12,8.13 | 1 | tester | [x] |
