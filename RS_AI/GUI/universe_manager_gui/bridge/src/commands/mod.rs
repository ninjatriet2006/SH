pub mod config;
pub mod helpers;
pub mod launcher;
pub mod lifecycle;
pub mod picker;
pub mod preferences;
pub mod scan;
pub mod search;

#[cfg(all(test, target_os = "linux"))]
mod tests;

pub use config::*;
pub use helpers::*;
pub use launcher::*;
pub use lifecycle::*;
pub use picker::*;
pub use preferences::*;
pub use scan::*;
pub use search::*;

use crate::contract::*;
use crate::security::PickerProvenance;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use tauri::{Emitter, Runtime, Window};
use universe_manager_backend as backend;

pub const JOB_SCAN_APPS: &str = "job:scan_apps";
pub const JOB_DETECT_APP: &str = "job:detect_app";
pub const JOB_START_APP: &str = "job:start_app";
pub const JOB_STOP_APP: &str = "job:stop_app";
pub const JOB_SEARCH_APPS: &str = "job:search_apps";
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
    CommandContract::job(
        "scan_apps",
        JOB_SCAN_APPS,
        &[IpcErrorCode::Io, IpcErrorCode::Cancelled],
    ),
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
        &[
            IpcErrorCode::Validation,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::command(
        "preferences_get",
        &[IpcErrorCode::Io, IpcErrorCode::Internal],
    ),
    CommandContract::command(
        "preferences_set",
        &[IpcErrorCode::Validation, IpcErrorCode::Io],
    ),
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

pub struct WindowLifecycle {
    pub manager: backend::LifecycleManager,
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
        let mut status = match self.status.lock() {
            Ok(s) => s,
            Err(_) => {
                return Err(error(
                    IpcErrorCode::Internal,
                    "lifecycle status lock is poisoned",
                ));
            }
        };
        status.revoked = true;
        while status.in_flight != 0 {
            status = match self.idle.wait(status) {
                Ok(s) => s,
                Err(_) => {
                    return Err(error(
                        IpcErrorCode::Internal,
                        "lifecycle status lock is poisoned",
                    ));
                }
            };
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
        match first_error {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

impl Drop for WindowLifecycle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

pub struct LifecycleAction {
    pub lifecycle: Arc<WindowLifecycle>,
}

impl LifecycleAction {
    pub fn run(
        &self,
        start: bool,
        config: &ManagerConfig,
        app_id: &str,
        cancellation: &backend::CancellationToken,
        progress: impl FnMut(backend::Progress),
    ) -> Result<backend::OperationResult, backend::BackendError> {
        let mut status = match self.lifecycle.status.lock() {
            Ok(s) => s,
            Err(_) => {
                return Err(backend::BackendError::new(
                    backend::ErrorKind::Internal,
                    "lifecycle status lock is poisoned",
                ));
            }
        };
        let result = match start {
            true => self
                .lifecycle
                .manager
                .start(config, app_id, true, cancellation, progress),
            false => self
                .lifecycle
                .manager
                .stop(config, app_id, true, cancellation, progress),
        };
        match &result {
            Ok(_) => match start {
                true => {
                    status.running.insert(app_id.to_owned(), config.clone());
                }
                false => {
                    status.running.remove(app_id);
                }
            },
            Err(_) => {}
        }
        result
    }
}

impl Drop for LifecycleAction {
    fn drop(&mut self) {
        match self.lifecycle.status.lock() {
            Ok(mut status) => {
                status.in_flight = status.in_flight.saturating_sub(1);
                self.lifecycle.idle.notify_all();
            }
            Err(_) => {}
        }
    }
}

#[derive(Default)]
struct WindowJobs {
    generation: u64,
    used: HashSet<String>,
    active: HashMap<String, (u64, backend::CancellationToken)>,
}

pub struct ActiveJob {
    jobs: Arc<Mutex<HashMap<String, WindowJobs>>>,
    window: String,
    request_id: String,
    generation: u64,
    pub cancellation: backend::CancellationToken,
}

impl Drop for ActiveJob {
    fn drop(&mut self) {
        match self.jobs.lock() {
            Ok(mut jobs) => match jobs.get_mut(&self.window) {
                Some(window) => match window.generation == self.generation {
                    true => {
                        window.active.remove(&self.request_id);
                    }
                    false => {}
                },
                None => {}
            },
            Err(_) => {}
        }
    }
}

impl BridgeState {
    pub fn generation(&self, window: &str, code: IpcErrorCode) -> Result<u64, IpcError> {
        let mut jobs = match self.jobs.lock() {
            Ok(j) => j,
            Err(_) => return Err(error(code, "job state lock is poisoned")),
        };
        Ok(jobs.entry(window.to_owned()).or_default().generation)
    }

    pub fn register(
        &self,
        window: &str,
        request_id: String,
        generation: u64,
        code: IpcErrorCode,
    ) -> Result<ActiveJob, IpcError> {
        let cancellation = backend::CancellationToken::new();
        let mut jobs = match self.jobs.lock() {
            Ok(j) => j,
            Err(_) => return Err(error(code, "job state lock is poisoned")),
        };
        let entry = jobs.entry(window.to_owned()).or_default();
        match entry.generation == generation {
            true => {}
            false => {
                return Err(error(
                    code,
                    "window lifecycle changed during job preflight",
                ));
            }
        }
        match entry.used.insert(request_id.clone()) {
            true => {}
            false => {
                return Err(error(
                    code,
                    "request_id was already used in this window lifecycle",
                ));
            }
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

    pub fn lifecycle_action(
        &self,
        window: &str,
        config_dir: PathBuf,
        managed_root: PathBuf,
    ) -> Result<LifecycleAction, IpcError> {
        let mut managers = match self.lifecycle.lock() {
            Ok(m) => m,
            Err(_) => return Err(error(IpcErrorCode::Io, "lifecycle lock is poisoned")),
        };
        let lifecycle = match managers.get(window) {
            Some(lifecycle) => Arc::clone(lifecycle),
            None => {
                let store = match backend::ConfigStore::new(config_dir, vec![managed_root]) {
                    Ok(s) => s,
                    Err(failure) => return Err(map_backend(failure)),
                };
                let lifecycle =
                    Arc::new(WindowLifecycle::new(backend::LifecycleManager::new(store)));
                managers.insert(window.to_owned(), Arc::clone(&lifecycle));
                lifecycle
            }
        };
        let mut status = match lifecycle.status.lock() {
            Ok(s) => s,
            Err(_) => return Err(error(IpcErrorCode::Io, "lifecycle status lock is poisoned")),
        };
        match status.revoked {
            true => {
                return Err(error(
                    IpcErrorCode::Forbidden,
                    "window lifecycle authority was revoked",
                ));
            }
            false => {}
        }
        status.in_flight = match status.in_flight.checked_add(1) {
            Some(count) => count,
            None => {
                return Err(error(
                    IpcErrorCode::Internal,
                    "lifecycle action count overflow",
                ));
            }
        };
        drop(status);
        Ok(LifecycleAction { lifecycle })
    }

    fn reset_lifecycle(&self, window: &str) -> Result<(), IpcError> {
        let mut managers = match self.lifecycle.lock() {
            Ok(m) => m,
            Err(_) => return Err(error(IpcErrorCode::Internal, "lifecycle lock is poisoned")),
        };
        let lifecycle = match managers.get(window).cloned() {
            Some(l) => l,
            None => return Ok(()),
        };
        let mut status = match lifecycle.status.lock() {
            Ok(s) => s,
            Err(_) => {
                return Err(error(
                    IpcErrorCode::Internal,
                    "lifecycle status lock is poisoned",
                ));
            }
        };
        match status.in_flight != 0 || !status.running.is_empty() {
            true => Err(error(
                IpcErrorCode::Conflict,
                "managed root cannot change while an application action or process is active",
            )),
            false => {
                status.revoked = true;
                drop(status);
                managers.remove(window);
                Ok(())
            }
        }
    }

    pub fn replace_picker_directory(
        &self,
        window: &str,
        kind: UniversePickerKind,
        path: &Path,
    ) -> Result<PathBuf, IpcError> {
        match kind {
            UniversePickerKind::Managed => {
                self.reset_lifecycle(window)?;
            }
            _ => {}
        }
        self.picker.replace_directory(window, kind, path)
    }

    pub fn clear_window(&self, window: &str) {
        match self.jobs.lock() {
            Ok(mut jobs) => {
                let generation = match jobs.get(window) {
                    Some(entry) => entry.generation.saturating_add(1),
                    None => 1,
                };
                match jobs.remove(window) {
                    Some(entry) => {
                        for (_, cancellation) in entry.active.into_values() {
                            cancellation.cancel();
                        }
                    }
                    None => {}
                }
                jobs.insert(
                    window.to_owned(),
                    WindowJobs {
                        generation,
                        ..WindowJobs::default()
                    },
                );
            }
            Err(_) => {}
        }
        match self.lifecycle.lock() {
            Ok(mut managers) => match managers.get(window).cloned() {
                Some(lifecycle) => match lifecycle.shutdown() {
                    Ok(_) => {
                        managers.remove(window);
                    }
                    Err(_) => {}
                },
                None => {}
            },
            Err(_) => {}
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
        match self.terminal || state.is_terminal() != result.is_some() {
            true => {
                return Err(error(
                    IpcErrorCode::Internal,
                    "invalid job lifecycle transition",
                ));
            }
            false => {}
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
        self.seq = match self.seq.checked_add(1) {
            Some(s) => s,
            None => return Err(error(IpcErrorCode::Internal, "job sequence overflow")),
        };
        self.terminal = state.is_terminal();
        Ok(event)
    }
}

pub struct JobLifecycle<'a, R: Runtime, T: Serialize + Clone> {
    window: &'a Window<R>,
    topic: &'static str,
    sequence: EventSequence<T>,
}

impl<'a, R: Runtime, T: Serialize + Clone> JobLifecycle<'a, R, T> {
    fn start(
        window: &'a Window<R>,
        topic: &'static str,
        request_id: String,
    ) -> Result<Self, IpcError> {
        let mut lifecycle = Self {
            window,
            topic,
            sequence: EventSequence::new(request_id),
        };
        lifecycle.emit(JobState::Started, None, None, None, None)?;
        Ok(lifecycle)
    }

    pub fn progress(&mut self, progress: backend::Progress) {
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
            Ok(value) => (
                JobState::Completed,
                JobResult::Completed { request_id, value },
            ),
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
        let event = self
            .sequence
            .event(state, completed, total, message, result)?;
        self.window
            .emit(self.topic, event)
            .map_err(|source| error(IpcErrorCode::Internal, source.to_string()))
    }
}

pub fn validate<T>(request: &Req<T>, job: bool) -> Result<Option<String>, IpcError> {
    match request.schema_version == SCHEMA_VERSION {
        true => {}
        false => return Err(error(IpcErrorCode::InvalidArgument, "schema_version must be 1")),
    }
    match (&request.request_id, job) {
        (Some(id), true) => match !id.trim().is_empty() {
            true => Ok(Some(id.clone())),
            false => Err(error(IpcErrorCode::InvalidArgument, "request_id is required")),
        },
        (None, false) => Ok(None),
        (_, true) => Err(error(IpcErrorCode::InvalidArgument, "request_id is required")),
        (_, false) => Err(error(
            IpcErrorCode::InvalidArgument,
            "request_id must be null",
        )),
    }
}

pub fn response<T>(request_id: Option<String>, data: T) -> Res<T> {
    Res {
        schema_version: SCHEMA_VERSION,
        request_id,
        data,
    }
}

pub fn job_request<T>(request: &Req<T>) -> Result<String, IpcError> {
    match validate(request, true)? {
        Some(id) => Ok(id),
        None => Err(error(
            IpcErrorCode::InvalidArgument,
            "validated request has no ID",
        )),
    }
}

pub async fn run_job<R, T, F>(
    window: Window<R>,
    topic: &'static str,
    active: ActiveJob,
    operation: F,
) -> IpcResult<T>
where
    R: Runtime,
    T: Serialize + Clone + Send + 'static,
    F: FnOnce(&mut JobLifecycle<'_, R, T>, &backend::CancellationToken) -> Result<T, IpcError>
        + Send
        + 'static,
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
