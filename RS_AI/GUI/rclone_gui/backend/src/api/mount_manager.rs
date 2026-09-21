//! UNIVERSAL S2 mount micro: wrapper mỏng — kiểm `confirmed` rồi `fastlane` về actions.
//! Tên hàm giữ NGUYÊN so với `api::mount` cũ. Lệnh trần `Result<T, String>`.

use crate::actions::mount_editor::MountConfig;
use crate::actions::mount_query::SystemdServiceInfo;
use crate::actions::{mount_control, mount_editor, mount_query};
use crate::logic::fastlane::fastlane;

#[tauri::command]
pub async fn check_fuse_installed() -> Result<bool, String> {
    fastlane(mount_query::fuse_installed).await
}

#[tauri::command]
pub async fn create_mount_service(config: MountConfig, confirmed: bool) -> Result<String, String> {
    mount_editor::validate_mount_config(&config)?;
    if !config.is_user_level {
        mount_editor::require_confirmation(confirmed)?;
    }
    fastlane(move || {
        let rclone_path = mount_editor::resolve_rclone_path()?;
        let content = mount_editor::render_unit(&config, &rclone_path);
        mount_editor::write_service_file(&config.service_name, config.is_user_level, &content)?;
        Ok("Tạo systemd service thành công!".to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_mount_service(
    service_name: String,
    is_user: bool,
    confirmed: bool,
) -> Result<String, String> {
    mount_editor::validate_service_name(&service_name)?;
    mount_editor::require_confirmation(confirmed)?;
    fastlane(move || {
        let _ = mount_control::run_action(&service_name, is_user, "stop");
        let _ = mount_control::run_action(&service_name, is_user, "disable");
        mount_editor::remove_service_file(&service_name, is_user)?;
        Ok("Đã xoá systemd service.".to_string())
    })
    .await
}

#[tauri::command]
pub async fn manage_mount_service(
    service_name: String,
    is_user: bool,
    action: String,
    confirmed: bool,
) -> Result<String, String> {
    mount_editor::validate_service_name(&service_name)?;
    mount_editor::validate_action(&action)?;
    if !is_user || matches!(action.as_str(), "stop" | "disable" | "restart") {
        mount_editor::require_confirmation(confirmed)?;
    }
    fastlane(move || mount_control::run_action(&service_name, is_user, &action)).await
}

#[tauri::command]
pub async fn list_mount_services() -> Result<Vec<SystemdServiceInfo>, String> {
    fastlane(mount_query::scan_mount_services).await
}

#[tauri::command]
pub async fn get_mount_service_config(
    service_name: String,
    is_user: bool,
) -> Result<MountConfig, String> {
    mount_editor::validate_service_name(&service_name)?;
    fastlane(move || mount_query::read_service_config(&service_name, is_user)).await
}
