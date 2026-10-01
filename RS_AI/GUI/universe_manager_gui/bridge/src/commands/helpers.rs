use crate::contract::*;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};
use universe_manager_backend as backend;

pub fn app_config_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    let path = match app.path().app_config_dir() {
        Ok(dir) => dir,
        Err(source) => return Err(error(IpcErrorCode::Internal, source.to_string())),
    };
    match fs::create_dir_all(&path) {
        Ok(_) => {}
        Err(err) => return Err(io_error(err)),
    }
    match fs::canonicalize(path) {
        Ok(canonical) => Ok(canonical),
        Err(err) => Err(io_error(err)),
    }
}

pub fn app_data_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    let path = match app.path().app_data_dir() {
        Ok(dir) => dir,
        Err(source) => return Err(error(IpcErrorCode::Internal, source.to_string())),
    };
    match fs::create_dir_all(&path) {
        Ok(_) => Ok(path),
        Err(err) => Err(io_error(err)),
    }
}

pub fn preferences_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, IpcError> {
    Ok(app_data_dir(app)?.join("preferences.json"))
}

pub fn load_config<R: Runtime>(
    app: &AppHandle<R>,
    code: IpcErrorCode,
) -> Result<backend::ManagerConfig, IpcError> {
    let dir = app_config_dir(app)?;
    match backend::load_config(dir) {
        Ok(config) => Ok(config),
        Err(failure) => Err(remap(map_backend(failure), code)),
    }
}

pub fn path_ref(path: PathBuf) -> PathRef {
    PathRef {
        path: path.to_string_lossy().into_owned(),
    }
}

pub fn map_detection(report: backend::DetectionReport) -> DetectionReport {
    DetectionReport {
        is_appimage: report.is_appimage,
        suggested_name: report.suggested_name,
        executables: report.executables.into_iter().map(path_ref).collect(),
        icons: report.icons.into_iter().map(path_ref).collect(),
        desktop_templates: report.desktop_templates.into_iter().map(path_ref).collect(),
    }
}

pub fn map_operation(result: backend::OperationResult) -> OperationResult {
    OperationResult {
        app_id: result.app_id,
        operation: result.operation,
        completed: result.completed,
    }
}

pub fn map_search(report: backend::SearchReport) -> SearchReport {
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

pub fn validate_preferences(value: &Preferences) -> Result<(), IpcError> {
    match value.language.as_str() {
        "vi" | "en" => {}
        _ => return Err(error(IpcErrorCode::Validation, "language must be vi or en")),
    }
    match value.theme.as_str() {
        "system" | "light" | "dark" => {}
        _ => {
            return Err(error(
                IpcErrorCode::Validation,
                "theme must be system, light, or dark",
            ));
        }
    }
    match value.font_id.as_str() {
        "system-default" | "dejavusans" => {}
        _ => {
            return Err(error(
                IpcErrorCode::Validation,
                "font_id must be system-default or dejavusans",
            ));
        }
    }
    Ok(())
}

pub fn map_backend(source: backend::BackendError) -> IpcError {
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

pub fn map_detect_path(failure: IpcError) -> IpcError {
    match failure.code {
        IpcErrorCode::NotFound | IpcErrorCode::Io => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

pub fn map_detect_backend(failure: backend::BackendError) -> IpcError {
    let failure = map_backend(failure);
    match failure.code {
        IpcErrorCode::NotFound | IpcErrorCode::Io | IpcErrorCode::Cancelled => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

pub fn map_action_error(failure: IpcError) -> IpcError {
    match failure.code {
        IpcErrorCode::NotFound
        | IpcErrorCode::Forbidden
        | IpcErrorCode::Io
        | IpcErrorCode::Cancelled => failure,
        _ => remap(failure, IpcErrorCode::InvalidArgument),
    }
}

pub fn map_action_backend(failure: backend::BackendError) -> IpcError {
    map_action_error(map_backend(failure))
}

pub fn map_search_backend(failure: backend::BackendError) -> IpcError {
    let failure = map_backend(failure);
    match failure.code {
        IpcErrorCode::Cancelled | IpcErrorCode::Io => failure,
        _ => remap(failure, IpcErrorCode::Validation),
    }
}

pub fn remap(mut failure: IpcError, code: IpcErrorCode) -> IpcError {
    failure.code = code;
    failure
}

pub fn io_error(source: std::io::Error) -> IpcError {
    error(IpcErrorCode::Io, source.to_string())
}

pub fn error(code: IpcErrorCode, message: impl Into<String>) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable: false,
        details: None,
    }
}
