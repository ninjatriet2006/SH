use crate::{actions::perm::Policy, api, core, logic::app_state::AppState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Req<T> {
    pub schema_version: u8,
    pub request_id: Option<String>,
    pub payload: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Res<T> {
    pub schema_version: u8,
    pub request_id: Option<String>,
    pub data: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    InvalidArgument,
    NotFound,
    Conflict,
    Unauthorized,
    Forbidden,
    Unavailable,
    Io,
    Validation,
    Cancelled,
    Internal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
}

pub type IpcResult<T> = Result<Res<T>, IpcError>;

fn validate<T>(request: &Req<T>) -> Result<Option<String>, IpcError> {
    if request.schema_version != 1 {
        return Err(error(IpcErrorCode::InvalidArgument, "schema_version must be 1"));
    }
    if request.request_id.is_some() {
        return Err(error(
            IpcErrorCode::InvalidArgument,
            "request_id must be null for non-job commands",
        ));
    }
    Ok(request.request_id.clone())
}

fn success<T>(request_id: Option<String>, data: T) -> Res<T> {
    Res {
        schema_version: 1,
        request_id,
        data,
    }
}

fn error(code: IpcErrorCode, message: impl Into<String>) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable: matches!(code, IpcErrorCode::Unavailable | IpcErrorCode::Io),
        details: None,
    }
}

fn backend_error(message: String) -> IpcError {
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("not found") || lower.contains("không tìm thấy") {
        IpcErrorCode::NotFound
    } else if lower.contains("invalid") || lower.contains("không hợp lệ") || lower.contains("thiếu ") {
        IpcErrorCode::InvalidArgument
    } else if lower.contains("cancel") || lower.contains("huỷ") {
        IpcErrorCode::Cancelled
    } else if lower.contains("permission") || lower.contains("quyền") || lower.contains("forbidden") {
        IpcErrorCode::Forbidden
    } else {
        IpcErrorCode::Io
    };
    error(code, message)
}

#[derive(Deserialize)]
pub struct Empty {}

