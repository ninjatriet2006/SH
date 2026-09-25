/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints chỉ-đọc / xem File (Command Tauri).
- Trách nhiệm: Nhận arg trực tiếp từ Frontend, gọi tầng `logic`/`actions`.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

use crate::actions::conflicts::ConflictInfo;
use crate::actions::list::FileItem;
use crate::actions::search::SearchResultItem;
use crate::actions::system::UserPlace;

#[tauri::command]
pub async fn fs_check_conflicts(
    srcs: Vec<String>,
    dest_path: String,
) -> Result<Vec<ConflictInfo>, String> {
    crate::actions::conflicts::check_conflicts(srcs, dest_path).await
}

#[tauri::command]
pub async fn list_files(
    app_handle: tauri::AppHandle,
    path: String,
    pane: Option<String>,
) -> Result<Vec<FileItem>, String> {
    crate::actions::list::execute_list(app_handle, path, pane).await
}

#[tauri::command]
pub async fn fs_stat_advanced(path: String) -> Result<crate::actions::stat::StatInfo, String> {
    crate::actions::stat::execute_stat(path).await
}

#[tauri::command]
pub async fn fs_search(path: String, query: String) -> Result<Vec<SearchResultItem>, String> {
    crate::actions::search::execute_search(path, query).await
}

#[tauri::command]
pub async fn get_home_dir() -> Result<String, String> {
    crate::actions::system::get_home_dir().await
}

#[tauri::command]
pub async fn get_user_places() -> Result<Vec<UserPlace>, String> {
    crate::actions::system::get_user_places().await
}

#[tauri::command]
pub async fn open_in_terminal(path: String) -> Result<(), String> {
    crate::actions::system::open_in_terminal(path).await
}

#[tauri::command]
/// UNIVERSAL: đuôi ngoài whitelist trả `Ok(None)` (UI hiện icon chung), không lỗi.
pub async fn fs_get_thumbnail(path: String) -> Result<Option<String>, String> {
    crate::actions::view::execute_thumbnail(path).await
}

#[tauri::command]
pub fn fs_temp_dir() -> Result<String, String> {
    Ok(crate::actions::system::fs_temp_dir())
}
