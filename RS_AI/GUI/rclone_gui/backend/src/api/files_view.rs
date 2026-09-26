/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints chỉ-đọc / xem File (Command Tauri).
- Trách nhiệm: Nhận Enveloped IPC `Req<T>` từ Frontend, gọi tầng `logic`/`actions`, trả `IpcResult<T>`.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::actions::conflicts::ConflictInfo;
use crate::actions::list::FileItem;
use crate::actions::search::SearchResultItem;
use crate::actions::system::UserPlace;
use crate::ipc::{
    async_command_result, command_result, deserialize_present_nullable, Empty, IpcErrorCode,
    IpcResult, Req,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsCheckConflictsRequest {
    pub srcs: Vec<String>,
    pub dest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsCheckRenameConflictRequest {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilesRequest {
    pub path: String,
    #[serde(default, deserialize_with = "deserialize_present_nullable")]
    pub pane: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub path: String,
    pub query: String,
}

#[tauri::command]
pub async fn fs_check_conflicts(
    request: Req<FsCheckConflictsRequest>,
) -> IpcResult<Vec<ConflictInfo>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::actions::conflicts::check_conflicts(p.srcs, p.dest_path).await
    })
    .await
}

#[tauri::command]
pub async fn fs_check_rename_conflict(
    request: Req<FsCheckRenameConflictRequest>,
) -> IpcResult<Option<ConflictInfo>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::logic::fastlane::fastlane(move || {
            crate::actions::conflicts::check_rename_conflict(&p.old_path, &p.new_path)
        })
        .await
    })
    .await
}

#[tauri::command]
pub async fn list_files(
    app_handle: tauri::AppHandle,
    request: Req<ListFilesRequest>,
) -> IpcResult<Vec<FileItem>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::actions::list::execute_list(app_handle, p.path, p.pane).await
    })
    .await
}

#[tauri::command]
pub async fn fs_stat_advanced(
    request: Req<PathRequest>,
) -> IpcResult<crate::actions::stat::StatInfo> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::actions::stat::execute_stat(p.path).await
    })
    .await
}

#[tauri::command]
pub async fn fs_search(request: Req<SearchRequest>) -> IpcResult<Vec<SearchResultItem>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::actions::search::execute_search(p.path, p.query).await
    })
    .await
}

#[tauri::command]
pub async fn get_home_dir(request: Req<Empty>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        crate::actions::system::get_home_dir().await
    })
    .await
}

#[tauri::command]
pub async fn get_user_places(request: Req<Empty>) -> IpcResult<Vec<UserPlace>> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        crate::actions::system::get_user_places().await
    })
    .await
}

#[tauri::command]
pub async fn open_in_terminal(request: Req<PathRequest>) -> IpcResult<()> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::actions::system::open_in_terminal(p.path).await
    })
    .await
}

#[tauri::command]
pub async fn fs_get_thumbnail(request: Req<PathRequest>) -> IpcResult<Option<String>> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        crate::actions::view::execute_thumbnail(p.path).await
    })
    .await
}

#[tauri::command]
pub fn fs_temp_dir(request: Req<Empty>) -> IpcResult<String> {
    command_result(request, IpcErrorCode::Io, |_| {
        Ok(crate::actions::system::fs_temp_dir())
    })
}
