use crate::contract::*;
use crate::preferences;
use crate::security::PickerProvenance;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, Window};
use universe_manager_backend as backend;

pub const JOB_SCAN_APPS: &str = "job.scan_apps";
pub const JOB_DETECT_APP: &str = "job.detect_app";
pub const JOB_START_APP: &str = "job.start_app";
pub const JOB_STOP_APP: &str = "job.stop_app";
pub const JOB_SEARCH_APPS: &str = "job.search_apps";
pub const TOPICS: [&str; 5] = [
    JOB_SCAN_APPS,
    JOB_DETECT_APP,
    JOB_START_APP,
    JOB_STOP_APP,
    JOB_SEARCH_APPS,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandContract {
    pub name: &'static str,
    pub topic: Option<&'static str>,
    pub errors: &'static [IpcErrorCode],
}

impl CommandContract {
    const fn command(name: &'static str, errors: &'static [IpcErrorCode]) -> Self {
        Self {
            name,
            topic: None,
            errors,
        }
    }

    const fn job(name: &'static str, topic: &'static str, errors: &'static [IpcErrorCode]) -> Self {
        Self {
            name,
            topic: Some(topic),
            errors,
        }
    }
}

pub const INVOKE_REGISTRY: [CommandContract; 10] = [
    CommandContract::command("config_load", &[IpcErrorCode::Io, IpcErrorCode::Internal]),
    CommandContract::command("config_save", &[IpcErrorCode::Validation, IpcErrorCode::Io]),
    CommandContract::job("scan_apps", JOB_SCAN_APPS, &[IpcErrorCode::Io, IpcErrorCode::Cancelled]),
    CommandContract::job(
        "detect_app",
        JOB_DETECT_APP,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::NotFound,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "start_app",
        JOB_START_APP,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::NotFound,
            IpcErrorCode::Forbidden,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "stop_app",
        JOB_STOP_APP,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::NotFound,
            IpcErrorCode::Forbidden,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "search_apps",
        JOB_SEARCH_APPS,
        &[IpcErrorCode::Validation, IpcErrorCode::Io, IpcErrorCode::Cancelled],
    ),
    CommandContract::command("preferences_get", &[IpcErrorCode::Io, IpcErrorCode::Internal]),
    CommandContract::command("preferences_set", &[IpcErrorCode::Validation, IpcErrorCode::Io]),
    CommandContract::command(
        "picker_select",
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::NotFound,
            IpcErrorCode::Conflict,
            IpcErrorCode::Io,
            IpcErrorCode::Internal,
        ],
    ),
];

#[derive(Clone, Default)]
pub struct BridgeState {
    pub picker: PickerProvenance,
    jobs: Arc<Mutex<HashMap<String, WindowJobs>>>,
    lifecycle: Arc<Mutex<HashMap<String, Arc<WindowLifecycle>>>>,
}

#[derive(Default)]
struct LifecycleStatus {
    in_flight: usize,
    running: HashMap<String, ManagerConfig>,
    revoked: bool,
}

struct WindowLifecycle {
    manager: backend::LifecycleManager,
    status: Mutex<LifecycleStatus>,
    idle: Condvar,
}

impl WindowLifecycle {
    fn new(manager: backend::LifecycleManager) -> Self {
        Self {
            manager,
            status: Mutex::new(LifecycleStatus::default()),
            idle: Condvar::new(),
        }
    }

