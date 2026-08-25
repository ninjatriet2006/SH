# sys/desktop_apps.rs
Tài liệu tham chiếu các hàm tìm kiếm ứng dụng hệ thống trên Linux.

- **Tên struct**: `DesktopApp`
- **Mô tả**: Chứa thông tin của một ứng dụng (`name`, `exec`, `icon`, `mime_types`) để xây dựng menu "Open With".

- **Tên hàm**: `get_desktop_apps`
- **Mô tả**: Quét các thư mục chuẩn XDG (`/usr/share/applications` và `~/.local/share/applications`) để lấy danh sách app hỗ trợ.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Vec<DesktopApp>` (Đã lọc trùng và sắp xếp)

- **Tên hàm**: `parse_desktop_file`
- **Mô tả**: (Hàm nội bộ) Đọc file `.desktop`, lấy các khóa `Name`, `Exec`, `MimeType` và dọn dẹp các ký tự giữ chỗ (placeholder) như `%f`, `%U` trong lệnh `Exec`.
- **Tham số đầu vào**: `path: &PathBuf`
- **Đầu ra**: `Option<DesktopApp>`
