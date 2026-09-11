use crate::contract::*;
use crate::security::{PickerKind, PickerProvenance, forbidden};
use fs2::FileExt;
use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, Window};
use universal_converter_backend as backend;

pub const JOB_DEPENDENCIES_CHECK: &str = "job.dependencies_check";
pub const JOB_CLASSIFY_FILE: &str = "job.classify_file";
pub const JOB_SCAN_DIRECTORY: &str = "job.scan_directory";
pub const JOB_BATCH_CONVERT: &str = "job.batch_convert";
pub const JOB_NATIVE_INSTALL: &str = "job.native_install";
pub const JOB_NATIVE_UNINSTALL: &str = "job.native_uninstall";
pub const INVOKE_REGISTRY: &[CommandContract] = &[
    CommandContract::job(
        "dependencies_check",
        JOB_DEPENDENCIES_CHECK,
        &[IpcErrorCode::Unavailable, IpcErrorCode::Internal],
    ),
    CommandContract::job(
        "classify_file",
        JOB_CLASSIFY_FILE,
        &[IpcErrorCode::InvalidArgument, IpcErrorCode::NotFound, IpcErrorCode::Io],
    ),
    CommandContract::job(
        "scan_directory",
        JOB_SCAN_DIRECTORY,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::NotFound,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "batch_convert",
        JOB_BATCH_CONVERT,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::Conflict,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "native_install",
        JOB_NATIVE_INSTALL,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::Forbidden,
            IpcErrorCode::Unavailable,
            IpcErrorCode::Conflict,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "native_uninstall",
        JOB_NATIVE_UNINSTALL,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::Forbidden,
            IpcErrorCode::Unavailable,
            IpcErrorCode::NotFound,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::command("preferences_get", &[IpcErrorCode::Io, IpcErrorCode::Internal]),
    CommandContract::command("preferences_set", &[IpcErrorCode::Validation, IpcErrorCode::Io]),
    // This bridge-owned picker is the ninth invoke command. It is required to establish
    // per-window provenance without granting the webview a dialog-plugin permission.
    CommandContract::command(
        "picker_select",
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::NotFound,
            IpcErrorCode::Io,
            IpcErrorCode::Internal,
        ],
    ),
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
static PREFERENCES_WRITE_ID: AtomicU64 = AtomicU64::new(0);
static PREFERENCES_WRITE_LOCK: Mutex<()> = Mutex::new(());
const DEJAVU_HASH: &str = "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280";
const LICENSE_HASH: &str = "63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9";
const FONT_MANIFEST: &str = "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280  DejaVuSans.ttf\n63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9  LICENSE.txt\n";
const NATIVE_STORE_LOCK_TIMEOUT: Duration = Duration::from_secs(2);
const NATIVE_STORE_LOCK_POLL: Duration = Duration::from_millis(10);

pub fn resource_initialization_script(resource_root: &Path) -> io::Result<String> {
    let root = fs::canonicalize(resource_root)?;
    let en = resource_or_missing(&root, "langs/en.json")?;
    let vi = resource_or_missing(&root, "langs/vi.json")?;
    let system = resource_or_missing(&root, "themes/system.json")?;
    let light = resource_or_missing(&root, "themes/light.json")?;
    let dark = resource_or_missing(&root, "themes/dark.json")?;
    let fonts = verified_fonts(&root)?;

    let paths = serde_json::json!({
        "languages": {"en": en, "vi": vi},
        "themes": {"system": system, "light": light, "dark": dark},
        "fonts": {"primary": fonts.primary},
    });
    let serialized = serde_json::to_string(&paths).map_err(io::Error::other)?;
    Ok(format!("window.__UNIVERSAL_CONVERTER_RESOURCES__={serialized};"))
}

fn resource_or_missing(root: &Path, relative: &str) -> io::Result<PathBuf> {
    match contained_physical_resource(root, relative) {
        Ok(path) => Ok(path),
        Err(_) => Ok(root.join(".unavailable-resource").join(relative)),
    }
}

struct VerifiedFonts {
    primary: Option<PathBuf>,
}

fn verified_fonts(root: &Path) -> io::Result<VerifiedFonts> {
    let primary = contained_physical_resource(root, "fonts/DejaVuSans.ttf")
        .and_then(|path| verify_sha256(&path, DEJAVU_HASH).map(|()| path))
        .ok();
    let license = contained_physical_resource(root, "fonts/LICENSE.txt")?;
    let manifest = contained_physical_resource(root, "fonts/manifest.sha256")?;
    verify_sha256(&license, LICENSE_HASH)?;
    if fs::read_to_string(manifest)? != FONT_MANIFEST {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "font manifest does not match the bundled contract",
        ));
    }
    Ok(VerifiedFonts { primary })
}

fn contained_physical_resource(root: &Path, relative: &str) -> io::Result<PathBuf> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "resource ID must be a contained relative path",
        ));
    }
    let mut candidate = root.to_path_buf();
    for component in relative_path.components() {
        candidate.push(component.as_os_str());
        if fs::symlink_metadata(&candidate)?.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "resource path components must be physical",
            ));
        }
    }
    let canonical = fs::canonicalize(candidate)?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "resource escapes the resource root",
        ));
    }
    Ok(canonical)
}

fn verify_sha256(path: &Path, expected: &str) -> io::Result<()> {
    let digest = format!("{:x}", Sha256::digest(fs::read(path)?));
    if digest == expected {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "bundled resource hash mismatch",
        ))
    }
}

#[derive(Clone)]
pub struct BridgeState {
    pub picker: PickerProvenance,
    native: Arc<Mutex<Option<backend::NativeManager>>>,
    native_operation: Arc<Mutex<()>>,
    native_records_path: Arc<Mutex<Option<PathBuf>>>,
    jobs: Arc<Mutex<HashMap<String, WindowJobs>>>,
    #[cfg(test)]
    native_lock_hook: Arc<Mutex<Option<NativeLockHook>>>,
    #[cfg(test)]
    native_preflight_hook: Arc<Mutex<Option<NativePreflightHook>>>,
    #[cfg(test)]
    fail_after_native_persist: Arc<Mutex<bool>>,
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum NativeLockEvent {
    Waiting,
    Acquired,
}

#[cfg(test)]
type NativeLockHook = Arc<dyn Fn(NativeLockEvent) + Send + Sync>;

#[cfg(test)]
type NativePreflightHook = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
struct WindowJobs {
    generation: u64,
    used: HashSet<String>,
    active: HashMap<String, (u64, backend::CancellationToken)>,
}

impl Default for BridgeState {
    fn default() -> Self {
        Self {
            picker: PickerProvenance::default(),
            native: Arc::new(Mutex::new(None)),
            native_operation: Arc::new(Mutex::new(())),
            native_records_path: Arc::new(Mutex::new(None)),
            jobs: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(test)]
            native_lock_hook: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            native_preflight_hook: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            fail_after_native_persist: Arc::new(Mutex::new(false)),
        }
    }
}

struct ActiveJob {
    jobs: Arc<Mutex<HashMap<String, WindowJobs>>>,
    window: String,
    request_id: String,
    generation: u64,
    cancellation: backend::CancellationToken,
}

struct PreparedNativeInstall {
    artifact_roots: Vec<PathBuf>,
    user_roots: backend::UserInstallRoots,
    request: backend::NativeInstallRequest,
}

impl Drop for ActiveJob {
    fn drop(&mut self) {
        if let Ok(mut jobs) = self.jobs.lock()
            && let Some(window) = jobs.get_mut(&self.window)
            && window.generation == self.generation
            && window
                .active
                .get(&self.request_id)
                .is_some_and(|(generation, _)| *generation == self.generation)
        {
            window.active.remove(&self.request_id);
        }
    }
}

struct JobLifecycle<'a, R: Runtime, T: Serialize + Clone> {
    window: &'a Window<R>,
    topic: &'static str,
    request_id: String,
    job_id: String,
    next_seq: u64,
    terminal: bool,
    marker: std::marker::PhantomData<T>,
}

impl<'a, R: Runtime, T: Serialize + Clone> JobLifecycle<'a, R, T> {
    fn start(window: &'a Window<R>, topic: &'static str, request_id: String) -> Result<Self, IpcError> {
        let mut lifecycle = Self {
            window,
            topic,
            job_id: request_id.clone(),
            request_id,
            next_seq: 0,
            terminal: false,
            marker: std::marker::PhantomData,
        };
        lifecycle.emit(JobState::Started, None, None, None, None)?;
        Ok(lifecycle)
    }

    fn progress(
        &mut self,
        completed: Option<u64>,
        total: Option<u64>,
        message: Option<String>,
    ) -> Result<(), IpcError> {
        self.emit(JobState::Progress, completed, total, message, None)
    }

    fn complete(&mut self, value: T) -> Result<(), IpcError> {
        self.emit(
            JobState::Completed,
            None,
            None,
            None,
            Some(JobResult::Completed {
                request_id: self.request_id.clone(),
                value,
            }),
        )
    }

    fn fail(&mut self, error: IpcError) -> Result<(), IpcError> {
        let state = if error.code == IpcErrorCode::Cancelled {
            JobState::Cancelled
        } else {
            JobState::Failed
        };
        let result = if state == JobState::Cancelled {
            JobResult::Cancelled {
                request_id: self.request_id.clone(),
                error,
            }
        } else {
            JobResult::Failed {
                request_id: self.request_id.clone(),
                error,
            }
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
        if self.terminal {
            return Err(internal("attempted to emit after terminal event"));
        }
        if state.is_terminal() != result.is_some() {
            return Err(internal("job terminal state/result mismatch"));
        }
        let event = JobEvent {
            schema_version: SCHEMA_VERSION,
            job_id: self.job_id.clone(),
            seq: self.next_seq,
            state,
            payload: JobProgress {
                request_id: self.request_id.clone(),
                completed,
                total,
                message,
                result,
            },
        };
        self.window
            .emit(self.topic, event)
            .map_err(|error| internal(error.to_string()))?;
        self.next_seq = self
            .next_seq
            .checked_add(1)
            .ok_or_else(|| internal("job sequence overflow"))?;
        self.terminal = state.is_terminal();
        Ok(())
    }
}

fn validate_request<T>(request: &Req<T>, job: bool) -> Result<Option<String>, IpcError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(invalid("schema_version must be 1"));
    }
    match (&request.request_id, job) {
        (Some(id), true) if !id.trim().is_empty() => Ok(Some(id.clone())),
        (None, false) => Ok(None),
        (_, true) => Err(invalid("request_id is required for job commands")),
        (_, false) => Err(invalid("request_id must be null for non-job commands")),
    }
}

fn job_request_id<T>(request: &Req<T>) -> Result<String, IpcError> {
    validate_request(request, true)?.ok_or_else(|| internal("validated job request has no request_id"))
}

