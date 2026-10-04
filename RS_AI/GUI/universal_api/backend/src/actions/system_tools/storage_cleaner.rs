//! Instance and temporary cache storage cleanup actions.

use serde::Deserialize;
use crate::core::storage_cleanup::{
    execute_cleanup, scan_storage, StorageCleanReport, StorageScanReport,
};
use crate::ipc::{respond, Empty, IpcResult, Req};

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