    fn shutdown(&self) -> Result<(), IpcError> {
        let mut status = self
            .status
            .lock()
            .map_err(|_| error(IpcErrorCode::Internal, "lifecycle status lock is poisoned"))?;
        status.revoked = true;
        while status.in_flight != 0 {
            status = self
                .idle
                .wait(status)
                .map_err(|_| error(IpcErrorCode::Internal, "lifecycle status lock is poisoned"))?;
        }
        let tracked = status
            .running
            .iter()
            .map(|(app_id, config)| (app_id.clone(), config.clone()))
            .collect::<Vec<_>>();
        let mut first_error = None;
        for (app_id, config) in tracked {
            match self
                .manager
                .stop(&config, &app_id, true, &backend::CancellationToken::new(), |_| {})
            {
                Ok(_) => {
                    status.running.remove(&app_id);
                }
                Err(failure) => {
                    first_error.get_or_insert_with(|| map_action_backend(failure));
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

impl Drop for WindowLifecycle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

struct LifecycleAction {
    lifecycle: Arc<WindowLifecycle>,
}

impl LifecycleAction {
    fn run(
        &self,
        start: bool,
        config: &ManagerConfig,
        app_id: &str,
        cancellation: &backend::CancellationToken,
        progress: impl FnMut(backend::Progress),
    ) -> Result<backend::OperationResult, backend::BackendError> {
        let mut status = self.lifecycle.status.lock().map_err(|_| {
            backend::BackendError::new(backend::ErrorKind::Internal, "lifecycle status lock is poisoned")
        })?;
        let result = if start {
            self.lifecycle
                .manager
                .start(config, app_id, true, cancellation, progress)
        } else {
            self.lifecycle
                .manager
                .stop(config, app_id, true, cancellation, progress)
        };
        if result.is_ok() {
            if start {
                status.running.insert(app_id.to_owned(), config.clone());
            } else {
                status.running.remove(app_id);
            }
        }
        result
    }
}

impl Drop for LifecycleAction {
    fn drop(&mut self) {
        if let Ok(mut status) = self.lifecycle.status.lock() {
            status.in_flight = status.in_flight.saturating_sub(1);
            self.lifecycle.idle.notify_all();
        }
    }
}

#[derive(Default)]
struct WindowJobs {
    generation: u64,
    used: HashSet<String>,
    active: HashMap<String, (u64, backend::CancellationToken)>,
}

struct ActiveJob {
    jobs: Arc<Mutex<HashMap<String, WindowJobs>>>,
    window: String,
    request_id: String,
    generation: u64,
    cancellation: backend::CancellationToken,
}

impl Drop for ActiveJob {
    fn drop(&mut self) {
        if let Ok(mut jobs) = self.jobs.lock()
            && let Some(window) = jobs.get_mut(&self.window)
            && window.generation == self.generation
        {
            window.active.remove(&self.request_id);
        }
    }
}

impl BridgeState {
    fn generation(&self, window: &str, code: IpcErrorCode) -> Result<u64, IpcError> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error(code, "job state lock is poisoned"))?;
        Ok(jobs.entry(window.to_owned()).or_default().generation)
    }

    fn register(
        &self,
        window: &str,
        request_id: String,
        generation: u64,
        code: IpcErrorCode,
    ) -> Result<ActiveJob, IpcError> {
        let cancellation = backend::CancellationToken::new();
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error(code, "job state lock is poisoned"))?;
        let entry = jobs.entry(window.to_owned()).or_default();
        if entry.generation != generation {
            return Err(error(code, "window lifecycle changed during job preflight"));
        }
        if !entry.used.insert(request_id.clone()) {
            return Err(error(code, "request_id was already used in this window lifecycle"));
        }
        entry
            .active
            .insert(request_id.clone(), (generation, cancellation.clone()));
        Ok(ActiveJob {
            jobs: Arc::clone(&self.jobs),
            window: window.to_owned(),
            request_id,
            generation,
            cancellation,
        })
    }

    fn lifecycle_action(
        &self,
        window: &str,
        config_dir: PathBuf,
        managed_root: PathBuf,
    ) -> Result<LifecycleAction, IpcError> {
        let mut managers = self
            .lifecycle
            .lock()
            .map_err(|_| error(IpcErrorCode::Io, "lifecycle lock is poisoned"))?;
        let lifecycle = if let Some(lifecycle) = managers.get(window) {
            Arc::clone(lifecycle)
        } else {
            let store = backend::ConfigStore::new(config_dir, vec![managed_root]).map_err(map_backend)?;
            let lifecycle = Arc::new(WindowLifecycle::new(backend::LifecycleManager::new(store)));
            managers.insert(window.to_owned(), Arc::clone(&lifecycle));
            lifecycle
        };
        let mut status = lifecycle
            .status
            .lock()
            .map_err(|_| error(IpcErrorCode::Io, "lifecycle status lock is poisoned"))?;
        if status.revoked {
            return Err(error(IpcErrorCode::Forbidden, "window lifecycle authority was revoked"));
        }
        status.in_flight = status
            .in_flight
            .checked_add(1)
            .ok_or_else(|| error(IpcErrorCode::Internal, "lifecycle action count overflow"))?;
        drop(status);
        Ok(LifecycleAction { lifecycle })
    }

    fn reset_lifecycle(&self, window: &str) -> Result<(), IpcError> {
        let mut managers = self
            .lifecycle
            .lock()
            .map_err(|_| error(IpcErrorCode::Internal, "lifecycle lock is poisoned"))?;
        let Some(lifecycle) = managers.get(window).cloned() else {
            return Ok(());
        };
        let mut status = lifecycle
            .status
            .lock()
            .map_err(|_| error(IpcErrorCode::Internal, "lifecycle status lock is poisoned"))?;
        if status.in_flight != 0 || !status.running.is_empty() {
            return Err(error(
                IpcErrorCode::Conflict,
                "managed root cannot change while an application action or process is active",
            ));
        }
        status.revoked = true;
        drop(status);
        managers.remove(window);
        Ok(())
    }

    fn replace_picker_directory(
        &self,
        window: &str,
        kind: UniversePickerKind,
        path: &Path,
    ) -> Result<PathBuf, IpcError> {
        if kind == UniversePickerKind::Managed {
            self.reset_lifecycle(window)?;
        }
        self.picker.replace_directory(window, kind, path)
    }

    pub fn clear_window(&self, window: &str) {
        if let Ok(mut jobs) = self.jobs.lock() {
            let generation = jobs.get(window).map_or(1, |entry| entry.generation.saturating_add(1));
            if let Some(entry) = jobs.remove(window) {
                for (_, cancellation) in entry.active.into_values() {
                    cancellation.cancel();
                }
            }
            jobs.insert(
                window.to_owned(),
                WindowJobs {
                    generation,
                    ..WindowJobs::default()
                },
            );
        }
        if let Ok(mut managers) = self.lifecycle.lock()
            && let Some(lifecycle) = managers.get(window).cloned()
            && lifecycle.shutdown().is_ok()
        {
            managers.remove(window);
        }
        let _ = self.picker.clear_window(window);
    }
}

pub struct EventSequence<T> {
    request_id: String,
    seq: u64,
    terminal: bool,
    marker: std::marker::PhantomData<T>,
}

impl<T> EventSequence<T> {
    pub(crate) fn new(request_id: String) -> Self {
        Self {
            request_id,
            seq: 0,
            terminal: false,
            marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn event(
        &mut self,
        state: JobState,
        completed: Option<u64>,
        total: Option<u64>,
        message: Option<String>,
        result: Option<JobResult<T>>,
    ) -> Result<JobEvent<JobProgress<T>>, IpcError> {
        if self.terminal || state.is_terminal() != result.is_some() {
            return Err(error(IpcErrorCode::Internal, "invalid job lifecycle transition"));
        }
        let event = JobEvent {
            schema_version: SCHEMA_VERSION,
            job_id: self.request_id.clone(),
            seq: self.seq,
            state,
            payload: JobProgress {
                request_id: self.request_id.clone(),
                completed,
                total,
                message,
                result,
            },
        };
        self.seq = self
            .seq
            .checked_add(1)
            .ok_or_else(|| error(IpcErrorCode::Internal, "job sequence overflow"))?;
        self.terminal = state.is_terminal();
        Ok(event)
    }
}

struct JobLifecycle<'a, R: Runtime, T: Serialize + Clone> {
    window: &'a Window<R>,
    topic: &'static str,
    sequence: EventSequence<T>,
}

impl<'a, R: Runtime, T: Serialize + Clone> JobLifecycle<'a, R, T> {
    fn start(window: &'a Window<R>, topic: &'static str, request_id: String) -> Result<Self, IpcError> {
        let mut lifecycle = Self {
            window,
            topic,
            sequence: EventSequence::new(request_id),
        };
        lifecycle.emit(JobState::Started, None, None, None, None)?;
        Ok(lifecycle)
    }

    fn progress(&mut self, progress: backend::Progress) {
        let _ = self.emit(
            JobState::Progress,
            Some(progress.completed),
            progress.total,
            Some(progress.message),
            None,
        );
    }

    fn finish(&mut self, value: Result<T, IpcError>) -> Result<(), IpcError> {
        let request_id = self.sequence.request_id.clone();
        let (state, result) = match value {
            Ok(value) => (JobState::Completed, JobResult::Completed { request_id, value }),
            Err(failure) if failure.code == IpcErrorCode::Cancelled => (
                JobState::Cancelled,
                JobResult::Cancelled {
                    request_id,
                    error: failure,
                },
            ),
            Err(failure) => (
                JobState::Failed,
                JobResult::Failed {
                    request_id,
                    error: failure,
                },
            ),
        };
        self.emit(state, None, None, None, Some(result))
    }

    fn emit(
        &mut self,
        state: JobState,
        completed: Option<u64>,
        total: Option<u64>,
        message: Option<String>,
        result: Option<JobResult<T>>,
    ) -> Result<(), IpcError> {
        let event = self.sequence.event(state, completed, total, message, result)?;
        self.window
            .emit(self.topic, event)
            .map_err(|source| error(IpcErrorCode::Internal, source.to_string()))
    }
}

fn validate<T>(request: &Req<T>, job: bool) -> Result<Option<String>, IpcError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(error(IpcErrorCode::InvalidArgument, "schema_version must be 1"));
    }
    match (&request.request_id, job) {
        (Some(id), true) if !id.trim().is_empty() => Ok(Some(id.clone())),
        (None, false) => Ok(None),
        (_, true) => Err(error(IpcErrorCode::InvalidArgument, "request_id is required")),
        (_, false) => Err(error(IpcErrorCode::InvalidArgument, "request_id must be null")),
    }
}

fn response<T>(request_id: Option<String>, data: T) -> Res<T> {
    Res {
        schema_version: SCHEMA_VERSION,
        request_id,
        data,
    }
}

async fn run_job<R, T, F>(window: Window<R>, topic: &'static str, active: ActiveJob, operation: F) -> IpcResult<T>
where
    R: Runtime,
    T: Serialize + Clone + Send + 'static,
    F: FnOnce(&mut JobLifecycle<'_, R, T>, &backend::CancellationToken) -> Result<T, IpcError> + Send + 'static,
{
    let request_id = active.request_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut lifecycle = JobLifecycle::start(&window, topic, request_id.clone())?;
        let result = operation(&mut lifecycle, &active.cancellation);
        lifecycle.finish(result.clone())?;
        drop(active);
        result.map(|value| response(Some(request_id), value))
    })
    .await
    .map_err(|source| error(IpcErrorCode::Io, format!("job worker failed: {source}")))?
}

#[tauri::command]
pub fn config_load<R: Runtime>(request: Req<Empty>, app: AppHandle<R>) -> IpcResult<ManagerConfig> {
    let request_id = validate(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Internal))?;
    let config =
        backend::load_config(app_config_dir(&app)?).map_err(|failure| remap(map_backend(failure), IpcErrorCode::Io))?;
    Ok(response(request_id, config))
}

