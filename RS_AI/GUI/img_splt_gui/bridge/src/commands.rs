use crate::contract::*;
use crate::security::PickerProvenance;
use img_splt_backend as backend;
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
use tauri::{AppHandle, Emitter, Manager, Runtime, State, Window};

pub const JOB_CAPABILITIES_CHECK: &str = "job.capabilities_check";
pub const JOB_SCAN_IMAGES: &str = "job.scan_images";
pub const JOB_PROCESS_IMAGES: &str = "job.process_images";
pub const JOB_DISTRIBUTE: &str = "job.distribute";
pub const INVOKE_REGISTRY: [CommandContract; 9] = [
    CommandContract::command("settings_load", &[IpcErrorCode::Io, IpcErrorCode::Internal]),
    CommandContract::command("settings_save", &[IpcErrorCode::Validation, IpcErrorCode::Io]),
    CommandContract::job(
        "capabilities_check",
        JOB_CAPABILITIES_CHECK,
        &[IpcErrorCode::Unavailable, IpcErrorCode::Internal],
    ),
    CommandContract::job(
        "scan_images",
        JOB_SCAN_IMAGES,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::NotFound,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "process_images",
        JOB_PROCESS_IMAGES,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::Conflict,
            IpcErrorCode::Unavailable,
            IpcErrorCode::Io,
            IpcErrorCode::Cancelled,
        ],
    ),
    CommandContract::job(
        "distribute",
        JOB_DISTRIBUTE,
        &[
            IpcErrorCode::InvalidArgument,
            IpcErrorCode::Conflict,
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
];
pub const TOPICS: [&str; 4] = [
    JOB_CAPABILITIES_CHECK,
    JOB_SCAN_IMAGES,
    JOB_PROCESS_IMAGES,
    JOB_DISTRIBUTE,
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

static WRITE_ID: AtomicU64 = AtomicU64::new(0);
static PREFERENCES_LOCK: Mutex<()> = Mutex::new(());
const DEJAVU_HASH: &str = "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280";
const LICENSE_HASH: &str = "63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9";
const FONT_MANIFEST: &str = "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280  DejaVuSans.ttf\n63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9  LICENSE.txt\n";

pub fn resource_initialization_script(resource_root: &Path) -> io::Result<String> {
    let root = fs::canonicalize(resource_root)?;
    let en = resource_or_missing(&root, "langs/en.json")?;
    let vi = resource_or_missing(&root, "langs/vi.json")?;
    let system = resource_or_missing(&root, "themes/system.json")?;
    let light = resource_or_missing(&root, "themes/light.json")?;
    let dark = resource_or_missing(&root, "themes/dark.json")?;
    let primary = verified_font(&root).ok();
    let paths = serde_json::json!({
        "languages": {"en": en, "vi": vi},
        "themes": {"system": system, "light": light, "dark": dark},
        "fonts": {"primary": primary},
    });
    let serialized = serde_json::to_string(&paths).map_err(io::Error::other)?;
    Ok(format!("window.__IMG_SPLT_RESOURCES__={serialized};"))
}

fn resource_or_missing(root: &Path, relative: &str) -> io::Result<PathBuf> {
    Ok(contained_resource(root, relative).unwrap_or_else(|_| root.join(".unavailable-resource").join(relative)))
}

fn verified_font(root: &Path) -> io::Result<PathBuf> {
    let font = contained_resource(root, "fonts/DejaVuSans.ttf")?;
    let license = contained_resource(root, "fonts/LICENSE.txt")?;
    let manifest = contained_resource(root, "fonts/manifest.sha256")?;
    verify_sha256(&font, DEJAVU_HASH)?;
    verify_sha256(&license, LICENSE_HASH)?;
    if fs::read_to_string(manifest)? != FONT_MANIFEST {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "font manifest does not match contract",
        ));
    }
    Ok(font)
}

