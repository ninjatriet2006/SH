use super::{AppSource, SourceKind};
use crate::cancellation::CancellationToken;
use crate::config::{AppEntry, ManagerConfig};
use crate::error::Result;
use crate::process_snapshot::ProcessSnapshot;
use std::path::Path;
#[cfg(windows)]
use std::path::PathBuf;
use std::process::Command;

pub struct WindowsSource;

impl WindowsSource {
    #[cfg(windows)]
    fn start_menu_directories() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        match std::env::var_os("APPDATA") {
            Some(appdata) => {
                dirs.push(PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs"));
            }
            None => {}
        }
        match std::env::var_os("ProgramData") {
            Some(progdata) => {
                dirs.push(PathBuf::from(progdata).join("Microsoft\\Windows\\Start Menu\\Programs"));
            }
            None => {}
        }
        dirs
    }
}

impl AppSource for WindowsSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Windows
    }

    fn scan(
        &self,
        _config: &ManagerConfig,
        _managed_dir: Option<&Path>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AppEntry>> {
        #[cfg(windows)]
        {
            let mut apps = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for dir in Self::start_menu_directories() {
                cancellation.check()?;
                match dir.is_dir() {
                    true => match std::fs::read_dir(&dir) {
                        Ok(entries) => {
                            for entry in entries.flatten() {
                                cancellation.check()?;
                                let path = entry.path();
                                match path.extension().and_then(|e| e.to_str()) {
                                    Some(ext) => match ext.to_ascii_lowercase().as_str() {
                                        "lnk" => {
                                            let stem = path
                                                .file_stem()
                                                .and_then(|s| s.to_str())
                                                .unwrap_or("App");
                                            let id = format!("win-{}", stem.to_ascii_lowercase().replace(' ', "-"));
                                            match seen.insert(id.clone()) {
                                                true => apps.push(AppEntry {
                                                    id,
                                                    name: stem.to_string(),
                                                    exec_path: path.to_string_lossy().into_owned(),
                                                    package_type: Some("Windows".to_string()),
                                                    inventory_sources: vec!["StartMenu".to_string()],
                                                    ..AppEntry::default()
                                                }),
                                                false => {}
                                            }
                                        }
                                        _ => {}
                                    },
                                    None => {}
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

        #[cfg(not(windows))]
        {
            let _ = cancellation.check();
            Ok(Vec::new())
        }
    }

    fn is_running(&self, app: &AppEntry, snapshot: &ProcessSnapshot) -> bool {
        match app.exec_path.trim().is_empty() {
            true => false,
            false => {
                let exec_path = Path::new(&app.exec_path);
                match exec_path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => {
                        let name_lower = name.to_lowercase();
                        match snapshot.names.contains(&name_lower) {
                            true => true,
                            false => {
                                let exe_name = match name_lower.ends_with(".exe") {
                                    true => name_lower.clone(),
                                    false => format!("{name_lower}.exe"),
                                };
                                snapshot.names.contains(&exe_name)
                            }
                        }
                    }
                    None => false,
                }
            }
        }
    }

    fn start(&self, app: &AppEntry) -> Result<()> {
        match app.exec_path.trim().is_empty() {
            true => Err(crate::error::BackendError::new(
                crate::error::ErrorKind::InvalidArgument,
                "Windows app exec_path is empty",
            )),
            false => {
                #[cfg(windows)]
                {
                    match Command::new("cmd")
                        .args(["/C", "start", "", &app.exec_path])
                        .spawn()
                    {
                        Ok(_) => Ok(()),
                        Err(err) => Err(crate::error::BackendError::new(
                            crate::error::ErrorKind::Io,
                            format!("Failed to start Windows app '{}': {err}", app.exec_path),
                        )),
                    }
                }
                #[cfg(not(windows))]
                {
                    match Command::new(&app.exec_path).spawn() {
                        Ok(_) => Ok(()),
                        Err(err) => Err(crate::error::BackendError::new(
                            crate::error::ErrorKind::Io,
                            format!("Failed to start app '{}': {err}", app.exec_path),
                        )),
                    }
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
                        #[cfg(windows)]
                        {
                            let exe_name = match name.to_lowercase().ends_with(".exe") {
                                true => name.to_string(),
                                false => format!("{name}.exe"),
                            };
                            let _ = Command::new("taskkill")
                                .args(["/F", "/IM", &exe_name])
                                .status();
                            Ok(())
                        }
                        #[cfg(not(windows))]
                        {
                            let _ = Command::new("killall").arg(name).status();
                            Ok(())
                        }
                    }
                    None => Ok(()),
                }
            }
        }
    }
}