impl BridgeState {
    fn window_generation(&self, window: &str, state_code: IpcErrorCode) -> Result<u64, IpcError> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| plain_error(state_code, "job state lock is poisoned"))?;
        Ok(jobs.entry(window.to_owned()).or_default().generation)
    }

    #[cfg(test)]
    fn register_job(
        &self,
        window: &str,
        request_id: String,
        duplicate_code: IpcErrorCode,
        state_code: IpcErrorCode,
    ) -> Result<ActiveJob, IpcError> {
        let generation = self.window_generation(window, state_code)?;
        self.register_job_if_generation(window, request_id, generation, duplicate_code, state_code)
    }

    fn register_job_if_generation(
        &self,
        window: &str,
        request_id: String,
        expected_generation: u64,
        duplicate_code: IpcErrorCode,
        state_code: IpcErrorCode,
    ) -> Result<ActiveJob, IpcError> {
        let cancellation = backend::CancellationToken::new();
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| plain_error(state_code, "job state lock is poisoned"))?;
        let window_jobs = jobs.entry(window.to_owned()).or_default();
        if window_jobs.generation != expected_generation {
            return Err(plain_error(state_code, "window lifecycle changed during job preflight"));
        }
        if !window_jobs.used.insert(request_id.clone()) {
            return Err(plain_error(
                duplicate_code,
                "request_id was already used in this window lifecycle",
            ));
        }
        let generation = expected_generation;
        window_jobs
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

    fn prepare_job<T, P, F>(
        &self,
        window: &str,
        request: &Req<T>,
        request_code: IpcErrorCode,
        duplicate_code: IpcErrorCode,
        state_code: IpcErrorCode,
        preflight: F,
    ) -> Result<(ActiveJob, P), IpcError>
    where
        F: FnOnce() -> Result<P, IpcError>,
    {
        let request_id = job_request_id(request).map_err(|error| remap_error(error, request_code))?;
        let generation = self.window_generation(window, state_code)?;
        let prepared = preflight()?;
        let active = self.register_job_if_generation(window, request_id, generation, duplicate_code, state_code)?;
        Ok((active, prepared))
    }

    async fn prepare_native_install(
        &self,
        window: &str,
        request: &Req<NativeInstallRequest>,
    ) -> Result<(ActiveJob, PreparedNativeInstall), IpcError> {
        let request_id = job_request_id(request).map_err(|error| remap_error(error, IpcErrorCode::InvalidArgument))?;
        require_native_platform()?;
        let generation = self.window_generation(window, IpcErrorCode::Io)?;
        let backend_request = validate_native_install_dto(&request.payload)?;
        let state = self.clone();
        let window_label = window.to_owned();
        let prepared = tauri::async_runtime::spawn_blocking(move || {
            state.preflight_native_install(&window_label, backend_request)
        })
        .await
        .map_err(|error| {
            plain_error(
                IpcErrorCode::Io,
                format!("native install preflight worker failed: {error}"),
            )
        })??;
        let active =
            self.register_job_if_generation(window, request_id, generation, IpcErrorCode::Conflict, IpcErrorCode::Io)?;
        Ok((active, prepared))
    }
}

fn response<T>(request_id: Option<String>, data: T) -> Res<T> {
    Res {
        schema_version: SCHEMA_VERSION,
        request_id,
        data,
    }
}

fn run_job<R, T, F>(
    window: &Window<R>,
    topic: &'static str,
    request_id: String,
    infrastructure_code: IpcErrorCode,
    operation: F,
) -> IpcResult<T>
where
    R: Runtime,
    T: Serialize + Clone,
    F: FnOnce(&mut JobLifecycle<'_, R, T>) -> Result<T, IpcError>,
{
    let mut lifecycle = JobLifecycle::start(window, topic, request_id.clone())
        .map_err(|error| remap_error(error, infrastructure_code))?;
    match operation(&mut lifecycle) {
        Ok(value) => {
            lifecycle
                .complete(value.clone())
                .map_err(|error| remap_error(error, infrastructure_code))?;
            Ok(response(Some(request_id), value))
        }
        Err(error) => {
            lifecycle
                .fail(error.clone())
                .map_err(|emit_error| remap_error(emit_error, infrastructure_code))?;
            Err(error)
        }
    }
}

async fn run_job_non_blocking<R, T, F>(
    window: Window<R>,
    topic: &'static str,
    active: ActiveJob,
    infrastructure_code: IpcErrorCode,
    operation: F,
) -> IpcResult<T>
where
    R: Runtime,
    T: Serialize + Clone + Send + 'static,
    F: FnOnce(&mut JobLifecycle<'_, R, T>, &backend::CancellationToken) -> Result<T, IpcError> + Send + 'static,
{
    let request_id = active.request_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = run_job(&window, topic, request_id, infrastructure_code, |job| {
            operation(job, &active.cancellation)
        });
        drop(active);
        result
    })
    .await
    .map_err(|error| plain_error(infrastructure_code, format!("job worker failed: {error}")))?
}

#[tauri::command]
pub async fn dependencies_check<R: Runtime>(request: Req<Empty>, window: Window<R>) -> IpcResult<DependencyReport> {
    let (active, ()) = window.state::<BridgeState>().prepare_job(
        window.label(),
        &request,
        IpcErrorCode::Internal,
        IpcErrorCode::Internal,
        IpcErrorCode::Internal,
        || Ok(()),
    )?;
    run_job_non_blocking(
        window,
        JOB_DEPENDENCIES_CHECK,
        active,
        IpcErrorCode::Internal,
        |_, cancellation| {
            backend::check_dependencies_cancellable(cancellation)
                .map(map_dependency_report)
                .map_err(map_dependency_error)
        },
    )
    .await
}

