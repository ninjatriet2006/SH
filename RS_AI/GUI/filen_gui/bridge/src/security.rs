use crate::contract::{forbidden, internal, invalid, IpcError};
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

#[derive(Default)]
pub struct PickerRoots(Mutex<HashMap<String, Vec<PathBuf>>>);

impl PickerRoots {
    pub fn replace(&self, window: &str, selected: &Path) -> Result<PathBuf, IpcError> {
        let root = canonical_directory(selected)?;
        self.0
            .lock()
            .map_err(|_| internal("picker root lock is poisoned"))?
            .insert(window.to_owned(), vec![root.clone()]);
        Ok(root)
    }

    pub fn is_empty(&self, window: &str) -> Result<bool, IpcError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| internal("picker root lock is poisoned"))?
            .get(window)
            .is_none_or(Vec::is_empty))
    }

    pub fn existing(&self, window: &str, candidate: &Path) -> Result<PathBuf, IpcError> {
        require_absolute(candidate)?;
        let resolved = candidate.canonicalize().map_err(|error| invalid(error.to_string()))?;
        self.require_contained(window, resolved)
    }

    pub fn create(&self, window: &str, candidate: &Path) -> Result<PathBuf, IpcError> {
        require_absolute(candidate)?;
        if candidate.exists() {
            return self.existing(window, candidate);
        }
        let name = candidate
            .file_name()
            .ok_or_else(|| invalid("target must have a basename"))?;
        if Path::new(name).components().count() != 1
            || matches!(Path::new(name).components().next(), Some(Component::ParentDir))
        {
            return Err(invalid("target basename is invalid"));
        }
        let parent = candidate
            .parent()
            .ok_or_else(|| invalid("target must have a parent"))?
            .canonicalize()
            .map_err(|error| invalid(error.to_string()))?;
        self.require_contained(window, parent.join(name))
    }

    fn require_contained(&self, window: &str, candidate: PathBuf) -> Result<PathBuf, IpcError> {
        let roots = self.0.lock().map_err(|_| internal("picker root lock is poisoned"))?;
        let allowed = roots
            .get(window)
            .ok_or_else(|| forbidden("no picker-approved root exists for this window"))?;
        if allowed.iter().any(|root| candidate.starts_with(root)) {
            Ok(candidate)
        } else {
            Err(forbidden("path resolves outside picker-approved roots"))
        }
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf, IpcError> {
    require_absolute(path)?;
    let path = path.canonicalize().map_err(|error| invalid(error.to_string()))?;
    if path.is_dir() {
        Ok(path)
    } else {
        Err(invalid("picker selection must be a directory"))
    }
}

fn require_absolute(path: &Path) -> Result<(), IpcError> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        Err(invalid("path must be non-empty and absolute"))
    } else {
        Ok(())
    }
}

pub fn require_confirmation(confirmed: bool) -> Result<(), IpcError> {
    if confirmed {
        Ok(())
    } else {
        Err(forbidden("confirmed=true is required"))
    }
}

pub fn validate_sudo_argv(action: &str, args: &[String]) -> Result<(), IpcError> {
    let _ = (action, args);
    Err(forbidden("privileged filesystem actions are disabled"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn denies_empty_roots_traversal_symlink_escape_and_bad_argv() {
        let base = std::env::temp_dir().join(format!("filen-security-{}", std::process::id()));
        let root = base.join("root");
        let outside = base.join("outside");
        fs::create_dir_all(&root).expect("root");
        fs::create_dir_all(&outside).expect("outside");
        let roots = PickerRoots::default();
        assert!(roots.existing("main", &root).is_err());
        roots.replace("main", &root).expect("approve root");
        assert!(roots.create("other", &root.join("new")).is_err());
        assert!(roots.create("main", &root.join("../outside/file")).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, root.join("escape")).expect("symlink");
            assert!(roots.create("main", &root.join("escape/file")).is_err());
            assert!(roots.create("main", &root.join("escape")).is_err());
        }
        assert!(validate_sudo_argv("rm", &["--no-preserve-root".into()]).is_err());
        assert!(validate_sudo_argv("sh", &[root.to_string_lossy().into_owned()]).is_err());
        assert!(require_confirmation(false).is_err());
        let _ = fs::remove_dir_all(base);
    }
}