fn contained_resource(root: &Path, relative: &str) -> io::Result<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid resource ID"));
    }
    let mut candidate = root.to_owned();
    for part in relative.components() {
        candidate.push(part.as_os_str());
        if fs::symlink_metadata(&candidate)?.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "resource symlink denied",
            ));
        }
    }
    let canonical = fs::canonicalize(candidate)?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "resource escapes bundle",
        ));
    }
    Ok(canonical)
}

fn verify_sha256(path: &Path, expected: &str) -> io::Result<()> {
    let actual = format!("{:x}", Sha256::digest(fs::read(path)?));
    if actual == expected {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, "resource hash mismatch"))
    }
}

#[derive(Clone, Default)]
pub struct BridgeState {
    pub picker: PickerProvenance,
    jobs: Arc<Mutex<HashMap<String, WindowJobs>>>,
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
    fn window_generation(&self, window: &str, state_code: IpcErrorCode) -> Result<u64, IpcError> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error(state_code, "job state lock is poisoned"))?;
        Ok(jobs.entry(window.to_owned()).or_default().generation)
    }

    fn register_job(
        &self,
        window: &str,
        request_id: String,
        expected_generation: u64,
        duplicate_code: IpcErrorCode,
        state_code: IpcErrorCode,
    ) -> Result<ActiveJob, IpcError> {
        let token = backend::CancellationToken::new();
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error(state_code, "job state lock is poisoned"))?;
        let window_jobs = jobs.entry(window.to_owned()).or_default();
        if window_jobs.generation != expected_generation {
            return Err(error(state_code, "window lifecycle changed during job preflight"));
        }
        if !window_jobs.used.insert(request_id.clone()) {
            return Err(error(
                duplicate_code,
                "request_id was already used in this window lifecycle",
            ));
        }
        let generation = window_jobs.generation;
        window_jobs
            .active
            .insert(request_id.clone(), (generation, token.clone()));
        Ok(ActiveJob {
            jobs: Arc::clone(&self.jobs),
            window: window.to_owned(),
            request_id,
            generation,
            cancellation: token,
        })
    }

    pub fn clear_window(&self, window: &str) {
        let _ = self.picker.clear_window(window);
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
    }
}

struct JobLifecycle<'a, R: Runtime, T: Serialize + Clone> {
    window: &'a Window<R>,
    topic: &'static str,
    request_id: String,
    seq: u64,
    terminal: bool,
    marker: std::marker::PhantomData<T>,
}

impl<'a, R: Runtime, T: Serialize + Clone> JobLifecycle<'a, R, T> {
    fn start(window: &'a Window<R>, topic: &'static str, request_id: String) -> Result<Self, IpcError> {
        let mut lifecycle = Self {
            window,
            topic,
            request_id,
            seq: 0,
            terminal: false,
            marker: std::marker::PhantomData,
        };
        lifecycle.emit(JobState::Started, None, None, None, None)?;
        Ok(lifecycle)
    }