#[tauri::command]
pub async fn classify_file<R: Runtime>(
    request: Req<ClassifyFileRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<Classification> {
    let (active, path) = state.prepare_job(
        window.label(),
        &request,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::Io,
        || {
            state
                .picker
                .resolve_existing(window.label(), PickerKind::Input, Path::new(&request.payload.path))
                .map_err(map_picker_argument_error)
        },
    )?;
    run_job_non_blocking(window, JOB_CLASSIFY_FILE, active, IpcErrorCode::Io, move |_, _| {
        backend::classify_file(path)
            .map(map_classification)
            .map_err(map_classify_error)
    })
    .await
}

#[tauri::command]
pub async fn scan_directory<R: Runtime>(
    request: Req<ScanDirectoryRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<ScanReport> {
    let (active, (directory, types)) = state.prepare_job(
        window.label(),
        &request,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::Io,
        || {
            let directory = state
                .picker
                .resolve_existing(window.label(), PickerKind::Input, Path::new(&request.payload.directory))
                .map_err(map_picker_argument_error)?;
            let types = parse_file_types(&request.payload.allowed_types)?;
            Ok((directory, types))
        },
    )?;
    run_job_non_blocking(
        window,
        JOB_SCAN_DIRECTORY,
        active,
        IpcErrorCode::Io,
        move |job, cancellation| {
            let report = backend::scan_directory_cancellable(directory, &types, cancellation, |progress| {
                let message = progress.current_path.map(|path| path.to_string_lossy().into_owned());
                let _ = job.progress(Some(progress.completed), progress.total, message);
            })
            .map_err(map_scan_error)?;
            Ok(map_scan_report(report))
        },
    )
    .await
}

#[tauri::command]
pub async fn batch_convert<R: Runtime>(
    request: Req<BatchConvertRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<BatchConvertReport> {
    let (active, (backend_request, roots)) = state.prepare_job(
        window.label(),
        &request,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::Conflict,
        IpcErrorCode::Io,
        || {
            let input_roots = state
                .picker
                .canonical_roots(window.label(), PickerKind::Input)
                .map_err(map_picker_argument_error)?;
            let output_roots = state
                .picker
                .canonical_roots(window.label(), PickerKind::Output)
                .map_err(map_picker_argument_error)?;
            let backend_request = backend::BatchConvertRequest {
                files: request
                    .payload
                    .files
                    .iter()
                    .map(|item| PathBuf::from(&item.path))
                    .collect(),
                output_directory: PathBuf::from(&request.payload.output_directory.path),
                output_format: request.payload.output_format.clone(),
                overwrite: request.payload.overwrite,
            };
            let roots = backend::ConversionRoots::new(input_roots, output_roots).map_err(map_batch_error)?;
            backend::preflight_batch_conversion(&backend_request, &roots).map_err(map_batch_error)?;
            Ok((backend_request, roots))
        },
    )?;
    run_job_non_blocking(
        window,
        JOB_BATCH_CONVERT,
        active,
        IpcErrorCode::Io,
        move |job, cancellation| {
            backend::Converter::default()
                .convert_batch(&backend_request, &roots, cancellation, |progress| {
                    let message = progress.current_file.map(|path| path.to_string_lossy().into_owned());
                    let _ = job.progress(Some(progress.completed), Some(progress.total), message);
                })
                .map(map_batch_report)
                .map_err(map_batch_error)
        },
    )
    .await
}

#[tauri::command]
pub async fn native_install<R: Runtime>(
    request: Req<NativeInstallRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<NativeOperationResult> {
    let (active, prepared) = state.prepare_native_install(window.label(), &request).await?;
    let state = state.inner().clone();
    let window_label = window.label().to_owned();
    run_job_non_blocking(
        window,
        JOB_NATIVE_INSTALL,
        active,
        IpcErrorCode::Io,
        move |job, cancellation| {
            state
                .install_native(
                    &window_label,
                    prepared.artifact_roots,
                    prepared.user_roots,
                    &prepared.request,
                    cancellation,
                    |progress| {
                        let message = progress.path.map(|path| path.to_string_lossy().into_owned());
                        let _ = job.progress(Some(progress.completed), Some(progress.total), message);
                    },
                )
                .map(map_native_result)
        },
    )
    .await
}

#[tauri::command]
pub async fn native_uninstall<R: Runtime>(
    request: Req<NativeUninstallRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<NativeOperationResult> {
    require_native_platform()?;
    require_confirmed(request.payload.confirmed)?;
    let state = state.inner().clone();
    let window_label = window.label().to_owned();
    let (active, ()) = state.prepare_job(
        window.label(),
        &request,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::Io,
        || {
            state
                .existing_native_manager(&window_label)
                .map(|_| ())
                .map_err(map_native_uninstall_preflight_error)
        },
    )?;
    run_job_non_blocking(
        window,
        JOB_NATIVE_UNINSTALL,
        active,
        IpcErrorCode::Io,
        move |job, cancellation| {
            let backend_request = backend::NativeUninstallRequest {
                installation_id: request.payload.installation_id,
                confirmed: true,
            };
            state
                .uninstall_native(&window_label, &backend_request, cancellation, |progress| {
                    let message = progress.path.map(|path| path.to_string_lossy().into_owned());
                    let _ = job.progress(Some(progress.completed), Some(progress.total), message);
                })
                .map(map_native_result)
        },
    )
    .await
}

#[tauri::command]
pub fn preferences_get<R: Runtime>(request: Req<Empty>, app: AppHandle<R>) -> IpcResult<Preferences> {
    let request_id = validate_request(&request, false).map_err(|error| remap_error(error, IpcErrorCode::Internal))?;
    let path = preferences_path(&app)?;
    let preferences = load_or_migrate_preferences(&path)?;
    Ok(response(request_id, preferences))
}

#[tauri::command]
pub fn preferences_set<R: Runtime>(request: Req<Preferences>, app: AppHandle<R>) -> IpcResult<Preferences> {
    let request_id = validate_request(&request, false).map_err(|error| remap_error(error, IpcErrorCode::Validation))?;
    validate_preferences(&request.payload)?;
    let path = preferences_path(&app).map_err(|error| remap_error(error, IpcErrorCode::Io))?;
    atomic_write_json(&path, &request.payload).map_err(|error| remap_error(error, IpcErrorCode::Io))?;
    Ok(response(request_id, request.payload))
}

#[tauri::command]
pub async fn picker_select<R: Runtime>(
    request: Req<PickerSelectRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<PickerSelectResult> {
    let request_id = validate_request(&request, false)?;
    validate_picker_intent(&request.payload)?;
    let kind = request.payload.kind;
    let selection = request.payload.selection;
    let dialog = rfd::FileDialog::new().set_parent(&window);
    let selected = tauri::async_runtime::spawn_blocking(move || match selection {
        PickerSelection::File => dialog.pick_file().map(|path| vec![path]),
        PickerSelection::Files => dialog.pick_files(),
        PickerSelection::Directory => dialog.pick_folder().map(|path| vec![path]),
    })
    .await
    .map_err(|error| internal(format!("picker worker failed: {error}")))?;

    let paths = apply_picker_result(&state.picker, window.label(), kind.into(), selected)?;
    Ok(response(
        request_id,
        PickerSelectResult {
            kind,
            paths: paths.into_iter().map(path_ref).collect(),
        },
    ))
}

fn apply_picker_result(
    picker: &PickerProvenance,
    window: &str,
    kind: PickerKind,
    paths: Option<Vec<PathBuf>>,
) -> Result<Vec<PathBuf>, IpcError> {
    let Some(paths) = paths else {
        return Ok(Vec::new());
    };
    picker.replace_paths(window, kind, &paths)?;
    Ok(paths)
}

fn validate_picker_intent(request: &PickerSelectRequest) -> Result<(), IpcError> {
    match (request.kind, request.selection) {
        (crate::contract::PickerKind::Output, PickerSelection::Directory)
        | (crate::contract::PickerKind::Artifact, PickerSelection::File | PickerSelection::Directory)
        | (crate::contract::PickerKind::Input, _) => Ok(()),
        (crate::contract::PickerKind::Output, _) => Err(invalid("output picker must select a directory")),
        (crate::contract::PickerKind::Artifact, PickerSelection::Files) => {
            Err(invalid("artifact picker must select one file or directory"))
        }
    }
}

fn parse_file_types(values: &[String]) -> Result<Vec<backend::FileType>, IpcError> {
    if values.is_empty() {
        return Err(invalid("allowed_types must not be empty"));
    }
    values
        .iter()
        .map(|value| match value.as_str() {
            "video" => Ok(backend::FileType::Video),
            "audio" => Ok(backend::FileType::Audio),
            "image" => Ok(backend::FileType::Image),
            "document" => Ok(backend::FileType::Document),
            "archive" => Ok(backend::FileType::Archive),
            "directory" => Ok(backend::FileType::Directory),
            "unknown" => Ok(backend::FileType::Unknown),
            _ => Err(invalid(format!("unsupported allowed type: {value}"))),
        })
        .collect()
}

fn map_dependency_report(value: backend::DependencyReport) -> DependencyReport {
    DependencyReport {
        is_ok: value.is_ok,
        missing: value.missing,
    }
}

fn map_classification(value: backend::Classification) -> Classification {
    Classification {
        path: value.path.to_string_lossy().into_owned(),
        file_type: value.file_type.as_str().to_owned(),
        size_bytes: value.size_bytes,
    }
}

fn map_scan_report(value: backend::ScanReport) -> ScanReport {
    ScanReport {
        directory: value.directory.to_string_lossy().into_owned(),
        files: value.files.into_iter().map(map_classification).collect(),
        total: value.total,
    }
}

fn path_ref(path: PathBuf) -> PathRef {
    PathRef {
        path: path.to_string_lossy().into_owned(),
    }
}

fn map_batch_report(value: backend::BatchConvertReport) -> BatchConvertReport {
    BatchConvertReport {
        output_directory: path_ref(value.output_directory),
        converted: value.converted.into_iter().map(path_ref).collect(),
        failed: value.failed.into_iter().map(path_ref).collect(),
    }
}

fn map_native_result(value: backend::NativeOperationResult) -> NativeOperationResult {
    NativeOperationResult {
        operation: match value.operation {
            backend::NativeOperation::Install => "install",
            backend::NativeOperation::Uninstall => "uninstall",
        }
        .to_owned(),
        installation_id: value.installation_id,
        completed: value.completed,
    }
}

fn user_install_roots() -> Result<backend::UserInstallRoots, IpcError> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| forbidden("HOME is unavailable"))?;
        backend::UserInstallRoots::from_home(PathBuf::from(home)).map_err(map_native_install_error)
    }
    #[cfg(target_os = "windows")]
    {
        let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| forbidden("LOCALAPPDATA is unavailable"))?;
        backend::UserInstallRoots::from_local_app_data(PathBuf::from(local)).map_err(map_native_install_error)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(forbidden("native installation is unsupported on this platform"))
    }
}

fn validate_native_install_dto(request: &NativeInstallRequest) -> Result<backend::NativeInstallRequest, IpcError> {
    require_confirmed(request.confirmed)?;
    let artifact = PathBuf::from(&request.artifact.path);
    let target_dir = PathBuf::from(&request.target_dir.path);
    if !artifact.is_absolute() || artifact.file_name().is_none() {
        return Err(invalid("artifact must be an absolute path with a file name"));
    }
    if !target_dir.is_absolute() {
        return Err(invalid("target_dir must be an absolute path"));
    }
    Ok(backend::NativeInstallRequest {
        artifact,
        target_dir,
        confirmed: true,
    })
}

impl BridgeState {
    pub fn initialize_native_records(&self, path: PathBuf) -> Result<(), IpcError> {
        let _store_lock = NativeStoreLock::acquire(&path, NATIVE_STORE_LOCK_TIMEOUT)?;
        let roots = user_install_roots()?;
        let records = reconcile_native_store(&path, roots.clone())?;
        let manager =
            backend::NativeManager::with_records_for_uninstall(roots, records).map_err(map_native_uninstall_error)?;
        *self
            .native
            .lock()
            .map_err(|_| internal("native state lock is poisoned"))? = Some(manager);
        *self
            .native_records_path
            .lock()
            .map_err(|_| internal("native records path lock is poisoned"))? = Some(path);
        Ok(())
    }

    fn native_records(&self, _window: &str) -> Result<Vec<backend::InstallationRecord>, IpcError> {
        let manager = self
            .native
            .lock()
            .map_err(|_| internal("native state lock is poisoned"))?;
        Ok(manager
            .as_ref()
            .map(backend::NativeManager::records)
            .unwrap_or_default())
    }

    fn preflight_native_install(
        &self,
        window: &str,
        request: backend::NativeInstallRequest,
    ) -> Result<PreparedNativeInstall, IpcError> {
        #[cfg(test)]
        self.notify_native_preflight_hook()?;
        let artifact_roots = self
            .picker
            .canonical_roots(window, PickerKind::Artifact)
            .map_err(map_native_install_preflight_error)?;
        let user_roots = user_install_roots()?;
        self.preflight_native_install_with_roots(window, artifact_roots.clone(), user_roots.clone(), &request)?;
        Ok(PreparedNativeInstall {
            artifact_roots,
            user_roots,
            request,
        })
    }

    fn preflight_native_install_with_roots(
        &self,
        window: &str,
        artifact_roots: Vec<PathBuf>,
        user_roots: backend::UserInstallRoots,
        request: &backend::NativeInstallRequest,
    ) -> Result<(), IpcError> {
        let manager = backend::NativeManager::with_records(artifact_roots, user_roots, self.native_records(window)?)
            .map_err(map_native_install_error)?;
        manager
            .preflight_install(request, &backend::CancellationToken::new())
            .map_err(map_native_install_error)?;
        Ok(())
    }

    #[cfg(test)]
    fn set_native_lock_hook(&self, hook: impl Fn(NativeLockEvent) + Send + Sync + 'static) {
        *self.native_lock_hook.lock().expect("native lock hook") = Some(Arc::new(hook));
    }

    #[cfg(test)]
    fn set_native_preflight_hook(&self, hook: impl Fn() + Send + Sync + 'static) {
        *self.native_preflight_hook.lock().expect("native preflight hook") = Some(Arc::new(hook));
    }

    #[cfg(test)]
    fn fail_after_native_persist(&self) {
        *self
            .fail_after_native_persist
            .lock()
            .expect("native persist failure hook") = true;
    }

    #[cfg(test)]
    fn notify_native_preflight_hook(&self) -> Result<(), IpcError> {
        let hook = self
            .native_preflight_hook
            .lock()
            .map_err(|_| internal("native preflight hook is poisoned"))?
            .clone();
        if let Some(hook) = hook {
            hook();
        }
        Ok(())
    }

    #[cfg(test)]
    fn notify_native_lock_hook(&self, event: NativeLockEvent) -> Result<(), IpcError> {
        let hook = self
            .native_lock_hook
            .lock()
            .map_err(|_| internal("native lock hook is poisoned"))?
            .clone();
        if let Some(hook) = hook {
            hook(event);
        }
        Ok(())
    }

    #[cfg(test)]
    fn notify_if_native_lock_is_contended(&self, operation: &Mutex<()>) -> Result<(), IpcError> {
        match operation.try_lock() {
            Ok(guard) => drop(guard),
            Err(std::sync::TryLockError::WouldBlock) => {
                self.notify_native_lock_hook(NativeLockEvent::Waiting)?;
            }
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(internal("native operation lock is poisoned"));
            }
        }
        Ok(())
    }

    #[cfg(test)]
    fn replace_native_manager(&self, _window: &str, manager: backend::NativeManager) -> Result<(), IpcError> {
        *self
            .native
            .lock()
            .map_err(|_| internal("native state lock is poisoned"))? = Some(manager);
        Ok(())
    }

    fn persist_native_records_at(&self, path: &Path, records: &[backend::InstallationRecord]) -> Result<(), IpcError> {
        atomic_write_native_records(path, records)?;
        #[cfg(test)]
        if *self
            .fail_after_native_persist
            .lock()
            .map_err(|_| internal("native persist failure hook is poisoned"))?
        {
            return Err(io_error(io::Error::other(
                "injected failure after native persistence commit",
            )));
        }
        Ok(())
    }

    fn native_store_path(&self) -> Result<PathBuf, IpcError> {
        self.native_records_path
            .lock()
            .map_err(|_| internal("native records path lock is poisoned"))?
            .clone()
            .ok_or_else(|| internal("native records store is not initialized"))
    }

    fn install_native(
        &self,
        _window: &str,
        artifact_roots: Vec<PathBuf>,
        user_roots: backend::UserInstallRoots,
        request: &backend::NativeInstallRequest,
        cancellation: &backend::CancellationToken,
        progress: impl FnMut(backend::NativeProgress),
    ) -> Result<backend::NativeOperationResult, IpcError> {
        let operation = Arc::clone(&self.native_operation);
        #[cfg(test)]
        self.notify_if_native_lock_is_contended(&operation)?;
        let _guard = operation
            .lock()
            .map_err(|_| internal("native operation lock is poisoned"))?;
        #[cfg(test)]
        self.notify_native_lock_hook(NativeLockEvent::Acquired)?;
        let path = self.native_store_path()?;
        let _store_lock = NativeStoreLock::acquire(&path, NATIVE_STORE_LOCK_TIMEOUT)?;
        let records = reconcile_native_store(&path, user_roots.clone())?;
        let mut manager = backend::NativeManager::with_records(artifact_roots, user_roots, records)
            .map_err(map_native_install_error)?;
        // Recheck under the per-window operation lock immediately before install
        // mutates the filesystem, closing the gap after the async preflight.
        manager
            .preflight_install(request, cancellation)
            .map_err(map_native_install_error)?;
        let mut records_to_commit = manager.records();
        let mut committed_record = None;
        let result = manager.install_with_commit(request, cancellation, progress, |record| {
            committed_record = Some(record.clone());
            records_to_commit.push(record.clone());
            self.persist_native_records_at(&path, &records_to_commit)
                .map_err(|error| io::Error::other(error.message))
        });
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                // A replace may have committed before a later durability error.
                // Reload and merge; never rewrite a possibly newer durable set.
                if let Ok(durable) = load_native_records(&path) {
                    manager.merge_records(durable).map_err(map_native_install_error)?;
                    if let Some(record) = committed_record
                        && !manager.records().contains(&record)
                    {
                        let _ = manager.abandon_pending_install(&record);
                    }
                }
                *self
                    .native
                    .lock()
                    .map_err(|_| internal("native state lock is poisoned"))? = Some(manager);
                return Err(map_native_install_error(error));
            }
        };
        *self
            .native
            .lock()
            .map_err(|_| internal("native state lock is poisoned"))? = Some(manager);
        Ok(result)
    }

    fn uninstall_native(
        &self,
        _window: &str,
        request: &backend::NativeUninstallRequest,
        cancellation: &backend::CancellationToken,
        progress: impl FnMut(backend::NativeProgress),
    ) -> Result<backend::NativeOperationResult, IpcError> {
        let operation = Arc::clone(&self.native_operation);
        #[cfg(test)]
        self.notify_if_native_lock_is_contended(&operation)?;
        let _guard = operation
            .lock()
            .map_err(|_| internal("native operation lock is poisoned"))?;
        #[cfg(test)]
        self.notify_native_lock_hook(NativeLockEvent::Acquired)?;
        let path = self.native_store_path()?;
        let _store_lock = NativeStoreLock::acquire(&path, NATIVE_STORE_LOCK_TIMEOUT)?;
        let roots = self
            .native
            .lock()
            .map_err(|_| plain_error(IpcErrorCode::Io, "native manager lock is poisoned"))?
            .as_ref()
            .map(backend::NativeManager::user_install_roots)
            .ok_or_else(|| not_found("installation is not recorded"))?;
        let records = reconcile_native_store(&path, roots.clone())?;
        let mut manager =
            backend::NativeManager::with_records_for_uninstall(roots, records).map_err(map_native_uninstall_error)?;
        let record = manager
            .installation_record(&request.installation_id)
            .ok_or_else(|| not_found("installation is not recorded"))?;
        atomic_write_pending_uninstall(&path, &record)?;
        let result = manager
            .uninstall(request, cancellation, progress)
            .map_err(map_native_uninstall_error)?;
        self.persist_native_records_at(&path, &manager.records())?;
        clear_pending_uninstall(&path)?;
        *self
            .native
            .lock()
            .map_err(|_| plain_error(IpcErrorCode::Io, "native manager lock is poisoned"))? = Some(manager);
        Ok(result)
    }

    fn existing_native_manager(&self, _window: &str) -> Result<(), IpcError> {
        if self
            .native
            .lock()
            .map_err(|_| internal("native state lock is poisoned"))?
            .as_ref()
            .is_some_and(|manager| !manager.records().is_empty())
        {
            Ok(())
        } else {
            Err(not_found("installation is not recorded"))
        }
    }

    pub fn clear_window(&self, window: &str) {
        let _ = self.picker.clear_window(window);
        if let Ok(mut jobs) = self.jobs.lock() {
            let generation = jobs
                .get(window)
                .map_or(1, |window_jobs| window_jobs.generation.saturating_add(1));
            if let Some(window_jobs) = jobs.remove(window) {
                for (_, cancellation) in window_jobs.active.into_values() {
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
    }
}

struct NativeStoreLock {
    file: fs::File,
}

impl NativeStoreLock {
    fn acquire(store_path: &Path, timeout: Duration) -> Result<Self, IpcError> {
        if !store_path.is_absolute() {
            return Err(io_error(io::Error::new(
                io::ErrorKind::InvalidInput,
                "native store path must be absolute",
            )));
        }
        let parent = store_path.parent().ok_or_else(|| {
            io_error(io::Error::new(
                io::ErrorKind::InvalidInput,
                "native store path has no app-data parent",
            ))
        })?;
        let lock_path = parent.join("native-installations.lock");
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(io_error)?;
        if !file.metadata().map_err(io_error)?.is_file() {
            return Err(io_error(io::Error::new(
                io::ErrorKind::InvalidData,
                "native store lock is not a regular app-data file",
            )));
        }
        let started = Instant::now();
        loop {
            match FileExt::try_lock_exclusive(&file) {
                Ok(()) => return Ok(Self { file }),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock && started.elapsed() < timeout => {
                    std::thread::sleep(NATIVE_STORE_LOCK_POLL.min(timeout.saturating_sub(started.elapsed())));
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    return Err(io_error(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "native store lock acquisition timed out",
                    )));
                }
                Err(error) => return Err(io_error(error)),
            }
        }
    }
}

impl Drop for NativeStoreLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

fn preferences_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    let directory = app.path().app_data_dir().map_err(|error| internal(error.to_string()))?;
    fs::create_dir_all(&directory).map_err(io_error)?;
    Ok(directory.join("preferences.json"))
}

