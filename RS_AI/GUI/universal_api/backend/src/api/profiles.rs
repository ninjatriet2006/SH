//! Tauri IPC Endpoints cho Quản lý Profile và Instance.
//!
//! Tương tác trực tiếp giữa giao diện người dùng và hệ thống mô phỏng môi trường/profile.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::core::inject::vscode_db;
use crate::core::instance::{kill_instance, launch_instance, LaunchOptions};
use crate::core::profiles::{
    clone_profile, create_profile, delete_profile, load_profiles, update_profile, InstanceProfile,
};
use crate::core::runtime::RuntimeState;
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};

#[tauri::command(rename_all = "snake_case")]
pub fn list_profiles(request: Req<Empty>) -> IpcResult<Vec<InstanceProfile>> {
    let (request_id, _) = request.validate()?;
    let list = load_profiles().map_err(from_string)?;
    Ok(respond(request_id, list))
}

#[derive(Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    pub platform_id: String,
    pub bound_account_id: Option<String>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_new_profile(request: Req<CreateProfileRequest>) -> IpcResult<InstanceProfile> {
    let (request_id, payload) = request.validate()?;
    let profile = create_profile(
        &payload.name,
        &payload.platform_id,
        payload.bound_account_id,
        payload.extra_args,
    )
    .map_err(from_string)?;
    Ok(respond(request_id, profile))
}

