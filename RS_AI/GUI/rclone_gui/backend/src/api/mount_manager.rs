//! UNIVERSAL S2 mount micro: wrapper mỏng 1 dòng — kiểm `confirmed` rồi `fastlane` về actions.
//! Tên hàm + JSON giữ NGUYÊN so với `api::mount` cũ (IPC/frontend không đổi).

use crate::actions::{mount_control, mount_files, mount_query, mount_creator};
use crate::logic::fastlane::fastlane;

// UNIVERSAL: re-export để IPC/frontend dùng 1 đường type duy nhất.
pub use mount_query::SystemdServiceInfo;
pub use mount_creator::MountConfig;

/// UNIVERSAL: kiểm tra FUSE (không cần confirm, chỉ đọc).
pub async fn check_fuse_installed() -> Result<bool, String> {
    fastlane(mount_query::fuse_installed).await
}

/// UNIVERSAL: tạo unit từ MountConfig — system level đòi `confirmed=true`.
pub async fn create_mount_service(config: MountConfig, confirmed: bool) -> Result<String, String> {
    mount_creator::validate_mount_config(&config)?;
    if !config.is_user_level {
        mount_creator::require_confirmation(confirmed)?;
    }
    fastlane(move || {
        let rclone_path = mount_creator::resolve_rclone_path()?;
        let content = mount_creator::render_unit(&config, &rclone_path);
        mount_files::write_service_file(&config.service_name, config.is_user_level, &content)?;
        Ok("Tạo systemd service thành công!".to_string())
    })
    .await
}

/// UNIVERSAL: dừng + vô hiệu hóa rồi xóa unit — luôn đòi `confirmed=true`.
pub async fn delete_mount_service(service_name: String, is_user: bool, confirmed: bool) -> Result<String, String> {
    mount_creator::validate_service_name(&service_name)?;
    mount_creator::require_confirmation(confirmed)?;
    fastlane(move || {
        let _ = mount_control::run_action(&service_name, is_user, "stop");
        let _ = mount_control::run_action(&service_name, is_user, "disable");
        mount_files::remove_service_file(&service_name, is_user)?;
        Ok("Đã xoá systemd service.".to_string())
    })
    .await
}

/// UNIVERSAL: start/stop/enable/disable — system hoặc stop/disable/restart đòi confirm.
pub async fn manage_mount_service(
    service_name: String,
    is_user: bool,
    action: String,
    confirmed: bool,
) -> Result<String, String> {
    mount_creator::validate_service_name(&service_name)?;
    mount_creator::validate_action(&action)?;
    if !is_user || matches!(action.as_str(), "stop" | "disable" | "restart") {
        mount_creator::require_confirmation(confirmed)?;
    }
    fastlane(move || mount_control::run_action(&service_name, is_user, &action)).await
}

/// UNIVERSAL: liệt kê service rclone mount (chỉ đọc).
pub async fn list_mount_services() -> Result<Vec<SystemdServiceInfo>, String> {
    fastlane(mount_query::scan_mount_services).await
}

/// UNIVERSAL: đọc lại MountConfig từ file unit (chỉ đọc).
pub async fn get_mount_service_config(service_name: String, is_user: bool) -> Result<MountConfig, String> {
    mount_creator::validate_service_name(&service_name)?;
    fastlane(move || mount_query::read_service_config(&service_name, is_user)).await
}
