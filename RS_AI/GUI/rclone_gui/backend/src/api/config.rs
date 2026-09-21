/*
[INTEGRITY NOTES]
- Mục đích: API cấu hình rclone + cờ engine toàn cục (Tauri commands).
- Trách nhiệm: Tầng API mỏng — validate Req rồi gọi `logic::config_manager` / `settings::engine` qua fastlane.
- Tương tác: Giữ nguyên tên lệnh Tauri + payload JSON + IpcError (dời từ `ipc.rs`).
*/
// Mỗi lệnh đúng 1 pub command fn: validate Req → gọi logic trực tiếp → map backend_error.

use super::envelope::{Empty, IpcResult, Req, backend_error, success, validate};
use crate::logic::fastlane::fastlane;

crate::payload!(ContentPayload { content: String });
crate::payload!(NamesPayload { names: Vec<String> });
crate::payload!(SnapshotPayload { name: String });
crate::payload!(ImportRemotePayload { name: String, ini: String });
crate::payload!(EngineFlagsPayload { flags: crate::settings::engine::GlobalFlags });

#[tauri::command]
pub async fn get_config_content(request: Req<Empty>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    crate::logic::config_manager::get_config_content()
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn set_config_content(request: Req<ContentPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    crate::logic::config_manager::set_config_content(request.payload.content)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn reorder_config(request: Req<NamesPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    crate::logic::config_manager::reorder_config(request.payload.names)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn list_config_snapshots(request: Req<Empty>) -> IpcResult<Vec<String>> {
    let request_id = validate(&request)?;
    crate::logic::config_manager::list_config_snapshots()
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn restore_config_snapshot(request: Req<SnapshotPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    crate::logic::config_manager::restore_config_snapshot(request.payload.name)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn export_config_remote(request: Req<SnapshotPayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    crate::logic::config_manager::export_config_remote(request.payload.name)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn import_config_remote(request: Req<ImportRemotePayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::logic::config_manager::import_config_remote(payload.name, payload.ini)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn get_engine_flags(
    request: Req<Empty>,
) -> IpcResult<crate::settings::engine::GlobalFlags> {
    let request_id = validate(&request)?;
    fastlane(crate::settings::engine::load_engine_flags)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn set_engine_flags(
    request: Req<EngineFlagsPayload>,
) -> IpcResult<crate::settings::engine::GlobalFlags> {
    let request_id = validate(&request)?;
    let flags = request.payload.flags;
    fastlane(move || {
        crate::settings::engine::save_engine_flags(&flags)?;
        Ok(flags)
    })
    .await
    .map(|data| success(request_id, data))
    .map_err(backend_error)
}
