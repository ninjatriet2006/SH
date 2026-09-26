/*
[INTEGRITY NOTES]
- Mục đích: API quản lý dịch vụ Mount qua Systemd.
- Trách nhiệm: Wrapper mỏng — kiểm `confirmed` rồi `fastlane` về actions.
- Chuẩn hóa: Enveloped IPC Pattern (A.1 Contract) tương thích chuẩn `subscription_manager_gui`.
*/

use crate::actions::mount_editor::MountConfig;
use crate::actions::mount_query::SystemdServiceInfo;
use crate::actions::{mount_control, mount_editor, mount_query};
use crate::ipc::{async_command_result, Empty, IpcErrorCode, IpcResult, Req};
use crate::logic::fastlane::fastlane;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMountRequest {
    pub config: MountConfig,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMountRequest {
    pub service_name: String,
    pub is_user: bool,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManageMountRequest {
    pub service_name: String,
    pub is_user: bool,
    pub action: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetMountConfigRequest {
    pub service_name: String,
    pub is_user: bool,
}

#[tauri::command]
pub async fn check_fuse_installed(request: Req<Empty>) -> IpcResult<bool> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(mount_query::fuse_installed).await
    })
    .await
}

#[tauri::command]
pub async fn create_mount_service(request: Req<CreateMountRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        mount_editor::validate_mount_config(&p.config)?;
        if !p.config.is_user_level {
            mount_editor::require_confirmation(p.confirmed)?;
        }
        fastlane(move || {
            let rclone_path = mount_editor::resolve_rclone_path()?;
            let content = mount_editor::render_unit(&p.config, &rclone_path);
            mount_editor::write_service_file(
                &p.config.service_name,
                p.config.is_user_level,
                &content,
            )?;
            Ok("Tạo systemd service thành công!".to_string())
        })
        .await
    })
    .await
}

#[tauri::command]
pub async fn delete_mount_service(request: Req<DeleteMountRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        mount_editor::validate_service_name(&p.service_name)?;
        mount_editor::require_confirmation(p.confirmed)?;
        fastlane(move || {
            let _ = mount_control::run_action(&p.service_name, p.is_user, "stop");
            let _ = mount_control::run_action(&p.service_name, p.is_user, "disable");
            mount_editor::remove_service_file(&p.service_name, p.is_user)?;
            Ok("Đã xoá systemd service.".to_string())
        })
        .await
    })
    .await
}

#[tauri::command]
pub async fn manage_mount_service(request: Req<ManageMountRequest>) -> IpcResult<String> {
    async_command_result(request, IpcErrorCode::Validation, |p| async move {
        mount_editor::validate_service_name(&p.service_name)?;
        mount_editor::validate_action(&p.action)?;
        if !p.is_user || matches!(p.action.as_str(), "stop" | "disable" | "restart") {
            mount_editor::require_confirmation(p.confirmed)?;
        }
        fastlane(move || mount_control::run_action(&p.service_name, p.is_user, &p.action)).await
    })
    .await
}

#[tauri::command]
pub async fn list_mount_services(request: Req<Empty>) -> IpcResult<Vec<SystemdServiceInfo>> {
    async_command_result(request, IpcErrorCode::Io, |_| async move {
        fastlane(mount_query::scan_mount_services).await
    })
    .await
}

#[tauri::command]
pub async fn get_mount_service_config(
    request: Req<GetMountConfigRequest>,
) -> IpcResult<MountConfig> {
    async_command_result(request, IpcErrorCode::Io, |p| async move {
        mount_editor::validate_service_name(&p.service_name)?;
        fastlane(move || mount_query::read_service_config(&p.service_name, p.is_user)).await
    })
    .await
}
