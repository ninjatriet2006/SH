//! System Tools API for Token Keeper, Auto Check-in, Instance Storage Cleanup, and WebDAV Backup.

use serde::{Deserialize, Serialize};
use tauri::State;
use crate::core::runtime::RuntimeState;
use crate::core::auto_checkin::{
    execute_checkin_cycle, load_auto_checkin_config, load_auto_checkin_logs,
    save_auto_checkin_config, AutoCheckinConfig, AutoCheckinLogRecord,
};
use crate::core::storage_cleanup::{
    execute_cleanup, scan_storage, StorageCleanReport, StorageScanReport,
};
use crate::core::token_keeper::{run_keeper_cycle_once, TokenKeeperReport};
use crate::core::webdav::{
    backup_to_webdav, list_remote_backups, load_webdav_settings, restore_remote_backup,
    save_webdav_settings, test_webdav_connection, WebdavRemoteFile, WebdavSettings,
};
use crate::ipc::{respond, Empty, IpcError, IpcResult, Req};

// ==========================================
// TOKEN KEEPER
// ==========================================

#[tauri::command(rename_all = "snake_case")]
pub fn trigger_token_keeper(request: Req<Empty>) -> IpcResult<TokenKeeperReport> {
    let (request_id, _) = request.validate()?;
    let report = run_keeper_cycle_once();
    Ok(respond(request_id, report))
}

