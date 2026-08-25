# sys/unix.rs
Tài liệu các hàm chuyên dụng trên môi trường Unix (Linux/macOS).

- **Tên hàm**: `copy_to_clipboard`
- **Mô tả**: Sao chép chuỗi vào Clipboard bằng cách ưu tiên gọi lần lượt `wl-copy`, `xclip`, hoặc `xsel`.
- **Tham số đầu vào**: `text: &str`
- **Đầu ra**: `Result<(), String>`

- **Tên hàm**: `get_interactive_command` / `get_interactive_tokio_command`
- **Mô tả**: Bọc tiến trình (process) qua `stdbuf -o0 -e0` (nếu có) để triệt tiêu bộ đệm output, hỗ trợ việc đọc tiến trình tương tác chuẩn thời gian thực.
- **Tham số đầu vào**: `bin: &Path` hoặc `tokio::process::Command`
- **Đầu ra**: `Command`

- **Tên hàm**: `mount_fuse_note`
- **Mô tả**: Trả về hướng dẫn cài đặt FUSE tùy thuộc là macOS hay Linux.
- **Đầu ra**: `String`

- **Tên hàm**: `default_filen_bin_name` / `scan_local_bins`
- **Mô tả**: Tìm kiếm fallback nhị phân `filen` trong các thư mục cài đặt mặc định trên môi trường Unix (`~/.filen-cli/bin/filen`).