#[tauri::command]
pub fn config_save<R: Runtime>(
    request: Req<ManagerConfig>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<ManagerConfig> {
    let request_id = validate(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Validation))?;
    let root = state
        .picker
        .root(window.label(), UniversePickerKind::Managed)
        .map_err(|failure| remap(failure, IpcErrorCode::Validation))?;
    let store = backend::ConfigStore::new(app_config_dir(&app)?, vec![root])
        .map_err(|failure| remap(map_backend(failure), IpcErrorCode::Validation))?;
    store
        .save(&request.payload)
        .map_err(|failure| remap(map_backend(failure), IpcErrorCode::Validation))?;
    Ok(response(request_id, request.payload))
}

#[tauri::command]
pub async fn scan_apps<R: Runtime>(
    request: Req<Empty>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<Vec<AppEntry>> {
    let request_id = job_request(&request)?;
    let generation = state.generation(window.label(), IpcErrorCode::Io)?;
    let root = state
        .picker
        .root(window.label(), UniversePickerKind::Managed)
        .map_err(|failure| remap(failure, IpcErrorCode::Io))?;
    let config = load_config(&app, IpcErrorCode::Io)?;
    state
        .picker
        .resolve_directory(
            window.label(),
            UniversePickerKind::Managed,
            Path::new(&config.settings.managed_dir),
        )
        .map_err(|failure| remap(failure, IpcErrorCode::Io))?;
    let active = state.register(window.label(), request_id, generation, IpcErrorCode::Io)?;
    run_job(window, JOB_SCAN_APPS, active, move |job, cancellation| {
        let service = backend::DiscoveryService::new(vec![root.clone()])
            .map_err(|failure| remap(map_backend(failure), IpcErrorCode::Io))?;
        service
            .scan_managed(&config, &root, cancellation, |progress| job.progress(progress))
            .map_err(|failure| remap(map_backend(failure), IpcErrorCode::Io))
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
    let path = state
        .picker
        .resolve_existing(
            window.label(),
            UniversePickerKind::Source,
            Path::new(&request.payload.path.path),
        )
        .map_err(map_detect_path)?;
    let source = state
        .picker
        .root(window.label(), UniversePickerKind::Source)
        .map_err(map_detect_path)?;
    let active = state.register(window.label(), request_id, generation, IpcErrorCode::InvalidArgument)?;
    run_job(window, JOB_DETECT_APP, active, move |job, cancellation| {
        backend::DiscoveryService::new(vec![source])
            .and_then(|service| service.detect(&path, cancellation, |progress| job.progress(progress)))
            .map(map_detection)
            .map_err(map_detect_backend)
    })
    .await
}

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
    if !request.payload.confirmed {
        return Err(error(
            IpcErrorCode::Forbidden,
            "start/stop requires explicit UI confirmation",
        ));
    }
    let generation = state.generation(window.label(), IpcErrorCode::Io)?;
    let root = state
        .picker
        .root(window.label(), UniversePickerKind::Managed)
        .map_err(map_action_error)?;
    let config_dir = app_config_dir(&app).map_err(map_action_error)?;
    let config = backend::load_config(config_dir.clone()).map_err(map_action_backend)?;
    state
        .picker
        .resolve_directory(
            window.label(),
            UniversePickerKind::Managed,
            Path::new(&config.settings.managed_dir),
        )
        .map_err(map_action_error)?;
    let action = state.lifecycle_action(window.label(), config_dir, root)?;
    let active = state.register(window.label(), request_id, generation, IpcErrorCode::InvalidArgument)?;
    let topic = if start { JOB_START_APP } else { JOB_STOP_APP };
    let app_id = request.payload.app_id;
    run_job(window, topic, active, move |job, cancellation| {
        let result = action.run(start, &config, &app_id, cancellation, |progress| job.progress(progress));
        result.map(map_operation).map_err(map_action_backend)
    })
    .await
}

#[tauri::command]
pub async fn search_apps<R: Runtime>(
    request: Req<SearchAppsRequest>,
    window: Window<R>,
    app: AppHandle<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<SearchReport> {
    let request_id = job_request(&request).map_err(|failure| remap(failure, IpcErrorCode::Validation))?;
    let generation = state.generation(window.label(), IpcErrorCode::Io)?;
    let root = state
        .picker
        .root(window.label(), UniversePickerKind::Managed)
        .map_err(|failure| remap(failure, IpcErrorCode::Io))?;
    let config = load_config(&app, IpcErrorCode::Io)?;
    state
        .picker
        .resolve_directory(
            window.label(),
            UniversePickerKind::Managed,
            Path::new(&config.settings.managed_dir),
        )
        .map_err(|failure| remap(failure, IpcErrorCode::Io))?;
    let active = state.register(window.label(), request_id, generation, IpcErrorCode::Validation)?;
    let query = request.payload.query;
    run_job(window, JOB_SEARCH_APPS, active, move |job, cancellation| {
        backend::search_apps(&config, vec![root], &query, cancellation, |progress| {
            job.progress(progress)
        })
        .map(map_search)
        .map_err(map_search_backend)
    })
    .await
}

#[tauri::command]
pub fn preferences_get<R: Runtime>(request: Req<Empty>, app: AppHandle<R>) -> IpcResult<Preferences> {
    let request_id = validate(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Internal))?;
    let value = preferences::load_or_migrate(&preferences_path(&app)?, None).map_err(io_error)?;
    Ok(response(request_id, value))
}

#[tauri::command]
pub fn preferences_set<R: Runtime>(request: Req<Preferences>, app: AppHandle<R>) -> IpcResult<Preferences> {
    let request_id = validate(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Validation))?;
    validate_preferences(&request.payload)?;
    preferences::atomic_write(
        &preferences_path(&app).map_err(|failure| remap(failure, IpcErrorCode::Io))?,
        &request.payload,
    )
    .map_err(io_error)?;
    Ok(response(request_id, request.payload))
}

#[tauri::command]
pub async fn picker_select<R: Runtime>(
    request: Req<UniversePickerSelectRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<UniversePickerSelectResult> {
    let request_id = validate(&request, false)?;
    let dialog = rfd::FileDialog::new().set_parent(&window);
    let selected = tauri::async_runtime::spawn_blocking(move || dialog.pick_folder())
        .await
        .map_err(|source| error(IpcErrorCode::Internal, format!("picker worker failed: {source}")))?
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "directory selection was cancelled"))?;
    let selected = state.replace_picker_directory(window.label(), request.payload.kind, &selected)?;
    Ok(response(
        request_id,
        UniversePickerSelectResult {
            kind: request.payload.kind,
            path: path_ref(selected),
        },
    ))
}

