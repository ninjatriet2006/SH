/*
[INTEGRITY NOTES]
- Mục đích: API cấu hình rclone + cờ engine toàn cục (Tauri commands).
- Trách nhiệm: Tầng API mỏng — gọi `logic::config_manager` / `settings::engine` qua fastlane.
- Tương tác: bare-core — lệnh trần `Result<T, String>`, không bao thư.
*/

use crate::logic::fastlane::fastlane;

#[tauri::command]
pub async fn get_config_content() -> Result<String, String> {
    crate::logic::config_manager::get_config_content().await
}

#[tauri::command]
pub async fn set_config_content(content: String) -> Result<(), String> {
    crate::logic::config_manager::set_config_content(content).await
}

#[tauri::command]
pub async fn reorder_config(names: Vec<String>) -> Result<(), String> {
    crate::logic::config_manager::reorder_config(names).await
}

#[tauri::command]
pub async fn list_config_snapshots() -> Result<Vec<String>, String> {
    crate::logic::config_manager::list_config_snapshots().await
}

#[tauri::command]
pub async fn restore_config_snapshot(name: String) -> Result<(), String> {
    crate::logic::config_manager::restore_config_snapshot(name).await
}

#[tauri::command]
pub async fn export_config_remote(name: String) -> Result<String, String> {
    crate::logic::config_manager::export_config_remote(name).await
}

#[tauri::command]
pub async fn import_config_remote(name: String, ini: String) -> Result<(), String> {
    crate::logic::config_manager::import_config_remote(name, ini).await
}

#[tauri::command]
pub async fn get_engine_flags() -> Result<crate::settings::engine::GlobalFlags, String> {
    fastlane(crate::settings::engine::load_engine_flags).await
}

#[tauri::command]
pub async fn set_engine_flags(
    flags: crate::settings::engine::GlobalFlags,
) -> Result<crate::settings::engine::GlobalFlags, String> {
    fastlane(move || {
        crate::settings::engine::save_engine_flags(&flags)?;
        Ok(flags)
    })
    .await
}
