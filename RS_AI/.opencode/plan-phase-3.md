# plan-phase-3.md — Phase 3: Tích hợp Operations async

Mục tiêu: wire list_local/list_remote thật vào 2 pane qua thread + mpsc + tokio runtime riêng.

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 3.1 | Wire list_local async qua thread + mpsc cho pane Local | - | 1 | rust-dev | [ ] |
| 3.2 | Thêm list_remote async cho pane Cloud (runtime riêng, block_on, mpsc) | 3.1 | 1 | rust-dev | [ ] |
| 3.3 | Thêm loading/error status hiển thị riêng cho từng pane | 3.2 | 1 | rust-dev | [ ] |
| 3.4 | Điều hướng vào thư mục: click dir + nút ".." cập nhật path/items | 3.3 | 2 | rust-dev | [ ] |
| 3.5 | Path input editable + nút Home/List cho mỗi pane | 3.4 | 2 | rust-dev | [ ] |
| 3.6 | Verify: cargo check/clippy/test -p filen_gui | 3.5 | 3 | tester | [ ] |
