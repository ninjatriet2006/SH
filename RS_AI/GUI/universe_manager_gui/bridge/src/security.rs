use crate::contract::{IpcError, IpcErrorCode, UniversePickerKind};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Default)]
struct WindowRoots {
    managed: Option<PathBuf>,
    source: Option<PathBuf>,
}

impl WindowRoots {
    fn root(&self, kind: UniversePickerKind) -> Option<&PathBuf> {
        match kind {
            UniversePickerKind::Managed => self.managed.as_ref(),
            UniversePickerKind::Source => self.source.as_ref(),
        }
    }

    fn root_mut(&mut self, kind: UniversePickerKind) -> &mut Option<PathBuf> {
        match kind {
            UniversePickerKind::Managed => &mut self.managed,
            UniversePickerKind::Source => &mut self.source,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PickerProvenance {
    windows: Arc<RwLock<HashMap<String, WindowRoots>>>,
}

impl PickerProvenance {
    pub fn replace_directory(&self, window: &str, kind: UniversePickerKind, path: &Path) -> Result<PathBuf, IpcError> {
        let canonical = canonical_directory(path)?;
        let mut windows = self.windows.write().map_err(|_| lock_error())?;
        *windows.entry(window.to_owned()).or_default().root_mut(kind) = Some(canonical.clone());
        Ok(canonical)
    }

    pub fn root(&self, window: &str, kind: UniversePickerKind) -> Result<PathBuf, IpcError> {
        self.windows
            .read()
            .map_err(|_| lock_error())?
            .get(window)
            .and_then(|roots| roots.root(kind))
            .cloned()
            .ok_or_else(|| forbidden("no picker-approved root exists for this window and kind"))
    }

    pub fn resolve_directory(&self, window: &str, kind: UniversePickerKind, path: &Path) -> Result<PathBuf, IpcError> {
        let candidate = canonical_directory(path)?;
        let root = self.root(window, kind)?;
        if candidate == root {
            Ok(candidate)
        } else {
            Err(forbidden("directory is not the picker-approved root"))
        }
    }

    pub fn resolve_existing(&self, window: &str, kind: UniversePickerKind, path: &Path) -> Result<PathBuf, IpcError> {
        require_absolute(path)?;
        let candidate = fs::canonicalize(path).map_err(path_error)?;
        let root = self.root(window, kind)?;
        if candidate.starts_with(root) {
            Ok(candidate)
        } else {
            Err(forbidden("path resolves outside the picker-approved root"))
        }
    }

    pub fn clear_window(&self, window: &str) -> Result<(), IpcError> {
        self.windows.write().map_err(|_| lock_error())?.remove(window);
        Ok(())
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf, IpcError> {
    require_absolute(path)?;
    let canonical = fs::canonicalize(path).map_err(path_error)?;
    if canonical.is_dir() {
        Ok(canonical)
    } else {
        Err(error(IpcErrorCode::InvalidArgument, "selected path is not a directory"))
    }
}

fn require_absolute(path: &Path) -> Result<(), IpcError> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(error(IpcErrorCode::InvalidArgument, "path must be absolute"))
    }
}

fn path_error(source: std::io::Error) -> IpcError {
    let code = if source.kind() == std::io::ErrorKind::NotFound {
        IpcErrorCode::NotFound
    } else {
        IpcErrorCode::Io
    };
    error(code, source.to_string())
}

fn forbidden(message: &str) -> IpcError {
    error(IpcErrorCode::Forbidden, message)
}

fn lock_error() -> IpcError {
    error(IpcErrorCode::Internal, "bridge provenance lock is poisoned")
}

fn error(code: IpcErrorCode, message: impl Into<String>) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable: false,
        details: None,
    }
}
