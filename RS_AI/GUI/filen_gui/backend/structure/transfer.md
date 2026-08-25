# transfer.rs
Tài liệu tham chiếu các hàm quản lý hàng đợi truyền tải.

- **Tên hàm**: `TransferManager::enqueue` / `cancel`
- **Mô tả**: Quản lý hàng đợi (Queue), hỗ trợ chạy nhiều transfer cùng lúc.
- **Tham số đầu vào**: 
  - `kind: TransferKind` (Bắt buộc)
  - `name: String` (Bắt buộc)
  - `src: String` (Bắt buộc)
  - `dst: String` (Bắt buộc)
  - `src_local: bool` (Bắt buộc)
  - `dst_local: bool` (Bắt buộc)
  - `cleanup_src: bool` (Bắt buộc)
  - `src_pane: usize` (Bắt buộc)
  - `dst_pane: usize` (Bắt buộc)
- **Đầu ra**: `usize` (Task ID).

- **Tên hàm**: `run_cli_transfer_terminal`
- **Mô tả**: Chạy transfer Upload/Download thực sự thông qua terminal CLI (có parse tiến trình progress bar).
- **Tham số đầu vào**:
  - `kind: TransferKind` (Bắt buộc)
  - `src: &str` (Bắt buộc)
  - `dst: &str` (Bắt buộc)
  - `timeout_secs: u64` (Bắt buộc)
  - `cancelled: Arc<AtomicBool>` (Bắt buộc)
  - `on_update: impl FnMut(ProgressUpdate)` (Bắt buộc)
- **Đầu ra**: `Result<(), TransferError>`

- **Tên hàm**: `copy_local`, `move_local`, `delete_local_path`
- **Mô tả**: Các thao tác nội bộ (Local to Local) hỗ trợ luồng truyền tải không qua CLI.
- **Tham số đầu vào**:
  - `src: &str` (Bắt buộc)
  - `dst: &str` (Bắt buộc đối với copy/move)
- **Đầu ra**: `Result<(), String>`
