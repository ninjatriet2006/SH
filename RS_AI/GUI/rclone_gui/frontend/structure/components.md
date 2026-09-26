[Pattern Docs]
# components.md

Tài liệu cấu trúc các thành phần giao diện tái sử dụng (Components & Modals) của hệ thống Rclone GUI.

- **Tên Component**: `Sidebar`
- **Mô tả**: Thanh điều hướng chính của ứng dụng, hiển thị các liên kết định tuyến và huy hiệu số lượng công việc đang xử lý.

- **Tên Component**: `Breadcrumbs`
- **Mô tả**: Thanh điều hướng đường dẫn thư mục, cho phép nhấp vào từng cấp thư mục hoặc nhấp đúp để gõ đường dẫn tự do.

- **Tên Component**: `FileTable`
- **Mô tả**: Bảng danh sách tệp tin và thư mục, hiển thị biểu tượng theo định dạng file, hỗ trợ chọn đơn / chọn nhiều với Ctrl/Shift, đổi thứ tự sắp xếp theo cột và nhấp đúp để duyệt/mở file.

- **Tên Component**: `DualPaneExplorer`
- **Mô tả**: Khung trình duyệt hai cửa sổ (Pane Trái / Phải), tích hợp thanh công cụ thao tác nhanh: tạo thư mục, tạo file, sao chép/di chuyển sang pane đối diện, xoá, mở terminal.

- **Tên Component**: `ConflictModal`
- **Mô tả**: Hộp thoại cảnh báo và xử lý xung đột tệp tin khi sao chép/di chuyển tệp đã tồn tại, cho phép người dùng chọn bỏ qua các tệp chỉ định hoặc ghi đè.

- **Tên Component**: `PropertiesModal`
- **Mô tả**: Hộp thoại hiển thị thông số chi tiết (kích thước, số tệp/thư mục con, quyền POSIX) và hỗ trợ cập nhật quyền chmod/chown cho file cục bộ.

- **Tên Component**: `SearchModal`
- **Mô tả**: Hộp thoại tìm kiếm sâu đệ quy theo từ khoá trong toàn bộ nhánh cây thư mục, hỗ trợ nhấp đúp để điều hướng tức thì tới tệp kết quả.

- **Tên Component**: `CreateRemoteModal`
- **Mô tả**: Hộp thoại cấu hình remote đám mây mới, hiển thị các trường nhập liệu tương ứng theo từng Cloud Provider.

- **Tên Component**: `CreateMountModal`
- **Mô tả**: Hộp thoại cấu hình và tạo dịch vụ Systemd rclone mount (User hoặc System unit, VFS cache mode).
