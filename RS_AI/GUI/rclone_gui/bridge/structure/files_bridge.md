[Pattern Docs]
# files_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri liên quan đến duyệt, thao tác file và thư mục tại tầng Bridge.

- **Tên hàm**: `listFiles`
- **Mô tả**: Liệt kê tệp và thư mục theo đường dẫn (hỗ trợ cả Local và Remote::path).
- **Tham số đầu vào**: `path: string`, `pane?: string | null`
- **Đầu ra**: `Promise<FileItem[]>`

- **Tên hàm**: `checkConflicts`
- **Mô tả**: Kiểm tra xung đột trước khi sao chép nhiều nguồn vào thư mục đích.
- **Tham số đầu vào**: `srcs: string[]`, `destPath: string`
- **Đầu ra**: `Promise<ConflictInfo[]>`

- **Tên hàm**: `checkRenameConflict`
- **Mô tả**: Kiểm tra xung đột tên trước khi đổi tên file.
- **Tham số đầu vào**: `oldPath: string`, `newPath: string`
- **Đầu ra**: `Promise<ConflictInfo | null>`

- **Tên hàm**: `statAdvanced`
- **Mô tả**: Lấy thống kê chi tiết của một đường dẫn (dung lượng, số file/thư mục con, quyền POSIX).
- **Tham số đầu vào**: `path: string`
- **Đầu ra**: `Promise<StatInfo | null>`

- **Tên hàm**: `searchFiles`
- **Mô tả**: Tìm kiếm tệp theo từ khoá trong cây thư mục.
- **Tham số đầu vào**: `path: string`, `query: string`
- **Đầu ra**: `Promise<SearchResultItem[]>`

- **Tên hàm**: `getHomeDir`
- **Mô tả**: Lấy đường dẫn thư mục Home của người dùng.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Promise<string>`

- **Tên hàm**: `getUserPlaces`
- **Mô tả**: Lấy danh sách các thư mục chuẩn hệ thống (Home, Desktop, Downloads...).
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Promise<UserPlace[]>`

- **Tên hàm**: `openInTerminal`
- **Mô tả**: Khởi chạy ứng dụng terminal tại thư mục chỉ định.
- **Tham số đầu vào**: `path: string`
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `getThumbnail`
- **Mô tả**: Tạo hoặc lấy ảnh thu nhỏ (thumbnail) cho ảnh, video hoặc PDF.
- **Tham số đầu vào**: `path: string`
- **Đầu ra**: `Promise<string | null>`

- **Tên hàm**: `chmodPath` / `chownPath`
- **Mô tả**: Thay đổi quyền (mode) và chủ sở hữu POSIX (chỉ áp dụng ổ Local).
- **Tham số đầu vào**: `path: string`, `mode/uid/gid: number`
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `makeDir` / `touchFile` / `deletePath` / `renamePath`
- **Mô tả**: Các thao tác thay đổi file đưa vào hàng đợi Job Queue.
- **Tham số đầu vào**: `path: string` hoặc `oldPath: string, newPath: string`
- **Đầu ra**: `Promise<Job>`
