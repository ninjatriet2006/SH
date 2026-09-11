use crate::contract::{IpcError, IpcErrorCode};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Input,
    Output,
    Artifact,
}

#[derive(Debug, Clone, Default)]
struct WindowRoots {
    input: Vec<PathBuf>,
    output: Vec<PathBuf>,
    artifact: Vec<PathBuf>,
}

impl WindowRoots {
    fn roots(&self, kind: PickerKind) -> &[PathBuf] {
        match kind {
            PickerKind::Input => &self.input,
            PickerKind::Output => &self.output,
            PickerKind::Artifact => &self.artifact,
        }
    }

    fn roots_mut(&mut self, kind: PickerKind) -> &mut Vec<PathBuf> {
        match kind {
            PickerKind::Input => &mut self.input,
            PickerKind::Output => &mut self.output,
            PickerKind::Artifact => &mut self.artifact,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PickerProvenance {
    windows: Arc<RwLock<HashMap<String, WindowRoots>>>,
}

impl PickerProvenance {
    pub fn replace_paths(&self, window: &str, kind: PickerKind, paths: &[PathBuf]) -> Result<(), IpcError> {
        let canonical = canonicalize_paths(paths)?;
        let mut windows = self.windows.write().map_err(|_| internal_lock_error())?;
        *windows.entry(window.to_owned()).or_default().roots_mut(kind) = canonical;
        Ok(())
    }

    pub fn record_paths(&self, window: &str, kind: PickerKind, paths: &[PathBuf]) -> Result<(), IpcError> {
        let canonical = canonicalize_paths(paths)?;
        let mut windows = self.windows.write().map_err(|_| internal_lock_error())?;
        let roots = windows.entry(window.to_owned()).or_default().roots_mut(kind);
        roots.extend(canonical);
        roots.sort();
        roots.dedup();
        Ok(())
    }

    pub fn canonical_roots(&self, window: &str, kind: PickerKind) -> Result<Vec<PathBuf>, IpcError> {
        let windows = self.windows.read().map_err(|_| internal_lock_error())?;
        let roots = windows.get(window).map(|entry| entry.roots(kind)).unwrap_or_default();
        if roots.is_empty() {
            return Err(forbidden("no picker-approved roots exist for this window"));
        }
        Ok(roots.to_vec())
    }

    pub fn resolve_existing(&self, window: &str, kind: PickerKind, path: &Path) -> Result<PathBuf, IpcError> {
        require_absolute(path)?;
        let resolved = fs::canonicalize(path).map_err(map_path_error)?;
        let roots = self.canonical_roots(window, kind)?;
        if roots.iter().any(|root| resolved.starts_with(root)) {
            Ok(resolved)
        } else {
            Err(forbidden("path resolves outside picker-approved roots"))
        }
    }

    pub fn clear_window(&self, window: &str) -> Result<(), IpcError> {
        self.windows.write().map_err(|_| internal_lock_error())?.remove(window);
        Ok(())
    }
}

impl From<crate::contract::PickerKind> for PickerKind {
    fn from(value: crate::contract::PickerKind) -> Self {
        match value {
            crate::contract::PickerKind::Input => Self::Input,
            crate::contract::PickerKind::Output => Self::Output,
            crate::contract::PickerKind::Artifact => Self::Artifact,
        }
    }
}

fn canonicalize_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>, IpcError> {
    let mut canonical = Vec::with_capacity(paths.len());
    for path in paths {
        require_absolute(path)?;
        canonical.push(fs::canonicalize(path).map_err(map_path_error)?);
    }
    canonical.sort();
    canonical.dedup();
    Ok(canonical)
}

fn require_absolute(path: &Path) -> Result<(), IpcError> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(IpcError {
            code: IpcErrorCode::InvalidArgument,
            message: "path must be absolute".to_owned(),
            retryable: false,
            details: None,
        })
    }
}

fn map_path_error(error: std::io::Error) -> IpcError {
    let code = if error.kind() == std::io::ErrorKind::NotFound {
        IpcErrorCode::NotFound
    } else {
        IpcErrorCode::Io
    };
    IpcError {
        code,
        message: error.to_string(),
        retryable: false,
        details: None,
    }
}

pub fn forbidden(message: &str) -> IpcError {
    IpcError {
        code: IpcErrorCode::Forbidden,
        message: message.to_owned(),
        retryable: false,
        details: None,
    }
}

fn internal_lock_error() -> IpcError {
    IpcError {
        code: IpcErrorCode::Internal,
        message: "bridge state lock is poisoned".to_owned(),
        retryable: false,
        details: None,
    }
}
