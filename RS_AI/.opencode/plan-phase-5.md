# plan-phase-5.md — Phase 5: Transfer upload/download có progress

Mục tiêu: wrapper transfer spawn CLI child + pipe + parse progress + timeout/cancel; transfer manager; UI panel transfer; toolbar thật.

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 5.1 | Tạo struct TransferItem state src/dst/mode/status/cancel | - | 1 | rust-dev | [ ] |
| 5.2 | Runner spawn CLI child pipe stdout parse progress | - | 1 | rust-dev | [ ] |
| 5.3 | Thêm timeout + cancel kill child vào runner | 5.2 | 1 | rust-dev | [ ] |
| 5.4 | TransferManager queue + concurrent tối đa | 5.1,5.2 | 2 | rust-dev | [ ] |
| 5.5 | UI panel transfer danh sách progress bar cancel | 5.1,5.4 | 2 | rust-dev | [ ] |
| 5.6 | Toolbar thật copy/move 2 pane account nguồn | 5.4 | 2 | rust-dev | [ ] |
| 5.7 | Refresh pane sau transfer kết thúc | 5.6 | 2 | rust-dev | [ ] |
| 5.8 | Test runner timeout/cancel/parse progress | 5.3 | 3 | tester | [ ] |

Lưu ý: CLI filen upload/download có thể in progress ra stdout; nếu không, dùng % từ file size nếu có, ngược lại hiện indeterminate. Copy/move 2 pane: Local→Local dùng fs (std::fs::copy/rename), Cloud→Cloud dùng Operations cp/mv, Local↔Cloud dùng upload/download.
