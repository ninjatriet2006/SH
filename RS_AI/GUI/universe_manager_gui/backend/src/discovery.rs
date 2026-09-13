use crate::path_security::AllowedRoots;
use crate::{AppEntry, BackendError, CancellationToken, ErrorKind, InstallType, ManagerConfig, Result};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub completed: u64,
    pub total: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetectionReport {
    pub is_appimage: bool,
    pub suggested_name: String,
    pub executables: Vec<PathBuf>,
    pub icons: Vec<PathBuf>,
    pub desktop_templates: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct DiscoveryService {
    source_roots: AllowedRoots,
}

impl DiscoveryService {
    pub fn new(source_roots: Vec<PathBuf>) -> Result<Self> {
        Ok(Self {
            source_roots: AllowedRoots::new(source_roots, "source")?,
        })
    }

    pub fn detect(
        &self,
        path: &Path,
        cancellation: &CancellationToken,
        mut progress: impl FnMut(Progress),
    ) -> Result<DetectionReport> {
        cancellation.check()?;
        let root = self.source_roots.resolve(path)?;
        progress(Progress {
            completed: 0,
            total: None,
            message: "Inspecting application".to_string(),
        });
        let mut report = DetectionReport {
            is_appimage: root.is_file() && is_appimage(&root),
            suggested_name: suggest_name(&root),
            executables: Vec::new(),
            icons: Vec::new(),
            desktop_templates: Vec::new(),
        };
        if root.is_file() {
            report.executables.push(root);
            progress(Progress {
                completed: 1,
                total: Some(1),
                message: "Detection complete".to_string(),
            });
            return Ok(report);
        }
        if !root.is_dir() {
            return Err(BackendError::at_path(
                ErrorKind::InvalidArgument,
                "source path is neither a file nor a directory",
                root,
            ));
        }
        let mut visited = 0_u64;
        self.walk(&root, &root, 0, cancellation, &mut visited, &mut progress, &mut report)?;
        sort_detection(&root, &mut report);
        progress(Progress {
            completed: visited,
            total: Some(visited),
            message: "Detection complete".to_string(),
        });
        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        source_root: &Path,
        directory: &Path,
        depth: u8,
        cancellation: &CancellationToken,
        visited: &mut u64,
        progress: &mut impl FnMut(Progress),
        report: &mut DetectionReport,
    ) -> Result<()> {
        if depth > 4 {
            return Ok(());
        }
        cancellation.check()?;
        let entries = fs::read_dir(directory)
            .map_err(|error| BackendError::from_io("cannot read source directory", directory, error))?;
        for entry in entries {
            cancellation.check()?;
            let entry = entry.map_err(|error| BackendError::from_io("cannot read source entry", directory, error))?;
            let path = entry.path();
            let canonical = fs::canonicalize(&path)
                .map_err(|error| BackendError::from_io("cannot resolve source entry", &path, error))?;
            self.source_roots.ensure_resolved(&canonical)?;
            if !canonical.starts_with(source_root) {
                return Err(BackendError::at_path(
                    ErrorKind::Forbidden,
                    "source entry escapes selected source",
                    canonical,
                ));
            }
            *visited += 1;
            progress(Progress {
                completed: *visited,
                total: None,
                message: canonical.to_string_lossy().into_owned(),
            });
            let file_type = entry
                .file_type()
                .map_err(|error| BackendError::from_io("cannot inspect source entry", &path, error))?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                self.walk(
                    source_root,
                    &canonical,
                    depth + 1,
                    cancellation,
                    visited,
                    progress,
                    report,
                )?;
            } else if file_type.is_file() {
                classify_file(&canonical, report);
            }
        }
        Ok(())
    }

    pub fn scan_managed(
        &self,
        config: &ManagerConfig,
        managed_dir: &Path,
        cancellation: &CancellationToken,
        mut progress: impl FnMut(Progress),
    ) -> Result<Vec<AppEntry>> {
        let managed = self.source_roots.resolve_directory(managed_dir)?;
        if Path::new(&config.settings.managed_dir) != managed_dir {
            return Err(BackendError::new(
                ErrorKind::Validation,
                "managed_dir does not match configuration",
            ));
        }
        let entries = fs::read_dir(&managed)
            .map_err(|error| BackendError::from_io("cannot scan managed directory", &managed, error))?;
        let mut paths = Vec::new();
        for entry in entries {
            cancellation.check()?;
            let path = entry
                .map_err(|error| BackendError::from_io("cannot read managed entry", &managed, error))?
                .path();
            if path.is_dir() {
                paths.push(path);
            }
        }
        paths.sort();
        let total = u64::try_from(paths.len()).unwrap_or(u64::MAX);
        let mut apps = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            cancellation.check()?;
            let report = self.detect(path, cancellation, |_| {})?;
            if let Some(executable) = report.executables.first() {
                let canonical_dir = fs::canonicalize(path)
                    .map_err(|error| BackendError::from_io("cannot resolve managed application", path, error))?;
                apps.push(AppEntry {
                    id: stable_app_id(&canonical_dir),
                    name: report.suggested_name,
                    install_type: InstallType::Moved,
                    install_path: canonical_dir.to_string_lossy().into_owned(),
                    exec_path: executable.to_string_lossy().into_owned(),
                    icon_path: report.icons.first().map(|value| value.to_string_lossy().into_owned()),
                    package_type: Some("Local".to_string()),
                    inventory_sources: vec!["Applications".to_string()],
                    ..AppEntry::default()
                });
            }
            progress(Progress {
                completed: u64::try_from(index + 1).unwrap_or(u64::MAX),
                total: Some(total),
                message: path.to_string_lossy().into_owned(),
            });
        }
        Ok(apps)
    }
}