#[derive(Deserialize)]
pub struct ProfileIdRequest {
    pub id: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_profile(request: Req<ProfileIdRequest>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    delete_profile(&payload.id).map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct CloneProfileRequest {
    pub id: String,
    pub new_name: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn duplicate_profile(request: Req<CloneProfileRequest>) -> IpcResult<InstanceProfile> {
    let (request_id, payload) = request.validate()?;
    let cloned = clone_profile(&payload.id, &payload.new_name).map_err(from_string)?;
    Ok(respond(request_id, cloned))
}

#[derive(Deserialize)]
pub struct BindAccountRequest {
    pub profile_id: String,
    pub account_uid: String,
}

/// Gán tài khoản vào profile và thực hiện In-place Injection vào file SQLite `state.vscdb`.
#[tauri::command(rename_all = "snake_case")]
pub fn bind_account_to_profile(
    request: Req<BindAccountRequest>,
    state: State<'_, RuntimeState>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let all = load_profiles().map_err(from_string)?;
    let profile = all
        .into_iter()
        .find(|p| p.id == payload.profile_id)
        .ok_or_else(|| from_string("Profile not found".into()))?;

    // Lấy token của account từ pool hoặc SQLite vault
    let store = state.storage().map_err(from_string)?;
    let accounts = store.list_accounts().map_err(from_string)?;
    let acct = accounts
        .into_iter()
        .find(|a| a.uid == payload.account_uid)
        .ok_or_else(|| from_string("Account not found in vault".into()))?;

    // Tiêm credential vào SQLite `state.vscdb` của profile
    match profile.platform_id.as_str() {
        "codebuddy_cn" => {
            vscode_db::inject_codebuddy_cn(&profile.user_data_dir, &acct.access_token)
                .map_err(from_string)?;
        }
        "codebuddy_global" => {
            vscode_db::inject_codebuddy_global(&profile.user_data_dir, &acct.access_token)
                .map_err(from_string)?;
        }
        "zed" => {
            use crate::core::platforms::zed::inject::inject_zed_credentials;
            let cred_path = profile.user_data_dir.join("credentials.json");
            inject_zed_credentials(&cred_path, &acct.uid, &acct.access_token)
                .map_err(from_string)?;
        }
        _ => {
            // Mặc định thử inject theo chuẩn CodeBuddy Global
            vscode_db::inject_codebuddy_global(&profile.user_data_dir, &acct.access_token)
                .map_err(from_string)?;
        }
    }

    // Cập nhật profile lưu bound_account_id
    update_profile(
        &payload.profile_id,
        None,
        Some(Some(payload.account_uid)),
        None,
    )
    .map_err(from_string)?;

    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct LaunchProfileRequest {
    pub profile_id: String,
    pub custom_binary_path: Option<String>,
}

#[derive(Serialize)]
pub struct LaunchProfileResponse {
    pub pid: u32,
    pub profile_id: String,
}

/// Khởi chạy tiến trình IDE gắn liền với Profile `--user-data-dir`.
#[tauri::command(rename_all = "snake_case")]
pub fn launch_profile_instance(request: Req<LaunchProfileRequest>) -> IpcResult<LaunchProfileResponse> {
    let (request_id, payload) = request.validate()?;
    let all = load_profiles().map_err(from_string)?;
    let profile = all
        .into_iter()
        .find(|p| p.id == payload.profile_id)
        .ok_or_else(|| from_string("Profile not found".into()))?;

    let custom_path = payload.custom_binary_path.map(PathBuf::from);
    let extra_args_vec = match &profile.extra_args {
        crate::core::profiles::model::ExtraArgsVal::Text(t) => {
            t.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>()
        }
        crate::core::profiles::model::ExtraArgsVal::List(l) => l.clone(),
    };

    let pid = launch_instance(LaunchOptions {
        platform_id: &profile.platform_id,
        user_data_dir: &profile.user_data_dir,
        custom_binary_path: custom_path.as_deref(),
        extra_args: &extra_args_vec,
        use_new_window: true,
    })
    .map_err(from_string)?;

    Ok(respond(
        request_id,
        LaunchProfileResponse {
            pid,
            profile_id: payload.profile_id,
        },
    ))
}

#[derive(Deserialize)]
pub struct KillInstanceRequest {
    pub pid: u32,
}

#[tauri::command(rename_all = "snake_case")]
pub fn kill_running_instance(request: Req<KillInstanceRequest>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    kill_instance(payload.pid).map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct ListPlatformInstancesRequest {
    pub platform: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_platform_instances(
    request: Req<ListPlatformInstancesRequest>,
) -> IpcResult<Vec<InstanceProfile>> {
    let (request_id, payload) = request.validate()?;
    let list = crate::core::profiles::load_platform_instances(&payload.platform).map_err(from_string)?;
    Ok(respond(request_id, list))
}

#[derive(Deserialize)]
pub struct CreatePlatformInstanceRequest {
    pub platform: String,
    pub name: String,
    #[serde(default = "default_init_mode")]
    pub init_mode: String,
    pub source_instance_id: Option<String>,
    pub existing_dir: Option<String>,
    pub bind_account_id: Option<String>,
    pub extra_args: Option<String>,
}

fn default_init_mode() -> String {
    "blank".to_string()
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_platform_instance(
    request: Req<CreatePlatformInstanceRequest>,
) -> IpcResult<InstanceProfile> {
    let (request_id, payload) = request.validate()?;
    let profile = crate::core::profiles::create_platform_instance(
        &payload.platform,
        &payload.name,
        &payload.init_mode,
        payload.source_instance_id.as_deref(),
        payload.existing_dir.as_deref(),
        payload.bind_account_id,
        payload.extra_args,
    )
    .map_err(from_string)?;
    Ok(respond(request_id, profile))
}

#[derive(Deserialize)]
pub struct UpdatePlatformInstanceRequest {
    pub platform: String,
    pub instance_id: String,
    pub name: Option<String>,
    pub bind_account_id: Option<Option<String>>,
    pub extra_args: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_platform_instance(
    request: Req<UpdatePlatformInstanceRequest>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    crate::core::profiles::update_platform_instance(
        &payload.platform,
        &payload.instance_id,
        payload.name,
        payload.bind_account_id,
        payload.extra_args,
    )
    .map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct DeletePlatformInstanceRequest {
    pub platform: String,
    pub instance_id: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_platform_instance(
    request: Req<DeletePlatformInstanceRequest>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    crate::core::profiles::delete_platform_instance(&payload.platform, &payload.instance_id)
        .map_err(from_string)?;
    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct LaunchPlatformInstanceRequest {
    pub platform: String,
    pub instance_id: String,
    pub custom_binary_path: Option<String>,
}

#[derive(Serialize)]
pub struct LaunchPlatformInstanceResponse {
    pub pid: u32,
    pub instance_id: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn launch_platform_instance(
    request: Req<LaunchPlatformInstanceRequest>,
) -> IpcResult<LaunchPlatformInstanceResponse> {
    let (request_id, payload) = request.validate()?;
    let instances = crate::core::profiles::load_platform_instances(&payload.platform)
        .map_err(from_string)?;
    let inst = instances
        .into_iter()
        .find(|p| p.id == payload.instance_id)
        .ok_or_else(|| from_string("Instance not found".into()))?;

    let custom_path = payload.custom_binary_path.map(PathBuf::from);
    let extra_args_vec = match &inst.extra_args {
        crate::core::profiles::model::ExtraArgsVal::Text(t) => {
            t.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>()
        }
        crate::core::profiles::model::ExtraArgsVal::List(l) => l.clone(),
    };

    let pid = launch_instance(LaunchOptions {
        platform_id: &payload.platform,
        user_data_dir: &inst.user_data_dir,
        custom_binary_path: custom_path.as_deref(),
        extra_args: &extra_args_vec,
        use_new_window: true,
    })
    .map_err(from_string)?;

    let _ = crate::core::profiles::record_platform_instance_launch(
        &payload.platform,
        &payload.instance_id,
        pid,
    );

    Ok(respond(
        request_id,
        LaunchPlatformInstanceResponse {
            pid,
            instance_id: payload.instance_id,
        },
    ))
}

#[derive(Deserialize)]
pub struct StopPlatformInstanceRequest {
    pub platform: String,
    pub instance_id: String,
    pub pid: Option<u32>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn stop_platform_instance(
    request: Req<StopPlatformInstanceRequest>,
) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let mut pid_to_kill = payload.pid;

    if pid_to_kill.is_none() {
        if let Ok(instances) = crate::core::profiles::load_platform_instances(&payload.platform) {
            if let Some(inst) = instances.into_iter().find(|p| p.id == payload.instance_id) {
                pid_to_kill = inst.last_pid;
            }
        }
    }

    if let Some(pid) = pid_to_kill {
        if crate::core::profiles::is_pid_alive(pid) {
            let _ = kill_instance(pid);
        }
    }

    let _ = crate::core::profiles::record_platform_instance_stop(&payload.platform, &payload.instance_id);
    Ok(respond(request_id, Empty {}))
}

#[derive(Deserialize)]
pub struct OpenFolderRequest {
    pub path: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn open_instance_folder(request: Req<OpenFolderRequest>) -> IpcResult<Empty> {
    let (request_id, payload) = request.validate()?;
    let p = payload.path.trim();
    if !p.is_empty() && p != "default" {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer").arg(p).spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg(p).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            let _ = std::process::Command::new("xdg-open").arg(p).spawn();
        }
    }
    Ok(respond(request_id, Empty {}))
}
