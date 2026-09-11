use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) struct CanonicalRoots {
    roots: Vec<PathBuf>,
}

impl CanonicalRoots {
    pub(crate) fn new(paths: impl IntoIterator<Item = PathBuf>, label: &str) -> io::Result<Self> {
        let mut roots = Vec::new();
        for path in paths {
            require_absolute(&path)?;
            roots.push(fs::canonicalize(path)?);
        }
        roots.sort();
        roots.dedup();
        if roots.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{label} roots must not be empty"),
            ));
        }
        Ok(Self { roots })
    }

    pub(crate) fn resolve_existing(&self, path: &Path) -> io::Result<PathBuf> {
        require_absolute(path)?;
        let resolved = fs::canonicalize(path)?;
        if self.contains(&resolved) {
            Ok(resolved)
        } else {
            Err(outside_roots())
        }
    }

    pub(crate) fn contains(&self, canonical_path: &Path) -> bool {
        self.roots.iter().any(|root| canonical_path.starts_with(root))
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) fn contains_exact(&self, canonical_path: &Path) -> bool {
        self.roots.iter().any(|root| canonical_path == root)
    }

    #[cfg(target_os = "linux")]
    pub(crate) fn paths(&self) -> &[PathBuf] {
        &self.roots
    }
}

pub(crate) fn require_absolute(path: &Path) -> io::Result<()> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "path must be absolute"))
    }
}

pub(crate) fn outside_roots() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "path resolves outside allowed roots")
}
