# sys/windows.rs
Tài liệu các hàm chuyên dụng trên môi trường Windows.

- **Tên hàm**: `copy_to_clipboard`
- **Mô tả**: Sao chép chuỗi vào Clipboard qua command hệ thống `clip`.
- **Tham số đầu vào**: `text: &str`
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `get_interactive_command` / `get_interactive_tokio_command`
- **Mô tả**: Trả thẳng đối tượng Command (Không can thiệp chống buffer vì Windows không có cơ chế `stdbuf` mặc định).
- **Tham số đầu vào**: `bin: &Path` hoặc `tokio::process::Command`
- **Đầu ra**: `Command`

- **Tên hàm**: `mount_fuse_note`
- **Mô tả**: Trả về hướng dẫn cài đặt WinFSP/WinFUSE.
- **Đầu ra**: `String`

- **Tên hàm**: `default_filen_bin_name` / `scan_local_bins`
- **Mô tả**: Tìm kiếm fallback cho `filen.cmd` hoặc `filen.exe` bao gồm cả npm global trong `%APPDATA%`.
