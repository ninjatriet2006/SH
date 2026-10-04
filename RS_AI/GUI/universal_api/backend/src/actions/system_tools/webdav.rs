//! WebDAV Cloud Backup, restore, and sync actions.

use serde::Deserialize;
use crate::core::webdav::{
    backup_to_webdav, list_remote_backups, load_webdav_settings, restore_remote_backup,
    save_webdav_settings, test_webdav_connection, WebdavRemoteFile, WebdavSettings,
};
use crate::ipc::{respond, Empty, IpcError, IpcResult, Req};

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
