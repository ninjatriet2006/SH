# sync.rs
Tài liệu tham chiếu các hàm đồng bộ hóa (Sync).

- **Tên hàm**: `sync_pairs`
- **Mô tả**: Đọc danh sách cấu hình đồng bộ từ file JSON (không dùng CLI).
- **Tham số đầu vào**: Không có.
- **Đầu ra**: `Result<Vec<SyncPair>, String>`

- **Tên hàm**: `sync_terminal`
- **Mô tả**: Chạy đồng bộ hóa cho nhiều cặp thư mục, hỗ trợ chạy liên tục (continuous).
- **Tham số đầu vào**:
  - `active_account: &Option<String>` (Tùy chọn)
  - `locations: &[String]` (Bắt buộc)
  - `continuous: bool` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `sync_once_terminal`
- **Mô tả**: Chạy đồng bộ 1 lần cho 1 cặp (local:remote).
- **Tham số đầu vào**:
  - `active_account: &Option<String>` (Tùy chọn)
  - `local: &str` (Bắt buộc)
  - `remote: &str` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `sync_pair_once_terminal`
- **Mô tả**: Chạy đồng bộ 1 lần từ object SyncPair.
- **Tham số đầu vào**:
  - `active_account: &Option<String>` (Tùy chọn)
  - `pair: &SyncPair` (Bắt buộc)
- **Đầu ra**: `Result<(), String>`
