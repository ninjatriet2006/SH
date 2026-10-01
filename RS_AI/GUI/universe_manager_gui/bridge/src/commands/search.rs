use super::helpers::*;
use super::{job_request, run_job, BridgeState, JOB_SEARCH_APPS};
use crate::contract::*;
use std::path::Path;
use tauri::{AppHandle, Runtime, State, Window};
use universe_manager_backend as backend;

#[tauri::command]
pub async fn search_apps<R: Runtime>(
    request: Req<SearchAppsRequest>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<SearchReport> {
    let request_id = match job_request(&request) {
        Ok(id) => id,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Validation)),
    };
    let generation = state.generation(window.label(), IpcErrorCode::Io)?;
    let root = match state.picker.root(window.label(), UniversePickerKind::Managed) {
        Ok(r) => r,
        Err(failure) => return Err(remap(failure, IpcErrorCode::Io)),
    };
    let config = load_config(&app, IpcErrorCode::Io)?;
    match state.picker.resolve_directory(
        window.label(),
        UniversePickerKind::Managed,
        Path::new(&config.settings.managed_dir),
    ) {
        Ok(_) => {}
        Err(failure) => return Err(remap(failure, IpcErrorCode::Io)),
    }
    let active =
        state.register(window.label(), request_id, generation, IpcErrorCode::Validation)?;
    let query = request.payload.query;
    crate::debug::info("SEARCH", format!("Executing application search for '{query}'..."));
    let query_clone = query.clone();
    run_job(window, JOB_SEARCH_APPS, active, move |job, cancellation| {
        match backend::search_apps(&config, vec![root], &query, cancellation, |progress| {
            job.progress(progress)
        }) {
            Ok(report) => {
                crate::debug::success("SEARCH", format!("Search for '{query_clone}' returned {} results.", report.results.len()));
                Ok(map_search(report))
            }
            Err(failure) => {
                crate::debug::error("SEARCH", format!("Search for '{query_clone}' failed: {failure}"));
                Err(map_search_backend(failure))
            }
        }
    })
    .await
}
