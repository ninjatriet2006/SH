use super::helpers::*;
use super::{job_request, run_job, BridgeState, JOB_DETECT_APP, JOB_SCAN_APPS};
use crate::contract::*;
use std::path::Path;
use tauri::{AppHandle, Runtime, State, Window};
use universe_manager_backend as backend;

#[tauri::command]
pub async fn scan_apps<R: Runtime>(
    request: Req<Empty>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<Vec<AppEntry>> {
    let request_id = job_request(&request)?;
    let generation = state.generation(window.label(), IpcErrorCode::Io)?;
    let config = load_config(&app, IpcErrorCode::Io)?;
    let managed_str = config.settings.managed_dir.trim();
    let managed_path = match managed_str.is_empty() {
        false => {
            let path = Path::new(managed_str);
            match path.is_absolute() && path.is_dir() {
                true => {
                    let _ = state.replace_picker_directory(
                        window.label(),
                        UniversePickerKind::Managed,
                        path,
                    );
                    Some(path.to_path_buf())
                }
                false => None,
            }
        }
        true => None,
    };

    let active = state.register(window.label(), request_id, generation, IpcErrorCode::Io)?;
    crate::debug::start("SCAN", format!("Starting application scan (managed_path: {managed_path:?})..."));
    run_job(window, JOB_SCAN_APPS, active, move |job, cancellation| {
        let mut apps = match backend::scan_all_applications(
            &config,
            managed_path.as_deref(),
            cancellation,
            |progress| job.progress(progress),
        ) {
            Ok(apps) => apps,
            Err(failure) => {
                crate::debug::error("SCAN", format!("Application scan failed: {failure}"));
                return Err(remap(map_backend(failure), IpcErrorCode::Io));
            }
        };

        let snapshot = backend::ProcessSnapshot::collect();
        for app in &mut apps {
            let running = snapshot.is_running(app);
            app.status = Some(match running {
                true => "Running".to_string(),
                false => "Stopped".to_string(),
            });
        }

        crate::debug::info("SCAN", format!("Scan completed: {} applications discovered.", apps.len()));
        Ok(apps)
    })
    .await
}

#[tauri::command]
pub async fn detect_app<R: Runtime>(
    request: Req<DetectAppRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<DetectionReport> {
    let request_id = job_request(&request)?;
    let generation = state.generation(window.label(), IpcErrorCode::Io)?;
    let (path, source) = match state.picker.resolve_existing(
        window.label(),
        UniversePickerKind::Source,
        Path::new(&request.payload.path.path),
    ) {
        Ok(path) => {
            let src = state
                .picker
                .root(window.label(), UniversePickerKind::Source)
                .unwrap_or_else(|_| {
                    if path.is_dir() {
                        path.clone()
                    } else {
                        path.parent().unwrap_or(&path).to_path_buf()
                    }
                });
            (path, src)
        }
        Err(failure) => {
            let req_path = Path::new(&request.payload.path.path);
            if req_path.is_absolute() && req_path.exists() {
                let canonical = std::fs::canonicalize(req_path).map_err(io_error)?;
                let root_dir = if canonical.is_dir() {
                    canonical.clone()
                } else {
                    canonical.parent().unwrap_or(&canonical).to_path_buf()
                };
                let _ = state.replace_picker_directory(
                    window.label(),
                    UniversePickerKind::Source,
                    &root_dir,
                );
                (canonical, root_dir)
            } else {
                crate::debug::error("DETECT", format!("Failed to resolve source path: {failure:?}"));
                return Err(map_detect_path(failure));
            }
        }
    };
    crate::debug::start("DETECT", format!("Inspecting app candidate at {:?}...", path));
    let active =
        state.register(window.label(), request_id, generation, IpcErrorCode::InvalidArgument)?;
    run_job(window, JOB_DETECT_APP, active, move |job, cancellation| {
        match backend::DiscoveryService::new(vec![source]) {
            Ok(service) => {
                match service.detect(&path, cancellation, |progress| job.progress(progress)) {
                    Ok(report) => {
                        crate::debug::info(
                            "DETECT",
                            format!(
                                "Detected app: '{}' with {} executables, {} icons",
                                report.suggested_name,
                                report.executables.len(),
                                report.icons.len()
                            ),
                        );
                        Ok(map_detection(report))
                    }
                    Err(failure) => {
                        crate::debug::error("DETECT", format!("Inspection detection error: {failure}"));
                        Err(map_detect_backend(failure))
                    }
                }
            }
            Err(failure) => {
                crate::debug::error("DETECT", format!("Discovery service init error: {failure}"));
                Err(map_detect_backend(failure))
            }
        }
    })
    .await
}
