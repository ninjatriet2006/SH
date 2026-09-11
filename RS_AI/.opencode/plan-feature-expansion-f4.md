# Feature Phase F4 — Universe Applications rescan/verify

| id | công việc | phụ thuộc | agent | trạng thái |
|---|---|---|---|---|
| F4.1 | Kiểm kê nguồn và luồng rescan | F1 | explorer | [x] |
| F4.2 | Hợp nhất/dedup inventory Applications | F4.1 | rust-dev | [x] |
| F4.3 | Giữ record broken qua rescan | F4.2 | rust-dev | [x] |
| F4.4 | Triển khai domain rescan/verify | F4.2,F4.3 | rust-dev | [x] |
| F4.5 | Luôn rescan sau mọi kết quả apt | F4.4 | rust-dev | [x] |
| F4.6 | Thêm lệnh rescan Universe TUI | F4.4 | rust-dev | [x] |
| F4.7 | Hiển thị verify sau apt | F4.5 | rust-dev | [x] |
| F4.8 | Test hợp nhất inventory sources | F4.4 | tester | [x] |
| F4.9 | Test broken retention/no-delete | F4.3,F4.4 | tester | [x] |
| F4.10 | Test apt/update-rescan errors riêng | F4.5,F4.7 | tester | [x] |

GUI integration deferred sang GUI architecture Phase 7. F4 chỉ sửa domain/TUI Universe Manager;
không xóa/move app, desktop entry hoặc user data.
Inventory identity/dedup phải cover `~/Applications`, desktop user/system, Flatpak, Snap, Homebrew và
Windows hiện có. F4.8 test từng source/collision; F4.9 snapshot từng root trước/sau và giữ broken.
F4.5/F4.10 bắt buộc rescan chạy sau apt success/partial/failure; update result và rescan error được
báo riêng, không che nhau. F4.6 là manual TUI action, độc lập với post-apt rendering F4.7.

Kết quả: Reviewer APPROVE; 16/16 unique tests PASS (32 lượt lib+bin), fmt/check/clippy sạch.
Realistic parsers cover Desktop/Flatpak/Snap/Homebrew/Registry/Winget; source errors giữ record cũ
unverified; apt mọi outcome đều rescan; snapshot xác nhận không delete/move.
