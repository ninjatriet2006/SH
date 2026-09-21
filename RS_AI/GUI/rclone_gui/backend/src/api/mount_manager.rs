//! UNIVERSAL S2 mount micro: wrapper mỏng 1 dòng — kiểm `confirmed` rồi `fastlane` về actions.
//! Tên hàm + JSON giữ NGUYÊN so với `api::mount` cũ (IPC/frontend không đổi).
//! Mỗi lệnh đúng 1 pub command fn: validate Req → gọi actions trực tiếp → map backend_error.

use super::envelope::{Empty, IpcResult, Req, backend_error, success, validate};
use crate::actions::mount_editor::MountConfig;
use crate::actions::mount_query::SystemdServiceInfo;
use crate::actions::{mount_control, mount_editor, mount_query};
use crate::logic::fastlane::fastlane;

crate::payload!(MountCreatePayload { config: MountConfig, confirmed: bool });
crate::payload!(MountTargetPayload { service_name: String, is_user: bool });
crate::payload!(MountDeletePayload { service_name: String, is_user: bool, confirmed: bool });
crate::payload!(MountManagePayload { service_name: String, is_user: bool, action: String, confirmed: bool });

#[tauri::command]
pub async fn check_fuse_installed(request: Req<Empty>) -> IpcResult<bool> {
    let request_id = validate(&request)?;
    fastlane(mount_query::fuse_installed)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn create_mount_service(request: Req<MountCreatePayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    mount_editor::validate_mount_config(&payload.config).map_err(backend_error)?;
    if !payload.config.is_user_level {
        mount_editor::require_confirmation(payload.confirmed).map_err(backend_error)?;
    }
    let config = payload.config;
    fastlane(move || {
        let rclone_path = mount_editor::resolve_rclone_path()?;
        let content = mount_editor::render_unit(&config, &rclone_path);
        mount_editor::write_service_file(&config.service_name, config.is_user_level, &content)?;
        Ok("Tạo systemd service thành công!".to_string())
    })
    .await
    .map(|data| success(request_id, data))
    .map_err(backend_error)
}

#[tauri::command]
pub async fn delete_mount_service(request: Req<MountDeletePayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    mount_editor::validate_service_name(&payload.service_name).map_err(backend_error)?;
    mount_editor::require_confirmation(payload.confirmed).map_err(backend_error)?;
    let (service_name, is_user) = (payload.service_name, payload.is_user);
    fastlane(move || {
        let _ = mount_control::run_action(&service_name, is_user, "stop");
        let _ = mount_control::run_action(&service_name, is_user, "disable");
        mount_editor::remove_service_file(&service_name, is_user)?;
        Ok("Đã xoá systemd service.".to_string())
    })
    .await
    .map(|data| success(request_id, data))
    .map_err(backend_error)
}

#[tauri::command]
pub async fn manage_mount_service(request: Req<MountManagePayload>) -> IpcResult<String> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    mount_editor::validate_service_name(&payload.service_name).map_err(backend_error)?;
    mount_editor::validate_action(&payload.action).map_err(backend_error)?;
    if !payload.is_user || matches!(payload.action.as_str(), "stop" | "disable" | "restart") {
        mount_editor::require_confirmation(payload.confirmed).map_err(backend_error)?;
    }
    let (service_name, is_user, action) = (payload.service_name, payload.is_user, payload.action);
    fastlane(move || mount_control::run_action(&service_name, is_user, &action))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn list_mount_services(request: Req<Empty>) -> IpcResult<Vec<SystemdServiceInfo>> {
    let request_id = validate(&request)?;
    fastlane(mount_query::scan_mount_services)
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}

#[tauri::command]
pub async fn get_mount_service_config(
    request: Req<MountTargetPayload>,
) -> IpcResult<MountConfig> {
    let request_id = validate(&request)?;
    let payload = request.payload;
    mount_editor::validate_service_name(&payload.service_name).map_err(backend_error)?;
    let (service_name, is_user) = (payload.service_name, payload.is_user);
    fastlane(move || mount_query::read_service_config(&service_name, is_user))
        .await
        .map(|data| success(request_id, data))
        .map_err(backend_error)
}
