# plan-phase-4.md — Phase 4: Sidebar tài khoản + login/logout + statfs

Mục tiêu: nối account thật vào GUI: load accounts, login (2FA), logout, statfs, active account.

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|----|-------|-----------|---------|-------|------------|
| 4.1 | Thêm AccountConfig/StoredAccount + load/save accounts vào GUI | - | 1 | rust-dev | [ ] |
| 4.2 | Mở rộng AsyncResult/mpsc cho sự kiện tài khoản | - | 1 | rust-dev | [ ] |
| 4.3 | Thay AccountState placeholder bằng state thật | 4.2 | 1 | rust-dev | [ ] |
| 4.4 | Sidebar: danh sách tài khoản + chọn active | 4.1,4.3 | 1 | rust-dev | [ ] |
| 4.5 | Login form + luồng login_new async (2FA) | 4.3 | 2 | rust-dev | [ ] |
| 4.6 | Luồng logout async + xóa khỏi danh sách | 4.4,4.5 | 2 | rust-dev | [ ] |
| 4.7 | Luồng statfs async + hiển thị used/max | 4.3 | 2 | rust-dev | [ ] |
| 4.8 | Nối active_account() thật + reload pane cloud | 4.4,4.5 | 2 | rust-dev | [ ] |
| 4.9 | whoami lúc khởi động nhận active session | 4.1 | 3 | rust-dev | [ ] |
| 4.10 | Verify cargo check/clippy/test | 4.4-4.9 | 3 | tester | [ ] |

Ghi chú: account file — tham khảo cách TUI load AccountConfig (có thể dùng serde_yaml hoặc JSON; nếu chưa có serde_yaml trong GUI thì dùng JSON qua serde_json đã có).
