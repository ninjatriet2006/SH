use crate::path_security::require_absolute;
use crate::{BackendError, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSettings {
    pub default_distribution_mode: String,
    pub max_files_per_folder: u64,
    pub fixed_folder_count: u64,
    pub max_retries: u64,
    pub min_upscale_width: u32,
    pub target_upscale_width: u32,
}

impl Default for ImageSettings {
    fn default() -> Self {
        Self {
            default_distribution_mode: "balanced".to_owned(),
            max_files_per_folder: 80,
            fixed_folder_count: 5,
            max_retries: 5,
            min_upscale_width: 600,
            target_upscale_width: 1280,
        }
    }
}

impl ImageSettings {
    pub fn validate(&self) -> Result<()> {
        if !matches!(self.default_distribution_mode.as_str(), "balanced" | "greedy" | "fixed") {
            return Err(BackendError::new(
                ErrorKind::Validation,
                "distribution mode must be balanced, greedy, or fixed",
            ));
        }
        if self.max_files_per_folder == 0 || self.fixed_folder_count == 0 || self.max_retries == 0 {
            return Err(BackendError::new(
                ErrorKind::Validation,
                "folder limits and max retries must be greater than zero",
            ));
        }
        if self.min_upscale_width == 0 || self.target_upscale_width <= self.min_upscale_width {
            return Err(BackendError::new(
                ErrorKind::Validation,
                "target upscale width must exceed the non-zero minimum width",
            ));
        }
        Ok(())
    }
}

pub fn load_settings(path: impl AsRef<Path>) -> Result<ImageSettings> {
    let path = path.as_ref();
    require_absolute(path)?;
    if !path.exists() {
        return Ok(ImageSettings::default());
    }
    let metadata =
        fs::symlink_metadata(path).map_err(|error| BackendError::from_io("cannot inspect settings", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "settings path must be a regular non-symlink file",
            path,
        ));
    }
    let contents =
        fs::read_to_string(path).map_err(|error| BackendError::from_io("cannot read settings", path, error))?;
    let settings: ImageSettings = serde_yaml::from_str(&contents)
        .map_err(|error| BackendError::at_path(ErrorKind::Validation, format!("invalid settings: {error}"), path))?;
    settings.validate()?;
    Ok(settings)
}

pub fn save_settings(path: impl AsRef<Path>, settings: &ImageSettings) -> Result<()> {
    let path = path.as_ref();
    require_absolute(path)?;
    settings.validate()?;
    let parent = path
        .parent()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "settings path has no parent", path))?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| BackendError::from_io("cannot resolve settings directory", parent, error))?;
    if !parent.is_dir() {
        return Err(BackendError::at_path(
            ErrorKind::InvalidArgument,
            "settings parent is not a directory",
            parent,
        ));
    }
    if path.exists() {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| BackendError::from_io("cannot inspect settings", path, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(BackendError::at_path(
                ErrorKind::InvalidArgument,
                "settings path must be a regular non-symlink file",
                path,
            ));
        }
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| BackendError::at_path(ErrorKind::InvalidArgument, "settings path has no file name", path))?;
    let final_path = parent.join(file_name);
    let temp_path = unique_temp_path(&parent, file_name)?;
    let yaml = serde_yaml::to_string(settings)
        .map_err(|error| BackendError::new(ErrorKind::Validation, format!("cannot encode settings: {error}")))?;

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| BackendError::from_io("cannot create temporary settings", &temp_path, error))?;
        file.write_all(yaml.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| BackendError::from_io("cannot write temporary settings", &temp_path, error))?;
        fs::rename(&temp_path, &final_path)
            .map_err(|error| BackendError::from_io("cannot replace settings", &final_path, error))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn unique_temp_path(parent: &Path, file_name: &std::ffi::OsStr) -> Result<PathBuf> {
    for index in 0..1_000_u16 {
        let candidate = parent.join(format!(".{}.img-splt-{index}.tmp", file_name.to_string_lossy()));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(BackendError::new(
        ErrorKind::Conflict,
        "cannot allocate a temporary settings path",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("img-splt-settings-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).expect("create test directory");
        path
    }

    #[test]
    fn settings_round_trip_and_defaults_match_legacy() {
        let directory = test_dir();
        let path = directory.join("settings.yaml");
        assert_eq!(
            load_settings(&path).expect("default settings"),
            ImageSettings::default()
        );
        save_settings(&path, &ImageSettings::default()).expect("save settings");
        assert_eq!(load_settings(&path).expect("load settings"), ImageSettings::default());
    }

    #[test]
    fn invalid_settings_are_typed_validation_errors() {
        let settings = ImageSettings {
            max_files_per_folder: 0,
            ..ImageSettings::default()
        };
        assert_eq!(
            settings.validate().expect_err("invalid settings").kind(),
            ErrorKind::Validation
        );
    }
}
