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

#[tauri::command]
pub fn app_integrate<R: Runtime>(
    request: Req<AppIntegrateRequest>,
    app: AppHandle<R>,
) -> IpcResult<AppEntry> {
    let request_id = match validate(&request, false) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Validation)),
    };

    let p = &request.payload;
    let source_path = Path::new(&p.source_path);
    if !source_path.exists() {
        return Err(error(
            IpcErrorCode::NotFound,
            format!("Đường dẫn nguồn không tồn tại: {}", p.source_path),
        ));
    }
    let exec_path = Path::new(&p.exec_path);
    if !exec_path.exists() {
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

    let managed_dir = if !config.settings.managed_dir.trim().is_empty() {
        PathBuf::from(&config.settings.managed_dir)
    } else {
        let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
            error(IpcErrorCode::Io, "HOME environment variable not set")
        })?;
        home.join("Applications")
    };

    // Step 1: Relocate if requested
    let mut final_install_path = p.source_path.clone();
    let mut final_exec_path = p.exec_path.clone();
    let mut final_icon_path = p.icon_path.clone();
    let install_type = if p.should_relocate {
        let file_or_folder_name = source_path
            .file_name()
            .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "Tên tệp/thư mục nguồn không hợp lệ"))?;
        let _ = std::fs::create_dir_all(&managed_dir);
        let dest = managed_dir.join(file_or_folder_name);

        if dest != source_path {
            if dest.exists() {
                return Err(error(
                    IpcErrorCode::Conflict,
                    format!(
                        "Thư mục đích '{}' đã tồn tại trong thư mục quản lý (~/Applications). Vui lòng đổi tên hoặc kiểm tra lại.",
                        dest.display()
                    ),
                ));
            }

            let move_ok = std::fs::rename(source_path, &dest).is_ok();
            if !move_ok {
                if source_path.is_dir() {
                    backend::copy_dir_recursive(source_path, &dest)
                        .map_err(|e| error(IpcErrorCode::Io, e.to_string()))?;
                    let _ = std::fs::remove_dir_all(source_path);
                } else {
                    std::fs::copy(source_path, &dest)
                        .map_err(|e| error(IpcErrorCode::Io, e.to_string()))?;
                    let _ = std::fs::remove_file(source_path);
                }
            }

            let old_prefix = source_path.to_string_lossy().into_owned();
            let new_prefix = dest.to_string_lossy().into_owned();
            final_install_path = new_prefix.clone();
            final_exec_path = p.exec_path.replace(&old_prefix, &new_prefix);
            final_icon_path = p.icon_path.as_ref().map(|ic| ic.replace(&old_prefix, &new_prefix));
        }
        backend::InstallType::Moved
    } else {
        backend::InstallType::InPlace
    };

    // Ensure executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&final_exec_path) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o111);
            let _ = std::fs::set_permissions(&final_exec_path, perms);
        }
    }

    // Step 2: Create desktop launcher if requested
    let mut desktop_file_path = String::new();
    if p.create_desktop {
        let written = backend::create_or_update_desktop_launcher(
            Path::new(&final_exec_path),
            &p.name,
            final_icon_path.as_deref(),
            false,
            p.desktop_categories.as_deref(),
            p.desktop_arguments.as_deref(),
            p.startup_wm_class.as_deref(),
            None,
        )
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Io))?;
        desktop_file_path = written.to_string_lossy().into_owned();
    }

    // Step 3: Create symlink in ~/.local/bin if requested
    let mut symlink_file_path = None;
    if p.create_symlink {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let bin_dir = home.join(".local/bin");
            let _ = std::fs::create_dir_all(&bin_dir);
            let cmd_name = p.symlink_name.as_deref().unwrap_or(&p.name);
            let clean_cmd: String = cmd_name
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            let final_cmd = if clean_cmd.is_empty() {
                "app".to_string()
            } else {
                clean_cmd.to_lowercase()
            };
            let symlink_dest = bin_dir.join(final_cmd);
            if symlink_dest.exists() || symlink_dest.is_symlink() {
                let _ = std::fs::remove_file(&symlink_dest);
            }
            #[cfg(unix)]
            {
                let _ = std::os::unix::fs::symlink(&final_exec_path, &symlink_dest);
            }
            symlink_file_path = Some(symlink_dest.to_string_lossy().into_owned());
        }
    }

    // Step 4: Add/Update config entry
    let slug = p
        .name
        .to_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-', "-")
        .trim_matches('-')
        .to_string();
    let app_id = if slug.is_empty() {
        format!("app-{}", std::process::id())
    } else {
        slug
    };

    let entry = backend::AppEntry {
        id: app_id.clone(),
        name: p.name.clone(),
        install_type: install_type.clone(),
        source_path: if install_type == backend::InstallType::Moved {
            Some(p.source_path.clone())
        } else {
            None
        },
        install_path: final_install_path,
        exec_path: final_exec_path,
        icon_path: final_icon_path,
        desktop_file: desktop_file_path,
        symlink_file: symlink_file_path,
        added_at: chrono::Utc::now().to_rfc3339(),
        is_custom: Some(true),
        start_cmd: None,
        stop_cmd: None,
        category: p.desktop_categories.clone(),
        package_type: Some(p.package_type.clone()),
        inventory_sources: vec!["Custom Integration".to_string()],
        registry_key: None,
        product_code: None,
        about_url: None,
        publisher: None,
        version: None,
        uninstall_cmd: None,
        status: Some("Ok".to_string()),
    };

    config.apps.retain(|a| a.id != entry.id && a.exec_path != entry.exec_path);
    config.apps.push(entry.clone());

    let store = backend::ConfigStore::new(config_dir, vec![])
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Internal))?;
    store
        .save(&config)
        .map_err(|e| remap(map_backend(e), IpcErrorCode::Io))?;

    crate::debug::success(
        "INTEGRATE",
        format!(
            "Successfully integrated app '{}' (relocate={}, desktop={}, symlink={})",
            entry.name, p.should_relocate, p.create_desktop, p.create_symlink
        ),
    );

    Ok(response(request_id, entry))
}
