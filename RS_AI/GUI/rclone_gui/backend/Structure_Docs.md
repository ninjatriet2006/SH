[Pattern Docs]
# KIẾN TRÚC THƯ MỤC BACKEND

Backend theo kiến trúc 3 tầng, phụ thuộc một chiều `api → logic → core`:

- **src/lib.rs**: Khởi tạo `AppState`, đăng ký plugin và toàn bộ Tauri command.
- **src/main.rs**: Entry point, chỉ gọi `rclone_gui_lib::run()`.
- **src/api/**: Tầng API — nhận request từ Frontend, không chứa logic phức tạp.
  - `files.rs`: Liệt kê, tạo, xoá, đổi tên, copy/move, tìm kiếm, thumbnail.
  - `remotes.rs`: Quản lý cấu hình remote (create/update/delete), truy vấn features.
  - `mount.rs`: Tạo và điều khiển systemd service cho `rclone mount`.
  - `trash.rs`: Thùng rác Local (`gio trash`) và Cloud (`rclone cleanup`).
  - `config.rs`: Đọc/ghi/sắp xếp lại `rclone.conf`.
- **src/logic/**: Tầng nghiệp vụ.
  - `app_state.rs`: Trạng thái toàn cục — map `task_id → PID` để hỗ trợ Hủy.
  - `file_ops.rs`: Bóc tách đường dẫn, sudo fallback, kiểm tra xung đột.
  - `transfer.rs`: Chạy tiến trình truyền tải, bóc tách tiến độ, hủy tác vụ.
- **src/core/**: Tầng giao tiếp hệ điều hành.
  - `rclone.rs`: Dựng và thực thi lệnh `rclone`.
  - `sys.rs`: Mở file bằng ứng dụng OS, clipboard, custom action.

# QUY ƯỚC ĐƯỜNG DẪN

Frontend luôn gửi xuống đường dẫn dạng **`Remote::/path`** (ví dụ `GDrive::/Docs`,
`Local::/home/user`). Backend bóc tách bằng `logic::file_ops::parse_remote_path`
rồi dựng lại theo cú pháp rclone bằng `core::rclone::build_target`:

- `Local` → đường dẫn hệ thống nguyên bản (`/home/user`).
- Remote khác → `Name:path` (`GDrive:/Docs`).

# CẤU TRÚC DỮ LIỆU (DATA STRUCTURES - STRUCTS)

## api/files.rs
- **FileItem** — một file/folder trả về cho Frontend.
  - `uuid: String`, `name: String`, `size: i64`, `is_dir: bool`,
    `mod_time: String`, `file_type: Option<String>`
- **StatInfo** — thống kê chi tiết một đường dẫn.
  - `size: u64`, `file_count: u64`, `dir_count: u64`,
    `permissions: u32`, `uid: u32`, `gid: u32`
  - Ghi chú: `permissions`/`uid`/`gid` chỉ có giá trị thật trên ổ `Local` (Unix);
    trên remote cloud luôn là 0.
- **ConflictInfo** — một xung đột tên khi copy/move.
  - `relative_path: String`, `src_full_path: String`, `dest_full_path: String`
- **SearchResultItem** — `item: FileItem`, `path: String` (path dạng `Remote::/...`).

## api/mount.rs
- **MountConfig** — cấu hình một systemd mount service.
  - `service_name`, `is_user_level`, `remote_name`, `remote_path`, `mount_path`,
    `description`, `vfs_cache_mode`, `vfs_cache_max_size`, `vfs_cache_max_age`,
    `dir_cache_time`, `buffer_size`, `allow_other`, `read_only`
- **SystemdServiceInfo** — `name`, `is_user`, `status`, `enabled`

## api/trash.rs
- **TrashItemLocal** — `id`, `name`, `original_path`, `time_deleted`

## core/sys.rs
- **DesktopApp** — `name`, `exec`, `icon`
- **CustomAction** — `id`, `name`, `exec`, `icon`, `selection`, `extensions`
- **OSClipboardItem** / **OSClipboardData** — `items`, `is_cut`

## logic/app_state.rs
- **AppState** — `pids: Mutex<HashMap<u32, u32>>` (task_id → PID)

# TÀI LIỆU HÀM (API DOCS)

## api/files.rs
- **list_files**(`path: String`) → `Result<Vec<FileItem>, String>`
  Liệt kê file/thư mục ở cấp 1 của `path`. Thư mục xếp trước file.
- **fs_mkdir**(`path: String`) → `Result<(), String>` — có sudo fallback trên Local.
- **fs_touch**(`path: String`) → `Result<(), String>`
- **fs_delete**(`path: String`) → `Result<(), String>`
  Thử `purge`; nếu target là file thì tự chuyển sang `deletefile`.
- **fs_rename**(`old_path: String`, `new_path: String`) → `Result<(), String>`
- **fs_copy**(`src`, `dst`, `task_id: Option<u32>`) → `Result<(), String>`
  Dùng `copyto` (đúng cho cả file và thư mục vì Frontend đã gộp basename vào `dst`).
  Nếu thất bại do thiếu quyền và cả hai đầu là Local → thử lại qua `pkexec cp -r`.
- **fs_move**(`src`, `dst`, `task_id: Option<u32>`) → `Result<(), String>` — tương tự với `moveto`.
- **fs_cancel**(`task_id: u32`) → `Result<(), String>`
- **fs_stat_advanced**(`path: String`) → `Result<StatInfo, String>`
- **fs_search**(`path: String`, `query: String`) → `Result<Vec<SearchResultItem>, String>`
  Tìm đệ quy theo tên (`lsjson -R --include *query*`).
- **fs_check_conflicts**(`srcs: Vec<String>`, `dest_path: String`) → `Result<Vec<ConflictInfo>, String>`
- **get_home_dir**() → `Result<String, String>` — trả về Desktop nếu tồn tại, ngược lại `$HOME`.
- **open_in_terminal**(`path: String`) → `Result<(), String>`
- **fs_get_thumbnail**(`path: String`) → `Result<String, String>` — data URI base64.
- **fs_temp_dir**() → `String`
- **fs_chmod**(`path: String`, `mode: u32`) → `Result<(), String>`
  Chỉ ổ Local (Unix). Giữ 12 bit quyền; có sudo fallback qua `pkexec chmod`.
- **fs_chown**(`path: String`, `uid: u32`, `gid: u32`) → `Result<(), String>`
  Chỉ ổ Local (Linux). Luôn đi qua `pkexec chown` vì cần quyền root.

## api/remotes.rs
- **list_remotes**() → `Result<Vec<Value>, String>` — từ `rclone config dump`, sắp xếp A-Z.
- **get_providers**() → `Result<String, String>` — JSON thô của `rclone config providers`.
- **create_remote**(`name`, `provider`, `options: HashMap<String,String>`) → `Result<String, String>`
- **update_remote**(`name`, `options`) → `Result<String, String>`
- **delete_remote**(`name`) → `Result<String, String>`
- **get_backend_features**(`remote: String`) → `Result<Value, String>`
- **check_transfer_capability**(`src`, `dst`) → `Result<Value, String>`
  Trả `{ canMove, canCopy, canCopyDelete }` để Frontend quyết định fallback.
- **rclone_about**(`remote`) → `Result<Value, String>`
- **rclone_size**(`remote`) → `Result<Value, String>`

## api/mount.rs
- **check_fuse_installed**() → `Result<bool, String>`
- **create_mount_service**(`config: MountConfig`) → `Result<String, String>`
  Service cấp hệ thống ghi qua `pkexec`; cấp user ghi thẳng vào `~/.config/systemd/user`.
- **delete_mount_service**(`service_name`, `is_user`) → `Result<String, String>`
- **manage_mount_service**(`service_name`, `is_user`, `action`) → `Result<String, String>`
- **list_mount_services**() → `Result<Vec<SystemdServiceInfo>, String>`
- **get_mount_service_config**(`service_name`, `is_user`) → `Result<MountConfig, String>`

## api/trash.rs
- **fs_trash_list_local**() → `Result<Vec<TrashItemLocal>, String>` — *chưa triển khai, trả rỗng.*
- **fs_trash_restore_local**(`item_id`) → `Result<(), String>` — *chưa triển khai.*
- **fs_trash_empty_local**() → `Result<(), String>` — `gio trash --empty`, kiểm tra exit code.
- **fs_trash_list_remote_terminal**(`account`) → `Result<Vec<FileItem>, String>` — *chưa triển khai.*
- **fs_trash_restore_remote_terminal**(`account`, `idx`) → *chưa triển khai.*
- **fs_trash_delete_remote_terminal**(`account`, `idx`) → *chưa triển khai.*
- **fs_trash_empty_remote_terminal**(`account`) → `Result<(), String>` — `rclone cleanup`.

## api/config.rs
- **get_config_content**() → `Result<String, String>`
- **set_config_content**(`content: String`) → `Result<(), String>`
- **reorder_config**(`names: Vec<String>`) → `Result<(), String>`

## core/sys.rs
- **sys_open_with**(`path`, `exec_cmd: Option<String>`, `app: Option<String>`) → `Result<(), String>`
  Chỉ hỗ trợ ổ `Local`. Tách lệnh theo cú pháp shell rồi `exec` trực tiếp
  (KHÔNG qua `sh -c`) để tên file không thể chèn thêm lệnh. Hỗ trợ placeholder
  `%f`/`%F`/`%u`/`%U` của Desktop Entry.
- **sys_list_apps**() → `Result<Vec<DesktopApp>, String>` — hiện chỉ trả `xdg-open`.
- **os_clipboard_set**(`items`, `is_cut`) / **os_clipboard_get**() — qua file JSON trong temp dir.
- **sys_get_custom_actions**() → `Result<Vec<CustomAction>, String>` — hiện trả rỗng.
- **sys_get_valid_actions**(`files`) → `Result<Vec<CustomAction>, String>`
- **sys_execute_custom_action**(`exec_template`, `base_path`, `file_names`) → `Result<(), String>`
  Chạy qua `sh -c` (mẫu lệnh do người dùng định nghĩa), nhưng mọi đường dẫn được
  bọc nháy đơn an toàn bằng `shell_quote`.

## logic/file_ops.rs
- **parse_remote_path**(`full_path: &str`) → `(String, String)` — tách `Remote::/path`.
  Không có `::` → mặc định remote là `Local`.
- **run_with_sudo_fallback**(`remote`, `action`, `args`, `fallback_cmd`) → `Result<(), String>`
  Chạy closure; nếu lỗi Permission Denied trên Local (Linux) thì thử lại qua `pkexec`.
  Action được hỗ trợ: `rm`, `mkdir`, `mv`, `cp`, `chmod`.
- **check_conflicts**(`srcs`, `dest_path`) → `Result<Vec<ConflictInfo>, String>`
  Dùng `lsjson --stat` để xác định kiểu của chính target (không phải của file con).
  Nếu cả nguồn và đích là thư mục → quét đệ quy tìm file con trùng đường dẫn.

## logic/transfer.rs
- **run_transfer_task**(`app_handle`, `state`, `cmd_name`, `src`, `dst`, `task_id`) → `Result<(), String>`
  Chạy rclone với `--use-json-log --stats 0.5s`, đọc stderr theo dòng, phát
  event `transfer_progress` (`{ id, stats }`) lên Frontend. Lưu PID vào `AppState`.
- **cancel_transfer**(`state`, `task_id`) → `Result<(), String>`
  Gửi **SIGTERM** để rclone tự dọn file `.partial`, chờ tối đa 5 s trên thread
  nền rồi mới SIGKILL. SIGKILL trực tiếp sẽ bỏ lại file rác ở thư mục đích.

## core/rclone.rs
- **build_target**(`remote`, `path`) → `String`
- **run_cmd**(`args`) → `Result<Output, String>` — đồng bộ.
- **spawn_cmd**(`args`) → `Result<(), String>` — chạy ngầm; tiến trình con được
  reap bởi thread nền để không để lại zombie.

# SỰ KIỆN PHÁT LÊN FRONTEND
- `transfer_progress` — payload `{ id: u32, stats: {...} }`, phát từ `logic/transfer.rs`.

# QUYỀN (CAPABILITIES)
`capabilities/main.json` cấp đúng mức tối thiểu:
- `core:default`, `core:event:default` — nền tảng + event.
- `core:window:allow-close` — cho MenuBar → File → Thoát.
- `shell:allow-open` — cho double-click mở file Local qua `plugin-shell`.
