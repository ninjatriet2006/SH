/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints chỉ-đọc / xem File (Command Tauri).
- Trách nhiệm: Nhận request từ Frontend, gọi tầng `logic`/`actions` để phân tích và thực thi.
- Tương tác: Giao tiếp trực tiếp với Frontend. Không đổi tên lệnh IPC, struct DTO hay JSON.
*/
// UNIVERSAL: S2 chẻ xem/sửa — file này giữ nhánh xem (read-only + món hệ điều hành đọc/mở).
// Mỗi lệnh đúng 1 pub command fn: validate Req → gọi actions trực tiếp → map backend_error.

use super::envelope::{Empty, IpcResult, Req, backend_error, success, validate};
use crate::actions::conflicts::ConflictInfo;
use crate::actions::list::FileItem;
use crate::actions::search::SearchResultItem;
use crate::actions::system::UserPlace;

crate::payload!(ViewPathPayload { path: String });
crate::payload!(ListFilesPayload { path: String, pane: Option<String> });
crate::payload!(ViewConflictsPayload { srcs: Vec<String>, dest_path: String });
crate::payload!(SearchPayload { path: String, query: String });

#[tauri::command]
pub async fn fs_check_conflicts(
    app_handle: tauri::AppHandle,
    request: Req<ViewConflictsPayload>,
) -> IpcResult<Vec<ConflictInfo>> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::actions::conflicts::check_conflicts(app_handle, payload.srcs, payload.dest_path)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn list_files(
    app_handle: tauri::AppHandle,
    request: Req<ListFilesPayload>,
) -> IpcResult<Vec<FileItem>> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::actions::list::execute_list(app_handle, payload.path, payload.pane)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_stat_advanced(request: Req<ViewPathPayload>) -> IpcResult<crate::actions::stat::StatInfo> {
    let request_id = validate(&request)?;
    crate::actions::stat::execute_stat(request.payload.path)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_search(request: Req<SearchPayload>) -> IpcResult<Vec<SearchResultItem>> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    crate::actions::search::execute_search(payload.path, payload.query)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn get_home_dir(request: Req<Empty>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    crate::actions::system::get_home_dir()
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn get_user_places(request: Req<Empty>) -> IpcResult<Vec<UserPlace>> {
    let request_id = validate(&request)?;
    crate::actions::system::get_user_places()
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn open_in_terminal(request: Req<ViewPathPayload>) -> IpcResult<()> {
    let request_id = validate(&request)?;
    crate::actions::system::open_in_terminal(request.payload.path)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn fs_get_thumbnail(request: Req<ViewPathPayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    crate::actions::view::execute_thumbnail(request.payload.path)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub fn fs_temp_dir(request: Req<Empty>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    Ok(success(request_id, crate::actions::system::fs_temp_dir()))
}
