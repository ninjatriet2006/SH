# Phase 2 — Contract bảy standalone Tauri v2 app

| id | mô tả | phụ thuộc | ưu tiên | agent | trạng thái |
|---|---|---|---|---|---|
| 2.1 | Chốt ma trận contract bảy ứng dụng | 1.9 | 1 | architect | [x] |
| 2.2 | Định nghĩa lớp vật lý và chiều phụ thuộc | 2.1 | 1 | architect | [x] |
| 2.3 | Kiểm kê command/event từng ứng dụng | 2.1 | 1 | explorer | [x] |
| 2.4 | Chuẩn hóa DTO và phiên bản IPC | 2.3 | 1 | rust-dev | [x] |
| 2.5 | Chuẩn hóa typed error và event lifecycle | 2.3,2.4 | 1 | rust-dev | [x] |
| 2.6 | Lập capability/permission tối thiểu từng app | 2.2,2.3 | 1 | reviewer | [x] |
| 2.7 | Chốt resource loader và package paths | 2.2 | 1 | architect | [x] |
| 2.8 | Chốt fallback và persistence resources | 2.7 | 1 | tester | [x] |
| 2.9 | Chốt project independence và artifact metadata | 2.2 | 1 | architect | [x] |
| 2.10 | Hoàn thiện contract và evidence matrix | 2.4-2.9 | 2 | docs | [x] |

## Quyết định đã khóa
- Cả bảy app dùng Tauri v2.
- Mỗi app sở hữu toàn bộ `frontend/`, `backend/`, `bridge/`, `langs/`, `themes/`, `fonts/`.
- Không tạo shared GUI package/component hoặc đường dẫn runtime chung; component giống nhau được copy.
- GUI được phép gọi TUI domain crate tương ứng, nhưng không được phụ thuộc GUI khác.

## Canonical plan
File này là plan Phase 2 canonical của roadmap GUI. Các file `.opencode/plan-phase-2.md` cũ thuộc
task khác và không áp dụng cho migration này.

## Physical layer và dependency direction
- Mỗi app có đúng các root `frontend/`, `backend/`, `bridge/`, `langs/`, `themes/`, `fonts/`.
- `frontend` chỉ gọi `invoke/listen` qua facade TypeScript nằm trong project; không import backend.
- `bridge` là crate Tauri v2 composition root, chỉ register commands/events, map DTO và gọi backend.
- `backend` là Rust crate không phụ thuộc Tauri; giữ domain/application/IO/persistence.
- Không cycle hoặc dependency đảo chiều. GUI-to-GUI dependency dưới mọi hình thức là FAIL.

## IPC, error và event schema
- Mọi request/response IPC là DTO serde + TypeScript tương ứng, có `schema_version: 1` khi payload
  được persist hoặc dùng cho event dài hạn; tên command và tham số giữ snake_case nhất quán.
- Boundary trả `Result<T, IpcError>`, với `IpcError { code, message, retryable, details }`; mọi lỗi
  command/event đều là `IpcError`, không có string error hoặc payload lỗi không định kiểu.
- Job event dùng `{ schema_version, job_id, seq, state, payload }`; lifecycle duy nhất:
  `started -> progress* -> completed | failed | cancelled`. Có đúng một terminal event, listener cleanup.

## Commands cần bảo toàn
- Filen: auth, local/cloud fs, transfer, system; events watcher và auth lifecycle.
- Rclone: explorer/file ops, remotes, mount, config, system; watcher và transfer progress.
- OpenCode Manager: provider, ckey, model, arbiter, settings; giữ formats opencode/auth/ckey.
- Subscription Manager: user/package/subscription/payment/settings; giữ backup và payment reference.
- Universal Converter: dependencies, classify, scan, batch, settings, native integration.
- IMG_SPLT: settings, capabilities, scan, process, distribute; mọi command nhận path explicit,
  không đổi CWD, prompt, sudo hoặc exit process từ library/bridge.
- Universe Manager: config, scan, detect, start/stop, search, preferences; async operation có request ID.

## Security và capabilities
- Capability target đúng window `main`, allow từng command/plugin; wildcard hoặc default rộng là FAIL
  nếu không có inventory/justification trong evidence.
- Production CSP non-null: deny `object-src`, `frame-src`, giới hạn `base-uri`; chỉ self/ipc và asset
  protocol cần thiết. Asset scope chỉ `$RESOURCE/{langs,themes,fonts}/**`.
- Resource path phải canonicalize, chặn traversal và symlink escape. Native/destructive operations
  như install, uninstall, kill, shell, sudo cần command riêng, confirmation và permission rõ ràng.

## Resource, fallback và persistence
- `keys(en) == keys(vi) == UI keys`; theme đủ token default; font ID duy nhất và default hợp lệ.
- Đổi language/theme/font phải thay đổi UI thực tế. JSON/font thiếu hoặc hỏng fallback an toàn và
  normalize preference về default; lựa chọn hợp lệ sống qua process restart.
- Loader dùng Tauri resource resolver, không dùng source path/CWD/GUI khác. Bundle chứa đủ ba dirs.
- Mỗi app có asset manifest relative path + SHA-256; font phân phối kèm license/attribution.

## Standalone metadata và evidence
- Mỗi app có product name, binary, reverse-DNS identifier và output riêng; version đồng bộ manifest.
- Updater phải cấu hình endpoint/channel/pubkey/signature đầy đủ hoặc tắt rõ ràng.
- Audit Cargo/npm path/file/link, TS imports, symlinks và resource lookups; chỉ allowlist TUI domain
  crate và toolchain/build output có lý do. Evidence theo path đã khóa trong roadmap.

## Appendix A — registry IPC 1:1 (2026-09-08)

### A.1 Envelope bắt buộc (Rust và TypeScript)

Mỗi hàng command dưới đây có **một** tên `invoke`/Rust command chính xác. Payload của `Req<T>` là
DTO trong cột `Req<T>`. Mọi Rust `Option<T>` là field bắt buộc được serialize thành JSON `null`
khi không có giá trị và TypeScript dùng `T | null`; không dùng field vắng mặt. Mọi command phải
trả `IpcResult<T>`. `schema_version` luôn là `1`; `request_id` là bắt buộc khi hàng có event job
(và không được sinh lại ở bridge), nếu không có thì truyền `null`.

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Req<T> { pub schema_version: u8, pub request_id: Option<String>, pub payload: T }
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Res<T> { pub schema_version: u8, pub request_id: Option<String>, pub data: T }
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode { InvalidArgument, NotFound, Conflict, Unauthorized, Forbidden,
    Unavailable, Io, Validation, Cancelled, Internal }