fn job_request<T>(request: &Req<T>) -> Result<String, IpcError> {
    validate(request, true)?.ok_or_else(|| error(IpcErrorCode::InvalidArgument, "validated request has no ID"))
}

fn app_config_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|source| error(IpcErrorCode::Internal, source.to_string()))?;
    fs::create_dir_all(&path).map_err(io_error)?;
    fs::canonicalize(path).map_err(io_error)
}

fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|source| error(IpcErrorCode::Internal, source.to_string()))?;
    fs::create_dir_all(&path).map_err(io_error)?;
    Ok(path)
}

fn preferences_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    Ok(app_data_dir(app)?.join("preferences.json"))
}

fn load_config<R: Runtime>(app: &AppHandle<R>, code: IpcErrorCode) -> Result<ManagerConfig, IpcError> {
    backend::load_config(app_config_dir(app)?).map_err(|failure| remap(map_backend(failure), code))
}

fn path_ref(path: PathBuf) -> PathRef {
    PathRef {
        path: path.to_string_lossy().into_owned(),
    }
}

fn map_detection(report: backend::DetectionReport) -> DetectionReport {
    DetectionReport {
        is_appimage: report.is_appimage,
        suggested_name: report.suggested_name,
        executables: report.executables.into_iter().map(path_ref).collect(),
        icons: report.icons.into_iter().map(path_ref).collect(),
        desktop_templates: report.desktop_templates.into_iter().map(path_ref).collect(),
    }
}

