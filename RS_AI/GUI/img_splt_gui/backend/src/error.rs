use std::fmt;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, BackendError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidArgument,
    NotFound,
    Conflict,
    Unavailable,
    Io,
    Validation,
    Cancelled,
}

#[derive(Debug)]
pub struct RollbackFailure {
    from_path: PathBuf,
    to_path: PathBuf,
    source: std::io::Error,
}

impl RollbackFailure {
    pub(crate) fn new(from_path: PathBuf, to_path: PathBuf, source: std::io::Error) -> Self {
        Self {
            from_path,
            to_path,
            source,
        }
    }

    pub fn from_path(&self) -> &std::path::Path {
        &self.from_path
    }

    pub fn to_path(&self) -> &std::path::Path {
        &self.to_path
    }

    pub fn io_error(&self) -> &std::io::Error {
        &self.source
    }
}

#[derive(Debug)]
pub struct PartialRollback {
    failures: Vec<RollbackFailure>,
}

impl PartialRollback {
    pub fn failures(&self) -> &[RollbackFailure] {
        &self.failures
    }
}

#[derive(Debug)]
pub struct BackendError {
    kind: ErrorKind,
    message: String,
    path: Option<PathBuf>,
    source: Option<std::io::Error>,
    partial_rollback: Option<PartialRollback>,
}

impl BackendError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: None,
            source: None,
            partial_rollback: None,
        }
    }

    pub fn at_path(kind: ErrorKind, message: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            message: message.into(),
            path: Some(path.into()),
            source: None,
            partial_rollback: None,
        }
    }

    pub(crate) fn from_io(message: impl Into<String>, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        let kind = match source.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::AlreadyExists => ErrorKind::Conflict,
            _ => ErrorKind::Io,
        };
        Self {
            kind,
            message: message.into(),
            path: Some(path.into()),
            source: Some(source),
            partial_rollback: None,
        }
    }

    pub(crate) fn with_partial_rollback(mut self, failures: Vec<RollbackFailure>) -> Self {
        if !failures.is_empty() {
            self.partial_rollback = Some(PartialRollback { failures });
        }
        self
    }

    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    pub fn partial_rollback(&self) -> Option<&PartialRollback> {
        self.partial_rollback.as_ref()
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)?;
        if let Some(path) = &self.path {
            write!(formatter, ": {}", path.display())?;
        }
        if let Some(rollback) = &self.partial_rollback {
            write!(
                formatter,
                "; rollback incomplete: {} move(s) remain at their destination",
                rollback.failures.len()
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for BackendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|source| source as _)
    }
}