pub fn native_records_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    let directory = app.path().app_data_dir().map_err(|error| internal(error.to_string()))?;
    fs::create_dir_all(&directory).map_err(io_error)?;
    Ok(directory.join("native-installations.json"))
}

fn load_native_records(path: &Path) -> Result<Vec<backend::InstallationRecord>, IpcError> {
    match fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|error| io_error(io::Error::new(io::ErrorKind::InvalidData, error)))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(io_error(error)),
    }
}

fn pending_uninstall_path(path: &Path) -> PathBuf {
    path.with_extension("pending-uninstall.json")
}

fn atomic_write_pending_uninstall(path: &Path, record: &backend::InstallationRecord) -> Result<(), IpcError> {
    atomic_write_native_value(&pending_uninstall_path(path), record)
}

fn clear_pending_uninstall(path: &Path) -> Result<(), IpcError> {
    let pending = pending_uninstall_path(path);
    match fs::remove_file(&pending) {
        Ok(()) => sync_parent(&pending).map_err(io_error),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

fn reconcile_native_store(
    path: &Path,
    roots: backend::UserInstallRoots,
) -> Result<Vec<backend::InstallationRecord>, IpcError> {
    let mut records = load_native_records(path)?;
    let pending_path = pending_uninstall_path(path);
    let pending = match fs::read(&pending_path) {
        Ok(bytes) => Some(
            serde_json::from_slice::<backend::InstallationRecord>(&bytes)
                .map_err(|error| io_error(io::Error::new(io::ErrorKind::InvalidData, error)))?,
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(io_error(error)),
    };
    if let Some(record) = pending {
        let retained =
            backend::NativeManager::reconcile_pending_uninstall(roots, &record, &backend::CancellationToken::new())
                .map_err(map_native_uninstall_error)?;
        records.retain(|candidate| candidate.installation_id != record.installation_id);
        if retained {
            records.push(record);
        }
        records.sort_by(|left, right| left.installation_id.cmp(&right.installation_id));
        atomic_write_native_records(path, &records)?;
        clear_pending_uninstall(path)?;
    }
    Ok(records)
}

#[cfg(test)]
fn merge_native_records(
    records: &mut Vec<backend::InstallationRecord>,
    durable: Vec<backend::InstallationRecord>,
) -> Result<(), IpcError> {
    for record in durable {
        match records
            .iter()
            .find(|candidate| candidate.installation_id == record.installation_id)
        {
            Some(existing) if existing == &record => {}
            Some(_) => return Err(plain_error(IpcErrorCode::Conflict, "conflicting durable native record")),
            None => records.push(record),
        }
    }
    records.sort_by(|left, right| left.installation_id.cmp(&right.installation_id));
    Ok(())
}

fn atomic_write_native_value<T: Serialize>(path: &Path, value: &T) -> Result<(), IpcError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| io_error(io::Error::other(error)))?;
    let temporary = unique_sibling(path, "native.tmp");
    write_and_sync(&temporary, &bytes)?;
    if let Err(error) = atomic_replace(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(error));
    }
    sync_parent(path).map_err(io_error)
}

fn atomic_write_native_records(path: &Path, records: &[backend::InstallationRecord]) -> Result<(), IpcError> {
    atomic_write_native_value(path, &records)
}

fn atomic_write_json(path: &Path, value: &Preferences) -> Result<(), IpcError> {
    let _guard = PREFERENCES_WRITE_LOCK
        .lock()
        .map_err(|_| internal("preferences write lock is poisoned"))?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| validation(error.to_string()))?;
    let backup = path.with_extension("json.bak");
    let temporary = unique_sibling(path, "tmp");
    write_and_sync(&temporary, &bytes)?;
    if path.exists() {
        let backup_temporary = unique_sibling(path, "bak.tmp");
        let backup_result = (|| {
            fs::copy(path, &backup_temporary).map_err(io_error)?;
            fs::File::open(&backup_temporary)
                .and_then(|file| file.sync_all())
                .map_err(io_error)?;
            atomic_replace(&backup_temporary, &backup).map_err(io_error)
        })();
        if let Err(error) = backup_result {
            let _ = fs::remove_file(&temporary);
            let _ = fs::remove_file(&backup_temporary);
            return Err(error);
        }
    }
    if let Err(error) = atomic_replace(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(error));
    }
    sync_parent(path).map_err(io_error)
}