fn map_operation(result: backend::OperationResult) -> OperationResult {
    OperationResult {
        app_id: result.app_id,
        operation: result.operation,
        completed: result.completed,
    }
}

fn map_search(report: backend::SearchReport) -> SearchReport {
    SearchReport {
        query: report.query,
        results: report
            .results
            .into_iter()
            .map(|result| SearchResult {
                name: result.name,
                id: result.id,
                version: result.version,
                source: result.source,
            })
            .collect(),
    }
}

fn validate_preferences(value: &Preferences) -> Result<(), IpcError> {
    if !matches!(value.language.as_str(), "vi" | "en") {
        return Err(error(IpcErrorCode::Validation, "language must be vi or en"));
    }
    if !matches!(value.theme.as_str(), "system" | "light" | "dark") {
        return Err(error(IpcErrorCode::Validation, "theme must be system, light, or dark"));
    }
    if !matches!(value.font_id.as_str(), "system-default" | "dejavusans") {
        return Err(error(
            IpcErrorCode::Validation,
            "font_id must be system-default or dejavusans",
        ));
    }
    Ok(())
}

fn map_backend(source: backend::BackendError) -> IpcError {
    let code = match source.kind() {
        backend::ErrorKind::InvalidArgument => IpcErrorCode::InvalidArgument,
        backend::ErrorKind::NotFound => IpcErrorCode::NotFound,
        backend::ErrorKind::Forbidden => IpcErrorCode::Forbidden,
        backend::ErrorKind::Unavailable => IpcErrorCode::Unavailable,
        backend::ErrorKind::Conflict => IpcErrorCode::Conflict,
        backend::ErrorKind::Validation => IpcErrorCode::Validation,
        backend::ErrorKind::Cancelled => IpcErrorCode::Cancelled,
        backend::ErrorKind::Io => IpcErrorCode::Io,
        backend::ErrorKind::Internal => IpcErrorCode::Internal,
    };
    IpcError {
        code,
        message: source.to_string(),
        retryable: false,
        details: None,
    }
}

