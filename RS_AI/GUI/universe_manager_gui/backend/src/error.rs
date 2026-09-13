use std::fmt;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, BackendError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidArgument,
    NotFound,
    Forbidden,
    Unavailable,
    Conflict,
    Validation,
    Cancelled,
    Io,
    Internal,
}

#[derive(Debug)]
pub struct BackendError {
    kind: ErrorKind,
    message: String,
    path: Option<PathBuf>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl BackendError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
            source: None,
        }
    }

    pub fn at_path(kind: ErrorKind, message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: Some(path.into()),
            source: None,
        }
    }

    pub(crate) fn from_io(message: impl Into<String>, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        let kind = match source.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::AlreadyExists => ErrorKind::Conflict,
            std::io::ErrorKind::PermissionDenied => ErrorKind::Forbidden,
            _ => ErrorKind::Io,
        };
        Self {
            kind,
            message: message.into(),
            path: Some(path.into()),
            source: Some(Box::new(source)),
        }
    }

    pub(crate) fn with_source(
        kind: ErrorKind,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
            source: Some(Box::new(source)),
        }
    }

    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)?;
        if let Some(path) = &self.path {
            write!(formatter, ": {}", path.display())?;
        }
        Ok(())
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|source| source as _)
    }
}
