/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints quản lý Remote.
- Trách nhiệm: Tầng API mỏng — chuyển cho `actions::{remote_view,remote_edit}` qua fastlane.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::actions::{checksize, remote_edit, remote_view};
use crate::actions::{feature::checkcap, feature::getfeature};
use crate::ipc::{async_command_result, Empty, IpcErrorCode, IpcResult, Req};
use crate::logic::fastlane::fastlane;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRemoteRequest {
    pub name: String,
    pub provider: String,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRemoteRequest {
    pub name: String,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteNameRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTargetRequest {
    pub remote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrcDstRequest {
    pub src: String,
    pub dst: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHashRequest {
    pub path: String,
    pub hash_type: String,
}

#[tauri::command]
pub async fn list_remotes(request: Req<Empty>) -> IpcResult<Vec<Value>> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(remote_view::list_remotes).await
    })
    .await
}

#[tauri::command]
pub async fn get_providers(request: Req<Empty>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(remote_view::get_providers).await
    })
    .await
}

#[tauri::command]
pub async fn create_remote(request: Req<CreateRemoteRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        fastlane(move || remote_edit::create_remote(&p.name, &p.provider, &p.options)).await
    })
    .await
}

#[tauri::command]
pub async fn update_remote(request: Req<UpdateRemoteRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        fastlane(move || remote_edit::update_remote(&p.name, &p.options)).await
    })
    .await
}

#[tauri::command]
pub async fn delete_remote(request: Req<RemoteNameRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::NotFound, |p| async move {
        fastlane(move || remote_edit::delete_remote(&p.name)).await
    })
    .await
}

#[tauri::command]
pub async fn get_backend_features(request: Req<RemoteTargetRequest>) -> IpcResult<Value> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || getfeature::query_backend_features(&p.remote)).await
    })
    .await
}

#[tauri::command]
pub async fn get_feature_flags(
    request: Req<RemoteTargetRequest>,
) -> IpcResult<crate::actions::getfeature::BackendFeatures> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || crate::actions::getfeature::query_feature_flags(&p.remote)).await
    })
    .await
}

#[tauri::command]
pub async fn check_transfer_capability(request: Req<SrcDstRequest>) -> IpcResult<Value> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || Ok(checkcap::query_transfer_options(&p.src, &p.dst))).await
    })
    .await
}

#[tauri::command]
pub async fn rclone_about(request: Req<RemoteTargetRequest>) -> IpcResult<Value> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || checksize::about(&p.remote)).await
    })
    .await
}

#[tauri::command]
pub async fn rclone_size(request: Req<RemoteTargetRequest>) -> IpcResult<Value> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || checksize::size(&p.remote)).await
    })
    .await
}

#[tauri::command]
pub async fn get_file_hash(request: Req<FileHashRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || crate::actions::information::hash::execute_hashsum(&p.path, &p.hash_type))
            .await
    })
    .await
}

#[tauri::command]
pub async fn get_remote_hashes(request: Req<RemoteTargetRequest>) -> IpcResult<Vec<String>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || Ok(crate::actions::information::hash::get_supported_hashes(&p.remote)))
            .await
    })
    .await
}

#[tauri::command]
pub async fn check_files_integrity(
    request: Req<SrcDstRequest>,
) -> IpcResult<crate::actions::information::hash::IntegrityCheckResult> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        fastlane(move || crate::actions::information::hash::execute_check_integrity(&p.src, &p.dst))
            .await
    })
    .await
}
