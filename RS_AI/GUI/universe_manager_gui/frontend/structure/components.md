[Pattern Docs]
# components.md

Tài liệu cấu trúc các thành phần giao diện tái sử dụng (Components) của hệ thống Universe Manager GUI.

- **Tên Component**: `AppTable`
- **Mô tả**: Bảng danh sách ứng dụng quản lý hoặc tùy chỉnh, hiển thị cột tên, phiên bản, loại ứng dụng, nguồn, ID và các nút thao tác nhanh (Khởi động / Dừng).

- **Tên Component**: `DetectionReportView`
- **Mô tả**: Khung hiển thị chi tiết kết quả phân tích và nhận diện cấu trúc ứng dụng từ thư mục nguồn (AppImage, tệp thực thi, biểu tượng icon, mẫu tệp desktop).

- **Tên Component**: `AppLayout` (trong `App.tsx`)
- **Mô tả**: Khung bao toàn cục ứng dụng với sidebar điều hướng, header hiển thị trạng thái tác vụ nền (job loading indicator/error banner), tích hợp Lucide icons và router.