pub fn stable_app_id(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("app")
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{}-{hash:016x}", name.trim_matches('-'))
}

fn classify_file(path: &Path, report: &mut DetectionReport) {
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "desktop" {
        report.desktop_templates.push(path.to_path_buf());
    } else if matches!(extension.as_str(), "png" | "svg" | "jpg" | "jpeg") {
        report.icons.push(path.to_path_buf());
    } else if is_executable(path) && !is_helper(path) {
        report.executables.push(path.to_path_buf());
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "exe" | "com" | "bat"))
}

fn is_helper(path: &Path) -> bool {
    let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
    name.contains(".so")
        || name.ends_with(".a")
        || name.ends_with(".node")
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "chrome-sandbox" | "chrome_crashpad_handler" | "crashpad_handler" | "updater" | "update"
        )
}

fn is_appimage(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".appimage"))
}

fn suggest_name(path: &Path) -> String {
    let raw = path.file_name().and_then(OsStr::to_str).unwrap_or("Application");
    let without_extension = raw.to_ascii_lowercase().find(".appimage").map_or_else(
        || raw.rsplit_once('.').map_or(raw, |(stem, _)| stem),
        |index| &raw[..index],
    );
    let words = without_extension
        .split(['-', '_'])
        .filter(|part| {
            !part.is_empty()
                && !part.starts_with(|character: char| character.is_ascii_digit())
                && !matches!(
                    part.to_ascii_lowercase().as_str(),
                    "x86" | "64" | "x64" | "amd64" | "linux" | "app" | "ide" | "portable"
                )
        })
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + chars.as_str()
            })
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        without_extension.to_string()
    } else {
        words.join(" ")
    }
}

fn sort_detection(root: &Path, report: &mut DetectionReport) {
    report.executables.sort_by(|left, right| {
        let left_root = left.parent() == Some(root);
        let right_root = right.parent() == Some(root);
        right_root.cmp(&left_root).then_with(|| left.cmp(right))
    });
    report.icons.sort();
    report.desktop_templates.sort();
}
