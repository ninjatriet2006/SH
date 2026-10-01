use super::helpers::*;
use super::{response, validate, BridgeState};
use crate::contract::*;
use std::path::Path;
use tauri::{AppHandle, Runtime, State, Window};
use universe_manager_backend as backend;

#[tauri::command]
pub fn config_load<R: Runtime>(
    request: Req<Empty>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<ManagerConfig> {
    crate::debug::info("CONFIG", "Executing config_load...");
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => {
            crate::debug::error("CONFIG", format!("Validation failed: {failure:?}"));
            return Err(remap(failure, IpcErrorCode::Internal));
        }
    };
    let app_cfg_dir = app_config_dir(&app)?;
    let config = match backend::load_config(app_cfg_dir) {
        Ok(cfg) => cfg,
        Err(failure) => {
            crate::debug::error("CONFIG", format!("Failed to load config: {failure}"));
            return Err(remap(map_backend(failure), IpcErrorCode::Io));
        }
    };
    let managed = config.settings.managed_dir.trim();
    match managed.is_empty() {
        false => {
            let p = Path::new(managed);
            match p.is_absolute() && p.is_dir() {
                true => {
                    let _ = state.replace_picker_directory(
                        window.label(),
                        UniversePickerKind::Managed,
                        p,
                    );
                }
                false => {
                    crate::debug::warn(
                        "CONFIG",
                        format!("Configured managed_dir '{managed}' does not exist or is invalid"),
                    );
                }
            }
        }
        true => {}
    }
    crate::debug::success(
        "CONFIG",
        format!(
            "Config loaded successfully. Managed dir: '{managed}', total apps: {}",
            config.apps.len()
        ),
    );
    Ok(response(request_id, config))
}

#[tauri::command]
pub fn config_save<R: Runtime>(
    request: Req<ManagerConfig>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<ManagerConfig> {
    crate::debug::info("CONFIG", "Executing config_save...");
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => {
            crate::debug::error("CONFIG", format!("Validation failed: {failure:?}"));
            return Err(remap(failure, IpcErrorCode::Validation));
        }
    };
    let managed = request.payload.settings.managed_dir.trim();
    let root = match state.picker.root(window.label(), UniversePickerKind::Managed) {
        Ok(root) => {
            let candidate = Path::new(managed);
            match !managed.is_empty()
                && candidate.is_absolute()
                && candidate.is_dir()
                && candidate != root
            {
                true => match state.replace_picker_directory(
                    window.label(),
                    UniversePickerKind::Managed,
                    candidate,
                ) {
                    Ok(new_root) => Some(new_root),
                    Err(failure) => {
                        crate::debug::error("CONFIG", format!("Failed to update managed directory: {failure:?}"));
                        return Err(remap(failure, IpcErrorCode::Validation));
                    }
                },
                false => match !managed.is_empty() {
                    true => Some(root),
                    false => None,
                },
            }
        }
        Err(_) => match !managed.is_empty() {
            true => {
                let candidate = Path::new(managed);
                match candidate.is_absolute() && candidate.is_dir() {
                    true => match state.replace_picker_directory(
                        window.label(),
                        UniversePickerKind::Managed,
                        candidate,
                    ) {
                        Ok(new_root) => Some(new_root),
                        Err(failure) => {
                            crate::debug::error("CONFIG", format!("Failed to set managed directory: {failure:?}"));
                            return Err(remap(failure, IpcErrorCode::Validation));
                        }
                    },
                    false => {
                        crate::debug::error(
                            "CONFIG",
                            format!("Managed directory does not exist or is not absolute: {managed}"),
                        );
                        return Err(error(
                            IpcErrorCode::Validation,
                            format!(
                                "Th\u{1b0} m\u{1ee5}c qu\u{1ea3}n l\u{fd} kh\u{f4}ng t\u{1ed3}n t\u{1ea1}i ho\u{1eb7}c kh\u{f4}ng ph\u{1ea3}i th\u{1b0} m\u{1ee5}c tuy\u{1ec7}t \u{111}\u{1ed1}i: {managed}"
                            ),
                        ));
                    }
                }
            }
            false => None,
        },
    };

    let config_dir = app_config_dir(&app)?;
    let allowed_roots = match root {
        Some(r) => vec![r],
        None => vec![config_dir.clone()],
    };

    let store = match backend::ConfigStore::new(config_dir, allowed_roots) {
        Ok(st) => st,
        Err(failure) => {
            crate::debug::error("CONFIG", format!("Failed to initialize config store: {failure}"));
            return Err(remap(map_backend(failure), IpcErrorCode::Validation));
        }
    };
    match store.save(&request.payload) {
        Ok(_) => {
            crate::debug::success(
                "CONFIG",
                format!("Config saved. Total registered apps: {}", request.payload.apps.len()),
            );
            Ok(response(request_id, request.payload))
        }
        Err(failure) => {
            crate::debug::error("CONFIG", format!("Failed to write config: {failure}"));
            Err(remap(map_backend(failure), IpcErrorCode::Validation))
        }
    }
}
