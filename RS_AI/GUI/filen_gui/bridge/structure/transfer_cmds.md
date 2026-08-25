# transfer_cmds.rs
Tài liệu tham chiếu các Tauri commands liên quan đến hàng đợi truyền tải (Transfer).

- **Tên hàm**: `transfer_enqueue`
- **Mô tả**: Đưa một tác vụ vào hàng đợi truyền tải (chưa khởi chạy ngay).
- **Tham số đầu vào**:
  - `state: tauri::State<'_, AppState>` (Bắt buộc)
  - `kind: String` (Bắt buộc)
  - `name: String` (Bắt buộc)
  - `src: String` (Bắt buộc)
  - `dst: String` (Bắt buộc)
  - `src_local: bool` (Bắt buộc)
  - `dst_local: bool` (Bắt buộc)
  - `cleanup_src: bool` (Bắt buộc)
  - `src_pane: usize` (Bắt buộc)
  - `dst_pane: usize` (Bắt buộc)
- **Đầu ra**: `Result<usize, String>` (trả về ID của tác vụ)

- **Tên hàm**: `transfer_start`
- **Mô tả**: Đánh thức và khởi chạy các tác vụ trong hàng đợi (giới hạn số luồng bằng max_concurrent). Mỗi tác vụ tạo một luồng ảo riêng.
- **Tham số đầu vào**:
  - `app: tauri::AppHandle` (Bắt buộc)
  - `state: tauri::State<'_, AppState>` (Bắt buộc)
  - `account: Option<String>` (Tùy chọn)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `transfer_cancel`
- **Mô tả**: Hủy một tác vụ thông qua ID của nó.
- **Tham số đầu vào**:
  - `state: tauri::State<'_, AppState>` (Bắt buộc)
  - `id: usize` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `transfer_cancel_all`
- **Mô tả**: Hủy toàn bộ tác vụ.
- **Tham số đầu vào**:
  - `state: tauri::State<'_, AppState>` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `transfer_remove_finished`
- **Mô tả**: Dọn dẹp danh sách các tác vụ đã hoàn thành hoặc bị hủy khỏi UI.
- **Tham số đầu vào**:
  - `state: tauri::State<'_, AppState>` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm nội bộ**: `run_transfer_worker`
- **Mô tả**: Hàm async Core chạy nền để thực hiện tải, copy, move. Gọi `emit` cập nhật tiến trình liên tục lên cho UI (`transfer:progress`) và trạng thái hoàn tất (`transfer:finished`).