fn unique_sibling(path: &Path, suffix: &str) -> PathBuf {
    let id = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.{suffix}-{}-{id}", std::process::id()))
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> Result<(), IpcError> {
    let mut options = fs::OpenOptions::new();
    let mut file = options.write(true).create_new(true).open(path).map_err(io_error)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(io_error(error));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(target_os = "windows")]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW, REPLACEFILE_WRITE_THROUGH, ReplaceFileW,
    };

    let destination_exists = destination.exists();
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    // Windows fs::rename cannot replace an existing destination. These APIs perform the
    // same-volume replacement atomically; all pointers reference live, NUL-terminated buffers.
    let succeeded = unsafe {
        if destination_exists {
            ReplaceFileW(
                destination.as_ptr(),
                source.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null(),
                std::ptr::null(),
            )
        } else {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
    };
    if succeeded == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> io::Result<()> {
    fs::File::open(
        path.parent()
            .ok_or_else(|| io::Error::other("preferences path has no parent"))?,
    )?
    .sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[derive(Deserialize)]
enum LegacyLanguage {
    English,
    Vietnamese,
}

#[derive(Deserialize)]
enum LegacyTheme {
    System,
    Light,
    Dark,
}

#[derive(Deserialize)]
struct LegacyPreferences {
    language: LegacyLanguage,
    theme: LegacyTheme,
    font: String,
}

fn load_or_migrate_preferences(path: &Path) -> Result<Preferences, IpcError> {
    match fs::read(path) {
        Ok(bytes) => {
            let parsed = serde_json::from_slice::<Preferences>(&bytes).ok();
            let normalized = parsed.clone().map(normalize_preferences).unwrap_or_default();
            if parsed.as_ref() != Some(&normalized) {
                atomic_write_json(path, &normalized)?;
            }
            Ok(normalized)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let preferences = import_eframe_preferences().unwrap_or_default();
            let preferences = normalize_preferences(preferences);
            atomic_write_json(path, &preferences)?;
            Ok(preferences)
        }
        Err(error) => Err(io_error(error)),
    }
}

fn import_eframe_preferences() -> Option<Preferences> {
    let path = legacy_eframe_storage_dir("universal_converter_gui")?.join("app.ron");
    let content = fs::read_to_string(path).ok()?;
    let values: HashMap<String, String> = ron::from_str(&content).ok()?;
    let legacy: LegacyPreferences = ron::from_str(values.get("preferences")?).ok()?;
    Some(Preferences {
        language: match legacy.language {
            LegacyLanguage::English => "en",
            LegacyLanguage::Vietnamese => "vi",
        }
        .to_owned(),
        theme: match legacy.theme {
            LegacyTheme::System => "system",
            LegacyTheme::Light => "light",
            LegacyTheme::Dark => "dark",
        }
        .to_owned(),
        font_id: if legacy.font == "Default" {
            "system-default".to_owned()
        } else if legacy.font == "DejaVu Sans" {
            "dejavusans".to_owned()
        } else {
            legacy.font
        },
    })
}

fn legacy_eframe_storage_dir(app_id: &str) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
        Some(
            base.join(
                app_id
                    .to_ascii_lowercase()
                    .replace(|character: char| character.is_ascii_whitespace(), ""),
            ),
        )
    }
    #[cfg(target_os = "macos")]
    {
        let home = PathBuf::from(std::env::var_os("HOME")?);
        Some(
            home.join("Library/Application Support")
                .join(app_id.replace(|character: char| character.is_ascii_whitespace(), "-")),
        )
    }
    #[cfg(target_os = "windows")]
    {
        Some(PathBuf::from(std::env::var_os("APPDATA")?).join(app_id).join("data"))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = app_id;
        None
    }
}

fn normalize_preferences(mut value: Preferences) -> Preferences {
    let defaults = Preferences::default();
    if !matches!(value.language.as_str(), "vi" | "en") {
        value.language = defaults.language;
    }
    if !matches!(value.theme.as_str(), "system" | "light" | "dark") {
        value.theme = defaults.theme;
    }
    if !matches!(value.font_id.as_str(), "system-default" | "dejavusans") {
        value.font_id = defaults.font_id;
    }
    value
}

fn validate_preferences(value: &Preferences) -> Result<(), IpcError> {
    if !matches!(value.language.as_str(), "vi" | "en") {
        return Err(validation("language must be vi or en"));
    }
    if !matches!(value.theme.as_str(), "system" | "light" | "dark") {
        return Err(validation("theme must be system, light, or dark"));
    }
    if !matches!(value.font_id.as_str(), "system-default" | "dejavusans") {
        return Err(validation("font_id must be system-default or dejavusans"));
    }
    Ok(())
}

fn require_confirmed(confirmed: bool) -> Result<(), IpcError> {
    if confirmed {
        Ok(())
    } else {
        Err(forbidden("native operation requires confirmed:true"))
    }
}

fn require_native_platform() -> Result<(), IpcError> {
    if cfg!(target_os = "linux") {
        Ok(())
    } else {
        Err(plain_error(
            IpcErrorCode::Unavailable,
            "native install and uninstall are available only on Linux",
        ))
    }
}

fn map_dependency_error(error: io::Error) -> IpcError {
    typed_error(IpcErrorCode::Unavailable, error)
}

fn map_picker_argument_error(error: IpcError) -> IpcError {
    match error.code {
        IpcErrorCode::NotFound => error,
        IpcErrorCode::Io => error,
        _ => IpcError {
            code: IpcErrorCode::InvalidArgument,
            ..error
        },
    }
}

fn map_native_install_preflight_error(error: IpcError) -> IpcError {
    match error.code {
        IpcErrorCode::InvalidArgument
        | IpcErrorCode::Forbidden
        | IpcErrorCode::Conflict
        | IpcErrorCode::Io
        | IpcErrorCode::Cancelled => error,
        IpcErrorCode::NotFound => IpcError {
            code: IpcErrorCode::InvalidArgument,
            ..error
        },
        _ => IpcError {
            code: IpcErrorCode::Io,
            ..error
        },
    }
}

fn map_native_uninstall_preflight_error(error: IpcError) -> IpcError {
    match error.code {
        IpcErrorCode::InvalidArgument
        | IpcErrorCode::Forbidden
        | IpcErrorCode::NotFound
        | IpcErrorCode::Io
        | IpcErrorCode::Cancelled => error,
        _ => IpcError {
            code: IpcErrorCode::Io,
            ..error
        },
    }
}

fn remap_error(mut error: IpcError, code: IpcErrorCode) -> IpcError {
    error.code = code;
    error
}

fn map_classify_error(error: io::Error) -> IpcError {
    match error.kind() {
        io::ErrorKind::InvalidInput => typed_error(IpcErrorCode::InvalidArgument, error),
        io::ErrorKind::NotFound => typed_error(IpcErrorCode::NotFound, error),
        _ => typed_error(IpcErrorCode::Io, error),
    }
}

fn map_scan_error(error: io::Error) -> IpcError {
    match error.kind() {
        io::ErrorKind::InvalidInput => typed_error(IpcErrorCode::InvalidArgument, error),
        io::ErrorKind::NotFound => typed_error(IpcErrorCode::NotFound, error),
        io::ErrorKind::Interrupted => typed_error(IpcErrorCode::Cancelled, error),
        _ => typed_error(IpcErrorCode::Io, error),
    }
}

fn map_batch_error(error: io::Error) -> IpcError {
    match error.kind() {
        io::ErrorKind::InvalidInput | io::ErrorKind::PermissionDenied => {
            typed_error(IpcErrorCode::InvalidArgument, error)
        }
        io::ErrorKind::AlreadyExists => typed_error(IpcErrorCode::Conflict, error),
        io::ErrorKind::Interrupted => typed_error(IpcErrorCode::Cancelled, error),
        _ => typed_error(IpcErrorCode::Io, error),
    }
}

fn map_native_install_error(error: io::Error) -> IpcError {
    match error.kind() {
        io::ErrorKind::InvalidInput => typed_error(IpcErrorCode::InvalidArgument, error),
        io::ErrorKind::PermissionDenied => typed_error(IpcErrorCode::Forbidden, error),
        io::ErrorKind::AlreadyExists => typed_error(IpcErrorCode::Conflict, error),
        io::ErrorKind::Interrupted => typed_error(IpcErrorCode::Cancelled, error),
        io::ErrorKind::Unsupported => typed_error(IpcErrorCode::Unavailable, error),
        _ => typed_error(IpcErrorCode::Io, error),
    }
}

fn map_native_uninstall_error(error: io::Error) -> IpcError {
    match error.kind() {
        io::ErrorKind::InvalidInput => typed_error(IpcErrorCode::InvalidArgument, error),
        io::ErrorKind::PermissionDenied => typed_error(IpcErrorCode::Forbidden, error),
        io::ErrorKind::NotFound => typed_error(IpcErrorCode::NotFound, error),
        io::ErrorKind::Interrupted => typed_error(IpcErrorCode::Cancelled, error),
        io::ErrorKind::Unsupported => typed_error(IpcErrorCode::Unavailable, error),
        _ => typed_error(IpcErrorCode::Io, error),
    }
}

fn typed_error(code: IpcErrorCode, error: io::Error) -> IpcError {
    IpcError {
        code,
        message: error.to_string(),
        retryable: matches!(error.kind(), io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock),
        details: None,
    }
}

fn invalid(message: impl Into<String>) -> IpcError {
    plain_error(IpcErrorCode::InvalidArgument, message)
}

fn validation(message: impl Into<String>) -> IpcError {
    plain_error(IpcErrorCode::Validation, message)
}

fn not_found(message: impl Into<String>) -> IpcError {
    plain_error(IpcErrorCode::NotFound, message)
}

fn internal(message: impl Into<String>) -> IpcError {
    plain_error(IpcErrorCode::Internal, message)
}

fn io_error(error: io::Error) -> IpcError {
    typed_error(IpcErrorCode::Io, error)
}

