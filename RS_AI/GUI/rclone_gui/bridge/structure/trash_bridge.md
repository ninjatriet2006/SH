[Pattern Docs]
# trash_bridge.ts

Tài liệu cấu trúc các hàm gọi Tauri liên quan đến Thùng rác Local và Cloud.

- **Tên hàm**: `listTrashLocal` / `emptyTrashLocal`
- **Mô tả**: Liệt kê hoặc dọn sạch thùng rác cục bộ trên hệ điều hành.
- **Đầu ra**: `Promise<TrashItemLocal[]>` / `Promise<void>`

- **Tên hàm**: `restoreTrashLocal` / `deleteTrashLocal`
- **Mô tả**: Khôi phục hoặc xoá vĩnh viễn một mục theo ID trong thùng rác cục bộ.
- **Tham số đầu vào**: `itemId: string`
- **Đầu ra**: `Promise<void>`

- **Tên hàm**: `listTrashRemote` / `emptyTrashRemote`
- **Mô tả**: Liệt kê hoặc dọn sạch thùng rác trên ổ Remote đám mây.
- **Tham số đầu vào**: `account: string`
- **Đầu ra**: `Promise<FileItem[]>` / `Promise<void>`

- **Tên hàm**: `restoreTrashRemote` / `deleteTrashRemote`
- **Mô tả**: Khôi phục hoặc xoá vĩnh viễn một mục trên thùng rác đám mây.
- **Tham số đầu vào**: `account: string`, `path: string`
- **Đầu ra**: `Promise<void>`
