use crate::path_security::{AllowedRoots, require_absolute};
use crate::{BackendError, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum InstallType {
    #[default]
    InPlace,
    Moved,
}

/// Serialization-compatible with `universe_manager::config::AppEntry`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub install_type: InstallType,
    pub source_path: Option<String>,
    pub install_path: String,
    pub exec_path: String,
    pub icon_path: Option<String>,
    pub desktop_file: String,
    pub symlink_file: Option<String>,
    pub added_at: String,
    pub is_custom: Option<bool>,
    pub start_cmd: Option<String>,
    pub stop_cmd: Option<String>,
    pub category: Option<String>,
    pub package_type: Option<String>,
    #[serde(default)]
    pub inventory_sources: Vec<String>,
    #[serde(default)]
    pub registry_key: Option<String>,
    #[serde(default)]
    pub product_code: Option<String>,
    #[serde(default)]
    pub about_url: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub uninstall_cmd: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct ManagerSettings {
    pub managed_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct ManagerConfig {
    pub settings: ManagerSettings,
    pub apps: Vec<AppEntry>,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    config_root: PathBuf,
    managed_roots: AllowedRoots,
}

pub fn load_config(app_config_dir: PathBuf) -> Result<ManagerConfig> {
    let config_roots = AllowedRoots::new([app_config_dir], "app config")?;
    let path = config_roots.first().join("config.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ManagerConfig {
                settings: ManagerSettings {
                    managed_dir: String::new(),
                },
                apps: Vec::new(),
            });
        }
        Err(error) => return Err(BackendError::from_io("cannot read manager config", &path, error)),
    };
    serde_json::from_slice(&bytes)
        .map_err(|error| BackendError::with_source(ErrorKind::Internal, "manager config is malformed", error))
}

impl ConfigStore {
    pub fn new(app_config_dir: PathBuf, managed_roots: Vec<PathBuf>) -> Result<Self> {
        let config_roots = AllowedRoots::new([app_config_dir], "app config")?;
        let managed_roots = AllowedRoots::new(managed_roots, "managed directory")?;
        Ok(Self {
            config_root: config_roots.first().to_path_buf(),
            managed_roots,
        })
    }

    pub fn config_path(&self) -> PathBuf {
        self.config_root.join("config.json")
    }

    pub fn default_config(&self) -> ManagerConfig {
        ManagerConfig {
            settings: ManagerSettings {
                managed_dir: self.managed_roots.first().to_string_lossy().into_owned(),
            },
            apps: Vec::new(),
        }
    }

    pub fn load(&self) -> Result<ManagerConfig> {
        self.ensure_config_parent()?;
        let path = self.config_path();
        if !path.exists() {
            return Ok(self.default_config());
        }
        let bytes =
            fs::read(&path).map_err(|error| BackendError::from_io("cannot read manager config", &path, error))?;
        let config = serde_json::from_slice::<ManagerConfig>(&bytes)
            .map_err(|error| BackendError::with_source(ErrorKind::Internal, "manager config is malformed", error))?;
        self.validate(&config)?;
        Ok(config)
    }

    pub fn save(&self, config: &ManagerConfig) -> Result<()> {
        self.ensure_config_parent()?;
        self.validate(config)?;
        let bytes = serde_json::to_vec_pretty(config).map_err(|error| {
            BackendError::with_source(ErrorKind::Internal, "cannot serialize manager config", error)
        })?;
        let target = self.config_path();
        for sequence in 0..100_u32 {
            let temporary = self
                .config_root
                .join(format!(".config.json.{}.{}.tmp", std::process::id(), sequence));
            match OpenOptions::new().write(true).create_new(true).open(&temporary) {
                Ok(mut file) => {
                    let write_result = (|| -> std::io::Result<()> {
                        file.write_all(&bytes)?;
                        file.sync_all()?;
                        fs::rename(&temporary, &target)?;
                        Ok(())
                    })();
                    if let Err(error) = write_result {
                        let _ = fs::remove_file(&temporary);
                        return Err(BackendError::from_io(
                            "cannot atomically save manager config",
                            target,
                            error,
                        ));
                    }
                    return Ok(());
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(BackendError::from_io(
                        "cannot create temporary config",
                        temporary,
                        error,
                    ));
                }
            }
        }
        Err(BackendError::new(
            ErrorKind::Conflict,
            "cannot allocate a temporary config file",
        ))
    }

    pub fn managed_directory(&self, config: &ManagerConfig) -> Result<PathBuf> {
        let managed_str = config.settings.managed_dir.trim();
        if managed_str.is_empty() {
            return Ok(self.config_root.clone());
        }
        self.managed_roots
            .resolve_directory(Path::new(managed_str))
    }

    pub fn resolve_app<'a>(&self, config: &'a ManagerConfig, app_id: &str) -> Result<&'a AppEntry> {
        validate_id(app_id)?;
        let mut matches = config.apps.iter().filter(|app| app.id == app_id);
        let app = matches
            .next()
            .ok_or_else(|| BackendError::new(ErrorKind::NotFound, "application ID is not registered"))?;
        if matches.next().is_some() {
            return Err(BackendError::new(ErrorKind::Conflict, "application ID is not unique"));
        }
        Ok(app)
    }

    pub(crate) fn resolve_executable(&self, config: &ManagerConfig, app: &AppEntry) -> Result<PathBuf> {
        let executable = PathBuf::from(&app.exec_path);
        let ptype = app.package_type.as_deref().unwrap_or("Local");
        if ptype != "Local" {
            return Ok(executable);
        }
        let managed = self.managed_directory(config)?;
        require_absolute(&executable)?;
        if !executable.starts_with(&managed) {
            return Err(BackendError::at_path(
                ErrorKind::Forbidden,
                "application executable path is not beneath managed_dir",
                executable,
            ));
        }
        Ok(executable)
    }

    fn validate(&self, config: &ManagerConfig) -> Result<()> {
        let managed_str = config.settings.managed_dir.trim();
        if !managed_str.is_empty() {
            self.managed_directory(config)?;
        }
        let mut ids = std::collections::HashSet::new();
        for app in &config.apps {
            validate_id(&app.id)?;
            if !ids.insert(&app.id) {
                return Err(BackendError::new(
                    ErrorKind::Validation,
                    "application IDs must be unique",
                ));
            }
        }
        Ok(())
    }

    fn ensure_config_parent(&self) -> Result<()> {
        let resolved = fs::canonicalize(&self.config_root).map_err(|error| {
            BackendError::from_io("cannot re-resolve app config directory", &self.config_root, error)
        })?;
        if resolved != self.config_root || !resolved.is_dir() {
            return Err(BackendError::at_path(
                ErrorKind::Forbidden,
                "app config directory identity changed",
                resolved,
            ));
        }
        Ok(())
    }
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 160
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        Err(BackendError::new(
            ErrorKind::Validation,
            "application ID has an invalid format",
        ))
    } else {
        Ok(())
    }
}