    fn progress(&mut self, completed: Option<u64>, total: Option<u64>, message: Option<String>) {
        let _ = self.emit(JobState::Progress, completed, total, message, None);
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

    fn fail(&mut self, failure: IpcError) -> Result<(), IpcError> {
        let (state, result) = if failure.code == IpcErrorCode::Cancelled {
            (
                JobState::Cancelled,
                JobResult::Cancelled {
                    request_id: self.request_id.clone(),
                    error: failure,
                },
            )
        } else {
            (
                JobState::Failed,
                JobResult::Failed {
                    request_id: self.request_id.clone(),
                    error: failure,
                },
            )
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
        if self.terminal || state.is_terminal() != result.is_some() {
            return Err(error(IpcErrorCode::Internal, "invalid job lifecycle transition"));
        }
        self.window
            .emit(
                self.topic,
                JobEvent {
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
                },
            )
            .map_err(|source| error(IpcErrorCode::Internal, source.to_string()))?;
        self.seq = self
            .seq
            .checked_add(1)
            .ok_or_else(|| error(IpcErrorCode::Internal, "job sequence overflow"))?;
        self.terminal = state.is_terminal();
        Ok(())
    }
}

fn validate_request<T>(request: &Req<T>, job: bool) -> Result<Option<String>, IpcError> {
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

async fn run_job<R, T, F>(
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
        let mut lifecycle = JobLifecycle::start(&window, topic, request_id.clone())
            .map_err(|failure| remap(failure, infrastructure_code))?;
        let result = operation(&mut lifecycle, &active.cancellation);
        let response = match result {
            Ok(value) => {
                lifecycle
                    .complete(value.clone())
                    .map_err(|failure| remap(failure, infrastructure_code))?;
                Ok(response(Some(request_id), value))
            }
            Err(failure) => {
                lifecycle
                    .fail(failure.clone())
                    .map_err(|emit_failure| remap(emit_failure, infrastructure_code))?;
                Err(failure)
            }
        };
        drop(active);
        response
    })
    .await
    .map_err(|source| error(infrastructure_code, format!("job worker failed: {source}")))?
}

#[tauri::command]
pub fn settings_load<R: Runtime>(request: Req<Empty>, app: AppHandle<R>) -> IpcResult<ImageSettings> {
    let request_id = validate_request(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Internal))?;
    let settings = backend::load_settings(settings_path(&app)?)
        .map(map_settings)
        .map_err(|failure| remap(map_backend(failure), IpcErrorCode::Io))?;
    Ok(response(request_id, settings))
}

#[tauri::command]
pub fn settings_save<R: Runtime>(request: Req<ImageSettings>, app: AppHandle<R>) -> IpcResult<ImageSettings> {
    let request_id = validate_request(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Validation))?;
    let settings = backend_settings(&request.payload);
    settings
        .validate()
        .map_err(|failure| remap(map_backend(failure), IpcErrorCode::Validation))?;
    backend::save_settings(
        settings_path(&app).map_err(|failure| remap(failure, IpcErrorCode::Io))?,
        &settings,
    )
    .map_err(|failure| remap(map_backend(failure), IpcErrorCode::Io))?;
    Ok(response(request_id, request.payload))
}

#[tauri::command]
pub async fn capabilities_check<R: Runtime>(
    request: Req<Empty>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<CapabilityReport> {
    let request_id = validate_request(&request, true)
        .map_err(|failure| remap(failure, IpcErrorCode::Internal))?
        .ok_or_else(|| error(IpcErrorCode::Internal, "validated request has no ID"))?;
    let generation = state.window_generation(window.label(), IpcErrorCode::Internal)?;
    let active = state.register_job(
        window.label(),
        request_id,
        generation,
        IpcErrorCode::Internal,
        IpcErrorCode::Internal,
    )?;
    run_job(
        window,
        JOB_CAPABILITIES_CHECK,
        active,
        IpcErrorCode::Internal,
        |_, cancellation| {
            backend::check_capabilities(&backend::SystemCapabilityProbe, cancellation)
                .map(map_capabilities)
                .map_err(|failure| match failure.kind() {
                    backend::ErrorKind::Cancelled => error(IpcErrorCode::Internal, failure.to_string()),
                    _ => remap(map_backend(failure), IpcErrorCode::Unavailable),
                })
        },
    )
    .await
}

#[tauri::command]
pub async fn scan_images<R: Runtime>(
    request: Req<ScanImagesRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<ImageScanReport> {
    let request_id = validate_request(&request, true)?
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "validated request has no ID"))?;
    let generation = state.window_generation(window.label(), IpcErrorCode::Io)?;
    let directory = state
        .picker
        .resolve_directory(
            window.label(),
            ImagePickerKind::Input,
            Path::new(&request.payload.directory.path),
        )
        .map_err(map_path_argument)?;
    let active = state.register_job(
        window.label(),
        request_id,
        generation,
        IpcErrorCode::InvalidArgument,
        IpcErrorCode::Io,
    )?;
    run_job(
        window,
        JOB_SCAN_IMAGES,
        active,
        IpcErrorCode::Io,
        move |job, cancellation| {
            backend::scan_images(directory, cancellation, |progress| {
                job.progress(
                    Some(progress.scanned),
                    None,
                    Some(format!("{} images", progress.images)),
                );
            })
            .map(map_scan_report)
            .map_err(map_backend)
        },
    )
    .await
}

