//! System Tools API for Token Keeper, Auto Check-in, Instance Storage Cleanup, and WebDAV Backup.

use serde::{Deserialize, Serialize};
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
