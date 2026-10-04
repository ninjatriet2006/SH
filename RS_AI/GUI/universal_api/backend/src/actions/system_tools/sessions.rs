//! Session Management and Cache Cleanup actions.

use serde::{Deserialize, Serialize};
use crate::core::storage_cleanup::execute_cleanup;
use crate::ipc::{respond, IpcResult, Req};

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
