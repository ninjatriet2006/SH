/*
[INTEGRITY NOTES]
- Mục đích: API cấu hình rclone + cờ engine toàn cục (Tauri commands).
- Trách nhiệm: Tầng API mỏng — gọi `logic::config_manager` / `settings::engine` qua fastlane.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::ipc::{async_command_result, Empty, IpcErrorCode, IpcResult, Req};
use crate::logic::fastlane::fastlane;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigContentRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorderConfigRequest {
    pub names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotNameRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRemoteConfigRequest {
    pub name: String,
    pub ini: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetEngineFlagsRequest {
    pub flags: crate::settings::engine::EngineSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDebugSettingsRequest {
    pub settings: crate::settings::diagnostics::DebugSettings,
}

#[tauri::command]
pub async fn get_config_content(request: Req<Empty>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        crate::logic::config_manager::get_config_content().await
    })
    .await
}

#[tauri::command]
pub async fn set_config_content(request: Req<ConfigContentRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::logic::config_manager::set_config_content(p.content).await
    })
    .await
}

#[tauri::command]
pub async fn reorder_config(request: Req<ReorderConfigRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::logic::config_manager::reorder_config(p.names).await
    })
    .await
}

#[tauri::command]
pub async fn list_config_snapshots(request: Req<Empty>) -> IpcResult<Vec<String>> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        crate::logic::config_manager::list_config_snapshots().await
    })
    .await
}

#[tauri::command]
pub async fn restore_config_snapshot(request: Req<SnapshotNameRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::NotFound, |p| async move {
        crate::logic::config_manager::restore_config_snapshot(p.name).await
    })
    .await
}

#[tauri::command]
pub async fn export_config_remote(request: Req<SnapshotNameRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::NotFound, |p| async move {
        crate::logic::config_manager::export_config_remote(p.name).await
    })
    .await
}

#[tauri::command]
pub async fn import_config_remote(request: Req<ImportRemoteConfigRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Conflict, |p| async move {
        crate::logic::config_manager::import_config_remote(p.name, p.ini).await
    })
    .await
}

#[tauri::command]
pub async fn get_engine_flags(
    request: Req<Empty>,
) -> IpcResult<crate::settings::engine::EngineSettings> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(crate::settings::engine::load_engine_flags).await
    })
    .await
}

#[tauri::command]
pub async fn set_engine_flags(
    request: Req<SetEngineFlagsRequest>,
) -> IpcResult<crate::settings::engine::EngineSettings> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        fastlane(move || {
            crate::settings::engine::save_engine_flags(&p.flags)?;
            Ok(p.flags)
        })
        .await
    })
    .await
}

#[tauri::command]
pub async fn get_debug_settings(
    request: Req<Empty>,
) -> IpcResult<crate::settings::diagnostics::DebugSettings> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(crate::settings::diagnostics::load_debug_settings).await
    })
    .await
}

/// UNIVERSAL: đọc toàn bộ nội dung file `backend.log` cho DebugView qua API
/// (kéo dữ liệu theo yêu cầu từ file đĩa, không dùng event stream).
#[tauri::command]
pub async fn get_backend_log(request: Req<Empty>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(crate::core::debug::read_log).await
    })
    .await
}

#[tauri::command]
pub async fn clear_backend_log(request: Req<Empty>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(crate::core::debug::clear_log).await
    })
    .await
}

#[tauri::command]
pub async fn set_debug_settings(
    request: Req<SetDebugSettingsRequest>,
) -> IpcResult<crate::settings::diagnostics::DebugSettings> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        fastlane(move || {
            crate::settings::diagnostics::save_debug_settings(&p.settings)?;
            Ok(p.settings)
        })
        .await
    })
    .await
}
