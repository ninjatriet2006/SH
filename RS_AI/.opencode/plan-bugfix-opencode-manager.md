# plan-bugfix-opencode-manager.md — Fix: Provider xoá model nhưng TUI không xoá khỏi config

## Mục tiêu
Sửa bug trong `TUI/opencode_manager`: khi provider xoá một model khỏi server, TUI không phát hiện
và không xoá model đó khỏi `provider.models` trong config.

## Nguyên nhân gốc (đã verify trong code)
1. `App::handle_message(AppMessage::Scan)` (app.rs) build `scanned_models` CHỈ từ danh sách model provider trả về.
2. Model trong config nhưng không còn trên provider → không xuất hiện trong `scanned_models` → vô hình.
3. `add_scanned_models()`: nhánh xoá (`else`) chỉ chạy trên model CÓ trong `scanned_models` mà unchecked;
   nhưng model như vậy thì không tồn tại trong config (`contains_key` = false) → không bao giờ xoá.
   → `removed_count` không bao giờ > 0. Config tích luỹ model chết vĩnh viễn.

## Giải pháp
Khi scan xong: tính `stale = config_models − fetched_models`, thêm vào `scanned_models`
dạng `(id, checked=false, stale=true)` (xuống cuối danh sách, hiển thị cảnh báo đỏ).
User có thể Space để giữ lại (checked) hoặc Enter để đồng bộ → model stale bị xoá khỏi config.
Cơ chế checkbox/unchecked hiện có của `add_scanned_models` tự xử lý phần xoá.

## Subtasks
| id | mô tả | phụ thuộc | ưu tiên | trạng thái |
|----|-------|-----------|---------|------------|
| 1 | app.rs: mở rộng `scanned_models` thành (id, checked, stale); Scan handler phát hiện stale | - | 1 | [x] |
| 2 | app.rs: cập nhật `filtered_scanned_models` + vòng lặp `add_scanned_models` | 1 | 1 | [x] |
| 3 | ui.rs: hiển thị cảnh báo đỏ cho model stale trong models modal | 1 | 1 | [x] |
| 4 | main.rs: cập nhật pattern destructuring (4 phần tử) | 1 | 1 | [x] |
| 5 | Test regression: scan phát hiện stale + sync xoá stale khỏi config (+ fix race env test) | 1-4 | 1 | [x] |
| 6 | Verify: cargo check / test / clippy | 5 | 1 | [x] |

## Kết quả verify (2026-08-02)
- `cargo check -p opencode_manager` → OK
- `cargo test -p opencode_manager` → 3/3 passed (kể cả chế độ song song, đã fix race env bằng TEST_ENV_LOCK)
- `cargo clippy -p opencode_manager --all-targets` → chỉ còn 1 warning CÓ SẴN `new_without_default` ở api.rs (ngoài phạm vi, không đụng)

## VERIFY
`cargo check -p opencode_manager && cargo test -p opencode_manager && cargo clippy -p opencode_manager`