macro_rules! payload {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(Deserialize)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

payload!(PathPayload { path: String });
payload!(ListFilesPayload { path: String, pane: Option<String> });
payload!(ConflictsPayload { srcs: Vec<String>, dest_path: String });
payload!(RenamePayload {
    old_path: String,
    new_path: String
});
payload!(TransferPayload { src: String, dst: String, task_id: Option<u32> });
payload!(TaskPayload { task_id: u32 });
payload!(SearchPayload {
    path: String,
    query: String
});
payload!(ChmodPayload {
    path: String,
    mode: u32
});
payload!(ChownPayload {
    path: String,
    uid: u32,
    gid: u32
});
payload!(OpenWithPayload { path: String, exec_cmd: Option<String>, app: Option<String> });
payload!(ClipboardSetPayload { items: Vec<core::sys::OSClipboardItem>, is_cut: bool });
payload!(FilesPayload { files: Vec<core::sys::SimpleFileItem> });
payload!(CustomActionPayload { exec_template: String, base_path: String, file_names: Vec<String> });
payload!(ItemPayload { item_id: String });
payload!(AccountPayload { account: Option<String> });
payload!(RemoteTrashItemPayload { account: Option<String>, path: String });
payload!(RemoteCreatePayload { name: String, provider: String, options: HashMap<String, String> });
payload!(RemoteUpdatePayload { name: String, options: HashMap<String, String> });
payload!(NamePayload { name: String });
payload!(RemotePayload { remote: String });
payload!(CapabilityPayload {
    src: String,
    dst: String
});
payload!(MountCreatePayload {
    config: api::mount::MountConfig,
    confirmed: bool
});
payload!(MountTargetPayload {
    service_name: String,
    is_user: bool
});
payload!(MountDeletePayload {
    service_name: String,
    is_user: bool,
    confirmed: bool
});
payload!(MountManagePayload {
    service_name: String,
    is_user: bool,
    action: String,
    confirmed: bool
});
payload!(ContentPayload { content: String });
payload!(NamesPayload { names: Vec<String> });
payload!(SnapshotPayload { name: String });
payload!(ImportRemotePayload { name: String, ini: String });
payload!(EngineFlagsPayload { flags: crate::settings::engine::GlobalFlags });
payload!(LangPayload { lang_code: String });
payload!(PermissionPolicyPayload { policy: String });
payload!(JobEnqueuePayload { kind: String, src: Option<String>, dst: Option<String> });
payload!(JobIdPayload { job_id: String });

macro_rules! async_command {
    ($name:ident, $payload:ty, $output:ty, $target:path, ($($field:ident),* $(,)?)) => {
        #[tauri::command]
        pub async fn $name(request: Req<$payload>) -> IpcResult<$output> {
            let request_id = validate(&request)?;
            #[allow(unused_variables)]
            let payload = request.payload;
            $target($(payload.$field),*)
                .await
                .map(|data| success(request_id, data))
                .map_err(backend_error)
        }
    };
}

macro_rules! sync_command {
    ($name:ident, $payload:ty, $output:ty, $target:path, ($($field:ident),* $(,)?)) => {
        #[tauri::command]
        pub fn $name(request: Req<$payload>) -> IpcResult<$output> {
            let request_id = validate(&request)?;
            #[allow(unused_variables)]
            let payload = request.payload;
            $target($(payload.$field),*)
                .map(|data| success(request_id, data))
                .map_err(backend_error)
        }
    };
}

#[tauri::command]
pub async fn fs_check_conflicts(
    app_handle: tauri::AppHandle,
    request: Req<ConflictsPayload>,
) -> IpcResult<Vec<api::files::ConflictInfo>> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    api::files::fs_check_conflicts(app_handle, payload.srcs, payload.dest_path)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn list_files(
    app_handle: tauri::AppHandle,
    request: Req<ListFilesPayload>,
) -> IpcResult<Vec<api::files::FileItem>> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    api::files::list_files(app_handle, payload.path, payload.pane)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_copy(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    request: Req<TransferPayload>,
) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    api::files::fs_copy(app_handle, state, payload.src, payload.dst, payload.task_id)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_move(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    request: Req<TransferPayload>,
) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    api::files::fs_move(app_handle, state, payload.src, payload.dst, payload.task_id)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_cancel(state: State<'_, AppState>, request: Req<TaskPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    api::files::fs_cancel(state, request.payload.task_id)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

async_command!(fs_mkdir, PathPayload, (), api::files::fs_mkdir, (path));
async_command!(fs_touch, PathPayload, (), api::files::fs_touch, (path));
async_command!(fs_delete, PathPayload, (), api::files::fs_delete, (path));
async_command!(
    fs_rename,
    RenamePayload,
    (),
    api::files::fs_rename,
    (old_path, new_path)
);
async_command!(
    fs_stat_advanced,
    PathPayload,
    api::files::StatInfo,
    api::files::fs_stat_advanced,
    (path)
);
async_command!(
    fs_search,
    SearchPayload,
    Vec<api::files::SearchResultItem>,
    api::files::fs_search,
    (path, query)
);
async_command!(get_home_dir, Empty, String, api::files::get_home_dir, ());
async_command!(
    get_user_places,
    Empty,
    Vec<api::files::UserPlace>,
    api::files::get_user_places,
    ()
);
async_command!(open_in_terminal, PathPayload, (), api::files::open_in_terminal, (path));
async_command!(
    fs_get_thumbnail,
    PathPayload,
    String,
    api::files::fs_get_thumbnail,
    (path)
);
/// S2: chmod tôn trọng policy — envelope/payload cũ giữ nguyên, policy đọc từ State.
#[tauri::command]
pub async fn fs_chmod(state: State<'_, AppState>, request: Req<ChmodPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    api::files::fs_chmod_with_policy(payload.path, payload.mode, policy)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

/// S2: chown tôn trọng policy — envelope/payload cũ giữ nguyên, policy đọc từ State.
#[tauri::command]
pub async fn fs_chown(state: State<'_, AppState>, request: Req<ChownPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    api::files::fs_chown_with_policy(payload.path, payload.uid, payload.gid, policy)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

/// S2: đọc policy hiện tại (`deny`/`ask_once`/`allow_system`).
#[tauri::command]
pub fn get_permission_policy(state: State<'_, AppState>, request: Req<Empty>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let policy = state.policy.lock().map(|p| *p).unwrap_or_default();
    Ok(success(request_id, policy.as_str().to_string()))
}

/// S2: đặt policy — sai chuỗi trả `InvalidArgument`, IPC cũ khác giữ nguyên.
#[tauri::command]
pub fn set_permission_policy(
    state: State<'_, AppState>,
    request: Req<PermissionPolicyPayload>,
) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let policy = Policy::parse(request.payload.policy.trim())
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "policy must be deny|ask_once|allow_system"))?;
    *state
        .policy
        .lock()
        .map_err(|_| error(IpcErrorCode::Internal, "policy lock poisoned"))? = policy;
    Ok(success(request_id, policy.as_str().to_string()))
}

#[tauri::command]
pub fn fs_temp_dir(request: Req<Empty>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    Ok(success(request_id, api::files::fs_temp_dir()))
}

async_command!(
    sys_open_with,
    OpenWithPayload,
    (),
    core::sys::sys_open_with,
    (path, exec_cmd, app)
);
async_command!(
    sys_list_apps,
    Empty,
    Vec<core::sys::DesktopApp>,
    core::sys::sys_list_apps,
    ()
);
async_command!(
    os_clipboard_set,
    ClipboardSetPayload,
    (),
    core::sys::os_clipboard_set,
    (items, is_cut)
);
async_command!(
    os_clipboard_get,
    Empty,
    Option<core::sys::OSClipboardData>,
    core::sys::os_clipboard_get,
    ()
);
async_command!(
    sys_get_custom_actions,
    Empty,
    Vec<core::sys::CustomAction>,
    core::sys::sys_get_custom_actions,
    ()
);
async_command!(
    sys_get_valid_actions,
    FilesPayload,
    Vec<core::sys::CustomAction>,
    core::sys::sys_get_valid_actions,
    (files)
);
async_command!(
    sys_execute_custom_action,
    CustomActionPayload,
    (),
    core::sys::sys_execute_custom_action,
    (exec_template, base_path, file_names)
);

async_command!(
    fs_trash_list_local,
    Empty,
    Vec<api::trash::TrashItemLocal>,
    api::trash::fs_trash_list_local,
    ()
);
async_command!(
    fs_trash_restore_local,
    ItemPayload,
    (),
    api::trash::fs_trash_restore_local,
    (item_id)
);
async_command!(
    fs_trash_delete_local,
    ItemPayload,
    (),
    api::trash::fs_trash_delete_local,
    (item_id)
);
async_command!(fs_trash_empty_local, Empty, (), api::trash::fs_trash_empty_local, ());
async_command!(
    fs_trash_list_remote_terminal,
    AccountPayload,
    Vec<api::files::FileItem>,
    api::trash::fs_trash_list_remote_terminal,
    (account)
);
async_command!(
    fs_trash_restore_remote_terminal,
    RemoteTrashItemPayload,
    (),
    api::trash::fs_trash_restore_remote_terminal,
    (account, path)
);
async_command!(
    fs_trash_delete_remote_terminal,
    RemoteTrashItemPayload,
    (),
    api::trash::fs_trash_delete_remote_terminal,
    (account, path)
);
async_command!(
    fs_trash_empty_remote_terminal,
    AccountPayload,
    (),
    api::trash::fs_trash_empty_remote_terminal,
    (account)
);

async_command!(list_remotes, Empty, Vec<Value>, api::remotes::list_remotes, ());
async_command!(get_providers, Empty, String, api::remotes::get_providers, ());
async_command!(
    create_remote,
    RemoteCreatePayload,
    String,
    api::remotes::create_remote,
    (name, provider, options)
);
async_command!(
    update_remote,
    RemoteUpdatePayload,
    String,
    api::remotes::update_remote,
    (name, options)
);
async_command!(delete_remote, NamePayload, String, api::remotes::delete_remote, (name));
async_command!(
    get_backend_features,
    RemotePayload,
    Value,
    api::remotes::get_backend_features,
    (remote)
);
async_command!(
    check_transfer_capability,
    CapabilityPayload,
    Value,
    api::remotes::check_transfer_capability,
    (src, dst)
);
async_command!(rclone_about, RemotePayload, Value, api::remotes::rclone_about, (remote));
async_command!(rclone_size, RemotePayload, Value, api::remotes::rclone_size, (remote));

async_command!(check_fuse_installed, Empty, bool, api::mount::check_fuse_installed, ());
async_command!(
    create_mount_service,
    MountCreatePayload,
    String,
    api::mount::create_mount_service,
    (config, confirmed)
);
async_command!(
    delete_mount_service,
    MountDeletePayload,
    String,
    api::mount::delete_mount_service,
    (service_name, is_user, confirmed)
);
async_command!(
    manage_mount_service,
    MountManagePayload,
    String,
    api::mount::manage_mount_service,
    (service_name, is_user, action, confirmed)
);
async_command!(
    list_mount_services,
    Empty,
    Vec<api::mount::SystemdServiceInfo>,
    api::mount::list_mount_services,
    ()
);
async_command!(
    get_mount_service_config,
    MountTargetPayload,
    api::mount::MountConfig,
    api::mount::get_mount_service_config,
    (service_name, is_user)
);

async_command!(get_config_content, Empty, String, crate::settings::config_manager::get_config_content, ());
async_command!(
    set_config_content,
    ContentPayload,
    (),
    crate::settings::config_manager::set_config_content,
    (content)
);
async_command!(reorder_config, NamesPayload, (), crate::settings::config_manager::reorder_config, (names));
async_command!(
    list_config_snapshots,
    Empty,
    Vec<String>,
    crate::settings::config_manager::list_config_snapshots,
    ()
);
async_command!(
    restore_config_snapshot,
    SnapshotPayload,
    (),
    crate::settings::config_manager::restore_config_snapshot,
    (name)
);
async_command!(
    export_config_remote,
    NamePayload,
    String,
    crate::settings::config_manager::export_config_remote,
    (name)
);
async_command!(
    import_config_remote,
    ImportRemotePayload,
    (),
    crate::settings::config_manager::import_config_remote,
    (name, ini)
);
async_command!(
    get_engine_flags,
    Empty,
    crate::settings::engine::GlobalFlags,
    crate::settings::engine::get_engine_flags,
    ()
);
async_command!(
    set_engine_flags,
    EngineFlagsPayload,
    crate::settings::engine::GlobalFlags,
    crate::settings::engine::set_engine_flags,
    (flags)
);
sync_command!(
    get_available_langs,
    Empty,
    Vec<String>,
    api::lang::get_available_langs,
    ()
);
sync_command!(
    get_lang_content,
    LangPayload,
    Value,
    api::lang::get_lang_content,
    (lang_code)
);
sync_command!(
    get_available_themes,
    Empty,
    Vec<api::appearance::ThemeInfo>,
    api::appearance::get_available_themes,
    ()
);
sync_command!(
    get_available_fonts,
    Empty,
    Vec<api::appearance::FontInfo>,
    api::appearance::get_available_fonts,
    ()
);

/// P1 job queue (song song, IPC cũ nguyên vẹn): enqueue/list/cancel.
/// Stub chưa gọi actions thật (P2); worker phát event `job_update`.
#[tauri::command]
pub async fn job_enqueue(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    request: Req<JobEnqueuePayload>,
) -> IpcResult<core::jobs::Job> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    let kind = core::jobs::JobKind::parse(&payload.kind)
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "kind must be copy|move|delete|list"))?;
    let job = state.jobs.enqueue(kind, payload.src, payload.dst);
    state.jobs.spawn_worker(app_handle);
    Ok(success(request_id, job))
}

