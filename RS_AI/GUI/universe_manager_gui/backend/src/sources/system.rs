use super::{AppSource, SourceKind};
use crate::cancellation::CancellationToken;
use crate::config::{AppEntry, ManagerConfig};
use crate::discovery::parse_desktop_file;
use crate::error::Result;
use crate::process_snapshot::ProcessSnapshot;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct SystemSource;

impl SystemSource {
    fn system_directories() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        #[cfg(unix)]
        {
            match std::env::var_os("HOME") {
                Some(home) => {
                    dirs.push(PathBuf::from(home).join(".local/share/applications"));
                }
                None => {}
            }
            dirs.push(PathBuf::from("/usr/share/applications"));
            dirs.push(PathBuf::from("/usr/local/share/applications"));
        }
        dirs
    }
}

impl AppSource for SystemSource {
    fn kind(&self) -> SourceKind {
        SourceKind::System
    }

    fn scan(
        &self,
        _config: &ManagerConfig,
        _managed_dir: Option<&Path>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AppEntry>> {
        let mut apps = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for dir in Self::system_directories() {
            cancellation.check()?;
            match dir.is_dir() {
                true => match fs::read_dir(&dir) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            cancellation.check()?;
                            let path = entry.path();
                            match path.extension().and_then(|e| e.to_str()) {
                                Some("desktop") => match parse_desktop_file(&path) {
                                    Some(app) => {
                                        // Ignore Flatpak and Snap entries here as they have dedicated sources
                                        let is_isolated = match app.package_type.as_deref() {
                                            Some("Flatpak" | "Snap") => true,
                                            _ => false,
                                        };
                                        match is_isolated {
                                            true => {}
                                            false => match seen.insert(app.id.clone()) {
                                                true => apps.push(app),
                                                false => {}
                                            },
                                        }
                                    }
                                    None => {}
                                },
                                _ => {}
                            }
                        }
                    }
                    Err(_) => {}
                },
                false => {}
            }
        }

        Ok(apps)
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
            true => match app.desktop_file.trim().is_empty() {
                true => Err(crate::error::BackendError::new(
                    crate::error::ErrorKind::InvalidArgument,
                    "System app has neither exec_path nor desktop_file",
                )),
                false => {
                    #[cfg(unix)]
                    {
                        match Command::new("gtk-launch").arg(&app.id).spawn() {
                            Ok(_) => Ok(()),
                            Err(err) => Err(crate::error::BackendError::new(
                                crate::error::ErrorKind::Io,
                                format!("Failed to start system app via gtk-launch: {err}"),
                            )),
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        Ok(())
                    }
                }
            },
            false => {
                let parts: Vec<&str> = app.exec_path.split_whitespace().collect();
                match parts.first() {
                    Some(&bin) => match Command::new(bin).args(&parts[1..]).spawn() {
                        Ok(_) => Ok(()),
                        Err(err) => Err(crate::error::BackendError::new(
                            crate::error::ErrorKind::Io,
                            format!("Failed to start system app '{bin}': {err}"),
                        )),
                    },
                    None => Err(crate::error::BackendError::new(
                        crate::error::ErrorKind::InvalidArgument,
                        "System app exec path is empty",
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