fn plain_error(code: IpcErrorCode, message: impl Into<String>) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable: false,
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_request_id_contract() {
        let job = Req {
            schema_version: 1,
            request_id: Some("frontend-id".to_owned()),
            payload: Empty {},
        };
        assert_eq!(validate_request(&job, true), Ok(Some("frontend-id".to_owned())));
        assert_eq!(
            validate_request(&job, false).map_err(|error| error.code),
            Err(IpcErrorCode::InvalidArgument)
        );
    }

    #[test]
    fn exact_invoke_registry_topics_and_error_sets_match_a6_a7() {
        assert_eq!(
            INVOKE_REGISTRY,
            [
                CommandContract::job(
                    "dependencies_check",
                    "job.dependencies_check",
                    &[IpcErrorCode::Unavailable, IpcErrorCode::Internal],
                ),
                CommandContract::job(
                    "classify_file",
                    "job.classify_file",
                    &[IpcErrorCode::InvalidArgument, IpcErrorCode::NotFound, IpcErrorCode::Io],
                ),
                CommandContract::job(
                    "scan_directory",
                    "job.scan_directory",
                    &[
                        IpcErrorCode::InvalidArgument,
                        IpcErrorCode::NotFound,
                        IpcErrorCode::Io,
                        IpcErrorCode::Cancelled,
                    ],
                ),
                CommandContract::job(
                    "batch_convert",
                    "job.batch_convert",
                    &[
                        IpcErrorCode::InvalidArgument,
                        IpcErrorCode::Conflict,
                        IpcErrorCode::Io,
                        IpcErrorCode::Cancelled,
                    ],
                ),
                CommandContract::job(
                    "native_install",
                    "job.native_install",
                    &[
                        IpcErrorCode::InvalidArgument,
                        IpcErrorCode::Forbidden,
                        IpcErrorCode::Unavailable,
                        IpcErrorCode::Conflict,
                        IpcErrorCode::Io,
                        IpcErrorCode::Cancelled,
                    ],
                ),
                CommandContract::job(
                    "native_uninstall",
                    "job.native_uninstall",
                    &[
                        IpcErrorCode::InvalidArgument,
                        IpcErrorCode::Forbidden,
                        IpcErrorCode::Unavailable,
                        IpcErrorCode::NotFound,
                        IpcErrorCode::Io,
                        IpcErrorCode::Cancelled,
                    ],
                ),
                CommandContract::command("preferences_get", &[IpcErrorCode::Io, IpcErrorCode::Internal]),
                CommandContract::command("preferences_set", &[IpcErrorCode::Validation, IpcErrorCode::Io]),
                CommandContract::command(
                    "picker_select",
                    &[
                        IpcErrorCode::InvalidArgument,
                        IpcErrorCode::NotFound,
                        IpcErrorCode::Io,
                        IpcErrorCode::Internal,
                    ],
                ),
            ]
        );
    }

    #[test]
    fn request_ids_are_unique_without_inventing_conflict_for_commands_that_disallow_it() {
        let state = BridgeState::default();
        let active = state
            .register_job(
                "main",
                "request-1".to_owned(),
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Io,
            )
            .expect("first use");
        drop(active);
        assert_eq!(
            state
                .register_job(
                    "main",
                    "request-1".to_owned(),
                    IpcErrorCode::InvalidArgument,
                    IpcErrorCode::Io,
                )
                .err()
                .expect("reuse rejected")
                .code,
            IpcErrorCode::InvalidArgument
        );
        assert!(
            state
                .register_job(
                    "main",
                    "uuid-style-id".to_owned(),
                    IpcErrorCode::InvalidArgument,
                    IpcErrorCode::Io,
                )
                .is_ok()
        );
        assert!(
            state
                .register_job(
                    "other",
                    "request-1".to_owned(),
                    IpcErrorCode::InvalidArgument,
                    IpcErrorCode::Io,
                )
                .is_ok()
        );
    }

    #[test]
    fn failed_preflight_does_not_register_or_leak_a_job() {
        let state = BridgeState::default();
        let request = Req {
            schema_version: 1,
            request_id: Some("request-1".to_owned()),
            payload: Empty {},
        };
        assert_eq!(
            state
                .prepare_job(
                    "main",
                    &request,
                    IpcErrorCode::InvalidArgument,
                    IpcErrorCode::InvalidArgument,
                    IpcErrorCode::Io,
                    || Err::<(), _>(invalid("preflight failed")),
                )
                .err()
                .expect("preflight must fail")
                .code,
            IpcErrorCode::InvalidArgument
        );
        let active = state
            .register_job(
                "main",
                "request-1".to_owned(),
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Io,
            )
            .expect("request ID remains unused after preflight failure");
        drop(active);
        let jobs = state.jobs.lock().expect("jobs lock");
        assert!(jobs["main"].active.is_empty());
    }

    #[test]
    fn batch_style_failed_preflight_keeps_request_id_reusable() {
        let state = BridgeState::default();
        let request = Req {
            schema_version: 1,
            request_id: Some("batch-retry".to_owned()),
            payload: Empty {},
        };
        let failed = state.prepare_job(
            "main",
            &request,
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::Conflict,
            IpcErrorCode::Io,
            || Err::<(), _>(invalid("invalid batch path or format")),
        );
        assert_eq!(
            failed.err().expect("preflight fails").code,
            IpcErrorCode::InvalidArgument
        );

        let (active, ()) = state
            .prepare_job(
                "main",
                &request,
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Conflict,
                IpcErrorCode::Io,
                || Ok(()),
            )
            .expect("same request ID remains reusable after failed preflight");
        drop(active);
    }

    #[test]
    fn window_teardown_cancels_all_active_jobs() {
        let state = BridgeState::default();
        let active = state
            .register_job(
                "main",
                "request-1".to_owned(),
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Io,
            )
            .expect("register");
        state.clear_window("main");
        assert!(active.cancellation.is_cancelled());
    }

    #[test]
    fn stale_active_job_drop_cannot_remove_reused_label_and_request_id() {
        let state = BridgeState::default();
        let stale = state
            .register_job(
                "main",
                "request-1".to_owned(),
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Io,
            )
            .expect("first lifecycle");
        state.clear_window("main");
        assert!(stale.cancellation.is_cancelled());

        let current = state
            .register_job(
                "main",
                "request-1".to_owned(),
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Io,
            )
            .expect("request ID is reusable in the new lifecycle");
        drop(stale);

        let jobs = state.jobs.lock().expect("jobs lock");
        let active = &jobs["main"].active;
        assert!(active.contains_key("request-1"));
        assert!(!current.cancellation.is_cancelled());
    }

    #[test]
    fn generation_change_during_preflight_does_not_register_job() {
        let state = BridgeState::default();
        let request = Req {
            schema_version: 1,
            request_id: Some("generation-race".to_owned()),
            payload: Empty {},
        };
        let state_during_preflight = state.clone();
        let error = state
            .prepare_job(
                "main",
                &request,
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Conflict,
                IpcErrorCode::Io,
                || {
                    state_during_preflight.clear_window("main");
                    Ok(())
                },
            )
            .err()
            .expect("changed lifecycle must reject registration");
        assert_eq!(error.code, IpcErrorCode::Io);
        let jobs = state.jobs.lock().expect("jobs lock");
        assert!(jobs["main"].used.is_empty());
        assert!(jobs["main"].active.is_empty());
    }

    #[test]
    fn app_data_native_lock_is_bounded_and_interprocess_visible() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-native-lock-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root).expect("app data");
        let store = root.join("native-installations.json");
        let held = NativeStoreLock::acquire(&store, Duration::from_secs(1)).expect("first lock");
        let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["--ignored", "--exact", "commands::tests::native_lock_child_probe"])
            .env("UNIVERSAL_NATIVE_LOCK_PROBE", &store)
            .status()
            .expect("child lock probe");
        assert!(status.success(), "separate process must observe the held lock");
        let started = Instant::now();
        let error = NativeStoreLock::acquire(&store, Duration::from_millis(25))
            .err()
            .expect("bounded contention");
        assert_eq!(error.code, IpcErrorCode::Io);
        assert!(error.retryable);
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(held);
        NativeStoreLock::acquire(&store, Duration::from_secs(1)).expect("released lock");
    }

    #[test]
    #[ignore = "spawned by app_data_native_lock_is_bounded_and_interprocess_visible"]
    fn native_lock_child_probe() {
        let store = PathBuf::from(std::env::var_os("UNIVERSAL_NATIVE_LOCK_PROBE").expect("probe store path"));
        let error = NativeStoreLock::acquire(&store, Duration::from_millis(50))
            .err()
            .expect("parent process holds lock");
        assert_eq!(error.code, IpcErrorCode::Io);
        assert!(error.retryable);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn pending_uninstall_reconciles_failure_crash_and_reopen() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "universal-native-pending-uninstall-{}-{nonce}",
            std::process::id()
        ));
        let home = root.join("home");
        let artifacts = root.join("artifacts");
        let records_path = root.join("app-data/native-installations.json");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&artifacts).expect("artifacts");
        fs::create_dir_all(records_path.parent().expect("app data")).expect("app data");
        fs::write(artifacts.join("converter"), b"fixture").expect("artifact");
        let roots = backend::UserInstallRoots::from_home(&home).expect("roots");
        let mut manager = backend::NativeManager::new(vec![artifacts.clone()], roots.clone()).expect("manager");
        let installed = manager
            .install(
                &backend::NativeInstallRequest {
                    artifact: artifacts.join("converter"),
                    target_dir: home.join(".local/bin"),
                    confirmed: true,
                },
                &backend::CancellationToken::new(),
                |_| {},
            )
            .expect("install");
        let record = manager.records()[0].clone();
        atomic_write_native_records(&records_path, std::slice::from_ref(&record)).expect("records");

        // Failure before mutation: reopening retains the record and clears the journal.
        atomic_write_pending_uninstall(&records_path, &record).expect("journal");
        let retained = reconcile_native_store(&records_path, roots.clone()).expect("failure reopen");
        assert_eq!(retained, vec![record.clone()]);
        assert!(!pending_uninstall_path(&records_path).exists());

        // Crash after filesystem commit but before record persistence: reopening
        // removes the stale durable record, then atomically clears the journal.
        atomic_write_pending_uninstall(&records_path, &record).expect("journal before mutation");
        manager
            .uninstall(
                &backend::NativeUninstallRequest {
                    installation_id: installed.installation_id,
                    confirmed: true,
                },
                &backend::CancellationToken::new(),
                |_| {},
            )
            .expect("filesystem mutation");
        assert_eq!(
            load_native_records(&records_path).expect("stale durable record"),
            vec![record]
        );
        assert!(
            reconcile_native_store(&records_path, roots)
                .expect("crash reopen")
                .is_empty()
        );
        assert!(
            load_native_records(&records_path)
                .expect("reconciled records")
                .is_empty()
        );
        assert!(!pending_uninstall_path(&records_path).exists());
    }

    #[test]
    fn durable_native_records_are_merged_without_overwrite_after_commit_error() {
        let first: backend::InstallationRecord = serde_json::from_value(serde_json::json!({
            "installation_id": "native-first",
            "installed_path": "/tmp/first",
            "operation_token": "00000000000000000000000000000000"
        }))
        .expect("first record");
        let second: backend::InstallationRecord = serde_json::from_value(serde_json::json!({
            "installation_id": "native-second",
            "installed_path": "/tmp/second",
            "operation_token": "11111111111111111111111111111111"
        }))
        .expect("second record");
        let mut memory = vec![first.clone()];
        merge_native_records(&mut memory, vec![first, second.clone()]).expect("merge durable pending state");
        assert_eq!(memory.len(), 2);
        assert!(memory.contains(&second));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn post_commit_persist_failure_reloads_durable_pending_record() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-native-post-commit-{}-{nonce}", std::process::id()));
        let home = root.join("home");
        let artifacts = root.join("artifacts");
        let records_path = root.join("app-data/native-installations.json");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&artifacts).expect("artifacts");
        fs::create_dir_all(records_path.parent().expect("app data")).expect("app data");
        fs::write(artifacts.join("converter"), b"fixture").expect("artifact");
        let roots = backend::UserInstallRoots::from_home(&home).expect("roots");
        let state = BridgeState::default();
        *state.native_records_path.lock().expect("records path") = Some(records_path.clone());
        state.fail_after_native_persist();

        let error = state
            .install_native(
                "main",
                vec![artifacts.clone()],
                roots.clone(),
                &backend::NativeInstallRequest {
                    artifact: artifacts.join("converter"),
                    target_dir: home.join(".local/bin"),
                    confirmed: true,
                },
                &backend::CancellationToken::new(),
                |_| {},
            )
            .expect_err("injected post-commit report must fail the command");
        assert_eq!(error.code, IpcErrorCode::Io);
        let durable = load_native_records(&records_path).expect("committed durable record");
        assert_eq!(durable.len(), 1);
        assert_eq!(state.native_records("main").expect("merged state"), durable);

        let reopened =
            backend::NativeManager::with_records_for_uninstall(roots, durable).expect("reopen pending publish");
        assert_eq!(reopened.records().len(), 1);
        assert!(home.join(".local/bin/converter").is_file());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn window_teardown_keeps_persisted_native_uninstall_capability() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-native-reopen-{}-{nonce}", std::process::id()));
        let home = root.join("home");
        let artifacts = root.join("artifacts");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&artifacts).expect("artifacts");
        fs::write(artifacts.join("converter"), b"fixture").expect("artifact");
        let path = root.join("app-data/native-installations.json");
        fs::create_dir_all(path.parent().expect("app data")).expect("app data");
        let roots = backend::UserInstallRoots::from_home(&home).expect("roots");
        let state = BridgeState::default();
        *state.native_records_path.lock().expect("record path") = Some(path.clone());
        state
            .install_native(
                "main",
                vec![artifacts.clone()],
                roots,
                &backend::NativeInstallRequest {
                    artifact: artifacts.join("converter"),
                    target_dir: home.join(".local/bin"),
                    confirmed: true,
                },
                &backend::CancellationToken::new(),
                |_| {},
            )
            .expect("install");
        state.clear_window("main");
        assert_eq!(state.native_records("reopened").expect("records").len(), 1);
        assert_eq!(load_native_records(&path).expect("persisted records").len(), 1);
    }

    #[test]
    fn corrupt_and_unknown_preferences_normalize_and_are_persisted() {
        let root = std::env::temp_dir().join(format!("universal-bridge-prefs-{}", std::process::id()));
        fs::create_dir_all(&root).expect("test root");
        let path = root.join("preferences.json");
        fs::write(&path, b"not-json").expect("corrupt fixture");
        assert_eq!(
            load_or_migrate_preferences(&path).expect("normalize"),
            Preferences::default()
        );
        assert_eq!(
            serde_json::from_slice::<Preferences>(&fs::read(&path).expect("canonical file")).expect("valid json"),
            Preferences::default()
        );
        assert!(path.with_extension("json.bak").is_file());

        let unknown = Preferences {
            language: "unknown".to_owned(),
            theme: "broken".to_owned(),
            font_id: "host-font".to_owned(),
        };
        fs::write(&path, serde_json::to_vec(&unknown).expect("serialize")).expect("unknown fixture");
        assert_eq!(
            load_or_migrate_preferences(&path).expect("normalize ids"),
            Preferences::default()
        );
    }

    #[test]
    fn preferences_survive_reload_and_legacy_migration_is_one_time() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-bridge-prefs-restart-{}-{nonce}", std::process::id()));
        let legacy_root = root.join("legacy-data");
        let legacy_directory = legacy_root.join("universal_converter_gui");
        fs::create_dir_all(&legacy_directory).expect("legacy storage directory");
        fs::write(
            legacy_directory.join("app.ron"),
            r#"{"preferences":"(language:English,theme:Dark,font:\"DejaVu Sans\")"}"#,
        )
        .expect("legacy eframe preferences");
        let canonical_path = root.join("app-data/preferences.json");
        fs::create_dir_all(canonical_path.parent().expect("app data directory")).expect("app data directory");
        let previous_xdg = std::env::var_os("XDG_DATA_HOME");

        // SAFETY: this test restores the process variable before asserting and no other test
        // exercises the missing-canonical-file legacy import path.
        unsafe { std::env::set_var("XDG_DATA_HOME", &legacy_root) };
        let migrated = load_or_migrate_preferences(&canonical_path).expect("legacy migration");
        fs::write(
            legacy_directory.join("app.ron"),
            r#"{"preferences":"(language:Vietnamese,theme:Light,font:\"Default\")"}"#,
        )
        .expect("changed legacy preferences");
        let restarted = load_or_migrate_preferences(&canonical_path).expect("restart reload");
        match previous_xdg {
            Some(value) => unsafe { std::env::set_var("XDG_DATA_HOME", value) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }

        let expected = Preferences {
            language: "en".to_owned(),
            theme: "dark".to_owned(),
            font_id: "dejavusans".to_owned(),
        };
        assert_eq!(migrated, expected);
        assert_eq!(
            restarted, expected,
            "existing canonical preferences must win after restart"
        );
        assert_eq!(
            serde_json::from_slice::<Preferences>(&fs::read(canonical_path).expect("canonical preferences"))
                .expect("canonical JSON"),
            expected
        );
    }

    #[test]
    fn only_registry_file_types_are_accepted() {
        assert_eq!(
            parse_file_types(&["video".to_owned()]),
            Ok(vec![backend::FileType::Video])
        );
        assert_eq!(
            parse_file_types(&["executable".to_owned()]).map_err(|error| error.code),
            Err(IpcErrorCode::InvalidArgument)
        );
    }

    #[test]
    fn native_confirmation_maps_to_forbidden() {
        assert_eq!(
            require_confirmed(false).map_err(|error| error.code),
            Err(IpcErrorCode::Forbidden)
        );
    }

    #[test]
    fn native_install_dto_is_validated_before_job_registration() {
        let state = BridgeState::default();
        let request = Req {
            schema_version: 1,
            request_id: Some("native-invalid".to_owned()),
            payload: NativeInstallRequest {
                artifact: PathRef {
                    path: "relative-artifact".to_owned(),
                },
                target_dir: PathRef {
                    path: "/absolute-target".to_owned(),
                },
                confirmed: true,
            },
        };

        let error = state
            .prepare_job(
                "main",
                &request,
                IpcErrorCode::InvalidArgument,
                IpcErrorCode::Conflict,
                IpcErrorCode::Io,
                || validate_native_install_dto(&request.payload),
            )
            .err()
            .expect("invalid DTO must fail before registration");
        assert_eq!(error.code, IpcErrorCode::InvalidArgument);
        let jobs = state.jobs.lock().expect("jobs lock");
        assert!(jobs["main"].used.is_empty());
        assert!(jobs["main"].active.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_install_full_preflight_runs_before_registration_and_failure_is_reusable() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-native-preflight-{}-{nonce}", std::process::id()));
        let home = root.join("home");
        let artifacts = root.join("artifacts");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&artifacts).expect("artifacts");
        let artifact = artifacts.join("converter");
        fs::write(&artifact, b"fixture").expect("artifact");

        let state = BridgeState::default();
        state
            .picker
            .replace_paths("main", PickerKind::Artifact, std::slice::from_ref(&artifact))
            .expect("artifact provenance");
        let request = Req {
            schema_version: 1,
            request_id: Some("native-preflight-retry".to_owned()),
            payload: NativeInstallRequest {
                artifact: PathRef {
                    path: artifact.to_string_lossy().into_owned(),
                },
                target_dir: PathRef {
                    path: home.join("outside-allowed-roots").to_string_lossy().into_owned(),
                },
                confirmed: true,
            },
        };

        let previous_home = std::env::var_os("HOME");
        // SAFETY: this test restores HOME before asserting and is the only bridge
        // test that invokes the platform-root preflight helper directly.
        unsafe { std::env::set_var("HOME", &home) };
        let result = tauri::async_runtime::block_on(state.prepare_native_install("main", &request));
        match previous_home {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }

        assert!(result.is_err(), "filesystem preflight must fail");
        let active = state
            .register_job(
                "main",
                "native-preflight-retry".to_owned(),
                IpcErrorCode::Conflict,
                IpcErrorCode::Io,
            )
            .expect("request ID remains reusable after full preflight failure");
        drop(active);
    }

    #[test]
    fn native_install_preflight_worker_keeps_async_executor_responsive() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let release_rx = Arc::new(Mutex::new(release_rx));
        let state = BridgeState::default();
        state.set_native_preflight_hook(move || {
            started_tx.send(()).expect("native preflight started");
            release_rx
                .lock()
                .expect("release receiver")
                .recv()
                .expect("release native preflight");
        });
        let request = Req {
            schema_version: 1,
            request_id: Some("native-nonblocking".to_owned()),
            payload: NativeInstallRequest {
                artifact: PathRef {
                    path: "/missing-native-preflight-artifact".to_owned(),
                },
                target_dir: PathRef {
                    path: "/missing-native-preflight-target".to_owned(),
                },
                confirmed: true,
            },
        };
        let runtime = tauri::async_runtime::handle();
        let worker_state = state.clone();
        let worker = runtime.spawn(async move { worker_state.prepare_native_install("main", &request).await });
        started_rx.recv().expect("blocking native preflight started");

        let (tick_tx, tick_rx) = mpsc::channel();
        runtime.spawn(async move {
            tick_tx.send(()).expect("async executor tick");
        });
        tick_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("async executor must remain responsive during filesystem preflight");
        release_tx.send(()).expect("release blocking native preflight");
        let result = tauri::async_runtime::block_on(worker).expect("worker completion");
        assert!(result.is_err(), "fixture must fail after the worker is released");
        let active = state
            .register_job(
                "main",
                "native-nonblocking".to_owned(),
                IpcErrorCode::Conflict,
                IpcErrorCode::Io,
            )
            .expect("failed async preflight must leave request ID reusable");
        drop(active);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_install_rechecks_preflight_under_operation_lock() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-native-recheck-{}-{nonce}", std::process::id()));
        let home = root.join("home");
        let artifacts = root.join("artifacts");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&artifacts).expect("artifacts");
        let artifact = artifacts.join("converter");
        fs::write(&artifact, b"fixture").expect("artifact");
        let user_roots = backend::UserInstallRoots::from_home(&home).expect("user roots");
        let request = backend::NativeInstallRequest {
            artifact: artifact.clone(),
            target_dir: home.join(".local/bin"),
            confirmed: true,
        };
        let state = BridgeState::default();
        *state.native_records_path.lock().expect("record path") = Some(root.join("native-installations.json"));
        state
            .preflight_native_install_with_roots("main", vec![artifacts.clone()], user_roots.clone(), &request)
            .expect("initial preflight");
        fs::write(home.join(".local/bin/converter"), b"racing target").expect("TOCTOU fixture");

        let error = state
            .install_native(
                "main",
                vec![artifacts],
                user_roots,
                &request,
                &backend::CancellationToken::new(),
                |_| {},
            )
            .expect_err("locked recheck must reject changed destination");
        assert_eq!(error.code, IpcErrorCode::Conflict);
        assert_eq!(
            fs::read(home.join(".local/bin/converter")).expect("fixture remains"),
            b"racing target"
        );
        assert!(state.native_records("main").expect("records").is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn concurrent_install_then_uninstall_does_not_restore_stale_record() {
        use std::sync::Barrier;
        use std::sync::mpsc;

        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-native-concurrency-{}-{nonce}", std::process::id()));
        let home = root.join("home");
        let artifacts = root.join("artifacts");
        let bundle = artifacts.join("bundle");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&bundle).expect("bundle");
        fs::write(artifacts.join("converter"), b"prior").expect("prior artifact");
        fs::write(bundle.join("first"), b"first").expect("first bundle file");
        fs::write(bundle.join("second"), b"second").expect("second bundle file");

        let user_roots = backend::UserInstallRoots::from_home(&home).expect("user roots");
        let artifact_roots = vec![artifacts.clone()];
        let mut initial =
            backend::NativeManager::new(artifact_roots.clone(), user_roots.clone()).expect("initial manager");
        let prior = initial
            .install(
                &backend::NativeInstallRequest {
                    artifact: artifacts.join("converter"),
                    target_dir: home.join(".local/bin"),
                    confirmed: true,
                },
                &backend::CancellationToken::new(),
                |_| {},
            )
            .expect("prior install");
        let state = BridgeState::default();
        *state.native_records_path.lock().expect("record path") = Some(root.join("native-installations.json"));
        atomic_write_native_records(&state.native_store_path().expect("store path"), &initial.records())
            .expect("seed durable records");
        state
            .replace_native_manager("main", initial)
            .expect("seed native manager");

        let (copy_paused_tx, copy_paused_rx) = mpsc::channel();
        let (resume_tx, resume_rx) = mpsc::channel();
        let install_state = state.clone();
        let install = std::thread::spawn(move || {
            install_state.install_native(
                "main",
                artifact_roots,
                user_roots,
                &backend::NativeInstallRequest {
                    artifact: bundle,
                    target_dir: home.join(".local/bin"),
                    confirmed: true,
                },
                &backend::CancellationToken::new(),
                |progress| {
                    if progress.completed == 1 {
                        copy_paused_tx.send(()).expect("signal paused copy");
                        resume_rx.recv().expect("resume copy");
                    }
                },
            )
        });
        copy_paused_rx.recv().expect("install reaches mid-copy");

        let lock_barrier = Arc::new(Barrier::new(2));
        let lock_events = Arc::new(Mutex::new(Vec::new()));
        let hook_barrier = Arc::clone(&lock_barrier);
        let hook_events = Arc::clone(&lock_events);
        state.set_native_lock_hook(move |event| {
            let label = match event {
                NativeLockEvent::Waiting => "waiting",
                NativeLockEvent::Acquired => "acquired",
            };
            hook_events.lock().expect("lock events").push(label);
            if matches!(event, NativeLockEvent::Waiting) {
                hook_barrier.wait();
            }
        });
        let uninstall_state = state.clone();
        let prior_id = prior.installation_id.clone();
        let uninstall = std::thread::spawn(move || {
            uninstall_state.uninstall_native(
                "main",
                &backend::NativeUninstallRequest {
                    installation_id: prior_id,
                    confirmed: true,
                },
                &backend::CancellationToken::new(),
                |_| {},
            )
        });
        lock_barrier.wait();
        lock_events.lock().expect("lock events").push("released");
        resume_tx.send(()).expect("release install");

        let installed = install.join().expect("install thread").expect("install");
        uninstall.join().expect("uninstall thread").expect("uninstall");
        assert_eq!(
            *lock_events.lock().expect("lock events"),
            ["waiting", "released", "acquired"],
            "uninstall must reach the held lock and acquire it only after install is released"
        );
        let records = state.native_records("main").expect("final records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].installation_id, installed.installation_id);
        assert_ne!(records[0].installation_id, prior.installation_id);
        assert!(!root.join("home/.local/bin/converter").exists());
        assert!(root.join("home/.local/bin/bundle").is_dir());
    }

    #[test]
    fn picker_intents_keep_output_and_artifact_scopes_explicit() {
        assert!(
            validate_picker_intent(&PickerSelectRequest {
                kind: crate::contract::PickerKind::Input,
                selection: PickerSelection::Files,
            })
            .is_ok()
        );
        assert_eq!(
            validate_picker_intent(&PickerSelectRequest {
                kind: crate::contract::PickerKind::Output,
                selection: PickerSelection::File,
            })
            .expect_err("output files are not output roots")
            .code,
            IpcErrorCode::InvalidArgument
        );
        assert!(
            validate_picker_intent(&PickerSelectRequest {
                kind: crate::contract::PickerKind::Input,
                selection: PickerSelection::Directory,
            })
            .is_ok()
        );
        assert!(
            validate_picker_intent(&PickerSelectRequest {
                kind: crate::contract::PickerKind::Artifact,
                selection: PickerSelection::Directory,
            })
            .is_ok()
        );
        assert_eq!(
            validate_picker_intent(&PickerSelectRequest {
                kind: crate::contract::PickerKind::Artifact,
                selection: PickerSelection::Files,
            })
            .expect_err("one artifact intent at a time")
            .code,
            IpcErrorCode::InvalidArgument
        );
    }

    #[test]
    fn picker_cancel_preserves_existing_provenance_and_selection_replaces_it() {
        let first = std::env::temp_dir().join(format!(
            "universal-picker-first-{}-{}",
            std::process::id(),
            PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let second = std::env::temp_dir().join(format!(
            "universal-picker-second-{}-{}",
            std::process::id(),
            PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&first, b"first").expect("first fixture");
        fs::write(&second, b"second").expect("second fixture");
        let picker = PickerProvenance::default();
        apply_picker_result(&picker, "main", PickerKind::Input, Some(vec![first.clone()])).expect("first selection");

        let cancelled = apply_picker_result(&picker, "main", PickerKind::Input, None).expect("picker cancellation");
        assert!(cancelled.is_empty());
        assert!(picker.resolve_existing("main", PickerKind::Input, &first).is_ok());

        apply_picker_result(&picker, "main", PickerKind::Input, Some(vec![second.clone()]))
            .expect("replacement selection");
        assert!(picker.resolve_existing("main", PickerKind::Input, &second).is_ok());
        assert_eq!(
            picker
                .resolve_existing("main", PickerKind::Input, &first)
                .expect_err("old selection revoked")
                .code,
            IpcErrorCode::Forbidden
        );
        let _ = fs::remove_file(first);
        let _ = fs::remove_file(second);
    }

    #[test]
    fn atomic_preferences_replace_retains_previous_file_as_backup() {
        let root = std::env::temp_dir().join(format!(
            "universal-bridge-atomic-prefs-{}-{}",
            std::process::id(),
            PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("test root");
        let path = root.join("preferences.json");
        let first = Preferences::default();
        atomic_write_json(&path, &first).expect("first write");
        let second = Preferences {
            language: "en".to_owned(),
            theme: "dark".to_owned(),
            font_id: "dejavusans".to_owned(),
        };
        atomic_write_json(&path, &second).expect("replacement");
        assert_eq!(
            serde_json::from_slice::<Preferences>(&fs::read(&path).expect("current")).expect("current json"),
            second
        );
        assert_eq!(
            serde_json::from_slice::<Preferences>(&fs::read(path.with_extension("json.bak")).expect("backup"))
                .expect("backup json"),
            first
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn native_io_errors_map_to_the_contract_codes() {
        let error = |kind| io::Error::new(kind, "fixture");
        assert_eq!(
            map_native_install_error(error(io::ErrorKind::InvalidInput)).code,
            IpcErrorCode::InvalidArgument
        );
        assert_eq!(
            map_native_install_error(error(io::ErrorKind::PermissionDenied)).code,
            IpcErrorCode::Forbidden
        );
        assert_eq!(
            map_native_install_error(error(io::ErrorKind::AlreadyExists)).code,
            IpcErrorCode::Conflict
        );
        assert_eq!(
            map_native_uninstall_error(error(io::ErrorKind::NotFound)).code,
            IpcErrorCode::NotFound
        );
        assert_eq!(
            map_native_uninstall_error(error(io::ErrorKind::Interrupted)).code,
            IpcErrorCode::Cancelled
        );
    }

    #[test]
    fn resource_loader_verifies_hash_manifest_and_containment() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("app root");
        let script = resource_initialization_script(root).expect("valid app-local resources");
        assert!(script.contains("langs/en.json"));
        assert!(script.contains("fonts/DejaVuSans.ttf"));
        assert!(!script.contains("DejaVuSansFallback.ttf"));
        assert!(contained_physical_resource(root, "../outside_fixture/fonts/DejaVuSans.ttf").is_err());
        assert!(contained_physical_resource(root, "/fonts/DejaVuSans.ttf").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resource_loader_rejects_symlink_in_parent_component() {
        use std::os::unix::fs::symlink;

        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "universal-bridge-resource-symlink-{}-{nonce}",
            std::process::id()
        ));
        let root = base.join("root");
        let external = base.join("external");
        fs::create_dir_all(&root).expect("resource root");
        fs::create_dir_all(&external).expect("external directory");
        fs::write(external.join("en.json"), b"{}").expect("external resource");
        symlink(&external, root.join("langs")).expect("parent symlink");

        let error = contained_physical_resource(&root, "langs/en.json").expect_err("symlink rejected");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn resource_loader_exposes_no_font_when_primary_is_corrupt() {
        let nonce = PREFERENCES_WRITE_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universal-bridge-font-fallback-{}-{nonce}", std::process::id()));
        fs::create_dir_all(root.join("fonts")).expect("font directory");
        fs::write(root.join("fonts/DejaVuSans.ttf"), b"corrupt").expect("corrupt primary");
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../fonts/LICENSE.txt"),
            root.join("fonts/LICENSE.txt"),
        )
        .expect("license fixture");
        fs::write(root.join("fonts/manifest.sha256"), FONT_MANIFEST).expect("manifest fixture");

        let fonts = verified_fonts(&root).expect("valid exact manifest");
        assert!(fonts.primary.is_none());
        let _ = fs::remove_dir_all(root);
    }
}