/// P1: liệt kê snapshot toàn bộ job.
#[tauri::command]
pub async fn job_list(state: State<'_, AppState>, request: Req<Empty>) -> IpcResult<Vec<core::jobs::Job>> {
    let request_id = validate(&request)?;
    Ok(success(request_id, state.jobs.list()))
}

/// P1: hủy job (queued → cancelled ngay; running → cờ dừng bước tiếp).
#[tauri::command]
pub async fn job_cancel(
    state: State<'_, AppState>,
    request: Req<JobIdPayload>,
) -> IpcResult<core::jobs::Job> {
    let request_id = validate(&request)?;
    state
        .jobs
        .request_cancel(&request.payload.job_id)
        .map(|job| success(request_id, job))
        .map_err(backend_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serializes_exact_contract() {
        let value = serde_json::to_value(success(None, true)).expect("serialize response");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version": 1,
                "request_id": null,
                "data": true
            })
        );

        let failure = error(IpcErrorCode::InvalidArgument, "bad request");
        assert_eq!(
            serde_json::to_value(failure).expect("serialize error"),
            serde_json::json!({
                "code": "invalid_argument",
                "message": "bad request",
                "retryable": false,
                "details": null
            })
        );
    }

    #[test]
    fn rejects_wrong_schema_and_non_job_request_id() {
        let wrong_schema = Req {
            schema_version: 2,
            request_id: None,
            payload: Empty {},
        };
        assert_eq!(
            validate(&wrong_schema).expect_err("schema must fail").code,
            IpcErrorCode::InvalidArgument
        );

        let job_id = Req {
            schema_version: 1,
            request_id: Some("unexpected".into()),
            payload: Empty {},
        };
        assert_eq!(
            validate(&job_id).expect_err("request id must fail").code,
            IpcErrorCode::InvalidArgument
        );
    }
}
