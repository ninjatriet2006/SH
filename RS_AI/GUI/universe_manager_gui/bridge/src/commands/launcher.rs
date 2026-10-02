use super::helpers::*;
use super::{response, validate};
use crate::contract::*;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Runtime};
use universe_manager_backend as backend;

#[tauri::command]
pub fn launcher_update<R: Runtime>(
    request: Req<LauncherUpdateRequest>,
    app: AppHandle<R>,
) -> IpcResult<AppEntry> {
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Validation)),
    };

    let p = &request.payload;
    let exec_path = Path::new(&p.exec_path);
    if !exec_path.is_file() {
        return Err(error(
            IpcErrorCode::NotFound,
            format!("Tệp thực thi không tồn tại: {}", p.exec_path),
        ));
    }

    let config_dir = app_config_dir(&app)?;
    let mut config = match backend::load_config(config_dir.clone()) {
        Ok(cfg) => cfg,
        Err(failure) => return Err(remap(map_backend(failure), IpcErrorCode::Io)),
    };

    let app_entry = config
        .apps
        .iter_mut()
        .find(|a| a.id == p.app_id)
        .ok_or_else(|| error(IpcErrorCode::NotFound, format!("Không tìm thấy app: {}", p.app_id)))?;

    let exec_stem = exec_path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let existing_desktop = if !app_entry.desktop_file.trim().is_empty() {
        let path = Path::new(&app_entry.desktop_file);
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let stripped_stem = file_stem.strip_prefix("universe-").unwrap_or(&file_stem);
        if stripped_stem == exec_stem || file_stem == exec_stem {
            Some(path)
        } else {
            None
        }
    } else {
        None
    };

    let written_desktop = backend::create_or_update_desktop_launcher(
        exec_path,
        &p.name,
        p.icon_path.as_deref(),
        p.terminal,
        p.categories.as_deref(),
        p.arguments.as_deref(),
        p.startup_wm_class.as_deref(),
        existing_desktop,
    )
    .map_err(|e| remap(map_backend(e), IpcErrorCode::Io))?;

    app_entry.name = p.name.clone();
    app_entry.exec_path = p.exec_path.clone();
    app_entry.icon_path = p.icon_path.clone();
    app_entry.desktop_file = written_desktop.to_string_lossy().into_owned();

    let updated_copy = app_entry.clone();

    let store = backend::ConfigStore::new(config_dir, vec![])
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Internal))?;
    store
        .save(&config)
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Io))?;

    crate::debug::success(
        "LAUNCHER",
        format!(
            "Desktop launcher updated for '{}' -> {}",
            updated_copy.name, updated_copy.desktop_file
        ),
    );

    Ok(response(request_id, updated_copy))
}

#[tauri::command]
pub fn launcher_delete<R: Runtime>(
    request: Req<LauncherDeleteRequest>,
    app: AppHandle<R>,
) -> IpcResult<AppEntry> {
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Validation)),
    };

    let p = &request.payload;
    if !p.desktop_file.trim().is_empty() {
        let path = Path::new(&p.desktop_file);
        let _ = backend::remove_desktop_launcher(path);
    }

    let config_dir = app_config_dir(&app)?;
    let mut config = match backend::load_config(config_dir.clone()) {
        Ok(cfg) => cfg,
        Err(failure) => return Err(remap(map_backend(failure), IpcErrorCode::Io)),
    };

    let app_entry = config
        .apps
        .iter_mut()
        .find(|a| a.id == p.app_id)
        .ok_or_else(|| error(IpcErrorCode::NotFound, format!("Không tìm thấy app: {}", p.app_id)))?;

    app_entry.desktop_file = String::new();
    let updated_copy = app_entry.clone();

    let store = backend::ConfigStore::new(config_dir, vec![])
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Internal))?;
    store
        .save(&config)
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Io))?;

    crate::debug::success(
        "LAUNCHER",
        format!("Desktop launcher deleted for '{}'", updated_copy.name),
    );

    Ok(response(request_id, updated_copy))
}

#[tauri::command]
pub fn app_executables(
    request: Req<AppExecutablesRequest>,
) -> IpcResult<AppExecutablesResponse> {
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Validation)),
    };

    let path = Path::new(&request.payload.path);
    let paths = backend::list_executables_in_dir(path);

    let executables = paths
        .into_iter()
        .map(|p| {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("binary")
                .to_string();
            let size_bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            ExecutableItem {
                path: p.to_string_lossy().into_owned(),
                name,
                size_bytes,
            }
        })
        .collect();

    Ok(response(request_id, AppExecutablesResponse { executables }))
}

#[tauri::command]
pub fn app_relocate<R: Runtime>(
    request: Req<AppRelocateRequest>,
    app: AppHandle<R>,
) -> IpcResult<AppEntry> {
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Validation)),
    };

    let p = &request.payload;
    let config_dir = app_config_dir(&app)?;
    let mut config = match backend::load_config(config_dir.clone()) {
        Ok(cfg) => cfg,
        Err(failure) => return Err(remap(map_backend(failure), IpcErrorCode::Io)),
    };

    let target_dir = if let Some(target) = &p.target_managed_dir {
        PathBuf::from(target)
    } else {
        let managed = config.settings.managed_dir.trim();
        if managed.is_empty() {
            let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
                error(IpcErrorCode::Io, "HOME environment variable not set")
            })?;
            home.join("Applications")
        } else {
            PathBuf::from(managed)
        }
    };

    let app_index = config
        .apps
        .iter()
        .position(|a| a.id == p.app_id)
        .ok_or_else(|| error(IpcErrorCode::NotFound, format!("Không tìm thấy app: {}", p.app_id)))?;

    let current_app = &config.apps[app_index];
    let relocated_app = backend::relocate_app_dir(current_app, &target_dir)
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Validation))?;

    config.apps[app_index] = relocated_app.clone();

    let store = backend::ConfigStore::new(config_dir, vec![])
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Internal))?;
    store
        .save(&config)
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Io))?;

    crate::debug::success(
        "RELOCATE",
        format!(
            "Successfully relocated '{}' to {}",
            relocated_app.name, relocated_app.install_path
        ),
    );

    Ok(response(request_id, relocated_app))
}
