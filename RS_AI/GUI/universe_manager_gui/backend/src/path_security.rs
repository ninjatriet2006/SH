use crate::{BackendError, ErrorKind, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct AllowedRoots {
    roots: Vec<PathBuf>,
}

impl AllowedRoots {
    pub(crate) fn new(paths: impl IntoIterator<Item = PathBuf>, label: &str) -> Result<Self> {
        let mut roots = Vec::new();
        for path in paths {
            require_absolute(&path)?;
            let resolved = fs::canonicalize(&path)
                .map_err(|error| BackendError::from_io(format!("cannot resolve {label} root"), &path, error))?;
            if !resolved.is_dir() {
                return Err(BackendError::at_path(
                    ErrorKind::InvalidArgument,
                    format!("{label} root is not a directory"),
                    path,
                ));
            }
            roots.push(resolved);
        }
        roots.sort();
        roots.dedup();
        if roots.is_empty() {
            return Err(BackendError::new(
                ErrorKind::Forbidden,
                format!("{label} roots must not be empty"),
            ));
        }
        Ok(Self { roots })
    }

    pub(crate) fn resolve(&self, path: &Path) -> Result<PathBuf> {
        require_absolute(path)?;
        let resolved =
            fs::canonicalize(path).map_err(|error| BackendError::from_io("cannot resolve path", path, error))?;
        self.ensure_resolved(&resolved)?;
        Ok(resolved)
    }

    pub(crate) fn resolve_directory(&self, path: &Path) -> Result<PathBuf> {
        let resolved = self.resolve(path)?;
        if !resolved.is_dir() {
            return Err(BackendError::at_path(
                ErrorKind::InvalidArgument,
                "path is not a directory",
                path,
            ));
        }
        Ok(resolved)
    }

    pub(crate) fn ensure_resolved(&self, path: &Path) -> Result<()> {
        if self.roots.iter().any(|root| path.starts_with(root)) {
            Ok(())
        } else {
            Err(BackendError::at_path(
                ErrorKind::Forbidden,
                "path resolves outside allowed roots",
                path,
            ))
        }
    }

    pub(crate) fn first(&self) -> &Path {
        &self.roots[0]
    }
}

pub(crate) fn require_absolute(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "path must be non-empty and absolute",
            path,
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn directory(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "universe-path-security-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("test directory");
        path
    }

    #[test]
    fn empty_roots_are_denied() {
        let error = AllowedRoots::new(Vec::new(), "managed").expect_err("empty roots must fail closed");
        assert_eq!(error.kind(), ErrorKind::Forbidden);
    }

    #[cfg(unix)]
    #[test]
    fn canonical_recheck_denies_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = directory("symlink");
        let managed = root.join("managed");
        let outside = root.join("outside");
        fs::create_dir_all(&managed).expect("managed");
        fs::create_dir_all(&outside).expect("outside");
        let link = managed.join("escape");
        symlink(&outside, &link).expect("symlink");
        let roots = AllowedRoots::new([managed], "managed").expect("roots");
        let error = roots
            .resolve_directory(&link)
            .expect_err("canonical escape must be denied");
        assert_eq!(error.kind(), ErrorKind::Forbidden);
    }
}
