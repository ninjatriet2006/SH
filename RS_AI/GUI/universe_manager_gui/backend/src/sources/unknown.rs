use super::{AppSource, SourceKind};
use crate::cancellation::CancellationToken;
use crate::config::{AppEntry, ManagerConfig};
use crate::error::Result;
use crate::process_snapshot::ProcessSnapshot;
use std::path::Path;
use std::process::Command;

pub struct UnknownSource;

impl AppSource for UnknownSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Unknown
    }

    fn scan(
        &self,
        _config: &ManagerConfig,
        _managed_dir: Option<&Path>,
        _cancellation: &CancellationToken,
    ) -> Result<Vec<AppEntry>> {
        Ok(Vec::new())
    }

    fn is_running(&self, app: &AppEntry, snapshot: &ProcessSnapshot) -> bool {
        match app.exec_path.trim().is_empty() {
            true => false,
            false => {
                let exec_path = Path::new(&app.exec_path);
                match exec_path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => match snapshot.names.contains(name) {
                        true => true,
                        false => snapshot.cmdlines.iter().any(|cmd| cmd.contains(name)),
                    },
                    None => false,
                }
            }
        }
    }

    fn start(&self, app: &AppEntry) -> Result<()> {
        match app.exec_path.trim().is_empty() {
            true => Err(crate::error::BackendError::new(
                crate::error::ErrorKind::InvalidArgument,
                "Unknown app has empty exec_path",
            )),
            false => match Command::new(&app.exec_path).spawn() {
                Ok(_) => Ok(()),
                Err(err) => Err(crate::error::BackendError::new(
                    crate::error::ErrorKind::Io,
                    format!("Failed to start unknown app '{}': {err}", app.exec_path),
                )),
            },
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
                            let _ = Command::new("killall").arg(name).status();
                        }
                        #[cfg(windows)]
                        {
                            let exe_name = match name.to_lowercase().ends_with(".exe") {
                                true => name.to_string(),
                                false => format!("{name}.exe"),
                            };
                            let _ = Command::new("taskkill")
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
