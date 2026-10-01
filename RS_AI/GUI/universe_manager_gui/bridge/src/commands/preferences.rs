use super::helpers::*;
use super::{response, validate};
use crate::contract::*;
use crate::preferences;
use tauri::{AppHandle, Runtime};

#[tauri::command]
pub fn preferences_get<R: Runtime>(
    request: Req<Empty>,
    app: AppHandle<R>,
) -> IpcResult<Preferences> {
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Internal)),
    };
    let path = preferences_path(&app)?;
    let value = match preferences::load_or_migrate(&path, None) {
        Ok(val) => val,
        Err(err) => return Err(io_error(err)),
    };
    Ok(response(request_id, value))
}

#[tauri::command]
pub fn preferences_set<R: Runtime>(
    request: Req<Preferences>,
    app: AppHandle<R>,
) -> IpcResult<Preferences> {
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Validation)),
    };
    validate_preferences(&request.payload)?;
    let path = match preferences_path(&app) {
        Ok(p) => p,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Io)),
    };
    match preferences::atomic_write(&path, &request.payload) {
        Ok(_) => Ok(response(request_id, request.payload)),
        Err(err) => Err(io_error(err)),
    }
}
