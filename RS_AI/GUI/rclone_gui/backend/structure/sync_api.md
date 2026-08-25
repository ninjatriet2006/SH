[Pattern Docs]
# sync_api.md

- **Tên hàm**: `sync_pair`
- **Mô tả**: Đồng bộ nội dung giữa 2 thư mục (Local to Cloud, Cloud to Cloud, Cloud to Local). Tương đương tính năng SyncPairsView của filen_gui.
- **Tham số đầu vào**:
  - `source_path` (Bắt buộc): Nguồn.
  - `dest_path` (Bắt buộc): Đích.
  - `sync_mode` (Bắt buộc): enum (sync, copy, move).
  - `bandwidth_limit` (Tùy chọn): Giới hạn băng thông (VD: 10M).
  - `filters` (Tùy chọn): Mảng các filter loại trừ hoặc bao gồm (VD: `--exclude *.tmp`).
- **Đầu ra**: JobID (để track bất đồng bộ trên giao diện).

- **Tên hàm**: `dedupe_path`
- **Mô tả**: Gọi lệnh rclone dedupe để tìm và xử lý các file trùng lặp (tính năng chuyên sâu của rclone/ModularTUI).
- **Tham số đầu vào**:
  - `remote_path` (Bắt buộc): Đường dẫn thư mục cần dedupe.
  - `dedupe_mode` (Tùy chọn): interactive, skip, first, newest, oldest, rename, largest, smallest. (Mặc định: skip).
- **Đầu ra**: JobID hoặc Log kết quả xử lý dedupe.
