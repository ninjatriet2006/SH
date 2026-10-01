use super::{AppSource, SourceKind};
use crate::cancellation::CancellationToken;
use crate::config::{AppEntry, ManagerConfig};
use crate::discovery::DiscoveryService;
use crate::error::Result;
use crate::process_snapshot::ProcessSnapshot;
use std::path::{Path, PathBuf};

pub struct PortableSource;

impl AppSource for PortableSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Portable
    }

    fn scan(
        &self,
        config: &ManagerConfig,
        managed_dir: Option<&Path>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AppEntry>> {
        match managed_dir {
            Some(dir) => match dir.is_dir() {
                true => match DiscoveryService::new(vec![dir.to_path_buf()]) {
                    Ok(service) => match service.scan_managed(config, dir, cancellation, |_| {}) {
                        Ok(apps) => Ok(apps),
                        Err(_) => Ok(Vec::new()),
                    },
                    Err(_) => Ok(Vec::new()),
                },
                false => Ok(Vec::new()),
            },
            None => Ok(Vec::new()),
        }
    }

    fn is_running(&self, app: &AppEntry, snapshot: &ProcessSnapshot) -> bool {
        match app.exec_path.trim().is_empty() {
            true => false,
            false => {
                let exec_buf = PathBuf::from(&app.exec_path);
                match snapshot.canonical_exes.contains(&exec_buf) {
                    true => true,
                    false => match exec_buf.file_name().and_then(|n| n.to_str()) {
                        Some(name) => snapshot.names.contains(name),
                        None => false,
                    },
                }
            }
        }
    }

    fn start(&self, app: &AppEntry) -> Result<()> {
        match app.exec_path.trim().is_empty() {
            true => Err(crate::error::BackendError::new(
                crate::error::ErrorKind::InvalidArgument,
                "Portable executable path is empty",
            )),
            false => {
                let mut cmd = std::process::Command::new(&app.exec_path);
                match Path::new(&app.exec_path).parent() {
                    Some(parent) => {
                        cmd.current_dir(parent);
                    }
                    None => {}
                }
                match cmd.spawn() {
                    Ok(_) => Ok(()),
                    Err(err) => Err(crate::error::BackendError::from_io(
                        "cannot spawn portable app",
                        Path::new(&app.exec_path),
                        err,
                    )),
                }
            }
        }
    }

    fn stop(&self, app: &AppEntry) -> Result<()> {
        match app.exec_path.trim().is_empty() {
            true => Ok(()),
            false => {
                let path = Path::new(&app.exec_path);
                match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => {
                        #[cfg(unix)]
                        {
                            let _ = std::process::Command::new("killall").arg(name).status();
                        }
                        #[cfg(windows)]
                        {
                            let exe_name = match name.to_lowercase().ends_with(".exe") {
                                true => name.to_string(),
                                false => format!("{name}.exe"),
                            };
                            let _ = std::process::Command::new("taskkill")
                                .args(["/F", "/IM", &exe_name])
                                .status();
                        }
                        Ok(())
                    }
                    None => Ok(()),
                }
            }
        }
    }
}
