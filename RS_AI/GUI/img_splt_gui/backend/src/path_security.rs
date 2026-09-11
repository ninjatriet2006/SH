use crate::{BackendError, ErrorKind, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn canonical_directory(path: &Path) -> Result<PathBuf> {
    require_absolute(path)?;
    let resolved =
        fs::canonicalize(path).map_err(|error| BackendError::from_io("cannot resolve directory", path, error))?;
    if !resolved.is_dir() {
        return Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "path is not a directory",
            path,
        ));
    }
    Ok(resolved)
}

pub(crate) fn canonical_direct_file(path: &Path, root: &Path) -> Result<PathBuf> {
    require_absolute(path)?;
    let resolved =
        fs::canonicalize(path).map_err(|error| BackendError::from_io("cannot resolve input file", path, error))?;
    if !resolved.is_file() {
        return Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "path is not a regular file",
            path,
        ));
    }
    if resolved.parent() != Some(root) {
        return Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "file resolves outside the input directory",
            path,
        ));
    }
    Ok(resolved)
}

pub(crate) fn require_absolute(path: &Path) -> Result<()> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "path must be absolute",
            path,
        ))
    }
}

pub(crate) fn file_name(path: &Path) -> Result<&std::ffi::OsStr> {
    path.file_name()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "path must have a file name", path))
}
