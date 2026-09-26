/*
[INTEGRITY NOTES]
- Mục đích: API Endpoints quản lý Remote (giữ nguyên tên IPC/api cũ).
- Trách nhiệm: Tầng API mỏng — chuyển cho `actions::{remote_view,remote_edit}`
  qua `logic::fastlane::fastlane`. Không chạy rclone trực tiếp.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

// Worker actions gọi qua path module (không alias, không import trùng tên lệnh).
use crate::actions::{checksize, remote_edit, remote_view};
use crate::actions::{feature::checkcap, feature::getfeature};
use crate::logic::fastlane::fastlane;
use serde_json::Value;
use std::collections::HashMap;

#[tauri::command]
pub async fn list_remotes() -> Result<Vec<Value>, String> {
    fastlane(remote_view::list_remotes).await
}

#[tauri::command]
pub async fn get_providers() -> Result<String, String> {
    fastlane(remote_view::get_providers).await
}

#[tauri::command]
pub async fn create_remote(
    name: String,
    provider: String,
    options: HashMap<String, String>,
) -> Result<String, String> {
    fastlane(move || remote_edit::create_remote(&name, &provider, &options)).await
}

#[tauri::command]
pub async fn update_remote(
    name: String,
    options: HashMap<String, String>,
) -> Result<String, String> {
    fastlane(move || remote_edit::update_remote(&name, &options)).await
}

#[tauri::command]
pub async fn delete_remote(name: String) -> Result<String, String> {
    fastlane(move || remote_edit::delete_remote(&name)).await
}

#[tauri::command]
pub async fn get_backend_features(remote: String) -> Result<Value, String> {
    fastlane(move || getfeature::query_backend_features(&remote)).await
}

/// UNIVERSAL: toàn bộ 52 cờ `Features` đã parse thành struct (thay vì JSON thô)
/// — frontend/bridge đọc field typed, không mò key PascalCase.
#[tauri::command]
pub async fn get_feature_flags(
    remote: String,
) -> Result<crate::actions::getfeature::BackendFeatures, String> {
    fastlane(move || crate::actions::getfeature::query_feature_flags(&remote)).await
}

#[tauri::command]
pub async fn check_transfer_capability(src: String, dst: String) -> Result<Value, String> {
    fastlane(move || Ok(checkcap::query_transfer_options(&src, &dst))).await
}

#[tauri::command]
pub async fn rclone_about(remote: String) -> Result<Value, String> {
    fastlane(move || checksize::about(&remote)).await
}

#[tauri::command]
pub async fn rclone_size(remote: String) -> Result<Value, String> {
    fastlane(move || checksize::size(&remote)).await
}

#[tauri::command]
pub async fn get_file_hash(path: String, hash_type: String) -> Result<String, String> {
    fastlane(move || crate::actions::information::hash::execute_hashsum(&path, &hash_type)).await
}

#[tauri::command]
pub async fn get_remote_hashes(remote: String) -> Result<Vec<String>, String> {
    fastlane(move || Ok(crate::actions::information::hash::get_supported_hashes(&remote))).await
}

#[tauri::command]
pub async fn check_files_integrity(
    src: String,
    dst: String,
) -> Result<crate::actions::information::hash::IntegrityCheckResult, String> {
    fastlane(move || crate::actions::information::hash::execute_check_integrity(&src, &dst)).await
}

