use super::helpers::*;
use super::{job_request, run_job, BridgeState, JOB_START_APP, JOB_STOP_APP};
use crate::contract::*;
use std::path::Path;
use tauri::{AppHandle, Runtime, State, Window};
use universe_manager_backend as backend;

#[tauri::command]
pub async fn start_app<R: Runtime>(
    request: Req<AppActionRequest>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<OperationResult> {
    action_job(request, window, app, state, true).await
}

#[tauri::command]
pub async fn stop_app<R: Runtime>(
    request: Req<AppActionRequest>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<OperationResult> {
    action_job(request, window, app, state, false).await
}

async fn action_job<R: Runtime>(
    request: Req<AppActionRequest>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
    start: bool,
) -> IpcResult<OperationResult> {
    let request_id = job_request(&request)?;
    match request.payload.confirmed {
        true => {}
        false => {
            return Err(error(
                IpcErrorCode::Forbidden,
                "start/stop requires explicit UI confirmation",
            ));
        }
    }
    let generation = state.generation(window.label(), IpcErrorCode::Io)?;
    let root = match state.picker.root(window.label(), UniversePickerKind::Managed) {
        Ok(r) => r,
        Err(err) => return Err(map_action_error(err)),
    };
    let config_dir = match app_config_dir(&app) {
        Ok(dir) => dir,
        Err(err) => return Err(map_action_error(err)),
    };
    let config = match backend::load_config(config_dir.clone()) {
        Ok(cfg) => cfg,
        Err(err) => return Err(map_action_backend(err)),
    };
    match state.picker.resolve_directory(
        window.label(),
        UniversePickerKind::Managed,
        Path::new(&config.settings.managed_dir),
    ) {
        Ok(_) => {}
        Err(err) => return Err(map_action_error(err)),
    }
    let action = state.lifecycle_action(window.label(), config_dir, root)?;
    let active =
        state.register(window.label(), request_id, generation, IpcErrorCode::InvalidArgument)?;
    let app_id = request.payload.app_id;
    let topic = match start {
        true => {
            crate::debug::start("LIFECYCLE", format!("Action START dispatched for app '{app_id}'"));
            JOB_START_APP
        }
        false => {
            crate::debug::stop("LIFECYCLE", format!("Action STOP dispatched for app '{app_id}'"));
            JOB_STOP_APP
        }
    };
    let app_id_clone = app_id.clone();
    run_job(window, topic, active, move |job, cancellation| {
        let result = action.run(start, &config, &app_id, cancellation, |progress| {
            job.progress(progress)
        });
        match result {
            Ok(op) => {
                match start {
                    true => crate::debug::success("LIFECYCLE", format!("App '{app_id_clone}' started successfully.")),
                    false => crate::debug::success("LIFECYCLE", format!("App '{app_id_clone}' stopped successfully.")),
                }
                Ok(map_operation(op))
            }
            Err(failure) => {
                crate::debug::error("LIFECYCLE", format!("App '{app_id_clone}' action failed: {failure}"));
                Err(map_action_backend(failure))
            }
        }
    })
    .await
}
