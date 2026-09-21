/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints quản lý Remote (giữ nguyên tên IPC/api cũ).
- Trách nhiệm: Tầng API mỏng — nhận request từ Frontend rồi chuyển cho
  `actions::{remote_view,remote_edit}` qua `logic::fastlane::fastlane`. Không chạy rclone trực tiếp.
- Tương tác: Được gọi từ frontend qua Tauri command (`list_remotes`, `create_remote`...).
*/
// Mỗi lệnh đúng 1 pub command fn: validate Req → gọi actions trực tiếp → map backend_error.

use super::envelope::{Empty, IpcResult, Req, backend_error, success, validate};
use crate::actions::checkfeature;
// Worker actions gọi qua path module (không alias, không import trùng tên lệnh).
use crate::actions::{checksize, remote_edit, remote_view};
use crate::logic::fastlane::fastlane;
use serde_json::Value;
use std::collections::HashMap;

crate::payload!(RemoteCreatePayload { name: String, provider: String, options: HashMap<String, String> });
crate::payload!(RemoteUpdatePayload { name: String, options: HashMap<String, String> });
crate::payload!(RemoteNamePayload { name: String });
crate::payload!(RemotePayload { remote: String });
crate::payload!(CapabilityPayload { src: String, dst: String });

#[tauri::command]
pub async fn list_remotes(request: Req<Empty>) -> IpcResult<Vec<Value>> {
    let request_id = validate(&request)?;
    fastlane(remote_view::list_remotes)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn get_providers(request: Req<Empty>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    fastlane(remote_view::get_providers)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn create_remote(request: Req<RemoteCreatePayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    fastlane(move || remote_edit::create_remote(&payload.name, &payload.provider, &payload.options))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn update_remote(request: Req<RemoteUpdatePayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    fastlane(move || remote_edit::update_remote(&payload.name, &payload.options))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn delete_remote(request: Req<RemoteNamePayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let name = request.payload.name;
    fastlane(move || remote_edit::delete_remote(&name))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn get_backend_features(request: Req<RemotePayload>) -> IpcResult<Value> {
    let request_id = validate(&request)?;
    let remote = request.payload.remote;
    fastlane(move || checkfeature::query_backend_features(&remote))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn check_transfer_capability(request: Req<CapabilityPayload>) -> IpcResult<Value> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    fastlane(move || checkfeature::query_transfer_options(&payload.src, &payload.dst))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn rclone_about(request: Req<RemotePayload>) -> IpcResult<Value> {
    let request_id = validate(&request)?;
    let remote = request.payload.remote;
    fastlane(move || checksize::about(&remote))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn rclone_size(request: Req<RemotePayload>) -> IpcResult<Value> {
    let request_id = validate(&request)?;
    let remote = request.payload.remote;
    fastlane(move || checksize::size(&remote))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}