fn map_detect_path(failure: IpcError) -> IpcError {
    match failure.code {
        IpcErrorCode::NotFound | IpcErrorCode::Io => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

fn map_detect_backend(failure: backend::BackendError) -> IpcError {
    let failure = map_backend(failure);
    match failure.code {
        IpcErrorCode::NotFound | IpcErrorCode::Io | IpcErrorCode::Cancelled => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

fn map_action_error(failure: IpcError) -> IpcError {
    match failure.code {
        IpcErrorCode::NotFound | IpcErrorCode::Forbidden | IpcErrorCode::Io | IpcErrorCode::Cancelled => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

fn map_action_backend(failure: backend::BackendError) -> IpcError {
    map_action_error(map_backend(failure))
}

fn map_search_backend(failure: backend::BackendError) -> IpcError {
    let failure = map_backend(failure);
    match failure.code {
        IpcErrorCode::Cancelled | IpcErrorCode::Io => failure,
        _ => remap(failure, IpcErrorCode::Validation),
    }
}

fn remap(mut failure: IpcError, code: IpcErrorCode) -> IpcError {
    failure.code = code;
    failure
}

fn io_error(source: std::io::Error) -> IpcError {
    error(IpcErrorCode::Io, source.to_string())
}

fn error(code: IpcErrorCode, message: impl Into<String>) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable: false,
        details: None,
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "universe-bridge-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("create test directory");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn running_fixture(root: &Path) -> (PathBuf, PathBuf, ManagerConfig) {
        let config_dir = root.join("config");
        let managed = root.join("managed");
        let app_dir = managed.join("yes-app");
        fs::create_dir_all(&config_dir).expect("config directory");
        fs::create_dir_all(&app_dir).expect("application directory");
        let executable = app_dir.join("yes");
        fs::copy("/usr/bin/yes", &executable).expect("copy fixture executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).expect("chmod fixture");
        let config = ManagerConfig {
            settings: backend::ManagerSettings {
                managed_dir: managed.to_string_lossy().into_owned(),
            },
            apps: vec![backend::AppEntry {
                id: "yes-app".into(),
                name: "Yes App".into(),
                install_path: app_dir.to_string_lossy().into_owned(),
                exec_path: executable.to_string_lossy().into_owned(),
                package_type: Some("Local".into()),
                ..backend::AppEntry::default()
            }],
        };
        (config_dir, managed, config)
    }

    fn start_fixture(
        state: &BridgeState,
        window: &str,
        config_dir: &Path,
        managed: &Path,
        config: &ManagerConfig,
    ) -> (Arc<WindowLifecycle>, u32) {
        state
            .picker
            .replace_directory(window, UniversePickerKind::Managed, managed)
            .expect("picker root");
        let action = state
            .lifecycle_action(window, config_dir.to_owned(), managed.to_owned())
            .expect("lifecycle action");
        action
            .run(true, config, "yes-app", &backend::CancellationToken::new(), |_| {})
            .expect("start fixture");
        let lifecycle = Arc::clone(&action.lifecycle);
        let pid = lifecycle
            .manager
            .identity("yes-app")
            .expect("identity lookup")
            .expect("tracked process")
            .pid;
        drop(action);
        (lifecycle, pid)
    }

    #[test]
    fn managed_picker_change_is_rejected_while_process_is_active() {
        let root = TestDir::new("picker-active");
        let (config_dir, managed, config) = running_fixture(&root.0);
        let replacement = root.0.join("replacement");
        fs::create_dir_all(&replacement).expect("replacement root");
        let state = BridgeState::default();
        let (lifecycle, _) = start_fixture(&state, "main", &config_dir, &managed, &config);

        let failure = state
            .replace_picker_directory("main", UniversePickerKind::Managed, &replacement)
            .expect_err("active process must retain its root authority");
        assert_eq!(failure.code, IpcErrorCode::Conflict);
        assert_eq!(
            state
                .picker
                .root("main", UniversePickerKind::Managed)
                .expect("original root"),
            fs::canonicalize(&managed).expect("canonical root")
        );

        let stop = state
            .lifecycle_action("main", config_dir, managed)
            .expect("stop authority");
        stop.run(false, &config, "yes-app", &backend::CancellationToken::new(), |_| {})
            .expect("stop fixture");
        drop(stop);
        assert!(lifecycle.manager.identity("yes-app").expect("identity").is_none());
    }

    #[test]
    fn window_teardown_stops_and_reaps_active_process_without_cross_window_authority() {
        let root = TestDir::new("teardown-active");
        let (config_dir, managed, config) = running_fixture(&root.0);
        let state = BridgeState::default();
        let (lifecycle, pid) = start_fixture(&state, "main", &config_dir, &managed, &config);

        assert!(state.lifecycle.lock().expect("registry").get("other").is_none());
        state.clear_window("other");
        assert!(lifecycle.manager.identity("yes-app").expect("identity").is_some());

        state.clear_window("main");
        assert!(lifecycle.manager.identity("yes-app").expect("identity").is_none());
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
        assert!(state.lifecycle.lock().expect("registry").get("main").is_none());
    }
}