#[tauri::command]
pub async fn process_images<R: Runtime>(
    request: Req<ProcessImagesRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<ProcessReport> {
    let request_id = validate_request(&request, true)?
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "validated request has no ID"))?;
    let generation = state.window_generation(window.label(), IpcErrorCode::Io)?;
    let options = process_options(&state.picker, window.label(), &request.payload)?;
    let active = state.register_job(
        window.label(),
        request_id,
        generation,
        IpcErrorCode::Conflict,
        IpcErrorCode::Io,
    )?;
    run_job(
        window,
        JOB_PROCESS_IMAGES,
        active,
        IpcErrorCode::Io,
        move |job, cancellation| {
            backend::process_images(&options, &backend::SystemMediaToolRunner, cancellation, |progress| {
                job.progress(Some(progress.completed), Some(progress.total), None);
            })
            .map(map_process_report)
            .map_err(map_process_error)
        },
    )
    .await
}

#[tauri::command]
pub async fn distribute<R: Runtime>(
    request: Req<DistributeRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<DistributionReport> {
    let request_id = validate_request(&request, true)?
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "validated request has no ID"))?;
    let generation = state.window_generation(window.label(), IpcErrorCode::Io)?;
    let options = distribution_options(&state.picker, window.label(), &request.payload)?;
    let active = state.register_job(
        window.label(),
        request_id,
        generation,
        IpcErrorCode::Conflict,
        IpcErrorCode::Io,
    )?;
    run_job(
        window,
        JOB_DISTRIBUTE,
        active,
        IpcErrorCode::Io,
        move |job, cancellation| {
            backend::distribute_images(&options, cancellation, |progress| {
                job.progress(Some(progress.completed), Some(progress.total), None);
            })
            .map(map_distribution_report)
            .map_err(map_distribution_error)
        },
    )
    .await
}

#[tauri::command]
pub fn preferences_get<R: Runtime>(request: Req<Empty>, app: AppHandle<R>) -> IpcResult<Preferences> {
    let request_id = validate_request(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Internal))?;
    let preferences = load_or_import_preferences(&preferences_path(&app)?)?;
    Ok(response(request_id, preferences))
}

#[tauri::command]
pub fn preferences_set<R: Runtime>(request: Req<Preferences>, app: AppHandle<R>) -> IpcResult<Preferences> {
    let request_id = validate_request(&request, false).map_err(|failure| remap(failure, IpcErrorCode::Validation))?;
    validate_preferences(&request.payload)?;
    atomic_write_json(
        &preferences_path(&app).map_err(|failure| remap(failure, IpcErrorCode::Io))?,
        &request.payload,
    )
    .map_err(|failure| remap(failure, IpcErrorCode::Io))?;
    Ok(response(request_id, request.payload))
}

#[tauri::command]
pub async fn picker_select<R: Runtime>(
    request: Req<ImagePickerSelectRequest>,
    window: Window<R>,
    state: State<'_, BridgeState>,
) -> IpcResult<ImagePickerSelectResult> {
    let request_id = validate_request(&request, false)?;
    let dialog = rfd::FileDialog::new().set_parent(&window);
    let selected = tauri::async_runtime::spawn_blocking(move || dialog.pick_folder())
        .await
        .map_err(|source| error(IpcErrorCode::Internal, format!("picker worker failed: {source}")))?
        .ok_or_else(|| error(IpcErrorCode::InvalidArgument, "directory selection was cancelled"))?;
    let selected = state
        .picker
        .replace_directory(window.label(), request.payload.kind, &selected)?;
    Ok(response(
        request_id,
        ImagePickerSelectResult {
            kind: request.payload.kind,
            path: path_ref(selected),
        },
    ))
}