#[derive(serde::Serialize, serde::Deserialize)]
pub struct IpcError { pub code: IpcErrorCode, pub message: String, pub retryable: bool,
    pub details: Option<serde_json::Value> }
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JobResult<T> {
    Completed { request_id: String, value: T },
    Failed { request_id: String, error: IpcError },
    Cancelled { request_id: String, error: IpcError },
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState { Started, Progress, Completed, Failed, Cancelled }
#[derive(serde::Serialize, serde::Deserialize)]
pub struct JobEvent<T> { pub schema_version: u8, pub job_id: String, pub seq: u64,
    pub state: JobState, pub payload: T }
#[derive(serde::Serialize, serde::Deserialize)]
pub struct AppEvent<T> { pub schema_version: u8, pub payload: T }
pub type IpcResult<T> = Result<Res<T>, IpcError>;
```

```ts
type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
type Req<T> = { schema_version: 1; request_id: string | null; payload: T };
type Res<T> = { schema_version: 1; request_id: string | null; data: T };
type IpcErrorCode = "invalid_argument" | "not_found" | "conflict" | "unauthorized" |
  "forbidden" | "unavailable" | "io" | "validation" | "cancelled" | "internal";
type IpcError = { code: IpcErrorCode; message: string; retryable: boolean; details: JsonValue | null };
type JobResult<T> =
  | { status: "completed"; request_id: string; value: T }
  | { status: "failed"; request_id: string; error: IpcError }
  | { status: "cancelled"; request_id: string; error: IpcError };
type JobState = "started" | "progress" | "completed" | "failed" | "cancelled";
type JobEvent<T> = { schema_version: 1; job_id: string; seq: number; state: JobState; payload: T };
type AppEvent<T> = { schema_version: 1; payload: T };
```

`JobResult` is a discriminated enum/union: `completed` has only `value`, while `failed` and
`cancelled` have only typed `IpcError`. `seq` increases per `job_id`; precisely one terminal event
is emitted; listener is unregistered at terminal event or window close. `state` never appears in
a non-job event.

### A.2 Filen — command registry

Source registry: [bridge/lib.rs](../GUI/filen_gui/bridge/src/lib.rs). Every individual row returns
`IpcResult<Response>` and its error is exactly `IpcError`; its runtime `code` is selected from A.1.

| Command exact | Req<T> payload fields | Res<T> data; error `IpcError` | Source |
|---|---|---|---|
| `auth_login_terminal` | `{email:String,password:String,twofa_code:String|null,keep_logged:bool}` | `()` | [auth_cmds.rs](../GUI/filen_gui/bridge/src/auth_cmds.rs) |
| `auth_login_twofa_terminal` | `{email:String,password:String,twofa_code:String,keep_logged:bool}` | `()` | auth_cmds.rs |
| `auth_logout_terminal` | `{account:String|null}` | `()` | auth_cmds.rs |
| `auth_whoami_terminal` | `{}` | `Option<String>` | auth_cmds.rs |
| `auth_statfs_terminal` | `{account:String|null}` | `[String,String]` | auth_cmds.rs |
| `accounts_load` | `{}` | `StoredAccount[]` | auth_cmds.rs |
| `accounts_save` | `{accounts:StoredAccount[]}` | `()` | auth_cmds.rs |
| `fs_list_remote_terminal` | `{account:String|null,path:String}` | `FileItem[]` | [fs_cmds.rs](../GUI/filen_gui/bridge/src/fs_cmds.rs) |
| `fs_list_remote_stream_terminal` | `{account:String|null,path:String,on_chunk:Channel<FileItem[]>}` | `()` | fs_cmds.rs |
| `fs_list_local` | `{path:String}` | `FileItem[]` | fs_cmds.rs |
| `fs_get_thumbnail` | `{path:String}` | `String` | fs_cmds.rs |
| `fs_mkdir_terminal` | `{account:String|null,path:String}` | `()` | fs_cmds.rs |
| `fs_rm_terminal` | `{account:String|null,path:String,recursive:bool}` | `()` | fs_cmds.rs |
| `fs_mv_terminal` | `{account:String|null,from:String,to:String}` | `()` | fs_cmds.rs |
| `fs_cp_terminal` | `{account:String|null,from:String,to:String}` | `()` | fs_cmds.rs |
| `fs_cp_local` | `{src:String,dest:String,overwrite:bool|null}` | `()` | fs_cmds.rs |
| `fs_mv_local` | `{src:String,dest:String,overwrite:bool|null}` | `()` | fs_cmds.rs |
| `fs_rm_local` | `{path:String}` | `()` | fs_cmds.rs |
| `fs_mkdir_local` | `{path:String}` | `()` | fs_cmds.rs |
| `fs_rename_local` | `{path:String,new_name:String}` | `()` | fs_cmds.rs |
| `fs_cp_batch` | `{srcs:String[],dst_dir:String,overwrite:bool}` | `()` | fs_cmds.rs |
| `fs_upload_terminal` | `{account:String|null,local:String,remote:String}` | `()` | fs_cmds.rs |
| `fs_download_terminal` | `{account:String|null,remote:String,local:String}` | `()` | fs_cmds.rs |
| `fs_cat_terminal` | `{account:String|null,path:String}` | `String` | fs_cmds.rs |
| `fs_link_create_terminal` | `{account:String|null,path:String}` | `String` | fs_cmds.rs |
| `fs_links_list_terminal` | `{account:String|null}` | `FileItem[]` | fs_cmds.rs |
| `fs_write_terminal` | `{account:String|null,path:String,content:String}` | `()` | fs_cmds.rs |
| `fs_write_local` | `{path:String,content:String}` | `()` | fs_cmds.rs |
| `fs_sudo_exec` | `{action:String,args:String[]}` | `()` | fs_cmds.rs |
| `fs_rename_terminal` | `{account:String|null,path:String,new_name:String}` | `()` | fs_cmds.rs |
| `fs_delete_terminal` | `{account:String|null,path:String}` | `()` | fs_cmds.rs |
| `fs_copy_terminal` | `{account:String|null,from:String,to:String}` | `()` | fs_cmds.rs |
| `fs_move_terminal` | `{account:String|null,from:String,to:String}` | `()` | fs_cmds.rs |
| `fs_open` | `{path:String}` | `()` | fs_cmds.rs |
| `fs_stat_advanced` | `{path:String}` | `StatInfo` | fs_cmds.rs |
| `fs_chmod` | `{path:String,mode:String}` | `()` | fs_cmds.rs |
| `fs_chown` | `{path:String,uid:u32,gid:u32}` | `()` | fs_cmds.rs |
| `fs_get_free_space` | `{path:String}` | `u64` | fs_cmds.rs |
| `fs_search_local` | `{path:String,query:String,options:SearchOptions|null}` | `SearchResult[]` | fs_cmds.rs |
| `fs_trash_list_local` | `{}` | `FileItem[]` | fs_cmds.rs |
| `fs_trash_restore_local` | `{item_id:String}` | `()` | fs_cmds.rs |
| `fs_trash_empty_local` | `{}` | `()` | fs_cmds.rs |
| `fs_trash_list_remote_terminal` | `{account:String|null}` | `FileItem[]` | fs_cmds.rs |
| `fs_trash_restore_remote_terminal` | `{account:String|null,idx:usize}` | `()` | fs_cmds.rs |
| `fs_trash_delete_remote_terminal` | `{account:String|null,idx:usize}` | `()` | fs_cmds.rs |
| `fs_trash_empty_remote_terminal` | `{account:String|null}` | `()` | fs_cmds.rs |
| `transfer_enqueue` | `{kind:String,name:String,src:String,dst:String,src_local:bool,dst_local:bool,cleanup_src:bool,src_pane:usize,dst_pane:usize}` | `usize` | [transfer_cmds.rs](../GUI/filen_gui/bridge/src/transfer_cmds.rs) |
| `transfer_start` | `{account:String|null}` | `()` | transfer_cmds.rs |
| `transfer_cancel` | `{id:usize}` | `()` | transfer_cmds.rs |
| `transfer_cancel_all` | `{}` | `()` | transfer_cmds.rs |
| `transfer_remove_finished` | `{}` | `()` | transfer_cmds.rs |
| `os_clipboard_set` | `{paths:String[],is_cut:bool}` | `()` | [sys_cmds.rs](../GUI/filen_gui/bridge/src/sys_cmds.rs) |
| `os_clipboard_get` | `{}` | `Option<OSClipboardData>` | sys_cmds.rs |
| `sys_list_apps` | `{}` | `DesktopApp[]` | sys_cmds.rs |
| `sys_get_custom_actions` | `{}` | `CustomAction[]` | sys_cmds.rs |
| `sys_execute_custom_action` | `{exec_template:String,file_paths:String[]}` | `()` | sys_cmds.rs |
| `sys_open_with` | `{path:String,exec_cmd:String}` | `()` | sys_cmds.rs |
| `open_in_terminal` | `{path:String}` | `()` | sys_cmds.rs |
| `appearance_list_themes` | `{}` | `ThemeResource[]` | [appearance_cmds.rs](../GUI/filen_gui/bridge/src/appearance_cmds.rs) |
| `appearance_list_fonts` | `{}` | `FontResource[]` | appearance_cmds.rs |

### A.3 Rclone — command registry

Source registry: [backend/lib.rs](../GUI/rclone_gui/backend/src/lib.rs). Every individual row returns
`IpcResult<Response>` and its error is exactly `IpcError`; its runtime `code` is selected from A.1.

| Command exact | Req<T> payload fields | Res<T> data; error `IpcError` | Source |
|---|---|---|---|
| `list_files` | `{path:String,pane:String|null}` | `FileItem[]` | [files.rs](../GUI/rclone_gui/backend/src/api/files.rs) |
| `fs_mkdir` | `{path:String}` | `()` | files.rs |
| `fs_touch` | `{path:String}` | `()` | files.rs |
| `fs_delete` | `{path:String}` | `()` | files.rs |
| `fs_rename` | `{old_path:String,new_path:String}` | `()` | files.rs |
| `fs_copy` | `{src:String,dst:String,task_id:u32|null}` | `()` | files.rs |
| `fs_move` | `{src:String,dst:String,task_id:u32|null}` | `()` | files.rs |
| `fs_cancel` | `{task_id:u32}` | `()` | files.rs |
| `fs_stat_advanced` | `{path:String}` | `StatInfo` | files.rs |
| `fs_search` | `{path:String,query:String}` | `SearchResultItem[]` | files.rs |
| `fs_check_conflicts` | `{srcs:String[],dest_path:String}` | `ConflictInfo[]` | files.rs |
| `get_home_dir` | `{}` | `String` | files.rs |
| `get_user_places` | `{}` | `UserPlace[]` | files.rs |
| `open_in_terminal` | `{path:String}` | `()` | files.rs |
| `fs_get_thumbnail` | `{path:String}` | `String` | files.rs |
| `fs_temp_dir` | `{}` | `String` | files.rs |
| `fs_chmod` | `{path:String,mode:String}` | `()` | files.rs |
| `fs_chown` | `{path:String,uid:u32,gid:u32}` | `()` | files.rs |
| `fs_read_text` | `{path:String,max_bytes:u64|null}` | `String` | files.rs |
| `fs_write_text` | `{path:String,content:String}` | `()` | files.rs |
| `sys_open_with` | `{path:String,exec_cmd:String|null,app:DesktopApp|null}` | `()` | [sys.rs](../GUI/rclone_gui/backend/src/core/sys.rs) |
| `sys_list_apps` | `{}` | `DesktopApp[]` | sys.rs |
| `os_clipboard_set` | `{items:String[],is_cut:bool}` | `()` | sys.rs |
| `os_clipboard_get` | `{}` | `Option<OSClipboardData>` | sys.rs |
| `sys_get_custom_actions` | `{}` | `CustomAction[]` | sys.rs |
| `sys_get_valid_actions` | `{files:String[]}` | `CustomAction[]` | sys.rs |
| `sys_execute_custom_action` | `{exec_template:String,file_paths:String[]}` | `()` | sys.rs |
| `fs_trash_list_local` | `{}` | `TrashItemLocal[]` | [trash.rs](../GUI/rclone_gui/backend/src/api/trash.rs) |
| `fs_trash_restore_local` | `{item_id:String}` | `()` | trash.rs |
| `fs_trash_delete_local` | `{item_id:String}` | `()` | trash.rs |
| `fs_trash_empty_local` | `{}` | `()` | trash.rs |
| `fs_trash_list_remote_terminal` | `{account:String|null}` | `FileItem[]` | trash.rs |
| `fs_trash_restore_remote_terminal` | `{account:String|null,path:String}` | `()` | trash.rs |
| `fs_trash_delete_remote_terminal` | `{account:String|null,path:String}` | `()` | trash.rs |
| `fs_trash_empty_remote_terminal` | `{account:String|null}` | `()` | trash.rs |
| `list_remotes` | `{}` | `Value[]` | [remotes.rs](../GUI/rclone_gui/backend/src/api/remotes.rs) |
| `get_providers` | `{}` | `Value[]` | remotes.rs |
| `create_remote` | `{name:String,provider:String,options:Value}` | `String` | remotes.rs |
| `update_remote` | `{name:String,options:Value}` | `String` | remotes.rs |
| `delete_remote` | `{name:String}` | `String` | remotes.rs |
| `get_backend_features` | `{remote:String}` | `Value` | remotes.rs |
| `check_transfer_capability` | `{src:String,dst:String}` | `Value` | remotes.rs |
| `rclone_about` | `{}` | `String` | remotes.rs |
| `rclone_size` | `{}` | `String` | remotes.rs |
| `check_fuse_installed` | `{}` | `bool` | [mount.rs](../GUI/rclone_gui/backend/src/api/mount.rs) |
| `create_mount_service` | `{config:MountConfig}` | `String` | mount.rs |
| `delete_mount_service` | `{service_name:String,is_user:bool}` | `String` | mount.rs |
| `manage_mount_service` | `{service_name:String,is_user:bool,action:String}` | `String` | mount.rs |
| `list_mount_services` | `{}` | `SystemdServiceInfo[]` | mount.rs |
| `get_mount_service_config` | `{service_name:String,is_user:bool}` | `MountConfig` | mount.rs |
| `get_config_content` | `{}` | `String` | [config.rs](../GUI/rclone_gui/backend/src/api/config.rs) |
| `set_config_content` | `{content:String}` | `()` | config.rs |
| `reorder_config` | `{names:String[]}` | `()` | config.rs |
| `get_available_langs` | `{}` | `String[]` | [lang.rs](../GUI/rclone_gui/backend/src/api/lang.rs) |
| `get_lang_content` | `{lang_code:String}` | `Value` | lang.rs |
| `get_available_themes` | `{}` | `ThemeInfo[]` | [appearance.rs](../GUI/rclone_gui/backend/src/api/appearance.rs) |
| `get_available_fonts` | `{}` | `FontInfo[]` | appearance.rs |

### A.4 OpenCode Manager — command registry

Source registry: [backend/lib.rs](../GUI/opencode_manager_gui/backend/src/lib.rs). Every individual
row returns `IpcResult<Response>` and its error is exactly `IpcError`; its runtime `code` is selected from A.1.

| Command exact | Req<T> payload fields | Res<T> data; error `IpcError` | Source |
|---|---|---|---|
| `list_providers` | `{}` | `ProviderView[]` | [provider.rs](../GUI/opencode_manager_gui/backend/src/api/provider.rs) |
| `list_presets` | `{}` | `PresetView[]` | provider.rs |
| `get_provider_secret` | `{provider_id:String}` | `String` | provider.rs |
| `save_provider` | `{provider_id:String,preset_id:String,name:String,base_url:String,api_key:String,force_overwrite_id:String|null,npm:bool|null,custom_id:String|null}` | `SaveResult` | provider.rs |
| `delete_provider` | `{provider_id:String}` | `()` | provider.rs |
| `delete_providers` | `{provider_ids:String[]}` | `usize` | provider.rs |
| `test_provider` | `{provider_id:String}` | `StatusView` | provider.rs |
| `test_connection` | `{base_url:String,api_key:String}` | `StatusView` | provider.rs |
| `test_all_providers` | `{}` | `StatusView[]` | provider.rs |
| `scan_provider_models` | `{provider_id:String}` | `ScannedModel[]` | provider.rs |
| `set_provider_models` | `{provider_id:String,selected:String[],caps:Value|null}` | `ProviderView` | provider.rs |
| `find_bad_providers` | `{}` | `BadProvider[]` | provider.rs |
| `bulk_add_providers` | `{endpoint:String,keys:String[]}` | `BulkAddResult` | [bulk.rs](../GUI/opencode_manager_gui/backend/src/api/bulk.rs) |
| `list_ckey_profiles` | `{}` | `CkeyProfileView[]` | [ckey.rs](../GUI/opencode_manager_gui/backend/src/api/ckey.rs) |
| `save_ckey_profile` | `{profile_id:String|null,name:String,key:String}` | `String` | ckey.rs |
| `delete_ckey_profile` | `{profile_id:String}` | `()` | ckey.rs |
| `set_active_ckey_profile` | `{profile_id:String}` | `Option<String>` | ckey.rs |
| `fetch_ckey_dashboard` | `{profile_id:String,since_days:u32|null,force:bool|null}` | `CkeyDashboard` | ckey.rs |
| `fetch_ckey_usage` | `{profile_id:String,page:u32,limit:u32,model:String|null,force:bool|null}` | `CkeyUsageView` | ckey.rs |
| `fetch_ckey_deposit` | `{profile_id:String,amount:String,page:u32,limit:u32,force:bool|null}` | `CkeyDepositView` | ckey.rs |
| `list_ckey_import_items` | `{profile_id:String}` | `CkeyImportList` | ckey.rs |
| `import_ckey_models` | `{profile_id:String,selected:String[]}` | `CkeyImportResult` | ckey.rs |
| `list_model_matrix` | `{}` | `ModelMatrixRow[]` | [models.rs](../GUI/opencode_manager_gui/backend/src/api/models.rs) |
| `set_primary_model` | `{provider_id:String|null,model_id:String|null}` | `Option<String>` | models.rs |
| `sync_limits_from_dev` | `{}` | `usize` | models.rs |
| `get_arbiter_state` | `{}` | `ArbiterState` | [arbiter.rs](../GUI/opencode_manager_gui/backend/src/api/arbiter.rs) |
| `run_arbiter_evaluation` | `{arbiter_provider:String,arbiter_model:String}` | `ArbiterVerdict[]` | arbiter.rs |
| `clear_arbiter_history` | `{}` | `()` | arbiter.rs |
| `recommend_models` | `{task:String|null}` | `RecommendationView` | arbiter.rs |
| `get_gui_settings` | `{}` | `GuiSettings` | [settings.rs](../GUI/opencode_manager_gui/backend/src/api/settings.rs) |
| `save_gui_settings` | `{language:String,theme_id:String,font_id:String}` | `()` | settings.rs |
| `get_config_paths` | `{}` | `ConfigPaths` | settings.rs |
| `open_external_url` | `{url:String}` | `()` | settings.rs |
| `get_available_langs` | `{}` | `String[]` | [lang.rs](../GUI/opencode_manager_gui/backend/src/api/lang.rs) |
| `get_lang_content` | `{lang_code:String}` | `Value` | lang.rs |
| `get_available_themes` | `{}` | `Theme[]` | [theme.rs](../GUI/opencode_manager_gui/backend/src/api/theme.rs) |
| `get_available_fonts` | `{}` | `FontInfo[]` | [font.rs](../GUI/opencode_manager_gui/backend/src/api/font.rs) |

### A.5 Subscription Manager — command registry

Source registry: [backend/lib.rs](../GUI/subscription_manager_gui/backend/src/lib.rs). Every
individual row returns `IpcResult<Response>` and its error is exactly `IpcError`; its runtime `code` is selected from A.1.

| Command exact | Req<T> payload fields | Res<T> data; error `IpcError` | Source |
|---|---|---|---|
| `add_user` | `{username:String,email:String|null,phone:String|null,contact_url:String|null}` | `User` | [user_api.rs](../GUI/subscription_manager_gui/backend/src/user_api.rs) |
| `update_user` | `{id:String,username:String|null,email:String|null,phone:String|null,contact_url:String|null}` | `User` | user_api.rs |
| `delete_user` | `{id:String}` | `()` | user_api.rs |
| `list_users` | `{page:u32|null,limit:u32|null}` | `User[]` | user_api.rs |
| `adjust_user_balance` | `{id:String,delta:i64,note:String|null}` | `User` | user_api.rs |
| `add_package` | `{name:String,duration_days:u32,description:String|null,price:i64|null}` | `Package` | [package_api.rs](../GUI/subscription_manager_gui/backend/src/package_api.rs) |
| `update_package` | `{id:String,name:String|null,duration_days:u32|null,description:String|null,price:i64|null}` | `Package` | package_api.rs |
| `delete_package` | `{id:String}` | `()` | package_api.rs |
| `list_packages` | `{page:u32|null,limit:u32|null}` | `Package[]` | package_api.rs |
| `add_subscription_to_user` | `{user_id:String,package_id:String,custom_expiration_date:String|null,amount:i64|null,auto_renew:bool|null}` | `Subscription` | [subscription_api.rs](../GUI/subscription_manager_gui/backend/src/subscription_api.rs) |
| `update_subscription_expiry` | `{subscription_id:String,new_expiration_date:String,amount:i64|null,auto_renew:bool|null}` | `Subscription` | subscription_api.rs |
| `remove_subscription_from_user` | `{subscription_id:String}` | `()` | subscription_api.rs |
| `list_user_subscriptions` | `{user_id:String}` | `Subscription[]` | subscription_api.rs |
| `check_subscription_status` | `{subscription_id:String}` | `bool` | subscription_api.rs |
| `list_all_subscriptions` | `{}` | `Subscription[]` | subscription_api.rs |
| `process_auto_renewals` | `{}` | `AutoRenewReport` | subscription_api.rs |
| `set_subscription_auto_renew` | `{subscription_id:String,auto_renew:bool}` | `Subscription` | subscription_api.rs |
| `list_user_transactions` | `{user_id:String}` | `Transaction[]` | [transaction_api.rs](../GUI/subscription_manager_gui/backend/src/transaction_api.rs) |
| `list_all_transactions` | `{}` | `Transaction[]` | transaction_api.rs |
| `delete_transaction` | `{id:String}` | `()` | transaction_api.rs |
| `issue_payment_ref` | `{user_id:String,transaction_ids:String[]}` | `PaymentRef` | [payment_api.rs](../GUI/subscription_manager_gui/backend/src/payment_api.rs) |
| `lookup_payment_ref` | `{input:String}` | `PaymentLookup` | payment_api.rs |
| `list_payment_refs` | `{user_id:String|null}` | `PaymentRef[]` | payment_api.rs |
| `settle_payment_ref` | `{code:String,settled:bool}` | `PaymentRef` | payment_api.rs |
| `get_settings` | `{}` | `Settings` | [settings_api.rs](../GUI/subscription_manager_gui/backend/src/settings_api.rs) |
| `save_settings` | `{language:String,timezone:String,theme_id:String,font_id:String}` | `()` | settings_api.rs |
| `export_backup` | `{destination:String}` | `()` | [storage.rs](../GUI/subscription_manager_gui/backend/src/storage.rs) |
| `get_available_langs` | `{}` | `String[]` | [lang_api.rs](../GUI/subscription_manager_gui/backend/src/lang_api.rs) |
| `get_lang_content` | `{lang_code:String}` | `Value` | lang_api.rs |
| `get_available_themes` | `{}` | `Theme[]` | [theme_api.rs](../GUI/subscription_manager_gui/backend/src/theme_api.rs) |
| `get_available_fonts` | `{}` | `FontInfo[]` | [font_api.rs](../GUI/subscription_manager_gui/backend/src/font_api.rs) |

### A.3 Capability, containment và resource matrix

Tauri application commands registered by `generate_handler!` use core IPC and receive **no** invented
custom-command permission. For the four app rows below, `capabilities/main.json` is the sole
capability, its `windows` is exactly `["main"]`, and the asset scope is exactly
`$RESOURCE/langs/**`, `$RESOURCE/themes/**`, `$RESOURCE/fonts/**`. The production CSP is
`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset: http://asset.localhost; font-src 'self' data: asset: http://asset.localhost; connect-src 'self' ipc: http://ipc.localhost asset: http://asset.localhost; object-src 'none'; base-uri 'self'; frame-src 'none'`. The two asset origins in `connect-src` are required because the local frontends fetch bundled language/theme JSON through Tauri's asset protocol; they do not authorize network hosts.

| App | exact capability / actual plugin permissions | bundled resources, defaults, persistence | evidence |
|---|---|---|---|
| Filen | `core:event:allow-listen`, `core:event:allow-unlisten`, `drag:allow-start-drag`; no window or shell permission. `tauri-plugin-drag` is required by `frontend/src/features/dragDrop.ts`; remove unused `tauri-plugin-shell` (no plugin API call). Backend emits `local-dir-changed`, `auth:whoami-finished`, `transfer:progress`, `transfer:finished`; frontend listens. | Add app-root `langs/{en,vi}.json` from the existing frontend dictionaries and copy existing themes into app-root `themes/`; map `../langs/ -> langs/`, `../themes/ -> themes/`, `../fonts/ -> fonts/`. Defaults `en/default/default`, `showHiddenFiles:true`; browser keys `filen_settings`, `filen_activity_log`, `filen_bookmarks`. | `GUI/filen_gui/bridge/{tauri.conf.json,capabilities/main.json,src/lib.rs}`, `GUI/filen_gui/{langs,themes,fonts}/`, `frontend/src/{settings.ts,store.ts,features/{dragDrop.ts,transferManager.ts}}`. |
| Rclone | `core:event:allow-listen`, `core:event:allow-unlisten`, `drag:allow-start-drag`; no window or shell permission. Add/register `tauri-plugin-drag` because `frontend/src/features/dragDrop.ts` calls it; remove unused `tauri-plugin-shell`. Backend emits `local-dir-changed`, `transfer_progress`; frontend listens. | Keep `../{langs,themes,fonts}/ -> {langs,themes,fonts}/`. Defaults `""/default/default`, with empty language resolving sorted bundled `en`; `showHiddenFiles:false`. Browser keys `rclonegui_settings`, `rclonegui_activity_log`, `rclonegui_bookmarks`; migrate equivalent legacy `filen_` keys once. | `GUI/rclone_gui/backend/{tauri.conf.json,capabilities/main.json,src/lib.rs}`, `frontend/src/{store.ts,features/{dragDrop.ts,transferManager.ts}}`. |
| OpenCode Manager | `core:event:allow-listen`, `core:event:allow-unlisten`; no window/plugin permission. Debug-only `tauri-plugin-log` is backend-only and has no webview permission. Backend emits `arbiter://progress`; frontend listens. | Keep `../{langs,themes,fonts}/ -> {langs,themes,fonts}/`. Defaults sorted bundled `en` / `default` / `default`. Persist GUI state `${XDG_CONFIG_HOME:-$HOME/.config}/opencode-manager/settings.json`; import `~/.config/opencode/manager_gui.json` once as `.legacy`. Runtime `opencode.json` and `auth.json` stay under `~/.config/opencode/`; manager `ckey.json`/arbiter state stay under `opencode-manager`. | `GUI/opencode_manager_gui/backend/{tauri.conf.json,capabilities/main.json,src/{lib.rs,api/settings.rs}}`, `frontend/src/pages/ModelsPage.tsx`. |
| Subscription Manager | `dialog:allow-save`; no event, window, or log permission. `tauri-plugin-dialog` is required only by `save()` in `frontend/src/pages/SettingsPage.tsx`; debug-only `tauri-plugin-log` is backend-only. Replace null CSP with the stated production CSP. | Keep `../{langs,themes,fonts}/ -> {langs,themes,fonts}/`. Defaults sorted bundled `en` / `Asia/Ho_Chi_Minh` / `default` / `default`. Canonical `${XDG_CONFIG_HOME:-$HOME/.config}/subscription_manager_gui/{settings.json,data.json,backups/data-*.json}`; migrate/mirror `storage/{settings.json,data.json}` from CWD, otherwise resource root. Browser keys `vietqr_bank_bin`, `vietqr_account_no`, `vietqr_account_name`. | `GUI/subscription_manager_gui/backend/{tauri.conf.json,capabilities/main.json,src/{lib.rs,settings_api.rs,storage.rs}}`, `frontend/src/{pages/{SettingsPage.tsx,InvoiceModal.tsx},store/useSettingsStore.ts}`. |
| Universal Converter | `bridge/capabilities/main.json`: `identifier:"main"`, `windows:["main"]`, permissions exactly `core:event:allow-listen`, `core:event:allow-unlisten`; no Tauri plugin. The bridge-owned picker uses exact-pinned `rfd = 0.16.0`, remains parented to the invoking window, and grants no webview dialog capability. | Command-specific absolute roots are locked in A.3.2; no CWD assumptions. | default `vi/system/system-default`; atomic `preferences.json` under app data; bundles `../{langs,themes,fonts}/` to `$RESOURCE/{langs,themes,fonts}/`. Evidence: `GUI/universal_converter_gui/bridge/{Cargo.toml,capabilities/main.json,tauri.conf.json,src/commands.rs}`, `frontend/src/ipc.ts`. |
| IMG_SPLT | planned `bridge/capabilities/main.json`: `identifier:"main"`, `windows:["main"]`, permissions exactly `core:event:allow-listen`, `core:event:allow-unlisten`; no Tauri plugin. The bridge-owned picker uses exact-pinned `rfd = 0.16.0`, remains parented to the invoking window, and grants no webview dialog capability. | Command-specific absolute roots are locked in A.3.2; no prompt/sudo/exit in backend or bridge. | default `en/dark/system-default`; atomic `preferences.json` under app data; bundles `../{langs,themes,fonts}/` to `$RESOURCE/{langs,themes,fonts}/`. Evidence: `GUI/img_splt_gui/bridge/{Cargo.toml,capabilities/main.json,tauri.conf.json,src/commands.rs}`, `frontend/src/ipc.ts`. |
| Universe Manager | planned `bridge/capabilities/main.json`: `identifier:"main"`, `windows:["main"]`, permissions exactly `core:event:allow-listen`, `core:event:allow-unlisten`; no Tauri plugin | Command-specific absolute roots are locked in A.3.2; start/stop requires UI confirmation before dispatch. | default `vi/system/system-default`; atomic `preferences.json` under app data; bundles `../{langs,themes,fonts}/` to `$RESOURCE/{langs,themes,fonts}/`. Evidence: `GUI/universe_manager_gui/bridge/{capabilities/main.json,tauri.conf.json,src/commands.rs}`, `frontend/src/ipc.ts`. |

Containment algorithm for every external path: (1) reject empty/non-absolute path where operation
requires absolute; (2) canonicalize existing target or canonicalize parent plus validate basename
for a create; (3) canonicalize configured allowed roots; (4) require candidate `starts_with` one
root; (5) re-check resolved target after open/rename to prevent symlink escape; (6) deny on any
resolution error. Resource loader is the only exception: resolve a validated relative asset ID via
Tauri resource resolver, then apply the same containment under resource root. Never use CWD,
source paths, or another GUI's tree.

#### A.3.1 Bundled DejaVu font contract (seven Tauri apps)

Filen, Rclone, OpenCode Manager, Subscription Manager, Universal Converter, IMG_SPLT, and Universe
Manager each own their app-local `fonts/DejaVuSans.ttf`, `fonts/LICENSE.txt`, and
`fonts/manifest.sha256`; no symlink or cross-app lookup is allowed. The bundled font ID is exactly
`dejavusans`; `default` remains the system-font sentinel. Every app's manifest has exactly these entries:
`ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280  DejaVuSans.ttf` and
`63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9  LICENSE.txt`.
Font discovery reads only the app-local resource bundle; placement READMEs are not license evidence.

#### A.3.2 Exact allowed roots cho ba app mới

These are bridge/backend containment roots, not webview filesystem permissions. A frontend-selected
root is accepted only after an explicit picker action and stored in bridge state for that window;
arbitrary request payloads cannot add roots. `app_data_dir` and `app_config_dir` are Tauri-resolved,
app-specific paths. An empty root set denies the operation.

| App | Exact command | Exact allowed roots |
|---|---|---|
| Universal Converter | `dependencies_check` | none; executable lookup only through the inherited process `PATH` |
| Universal Converter | `classify_file`, `scan_directory` | canonical frontend-selected input roots |
| Universal Converter | `batch_convert` | reads: canonical frontend-selected input roots; writes: canonical frontend-selected output roots |
| Universal Converter | `native_install` | Linux only: reads canonical frontend-selected artifact roots; writes only `$HOME/.local/{bin,share/applications}`. Other platforms return typed `Unavailable` during preflight before job registration or filesystem mutation. |
| Universal Converter | `native_uninstall` | Linux only: removes only a recorded installation beneath `$HOME/.local/{bin,share/applications}`. Other platforms return typed `Unavailable` during preflight before job registration or filesystem mutation. |
| Universal Converter | `preferences_get` | `app_data_dir` only |
| Universal Converter | `preferences_set` | `app_data_dir` only |
| IMG_SPLT | `settings_load` | `app_data_dir` only |
| IMG_SPLT | `settings_save` | `app_data_dir` only |
| IMG_SPLT | `capabilities_check` | none; executable lookup only through the inherited process `PATH` |
| IMG_SPLT | `scan_images` | canonical frontend-selected input-directory roots |
| IMG_SPLT | `process_images` | reads: canonical frontend-selected input-directory roots; writes: canonical frontend-selected output-directory roots |
| IMG_SPLT | `distribute` | reads: canonical frontend-selected input-directory roots; writes: canonical frontend-selected output-directory roots |
| IMG_SPLT | `preferences_get` | `app_data_dir` only |
| IMG_SPLT | `preferences_set` | `app_data_dir` only |
| Universe Manager | `config_load` | `app_config_dir` only |
| Universe Manager | `config_save` | `app_config_dir` only; `managed_dir` value must resolve beneath a canonical frontend-selected managed-directory root |
| Universe Manager | `scan_apps` | configured `managed_dir` canonicalized beneath a canonical frontend-selected managed-directory root; config reads remain in `app_config_dir` |
| Universe Manager | `detect_app` | canonical frontend-selected source roots |
| Universe Manager | `start_app` | executable identity resolved from an `AppEntry` beneath canonical configured `managed_dir`; request payload cannot supply a path |
| Universe Manager | `stop_app` | process identity resolved from an `AppEntry` beneath canonical configured `managed_dir`; request payload cannot supply a path |
| Universe Manager | `search_apps` | configured `managed_dir` canonicalized beneath a canonical frontend-selected managed-directory root; config reads remain in `app_config_dir` |
| Universe Manager | `preferences_get` | `app_data_dir` only |
| Universe Manager | `preferences_set` | `app_data_dir` only |

### A.4 Metadata and release evidence matrix

For all seven Tauri apps, updater is explicitly disabled: no `tauri-plugin-updater`, no
`plugins.updater`, and `bundle.createUpdaterArtifacts:false`. Run `cargo tauri build` from the
listed bridge/backend directory. Workspace members output under `RS_AI/target/release/`; Filen's
standalone bridge outputs under `GUI/filen_gui/bridge/target/release/`.

| App | exact product / Cargo binary / identifier | exact targets and outputs | evidence paths |
|---|---|---|---|
| Filen | `filen_gui` / `filen_gui_tauri` / `io.filen.gui` | `targets:["deb","appimage","msi","dmg"]`; `bridge/target/release/filen_gui_tauri`; `bridge/target/release/bundle/{deb,appimage,msi,dmg}/`. | `GUI/filen_gui/bridge/{Cargo.toml,tauri.conf.json,capabilities/main.json}`, `GUI/filen_gui/{langs,themes,fonts}/`. |
| Rclone | `rclone_gui` / `rclone_gui` / `com.rclone.gui` | `targets:["deb","appimage","msi","dmg"]`; `target/release/rclone_gui`; `target/release/bundle/{deb,appimage,msi,dmg}/`. | `GUI/rclone_gui/backend/{Cargo.toml,tauri.conf.json,capabilities/main.json}`, `GUI/rclone_gui/{langs,themes,fonts}/`. |
| OpenCode Manager | `opencode_manager_gui` / `opencode_manager_gui` / `com.opencode.manager` (`opencode_manager_gui_lib` is library only) | `targets:["deb","appimage","msi","dmg"]`; `target/release/opencode_manager_gui`; `target/release/bundle/{deb,appimage,msi,dmg}/`. | `GUI/opencode_manager_gui/backend/{Cargo.toml,tauri.conf.json,capabilities/main.json}`, `GUI/opencode_manager_gui/{langs,themes,fonts}/`. |
| Subscription Manager | `subscription_manager_gui` / `subscription_manager_gui` / `com.subscription.manager` (`subscription_manager_gui_lib` is library only) | `targets:["deb","appimage","msi","dmg"]`; `target/release/subscription_manager_gui`; `target/release/bundle/{deb,appimage,msi,dmg}/`. | `GUI/subscription_manager_gui/backend/{Cargo.toml,tauri.conf.json,capabilities/main.json}`, `GUI/subscription_manager_gui/{langs,themes,fonts}/`. |
| Universal Converter | `Universal Converter` / `universal-converter-gui` / `com.sh.universal-converter` | bridge `GUI/universal_converter_gui/bridge`; workspace output under `RS_AI/target/release/`; Linux binary plus `bundle/{deb,appimage}/` verified; MSI/DMG deferred/platform-unverified. | `GUI/universal_converter_gui/bridge/{Cargo.toml,tauri.conf.json,capabilities/main.json}`, `GUI/universal_converter_gui/{langs,themes,fonts}/`, `.opencode/evidence/gui/universal_converter_gui/package.md`. |
| IMG_SPLT | `Image Splitter` / `img-splt-gui` / `com.sh.image-splitter` | bridge `GUI/img_splt_gui/bridge`; workspace output under `RS_AI/target/release/`; Linux binary plus `bundle/{deb,appimage}/` required; MSI/DMG deferred/platform-unverified. | `GUI/img_splt_gui/bridge/{Cargo.toml,tauri.conf.json,capabilities/main.json}`, `GUI/img_splt_gui/{langs,themes,fonts}/`, `.opencode/evidence/gui/img_splt_gui/package.md`. |
| Universe Manager | `Universe Manager` / `universe-manager-gui` / `com.sh.universe-manager` | bridge `GUI/universe_manager_gui/bridge`; `targets:["deb","appimage","msi","dmg"]`; binary `GUI/universe_manager_gui/bridge/target/release/universe-manager-gui`; bundles `GUI/universe_manager_gui/bridge/target/release/bundle/{deb,appimage,msi,dmg}/`. | `GUI/universe_manager_gui/bridge/{Cargo.toml,tauri.conf.json,capabilities/main.json}`, `GUI/universe_manager_gui/{langs,themes,fonts}/`, `GUI/universe_manager_gui/bridge/target/release/{universe-manager-gui,bundle/{deb,appimage,msi,dmg}/}`. |

No row authorizes sharing: all identifiers, bridge DTOs, resource manifests, capability files,
preferences, build outputs, and release evidence remain local to the named standalone app.

### A.5 Khóa migration ba app egui

Mọi tên dưới đây là tên command Rust/Tauri và key `invoke` TypeScript chính xác (snake_case).
Mỗi request là `Req<T>` và mỗi response thành công là `Res<T>` theo A.1; `request_id` là bắt buộc
cho command có thể chạy nền. `PathRef { path: String }` luôn là đường dẫn tuyệt đối. Mỗi app tự
khai báo các DTO này trong bridge của chính nó; không tạo crate, package hay component dùng chung.

#### A.5.1 Universal Converter

- Owns dependency/classification/scan, roadmap-required `batch_convert`, `native_install`,
  `native_uninstall`, and preferences commands. Exact DTOs/errors are only those in A.6.
- Job-producing commands and exact topics are only those in A.7; there is no non-job event.

#### A.5.2 IMG_SPLT

- Owns only settings, capabilities, image scan/process/distribution, and preferences commands;
  it has no batch conversion or native install/uninstall command. Exact DTOs/errors are only A.6.
- Additionally owns the explicit bridge picker in A.6 solely to establish per-window input/output
  directory provenance; this does not grant a webview dialog capability.
- Job-producing commands and exact topics are only A.7; destructive output is confirmed before invoke.

#### A.5.3 Universe Manager

- Owns only config, app scan/detect/start/stop/search, and preferences commands; it has no batch
  conversion or native install/uninstall command. Exact DTOs/errors are only A.6.
- `AppEntry` is exactly the serialized `universe_manager::config::AppEntry`, with no reduced model.
  Job-producing commands and exact topics are only A.7.

#### A.5.4 Tauri metadata, window, resources và persistence

| App | main window | resource mapping and default IDs | canonical persistence and legacy import |
|---|---|---|---|
| Universal Converter | label `main`; title `Universal Converter — GUI`; `900x650`; resizable | `../langs/ -> langs/`, `../themes/ -> themes/`, `../fonts/ -> fonts/`; asset IDs begin `$RESOURCE/langs/`, `$RESOURCE/themes/`, `$RESOURCE/fonts/`; `vi` / `system` / `system-default` | `app_data_dir()/preferences.json`; import eframe key `preferences` once, validate IDs, then atomic temp-file + rename write; retain previous file as `preferences.json.bak`. |
| IMG_SPLT | label `main`; title `Image Splitter — GUI`; `900x650`; resizable | same three local bundle mappings and asset roots; `en` / `dark` / `system-default` | `app_data_dir()/preferences.json`; import eframe keys `language`, `theme`, `font` once, validate IDs, then atomic temp-file + rename write; retain `preferences.json.bak`. |
| Universe Manager | label `main`; title `Universe Manager — GUI`; `1000x680`; resizable | same three local bundle mappings and asset roots; `vi` / `system` / `system-default` | `app_data_dir()/preferences.json`; import legacy `${XDG_CONFIG_HOME:-$HOME/.config}/universe_manager_gui/preferences.conf` (Windows `%APPDATA%\\universe_manager_gui\\preferences.conf`) once, validate IDs, then atomic temp-file + rename write; retain `preferences.json.bak`. |

For these three configurations, `plugins` is `{}` and `capabilities/main.json` is the only
capability file. Updater remains deliberately disabled by omitting `tauri-plugin-updater` and
`plugins.updater`, and setting `bundle.createUpdaterArtifacts` to `false`.

### A.6 Registry migration 1:1 (authoritative; replaces grouped prose in A.5)

All rows below are exact Rust command names and exact TypeScript `invoke` keys. Every command
accepts `Req<T>`, returns `IpcResult<T>` (`Res<T>` or `IpcError`), and is local to the named app.
`PathRef { path:String }` is absolute. `Preferences { language:"vi"|"en", theme:"system"|"light"|"dark",
font_id:String }`; `NativeInstallRequest { artifact:PathRef, target_dir:PathRef, confirmed:bool }`;
`NativeUninstallRequest { installation_id:String, confirmed:bool }`; and `NativeOperationResult {
operation:"install"|"uninstall", installation_id:String, completed:bool }`. Native requests require
`confirmed:true`; bridge/backend never prompts, uses sudo, changes CWD, or exits the process. These
batch/native DTOs and commands belong only to Universal Converter; IMG_SPLT and Universe Manager
do not register or emit them. Universal Converter additionally owns the explicit bridge picker:
`PickerSelectRequest {kind:"input"|"output"|"artifact",selection:"file"|"files"|"directory"}` and
`PickerSelectResult {kind:"input"|"output"|"artifact",paths:PathRef[]}`. Only a successful
`picker_select` may establish per-window/per-kind path provenance; generic drag/drop never does.
IMG_SPLT owns the same command name with a narrower DTO:
`ImagePickerSelectRequest {kind:"input"|"output"}` and
`ImagePickerSelectResult {kind:"input"|"output",path:PathRef}`. It selects exactly one directory,
is parented to the invoking window, and only a successful result establishes that window's root.

| App | Exact command | Request DTO (named DTO fields exact) | Response DTO | Errors | Source |
|---|---|---|---|---|---|
| Universal Converter | `dependencies_check` | `Empty {}` | `DependencyReport {is_ok:bool,missing:String[]}` | `Unavailable\|Internal` | `GUI/universal_converter_gui/src/main.rs`; planned `bridge/src/commands.rs` |
| Universal Converter | `classify_file` | `ClassifyFileRequest {path:String}` | `Classification {path:String,file_type:String,size_bytes:u64}` | `InvalidArgument\|NotFound\|Io` | same |
| Universal Converter | `scan_directory` | `ScanDirectoryRequest {directory:String,allowed_types:String[]}` | `ScanReport {directory:String,files:Classification[],total:u64}` | `InvalidArgument\|NotFound\|Io\|Cancelled` | same |
| Universal Converter | `batch_convert` | `BatchConvertRequest {files:PathRef[],output_directory:PathRef,output_format:String,overwrite:bool}` | `BatchConvertReport {output_directory:PathRef,converted:PathRef[],failed:PathRef[]}` | `InvalidArgument\|Conflict\|Io\|Cancelled` | planned `GUI/universal_converter_gui/bridge/src/commands.rs` |
| Universal Converter | `native_install` | `NativeInstallRequest` | `NativeOperationResult` | `InvalidArgument\|Forbidden\|Unavailable\|Conflict\|Io\|Cancelled` | `bridge/src/commands.rs`; Linux-only native mutation |
| Universal Converter | `native_uninstall` | `NativeUninstallRequest` | `NativeOperationResult` | `InvalidArgument\|Forbidden\|Unavailable\|NotFound\|Io\|Cancelled` | `bridge/src/commands.rs`; Linux-only native mutation |
| Universal Converter | `preferences_get` | `Empty {}` | `Preferences` | `Io\|Internal` | src/main.rs; planned bridge/src/commands.rs |
| Universal Converter | `preferences_set` | `Preferences` | `Preferences` | `Validation\|Io` | same |
| Universal Converter | `picker_select` | `PickerSelectRequest` | `PickerSelectResult` | `InvalidArgument\|NotFound\|Io\|Internal` | bridge-owned explicit native picker; planned bridge/src/commands.rs |
| IMG_SPLT | `settings_load` | `Empty {}` | `ImageSettings {default_distribution_mode:"balanced"|"greedy"|"fixed",max_files_per_folder:u64,fixed_folder_count:u64,max_retries:u64,min_upscale_width:u32,target_upscale_width:u32}` | `Io\|Internal` | `GUI/img_splt_gui/src/main.rs`; planned `bridge/src/commands.rs` |
| IMG_SPLT | `settings_save` | `ImageSettings` | `ImageSettings` | `Validation\|Io` | same |
| IMG_SPLT | `capabilities_check` | `Empty {}` | `CapabilityReport {ffmpeg:ToolStatus,ffprobe:ToolStatus}`; `ToolStatus {available:bool,version:String|null}` | `Unavailable\|Internal` | same |
| IMG_SPLT | `scan_images` | `ScanImagesRequest {directory:PathRef}` | `ImageScanReport {directory:PathRef,images:PathRef[],total:u64}` | `InvalidArgument\|NotFound\|Io\|Cancelled` | same |
| IMG_SPLT | `process_images` | `ProcessImagesRequest {input_directory:PathRef,files:PathRef[],output_directory:PathRef,output_format:String|null,upscale:bool,settings:ImageSettings}` | `ProcessReport {output_directory:PathRef,processed:PathRef[],failed:PathRef[]}` | `InvalidArgument\|Conflict\|Unavailable\|Io\|Cancelled` | planned bridge/src/commands.rs |
| IMG_SPLT | `distribute` | `DistributeRequest {input_directory:PathRef,files:PathRef[],output_directory:PathRef,chapter:u32|null,mode:"balanced"|"greedy"|"fixed",max_files_per_folder:u64,fixed_folder_count:u64}` | `DistributionReport {output_directory:PathRef,folders:PathRef[],distributed:u64}` | `InvalidArgument\|Conflict\|Io\|Cancelled` | planned bridge/src/commands.rs |
| IMG_SPLT | `preferences_get` | `Empty {}` | `Preferences` | `Io\|Internal` | src/main.rs; planned bridge/src/commands.rs |
| IMG_SPLT | `preferences_set` | `Preferences` | `Preferences` | `Validation\|Io` | same |
| IMG_SPLT | `picker_select` | `ImagePickerSelectRequest` | `ImagePickerSelectResult` | `InvalidArgument\|NotFound\|Io\|Internal` | bridge-owned explicit native directory picker; planned `bridge/src/commands.rs` |
| Universe Manager | `config_load` | `Empty {}` | `ManagerConfig {settings:{managed_dir:String},apps:AppEntry[]}` | `Io\|Internal` | `GUI/universe_manager_gui/src/main.rs`; planned `bridge/src/commands.rs` |
| Universe Manager | `config_save` | `ManagerConfig` | `ManagerConfig` | `Validation\|Io` | same |
| Universe Manager | `scan_apps` | `Empty {}` | `AppEntry[]` | `Io\|Cancelled` | same |
| Universe Manager | `detect_app` | `DetectAppRequest {path:PathRef}` | `DetectionReport {is_appimage:bool,suggested_name:String,executables:PathRef[],icons:PathRef[],desktop_templates:PathRef[]}` | `InvalidArgument\|NotFound\|Io\|Cancelled` | same |
| Universe Manager | `start_app` | `AppActionRequest {app_id:String,confirmed:bool}` | `OperationResult {app_id:String,operation:"start",completed:bool}` | `InvalidArgument\|NotFound\|Forbidden\|Io\|Cancelled` | same |
| Universe Manager | `stop_app` | `AppActionRequest {app_id:String,confirmed:bool}` | `OperationResult {app_id:String,operation:"stop",completed:bool}` | `InvalidArgument\|NotFound\|Forbidden\|Io\|Cancelled` | same |
| Universe Manager | `search_apps` | `SearchAppsRequest {query:String}` | `SearchReport {query:String,results:SearchResult[]}`; `SearchResult {name:String,id:String,version:String,source:String}` | `Validation\|Io\|Cancelled` | same |
| Universe Manager | `preferences_get` | `Empty {}` | `Preferences` | `Io\|Internal` | src/main.rs; planned bridge/src/commands.rs |
| Universe Manager | `preferences_set` | `Preferences` | `Preferences` | `Validation\|Io` | same |

### A.7 Event registry 1:1

`JobProgress<T> { request_id:String, completed:Option<u64>, total:Option<u64>,
message:Option<String>, result:Option<JobResult<T>> }` in Rust and `{ request_id:string,
completed:number|null, total:number|null, message:string|null, result:JobResult<T>|null }` in TS.
For a job topic, `started`/`progress` carry `result:null`; a terminal event carries the discriminated
`JobResult<T>` from A.1. This is the identical JSON shape after serde casing conversion. Each of the
three new frontend facades calls Tauri `listen` once per active job and invokes its returned
`UnlistenFn` exactly once on terminal event, component disposal, or window close.

| App | Exact topic | Kind and typed payload (Rust / TS) | Producer source | Listener source |
|---|---|---|---|---|
| Filen | `local-dir-changed` | raw `{}` / `{}` payload | `GUI/filen_gui/bridge/src/lib.rs` | `frontend/src/components/DualPaneExplorer.ts` |
| Filen | `auth:whoami-finished` | raw `WhoAmIPayload {email:Option<String>,error:Option<IpcError>}` / `{email:string|null,error:IpcError|null}` payload | bridge/src/lib.rs | none found |
| Filen | `transfer:progress` | raw `TransferProgressPayload {id:usize,progress:f32,bytes_done:u64,total_bytes:u64}` / `{id:number,progress:number,bytes_done:number,total_bytes:number}` payload | bridge/src/transfer_cmds.rs | none found |
| Filen | `transfer:finished` | raw `TransferFinishedPayload {id:usize,ok:bool,error:Option<IpcError>}` / `{id:number,ok:boolean,error:IpcError|null}` payload | bridge/src/transfer_cmds.rs | none found |
| Rclone | `local-dir-changed` | raw `{}` / `{}` payload | `GUI/rclone_gui/backend/src/logic/watcher.rs` | frontend/src/components/DualPaneExplorer.ts |
| Rclone | `transfer_progress` | raw `TransferProgress {id:u32,stats:TransferStats}` / `{id:number,stats:TransferStats}` payload | backend/src/logic/transfer.rs | frontend/src/features/transferManager.ts |
| OpenCode Manager | `arbiter://progress` | raw `ArbiterProgress {stage:String,message:String,current:usize,total:usize,scored:usize}` / `{stage:string,message:string,current:number,total:number,scored:number}` payload | `GUI/opencode_manager_gui/backend/src/api/arbiter.rs` | `frontend/src/pages/ModelsPage.tsx` |
| Subscription Manager | _no event_ | no producer or listener found | audited backend/frontend | audited backend/frontend |
| Universal Converter | `job.dependencies_check` | job `JobEvent<JobProgress<DependencyReport>>` / same TS | planned `GUI/universal_converter_gui/bridge/src/commands.rs` | planned local frontend IPC facade |
| Universal Converter | `job.classify_file` | job `JobEvent<JobProgress<Classification>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universal Converter | `job.scan_directory` | job `JobEvent<JobProgress<ScanReport>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universal Converter | `job.batch_convert` | job `JobEvent<JobProgress<BatchConvertReport>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universal Converter | `job.native_install` | job `JobEvent<JobProgress<NativeOperationResult>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universal Converter | `job.native_uninstall` | job `JobEvent<JobProgress<NativeOperationResult>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| IMG_SPLT | `job.capabilities_check` | job `JobEvent<JobProgress<CapabilityReport>>` / same TS | planned `GUI/img_splt_gui/bridge/src/commands.rs` | planned local frontend IPC facade |
| IMG_SPLT | `job.scan_images` | job `JobEvent<JobProgress<ImageScanReport>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| IMG_SPLT | `job.process_images` | job `JobEvent<JobProgress<ProcessReport>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| IMG_SPLT | `job.distribute` | job `JobEvent<JobProgress<DistributionReport>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universe Manager | `job.scan_apps` | job `JobEvent<JobProgress<AppEntry[]>>` / same TS | planned `GUI/universe_manager_gui/bridge/src/commands.rs` | planned local frontend IPC facade |
| Universe Manager | `job.detect_app` | job `JobEvent<JobProgress<DetectionReport>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universe Manager | `job.start_app` | job `JobEvent<JobProgress<OperationResult>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universe Manager | `job.stop_app` | job `JobEvent<JobProgress<OperationResult>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
| Universe Manager | `job.search_apps` | job `JobEvent<JobProgress<SearchReport>>` / same TS | planned bridge/src/commands.rs | planned local frontend IPC facade |
