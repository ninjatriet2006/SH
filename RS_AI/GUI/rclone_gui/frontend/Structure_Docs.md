[Pattern Docs]
# KIẾN TRÚC THƯ MỤC FRONTEND
- **components/**: Chứa các thành phần UI (chia theo Pane, Modal, v.v.).
- **features/**: Chứa các tính năng phức tạp như Quản lý tiến trình (transferManager), Chuột phải (contextMenu), v.v.
- **services/**: Chứa các module giao tiếp Backend và File (fileOps, trashOps, undoManager, v.v.).

# CẤU TRÚC DỮ LIỆU (DATA STRUCTURES - INTERFACES)

## store.ts
- **Tên cấu trúc (Interface)**: FileItem
- **Mô tả**: Định nghĩa một đối tượng File hoặc Folder trả về từ Backend.
- **Thuộc tính**:
  - `uuid: string` (Bắt buộc)
  - `name: string` (Bắt buộc)
  - `is_dir: boolean` (Bắt buộc)
  - `size: number` (Bắt buộc)
  - `mod_time: string` (Bắt buộc)
  - `file_type: string | null` (Tùy chọn)
  - `owner: string` (Tùy chọn)
  - `group: string` (Tùy chọn)
  - `permissions: string` (Tùy chọn)

- **Tên cấu trúc (Interface)**: ActivityItem
- **Mô tả**: Định nghĩa một bản ghi trong hộp thoại lịch sử (Activity Log).
- **Thuộc tính**:
  - `id: string` (Bắt buộc)
  - `timestamp: number` (Bắt buộc)
  - `action: string` (Bắt buộc)
  - `details: string` (Bắt buộc)

- **Tên cấu trúc (Interface)**: AppState
- **Mô tả**: Trạng thái toàn cục (Global State) của ứng dụng Frontend.
- **Thuộc tính**: 
  - `auth`, `explorer`, `activityLog`, `bookmarks`, `settings` (Tùy chọn)

## services/undoManager.ts
- **Tên cấu trúc (Interface)**: UndoAction
- **Mô tả**: Lưu trữ trạng thái để phục vụ tính năng Hoàn tác (Undo) và Làm lại (Redo).
- **Thuộc tính**:
  - `type: UndoActionType` (Bắt buộc)
  - `src: string` (Bắt buộc)
  - `dest: string` (Bắt buộc)
  - `account: string` (Tùy chọn)
  - `isLocal: boolean` (Bắt buộc)

# TÀI LIỆU HÀM (API DOCS)

# services/fileOps.ts
- **Tên hàm**: parseRemotePath
- **Mô tả**: Tách chuỗi dạng "Remote::/path" thành tên remote và đường dẫn thực.
- **Tham số đầu vào**: `fullPath: string` (Bắt buộc)
- **Đầu ra**: `{ remote: string, realPath: string }`

- **Tên hàm**: runWithSudoFallback
- **Mô tả**: Hàm tiện ích bọc các lời gọi API để tự động phát hiện lỗi Permission Denied và hiện hộp thoại xin quyền Sudo (Admin) trên Local.
- **Tham số đầu vào**: `action: string` (Bắt buộc), `args: string[]` (Bắt buộc), `remote: string` (Bắt buộc), `fn: () => Promise<T>` (Bắt buộc)
- **Đầu ra**: `Promise<T>`

- **Tên hàm**: copy
- **Mô tả**: Thực thi tác vụ copy file/thư mục. Hỗ trợ fallback Sudo nếu copy nội bộ Local.
- **Tham số đầu vào**: `src: string` (Bắt buộc), `dest: string` (Bắt buộc), `_account: string` (Tùy chọn), `taskId: number` (Tùy chọn)
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: move
- **Mô tả**: Thực thi tác vụ di chuyển (move) file/thư mục. Hỗ trợ fallback Sudo nếu copy nội bộ Local.
- **Tham số đầu vào**: `src: string` (Bắt buộc), `dest: string` (Bắt buộc), `_account: string` (Tùy chọn), `taskId: number` (Tùy chọn)
- **Đầu ra**: `Promise<void>`

# services/trashOps.ts
- **Tên hàm**: listRemoteTrash
- **Mô tả**: Liệt kê các file nằm trong thùng rác của ổ đĩa đám mây (Cloud).
- **Tham số đầu vào**: `account: string` (Tùy chọn)
- **Đầu ra**: `Promise<FileItem[]>`

- **Tên hàm**: restoreRemoteTrash
- **Mô tả**: Khôi phục một file cụ thể từ thùng rác đám mây.
- **Tham số đầu vào**: `idx: number` (Bắt buộc), `account: string` (Tùy chọn)
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: listLocalTrash
- **Mô tả**: Liệt kê các file trong thùng rác cục bộ của hệ điều hành.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Promise<TrashItemLocal[]>`

# services/actionStore.ts
- **Tên hàm**: fetchActions
- **Mô tả**: Lấy danh sách các lệnh tùy chỉnh (Custom Actions) từ Backend.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: getValidActionsForSelection
- **Mô tả**: Lọc danh sách các action hợp lệ để hiển thị trên menu ngữ cảnh dựa vào file đang chọn.
- **Tham số đầu vào**: `files: FileItem[]` (Bắt buộc)
- **Đầu ra**: `CustomAction[]`

- **Tên hàm**: executeAction
- **Mô tả**: Gửi lệnh thực thi Action kèm tham số đường dẫn xuống Backend.
- **Tham số đầu vào**: `action: CustomAction` (Bắt buộc), `files: FileItem[]` (Bắt buộc), `basePath: string` (Bắt buộc)
- **Đầu ra**: `Promise<void>`

# features/transferManager.ts
- **Tên hàm**: enqueue
- **Mô tả**: Đẩy một tác vụ truyền tải (copy/move/delete) vào hàng đợi. Tự động kiểm tra fallback đối với Move trên Cloud không được hỗ trợ.
- **Tham số đầu vào**: `kind: TransferKind` (Bắt buộc), `name: string` (Bắt buộc), `src: string` (Bắt buộc), `dst: string` (Bắt buộc), `onSuccess: () => void` (Tùy chọn)
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: processQueue
- **Mô tả**: (Private) Vòng lặp chính xử lý các tác vụ trong hàng đợi tuần tự.
- **Tham số đầu vào**: Không có
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: cancel
- **Mô tả**: Hủy một tác vụ đang chạy thông qua ID.
- **Tham số đầu vào**: `id: number` (Bắt buộc)
- **Đầu ra**: `Promise<void>`