fn process_options(
    picker: &PickerProvenance,
    window: &str,
    request: &ProcessImagesRequest,
) -> Result<backend::ProcessOptions, IpcError> {
    let input = picker
        .resolve_directory(window, ImagePickerKind::Input, Path::new(&request.input_directory.path))
        .map_err(map_mutating_path_argument)?;
    let output = picker
        .resolve_directory(
            window,
            ImagePickerKind::Output,
            Path::new(&request.output_directory.path),
        )
        .map_err(map_mutating_path_argument)?;
    let files = request
        .files
        .iter()
        .map(|file| {
            picker
                .resolve_existing(window, ImagePickerKind::Input, Path::new(&file.path))
                .map_err(map_mutating_path_argument)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(backend::ProcessOptions {
        input_directory: input,
        files,
        output_directory: output,
        output_format: request.output_format.clone(),
        upscale: request.upscale,
        settings: backend_settings(&request.settings),
    })
}

fn distribution_options(
    picker: &PickerProvenance,
    window: &str,
    request: &DistributeRequest,
) -> Result<backend::DistributionOptions, IpcError> {
    let input = picker
        .resolve_directory(window, ImagePickerKind::Input, Path::new(&request.input_directory.path))
        .map_err(map_mutating_path_argument)?;
    let output = picker
        .resolve_directory(
            window,
            ImagePickerKind::Output,
            Path::new(&request.output_directory.path),
        )
        .map_err(map_mutating_path_argument)?;
    let files = request
        .files
        .iter()
        .map(|file| {
            picker
                .resolve_existing(window, ImagePickerKind::Input, Path::new(&file.path))
                .map_err(map_mutating_path_argument)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(backend::DistributionOptions {
        input_directory: input,
        files,
        output_directory: output,
        chapter: request.chapter,
        mode: match request.mode {
            DistributionMode::Balanced => backend::DistributionMode::Balanced,
            DistributionMode::Greedy => backend::DistributionMode::Greedy,
            DistributionMode::Fixed => backend::DistributionMode::Fixed,
        },
        max_files_per_folder: request.max_files_per_folder,
        fixed_folder_count: request.fixed_folder_count,
    })
}

fn backend_settings(settings: &ImageSettings) -> backend::ImageSettings {
    backend::ImageSettings {
        default_distribution_mode: match settings.default_distribution_mode {
            DistributionMode::Balanced => "balanced",
            DistributionMode::Greedy => "greedy",
            DistributionMode::Fixed => "fixed",
        }
        .to_owned(),
        max_files_per_folder: settings.max_files_per_folder,
        fixed_folder_count: settings.fixed_folder_count,
        max_retries: settings.max_retries,
        min_upscale_width: settings.min_upscale_width,
        target_upscale_width: settings.target_upscale_width,
    }
}

fn map_settings(settings: backend::ImageSettings) -> ImageSettings {
    ImageSettings {
        default_distribution_mode: match settings.default_distribution_mode.as_str() {
            "greedy" => DistributionMode::Greedy,
            "fixed" => DistributionMode::Fixed,
            _ => DistributionMode::Balanced,
        },
        max_files_per_folder: settings.max_files_per_folder,
        fixed_folder_count: settings.fixed_folder_count,
        max_retries: settings.max_retries,
        min_upscale_width: settings.min_upscale_width,
        target_upscale_width: settings.target_upscale_width,
    }
}

fn map_capabilities(report: backend::CapabilityReport) -> CapabilityReport {
    CapabilityReport {
        ffmpeg: map_tool(report.ffmpeg),
        ffprobe: map_tool(report.ffprobe),
    }
}

fn map_tool(tool: backend::ToolStatus) -> ToolStatus {
    ToolStatus {
        available: tool.available,
        version: tool.version,
    }
}

fn map_scan_report(report: backend::ImageScanReport) -> ImageScanReport {
    ImageScanReport {
        directory: path_ref(report.directory),
        images: report.images.into_iter().map(path_ref).collect(),
        total: report.total,
    }
}

fn map_process_report(report: backend::ProcessReport) -> ProcessReport {
    ProcessReport {
        output_directory: path_ref(report.output_directory),
        processed: report.processed.into_iter().map(path_ref).collect(),
        failed: report.failed.into_iter().map(path_ref).collect(),
    }
}

fn map_distribution_report(report: backend::DistributionReport) -> DistributionReport {
    DistributionReport {
        output_directory: path_ref(report.output_directory),
        folders: report.folders.into_iter().map(path_ref).collect(),
        distributed: report.distributed,
    }
}

fn path_ref(path: PathBuf) -> PathRef {
    PathRef {
        path: path.to_string_lossy().into_owned(),
    }
}

fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|source| error(IpcErrorCode::Internal, source.to_string()))?;
    fs::create_dir_all(&directory).map_err(io_error)?;
    Ok(directory)
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    Ok(app_data_dir(app)?.join("settings.yaml"))
}

fn preferences_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    Ok(app_data_dir(app)?.join("preferences.json"))
}

fn load_or_import_preferences(path: &Path) -> Result<Preferences, IpcError> {
    match fs::read(path) {
        Ok(bytes) => {
            let parsed = serde_json::from_slice::<Preferences>(&bytes).ok();
            let normalized = parsed.clone().map(normalize_preferences).unwrap_or_default();
            if parsed.as_ref() != Some(&normalized) {
                atomic_write_json(path, &normalized)?;
            }
            Ok(normalized)
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            let preferences = normalize_preferences(import_eframe_preferences().unwrap_or_default());
            atomic_write_json(path, &preferences)?;
            Ok(preferences)
        }
        Err(source) => Err(io_error(source)),
    }
}

fn import_eframe_preferences() -> Option<Preferences> {
    let values: HashMap<String, String> =
        ron::from_str(&fs::read_to_string(legacy_eframe_storage_dir("img_splt_gui")?.join("app.ron")).ok()?).ok()?;
    let language = values.get("language").map_or("en", String::as_str);
    let theme = values.get("theme").map_or("dark", String::as_str);
    let font = values.get("font").map_or("", String::as_str);
    Some(Preferences {
        language: language.to_owned(),
        theme: theme.to_owned(),
        font_id: if font.is_empty() || font == "Default" {
            "system-default".to_owned()
        } else if font.contains("DejaVuSans") || font == "DejaVu Sans" {
            "dejavusans".to_owned()
        } else {
            font.to_owned()
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
        Some(base.join(app_id))
    }
    #[cfg(target_os = "macos")]
    {
        Some(
            PathBuf::from(std::env::var_os("HOME")?)
                .join("Library/Application Support")
                .join(app_id),
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

fn normalize_preferences(mut preferences: Preferences) -> Preferences {
    let defaults = Preferences::default();
    if !matches!(preferences.language.as_str(), "en" | "vi") {
        preferences.language = defaults.language;
    }
    if !matches!(preferences.theme.as_str(), "system" | "light" | "dark") {
        preferences.theme = defaults.theme;
    }
    if !matches!(preferences.font_id.as_str(), "system-default" | "dejavusans") {
        preferences.font_id = defaults.font_id;
    }
    preferences
}

fn validate_preferences(preferences: &Preferences) -> Result<(), IpcError> {
    if !matches!(preferences.language.as_str(), "en" | "vi") {
        return Err(error(IpcErrorCode::Validation, "language must be en or vi"));
    }
    if !matches!(preferences.theme.as_str(), "system" | "light" | "dark") {
        return Err(error(IpcErrorCode::Validation, "theme must be system, light, or dark"));
    }
    if !matches!(preferences.font_id.as_str(), "system-default" | "dejavusans") {
        return Err(error(
            IpcErrorCode::Validation,
            "font_id must be system-default or dejavusans",
        ));
    }
    Ok(())
}

fn atomic_write_json(path: &Path, value: &Preferences) -> Result<(), IpcError> {
    let _guard = PREFERENCES_LOCK
        .lock()
        .map_err(|_| error(IpcErrorCode::Io, "preferences write lock is poisoned"))?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| error(IpcErrorCode::Io, source.to_string()))?;
    let temporary = unique_sibling(path, "tmp");
    write_and_sync(&temporary, &bytes)?;
    if path.exists() {
        let backup = path.with_extension("json.bak");
        let backup_temporary = unique_sibling(path, "bak.tmp");
        if let Err(source) = fs::copy(path, &backup_temporary)
            .and_then(|_| fs::File::open(&backup_temporary)?.sync_all())
            .and_then(|()| atomic_replace(&backup_temporary, &backup))
        {
            let _ = fs::remove_file(&temporary);
            let _ = fs::remove_file(&backup_temporary);
            return Err(io_error(source));
        }
    }
    if let Err(source) = atomic_replace(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(io_error(source));
    }
    sync_parent(path).map_err(io_error)
}

fn unique_sibling(path: &Path, suffix: &str) -> PathBuf {
    let id = WRITE_ID.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.{suffix}-{}-{id}", std::process::id()))
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> Result<(), IpcError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io_error)?;
    if let Err(source) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(io_error(source));
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
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    // Both APIs atomically replace within one volume; pointers reference live NUL-terminated buffers.
    let succeeded = unsafe {
        if destination.exists() {
            ReplaceFileW(
                destination_wide.as_ptr(),
                source.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null(),
                std::ptr::null(),
            )
        } else {
            MoveFileExW(
                source.as_ptr(),
                destination_wide.as_ptr(),
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
    fs::File::open(path.parent().ok_or_else(|| io::Error::other("path has no parent"))?)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn map_backend(source: backend::BackendError) -> IpcError {
    let code = match source.kind() {
        backend::ErrorKind::InvalidArgument => IpcErrorCode::InvalidArgument,
        backend::ErrorKind::NotFound => IpcErrorCode::NotFound,
        backend::ErrorKind::Conflict => IpcErrorCode::Conflict,
        backend::ErrorKind::Unavailable => IpcErrorCode::Unavailable,
        backend::ErrorKind::Io => IpcErrorCode::Io,
        backend::ErrorKind::Validation => IpcErrorCode::Validation,
        backend::ErrorKind::Cancelled => IpcErrorCode::Cancelled,
    };
    error(code, source.to_string())
}

fn map_process_error(source: backend::BackendError) -> IpcError {
    let failure = map_backend(source);
    match failure.code {
        IpcErrorCode::NotFound => remap(failure, IpcErrorCode::Io),
        IpcErrorCode::Validation => remap(failure, IpcErrorCode::InvalidArgument),
        _ => failure,
    }
}

fn map_distribution_error(source: backend::BackendError) -> IpcError {
    let failure = map_backend(source);
    match failure.code {
        IpcErrorCode::NotFound => remap(failure, IpcErrorCode::Io),
        IpcErrorCode::Unavailable | IpcErrorCode::Validation => remap(failure, IpcErrorCode::InvalidArgument),
        _ => failure,
    }
}

fn map_path_argument(failure: IpcError) -> IpcError {
    match failure.code {
        IpcErrorCode::NotFound => failure,
        IpcErrorCode::Io => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

fn map_mutating_path_argument(failure: IpcError) -> IpcError {
    match failure.code {
        IpcErrorCode::Io => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

fn remap(mut failure: IpcError, code: IpcErrorCode) -> IpcError {
    failure.code = code;
    failure
}

fn io_error(source: io::Error) -> IpcError {
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