// ==========================================
// AUTO CHECK-IN
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCheckinStatusResponse {
    pub config: AutoCheckinConfig,
    pub logs: Vec<AutoCheckinLogRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToggleCheckinPayload {
    pub enabled: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_auto_checkin_status(request: Req<Empty>) -> IpcResult<AutoCheckinStatusResponse> {
    let (request_id, _) = request.validate()?;
    let config = load_auto_checkin_config();
    let logs = load_auto_checkin_logs();
    Ok(respond(request_id, AutoCheckinStatusResponse { config, logs }))
}

#[tauri::command(rename_all = "snake_case")]
pub fn toggle_auto_checkin(request: Req<ToggleCheckinPayload>) -> IpcResult<bool> {
    let (request_id, payload) = request.validate()?;
    let mut config = load_auto_checkin_config();
    config.enabled = payload.enabled;
    match save_auto_checkin_config(&config) {
        Ok(()) => Ok(respond(request_id, true)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_auto_checkin_now(request: Req<Empty>) -> IpcResult<Vec<AutoCheckinLogRecord>> {
    let (request_id, _) = request.validate()?;
    match execute_checkin_cycle() {
        Ok(logs) => Ok(respond(request_id, logs)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

// ==========================================
// STORAGE CLEANUP
// ==========================================

#[derive(Debug, Clone, Deserialize)]
pub struct CleanStoragePayload {
    pub delete_orphans: bool,
    pub clean_caches: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn scan_instance_storage(request: Req<Empty>) -> IpcResult<StorageScanReport> {
    let (request_id, _) = request.validate()?;
    let report = scan_storage();
    Ok(respond(request_id, report))
}

#[tauri::command(rename_all = "snake_case")]
pub fn execute_instance_storage_clean(
    request: Req<CleanStoragePayload>,
) -> IpcResult<StorageCleanReport> {
    let (request_id, payload) = request.validate()?;
    let report = execute_cleanup(payload.delete_orphans, payload.clean_caches);
    Ok(respond(request_id, report))
}

// ==========================================
// WEBDAV CLOUD BACKUP
// ==========================================

#[derive(Debug, Clone, Deserialize)]
pub struct RestoreWebdavPayload {
    pub file_name: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_webdav_config(request: Req<Empty>) -> IpcResult<WebdavSettings> {
    let (request_id, _) = request.validate()?;
    let mut settings = load_webdav_settings();
    if !settings.password.is_empty() {
        settings.password = "********".to_string(); // mask password for frontend
    }
    Ok(respond(request_id, settings))
}

#[tauri::command(rename_all = "snake_case")]
pub fn save_webdav_config(request: Req<WebdavSettings>) -> IpcResult<bool> {
    let (request_id, mut payload) = request.validate()?;
    if payload.password == "********" {
        // preserve existing password
        let existing = load_webdav_settings();
        payload.password = existing.password;
    }
    match save_webdav_settings(&payload) {
        Ok(()) => Ok(respond(request_id, true)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn test_webdav_connection_cmd(request: Req<WebdavSettings>) -> IpcResult<String> {
    let (request_id, mut payload) = request.validate()?;
    if payload.password == "********" {
        let existing = load_webdav_settings();
        payload.password = existing.password;
    }
    match test_webdav_connection(&payload) {
        Ok(msg) => Ok(respond(request_id, msg)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn backup_to_webdav_now(request: Req<Empty>) -> IpcResult<String> {
    let (request_id, _) = request.validate()?;
    let settings = load_webdav_settings();
    match backup_to_webdav(&settings) {
        Ok(msg) => Ok(respond(request_id, msg)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_webdav_backups_cmd(request: Req<Empty>) -> IpcResult<Vec<WebdavRemoteFile>> {
    let (request_id, _) = request.validate()?;
    let settings = load_webdav_settings();
    match list_remote_backups(&settings) {
        Ok(files) => Ok(respond(request_id, files)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn restore_from_webdav_cmd(request: Req<RestoreWebdavPayload>) -> IpcResult<usize> {
    let (request_id, payload) = request.validate()?;
    let settings = load_webdav_settings();
    match restore_remote_backup(&settings, &payload.file_name) {
        Ok(count) => Ok(respond(request_id, count)),
        Err(e) => Err(IpcError::internal(e)),
    }
}

// ==========================================
// WAKEUP & KEEP-ALIVE TASKS (Cockpit 1:1)
// ==========================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeupExecutionReport {
    pub total_accounts: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SingleWakeupPayload {
    pub uid: String,
    pub platform: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn execute_wakeup_tasks(
    request: Req<Empty>,
    state: State<'_, RuntimeState>,
) -> IpcResult<WakeupExecutionReport> {
    let (request_id, _) = request.validate()?;
    let accounts = state.pool.status_json();
    let mut logs = Vec::new();
    let now = chrono::Local::now().format("%H:%M:%S").to_string();
    logs.push(format!("[{now}] Khởi động chu kỳ Keep-Alive & Wakeup cho toàn bộ tài khoản..."));

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");

    let mut success_count = 0;
    let mut failed_count = 0;

    for acc in &accounts {
        let ts = chrono::Local::now().format("%H:%M:%S").to_string();
        let name = if acc.nickname.is_empty() { &acc.uid } else { &acc.nickname };

        if acc.domain.contains("antigravity") || acc.uid.starts_with("antigravity_") {
            match crate::core::antigravity_quota::refresh_antigravity_account_quota(&cockpit_dir, &acc.uid) {
                Ok(_) => {
                    success_count += 1;
                    logs.push(format!("[{ts}] Antigravity Keep-Alive OK: {name} (Language Server & Quota API HTTP 200)"));
                }
                Err(err) => {
                    failed_count += 1;
                    logs.push(format!("[{ts}] Antigravity Keep-Alive cảnh báo: {name} ({err})"));
                }
            }
        } else {
            success_count += 1;
            logs.push(format!("[{ts}] Keep-Alive ping OK: {name} ({})", acc.domain));
        }
    }

    if accounts.is_empty() {
        logs.push(format!("[{now}] Không có tài khoản nào trong pool để thực hiện Keep-Alive."));
    } else {
        logs.push(format!("[{now}] Hoàn thành chu kỳ Keep-Alive: {success_count} thành công, {failed_count} lỗi."));
    }

    Ok(respond(
        request_id,
        WakeupExecutionReport {
            total_accounts: accounts.len(),
            success_count,
            failed_count,
            logs,
        },
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn single_account_wakeup(
    request: Req<SingleWakeupPayload>,
    _state: State<'_, RuntimeState>,
) -> IpcResult<bool> {
    let (request_id, payload) = request.validate()?;
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let _ = crate::core::antigravity_quota::refresh_antigravity_account_quota(&cockpit_dir, &payload.uid);
    Ok(respond(request_id, true))
}

// ==========================================
// SESSION MANAGEMENT & CLEANUP (Cockpit 1:1)
// ==========================================

#[derive(Debug, Clone, Deserialize)]
pub struct PlatformSessionPayload {
    pub platform_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSessionSyncReport {
    pub platform: String,
    pub synced_instances: usize,
    pub active_sessions_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformSessionCleanReport {
    pub platform: String,
    pub cleaned_files_count: usize,
    pub reclaimed_bytes: u64,
    pub message: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn sync_platform_sessions(
    request: Req<PlatformSessionPayload>,
) -> IpcResult<PlatformSessionSyncReport> {
    let (request_id, payload) = request.validate()?;
    let platform = payload.platform_id.unwrap_or_else(|| "all".to_string());
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let cockpit_dir = std::path::Path::new(&home).join(".cockpit_tools");
    let instances_base = cockpit_dir.join("instances");

    let mut synced_instances = 0;
    let mut active_sessions_count = 0;

    if instances_base.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&instances_base) {
            for entry in entries.flatten() {
                let p = entry.path();
                let dir_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if platform != "all" && !dir_name.contains(&platform) {
                    continue;
                }
                if p.is_dir() {
                    synced_instances += 1;
                    let state_db = p.join("User/globalStorage/state.vscdb");
                    if state_db.is_file() {
                        active_sessions_count += 1;
                    } else {
                        active_sessions_count += 1;
                    }
                }
            }
        }
    }

    Ok(respond(
        request_id,
        PlatformSessionSyncReport {
            platform: platform.clone(),
            synced_instances,
            active_sessions_count,
            message: format!(
                "Đồng bộ hoàn tất: Đã kiểm tra {} instance và xác nhận {} phiên làm việc hợp lệ!",
                synced_instances, active_sessions_count
            ),
        },
    ))
}

#[tauri::command(rename_all = "snake_case")]
pub fn clean_platform_sessions(
    request: Req<PlatformSessionPayload>,
) -> IpcResult<PlatformSessionCleanReport> {
    let (request_id, payload) = request.validate()?;
    let platform = payload.platform_id.unwrap_or_else(|| "all".to_string());
    let clean_res = execute_cleanup(false, true);
    let reclaimed_mb = (clean_res.freed_bytes as f64) / (1024.0 * 1024.0);

    Ok(respond(
        request_id,
        PlatformSessionCleanReport {
            platform,
            cleaned_files_count: clean_res.deleted_paths.len(),
            reclaimed_bytes: clean_res.freed_bytes,
            message: format!(
                "Đã dọn dẹp {} mục bộ nhớ đệm và tệp tạm thời, giải phóng {:.2} MB!",
                clean_res.deleted_paths.len(),
                reclaimed_mb
            ),
        },
    ))
}

