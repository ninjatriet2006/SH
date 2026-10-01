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
    run_job(window, JOB_SCAN_APPS, active, move |job, cancellation| {
        let mut apps = match backend::scan_all_applications(
            &config,
            managed_path.as_deref(),
            cancellation,
            |progress| job.progress(progress),
        ) {
            Ok(apps) => apps,
            Err(failure) => return Err(remap(map_backend(failure), IpcErrorCode::Io)),
        };

        let snapshot = backend::ProcessSnapshot::collect();
        for app in &mut apps {
            let running = snapshot.is_running(app);
            app.status = Some(match running {
                true => "Running".to_string(),
                false => "Stopped".to_string(),
            });
        }

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
    let path = match state.picker.resolve_existing(
        window.label(),
        UniversePickerKind::Source,
        Path::new(&request.payload.path.path),
    ) {
        Ok(path) => path,
        Err(failure) => return Err(map_detect_path(failure)),
    };
    let source = match state.picker.root(window.label(), UniversePickerKind::Source) {
        Ok(src) => src,
        Err(failure) => return Err(map_detect_path(failure)),
    };
    let active =
        state.register(window.label(), request_id, generation, IpcErrorCode::InvalidArgument)?;
    run_job(window, JOB_DETECT_APP, active, move |job, cancellation| {
        match backend::DiscoveryService::new(vec![source]) {
            Ok(service) => {
                match service.detect(&path, cancellation, |progress| job.progress(progress)) {
                    Ok(report) => Ok(map_detection(report)),
                    Err(failure) => Err(map_detect_backend(failure)),
                }
            }
            Err(failure) => Err(map_detect_backend(failure)),
        }
    })
    .await
}
