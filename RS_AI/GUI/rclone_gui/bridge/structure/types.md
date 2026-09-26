[Pattern Docs]
# types.ts

Tài liệu cấu trúc các Interface và Type TypeScript định nghĩa dữ liệu để giao tiếp với Rust backend thông qua chuẩn Enveloped IPC Pattern (A.1 Contract).

- **Tên cấu trúc**: `Req<T>`
- **Mô tả**: Vỏ bọc bao thư gửi xuống Backend (schema_version: 1, request_id, payload: T).

- **Tên cấu trúc**: `Res<T>`
- **Mô tả**: Vỏ bọc bao thư phản hồi thành công từ Backend (schema_version: 1, request_id, data: T).

- **Tên cấu trúc**: `IpcError`
- **Mô tả**: Lỗi chuẩn hoá trả về từ Backend gồm `code: IpcErrorCode`, `message: string`, `retryable: boolean`, `details: JsonValue | null`.

- **Tên cấu trúc**: `FileItem`
- **Mô tả**: Giao diện dữ liệu mô tả tệp tin / thư mục trong hệ thống hoặc cloud remote.
- **Thuộc tính**:
  - `uuid: string`
  - `name: string`
  - `size: number`
  - `is_dir: boolean`
  - `mod_time: string`
  - `file_type: string | null`

- **Tên cấu trúc**: `Job`
- **Mô tả**: Snapshot của một tiến trình việc trong hàng đợi (Job Queue).
- **Thuộc tính**:
  - `id: string`
  - `kind: JobKind`
  - `src: string | null`
  - `dst: string | null`
  - `status: JobStatus`
  - `progress: number`
  - `error: string | null`
  - `child_done: number`
  - `child_total: number`
  - `policy: PermissionPolicy`
  - `skip_paths: string[]`
  - `skipped: number`

- **Tên cấu trúc**: `RemoteConfig`
- **Mô tả**: Cấu hình của một remote đám mây trong file `rclone.conf`.

- **Tên cấu trúc**: `MountConfig`
- **Mô tả**: Cấu hình dịch vụ mount Systemd của rclone.

- **Tên cấu trúc**: `EngineSettings`
- **Mô tả**: Tuỳ chỉnh hiệu năng rclone engine (transfers, checkers, fast_list, backup_dir...).

- **Tên cấu trúc**: `AppearanceSettings`
- **Mô tả**: Cấu hình giao diện (ngôn ngữ, theme, font).
