use super::{AppSource, SourceKind};
use crate::cancellation::CancellationToken;
use crate::config::AppEntry;
use crate::discovery::parse_desktop_file;
use crate::error::Result;
use crate::process_snapshot::ProcessSnapshot;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct SnapSource;

impl SnapSource {
    fn clean_id(raw_id: &str) -> &str {
        raw_id.strip_suffix("-snap").unwrap_or(raw_id)
    }

    fn snap_directories() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/var/lib/snapd/desktop/applications"),
            PathBuf::from("/snap/bin"),
        ]
    }
}

impl AppSource for SnapSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Snap
    }

    fn scan(
        &self,
        _config: &crate::config::ManagerConfig,
        _managed_dir: Option<&Path>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AppEntry>> {
        let mut apps = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for dir in Self::snap_directories() {
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
                                        match seen.insert(app.id.clone()) {
                                            true => apps.push(app),
                                            false => {}
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
        let id = Self::clean_id(&app.id);
        match snapshot.cmdlines.iter().any(|cmd| cmd.contains(id)) {
            true => true,
            false => match app.exec_path.trim().is_empty() {
                true => false,
                false => match Path::new(&app.exec_path).file_name().and_then(|n| n.to_str()) {
                    Some(name) => snapshot.names.contains(name),
                    None => false,
                },
            },
        }
    }

    fn start(&self, app: &AppEntry) -> Result<()> {
        let id = Self::clean_id(&app.id);
        match Command::new(&app.exec_path).spawn() {
            Ok(_) => Ok(()),
            Err(_) => match Command::new("snap").args(["run", id]).spawn() {
                Ok(_) => Ok(()),
                Err(err) => Err(crate::error::BackendError::new(
                    crate::error::ErrorKind::Io,
                    format!("Failed to start Snap app '{id}': {err}"),
                )),
            },
        }
    }

    fn stop(&self, app: &AppEntry) -> Result<()> {
        let id = Self::clean_id(&app.id);
        let _ = Command::new("killall").arg(id).status();
        Ok(())
    }
}
