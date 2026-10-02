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
                let (desktop_file, discovered_icon) = find_matching_desktop_file(executable, &report.suggested_name)
                    .map(|(p, i)| (p.to_string_lossy().into_owned(), i))
                    .unwrap_or_else(|| (String::new(), None));
                let icon = report.icons.first().map(|value| value.to_string_lossy().into_owned())
                    .or(discovered_icon);

                let stem = executable.file_stem().and_then(OsStr::to_str).unwrap_or("").to_lowercase();
                let symlink_file = std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|h| h.join(".local/bin").join(&stem))
                    .filter(|p| {
                        fs::canonicalize(p)
                            .map(|c| c == *executable || fs::canonicalize(executable).ok() == Some(c))
                            .unwrap_or(false)
                    })
                    .map(|p| p.to_string_lossy().into_owned());

                apps.push(AppEntry {
                    id: stable_app_id(&canonical_dir),
                    name: report.suggested_name,
                    install_type: InstallType::Moved,
                    install_path: canonical_dir.to_string_lossy().into_owned(),
                    exec_path: executable.to_string_lossy().into_owned(),
                    icon_path: icon,
                    desktop_file,
                    symlink_file,
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
    let lower = name.to_ascii_lowercase();
    name.contains(".so")
        || name.ends_with(".a")
        || name.ends_with(".node")
        || matches!(
            lower.as_str(),
            "chrome-sandbox"
                | "chrome_crashpad_handler"
                | "crashpad_handler"
                | "qtwebengineprocess"
                | "updater"
                | "update"
                | "pingsender"
                | "minidump_stackwalk"
                | "elevation_service"
                | "notification_helper"
                | "vk_swiftshader"
                | "nacl_helper"
                | "nacl_helper_bootstrap"
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
                    "x86" | "64" | "x64" | "amd64" | "linux" | "app" | "portable"
                )
        })
        .map(|part| {
            if part.eq_ignore_ascii_case("ide") {
                "IDE".to_string()
            } else if part.eq_ignore_ascii_case("cli") {
                "CLI".to_string()
            } else if part.eq_ignore_ascii_case("gui") {
                "GUI".to_string()
            } else if part.eq_ignore_ascii_case("sdk") {
                "SDK".to_string()
            } else {
                let mut chars = part.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                })
            }
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        without_extension.to_string()
    } else {
        words.join(" ")
    }
}

fn sort_detection(root: &Path, report: &mut DetectionReport) {
    let root_stem = root
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let suggested_stem = report.suggested_name.to_ascii_lowercase().replace(' ', "");

    report.executables.sort_by(|left, right| {
        let left_stem = left
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let right_stem = right
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();

        let left_match = left_stem == root_stem || left_stem == suggested_stem;
        let right_match = right_stem == root_stem || right_stem == suggested_stem;

        let left_root = left.parent() == Some(root);
        let right_root = right.parent() == Some(root);

        right_match
            .cmp(&left_match)
            .then_with(|| right_root.cmp(&left_root))
            .then_with(|| left.cmp(right))
    });

    report.icons.sort_by(|left, right| {
        let left_str = left.to_string_lossy().to_ascii_lowercase();
        let right_str = right.to_string_lossy().to_ascii_lowercase();

        let left_preferred = (left_str.contains(&root_stem) || left_str.contains("icon") || left_str.contains("logo"))
            && !left_str.contains("busy") && !left_str.contains("indicator");
        let right_preferred = (right_str.contains(&root_stem) || right_str.contains("icon") || right_str.contains("logo"))
            && !right_str.contains("busy") && !right_str.contains("indicator");

        right_preferred.cmp(&left_preferred).then_with(|| left.cmp(right))
    });
    report.desktop_templates.sort();
}

pub fn parse_desktop_file(path: &Path) -> Option<AppEntry> {
    let content = fs::read_to_string(path).ok()?;
    let filename = path.file_name()?.to_string_lossy();
    let raw_id = filename.strip_suffix(".desktop").unwrap_or(&filename);
    let id: String = raw_id
        .chars()
        .map(|c| match c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':') {
            true => c,
            false => '-',
        })
        .collect();

    let mut name = String::new();
    let mut exec = String::new();
    let mut categories_str = String::new();
    let mut icon = None;
    let mut no_display = false;
    let mut is_application = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some(index) = line.find('=') {
            let key = line[..index].trim();
            let val = line[index + 1..].trim();
            match key {
                "Name" if name.is_empty() => name = val.to_string(),
                "Exec" => exec = val.to_string(),
                "Categories" => categories_str = val.to_string(),
                "Icon" => icon = Some(val.to_string()),
                "NoDisplay" if val.eq_ignore_ascii_case("true") => no_display = true,
                "Type" if val.eq_ignore_ascii_case("application") => is_application = true,
                _ => {}
            }
        }
    }

    if !is_application || no_display || name.is_empty() {
        return None;
    }

    let clean_exec = exec
        .split_whitespace()
        .filter(|part| !part.starts_with('%'))
        .collect::<Vec<&str>>()
        .join(" ")
        .replace(['"', '\''], "");

    let path_str = path.to_string_lossy().to_string();
    let (package_type, final_id, inventory_source) = if path_str.contains("flatpak") {
        ("Flatpak".to_string(), format!("{id}-flatpak"), "Flatpak".to_string())
    } else if path_str.contains("snap") {
        ("Snap".to_string(), format!("{id}-snap"), "Snap".to_string())
    } else if path_str.contains("/.local/share/applications") {
        ("Local".to_string(), id.clone(), "Desktop (user)".to_string())
    } else {
        ("APT".to_string(), id.clone(), "Desktop (system)".to_string())
    };

    let category = if !categories_str.is_empty() {
        categories_str.split(';').map(str::trim).find(|s| !s.is_empty()).map(String::from)
    } else {
        Some("Utility".to_string())
    };

    Some(AppEntry {
        id: final_id,
        name,
        install_type: InstallType::InPlace,
        source_path: None,
        install_path: path.parent().unwrap_or(Path::new("")).to_string_lossy().to_string(),
        exec_path: clean_exec,
        icon_path: icon,
        desktop_file: path_str,
        symlink_file: None,
        added_at: String::new(),
        is_custom: Some(package_type == "Flatpak" || package_type == "Snap"),
        start_cmd: if package_type == "Flatpak" {
            Some(format!("flatpak run {id}"))
        } else {
            None
        },
        stop_cmd: if package_type == "Flatpak" {
            Some(format!("flatpak kill {id}"))
        } else {
            None
        },
        category,
        package_type: Some(package_type),
        inventory_sources: vec![inventory_source],
        ..AppEntry::default()
    })
}

pub fn scan_system_applications(
    cancellation: &CancellationToken,
    mut progress: impl FnMut(Progress),
) -> Result<Vec<AppEntry>> {
    let mut scan_dirs = Vec::new();

    #[cfg(unix)]
    {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            scan_dirs.push(home.join(".local/share/applications"));
            scan_dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
        }
        scan_dirs.push(PathBuf::from("/usr/share/applications"));
        scan_dirs.push(PathBuf::from("/usr/local/share/applications"));
        scan_dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
        scan_dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
    }

    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
            scan_dirs.push(appdata.join("Microsoft\\Windows\\Start Menu\\Programs"));
        }
        if let Some(program_data) = std::env::var_os("ProgramData").map(PathBuf::from) {
            scan_dirs.push(program_data.join("Microsoft\\Windows\\Start Menu\\Programs"));
        }
    }

    let mut apps = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for dir in scan_dirs {
        cancellation.check()?;
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                cancellation.check()?;
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|e| e == "desktop") {
                    if let Some(app) = parse_desktop_file(&path) {
                        if !seen_ids.contains(&app.id) {
                            seen_ids.insert(app.id.clone());
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    progress(Progress {
        completed: apps.len() as u64,
        total: Some(apps.len() as u64),
        message: format!("Discovered {} system applications", apps.len()),
    });

    Ok(apps)
}

fn find_matching_desktop_file(exec_path: &Path, app_name: &str) -> Option<(PathBuf, Option<String>)> {
    let mut search_dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        search_dirs.push(home.join(".local/share/applications"));
    }
    search_dirs.push(PathBuf::from("/usr/share/applications"));
    search_dirs.push(PathBuf::from("/usr/local/share/applications"));

    let exec_str = exec_path.to_string_lossy();
    let exec_canon = fs::canonicalize(exec_path).ok();
    let exec_stem = exec_path.file_stem().and_then(OsStr::to_str).unwrap_or("").to_lowercase();
    let name_lower = app_name.to_lowercase();

    for dir in search_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|e| e == "desktop") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let mut entry_exec = String::new();
                        let mut entry_icon = None;
                        let mut entry_name = String::new();
                        let mut in_de = false;
                        for line in content.lines() {
                            let line = line.trim();
                            if line.starts_with('[') {
                                in_de = line == "[Desktop Entry]";
                                continue;
                            }
                            if !in_de {
                                continue;
                            }
                            if let Some(idx) = line.find('=') {
                                let key = line[..idx].trim();
                                let val = line[idx + 1..].trim();
                                match key {
                                    "Exec" => entry_exec = val.to_string(),
                                    "Icon" => entry_icon = Some(val.to_string()),
                                    "Name" => entry_name = val.to_string(),
                                    _ => {}
                                }
                            }
                        }

                        // 1. Direct match on Exec command path
                        if !entry_exec.is_empty() {
                            let clean_exec = entry_exec.replace(['"', '\''], "");
                            if clean_exec.contains(&*exec_str) {
                                return Some((path, entry_icon));
                            }
                            if let Some(token) = clean_exec.split_whitespace().next() {
                                let token_path = Path::new(token);
                                if token_path == exec_path || fs::canonicalize(token_path).ok() == exec_canon {
                                    return Some((path, entry_icon));
                                }
                                // If token points to an existing file that is DIFFERENT from exec_path,
                                // do NOT allow this desktop file to match via name or stem!
                                if token_path.is_file() {
                                    continue;
                                }
                            }
                        }

                        // 2. Exact Stem match in filename (e.g. universe-antigravity.desktop or antigravity.desktop)
                        let filename = path.file_name().and_then(OsStr::to_str).unwrap_or("").to_lowercase();
                        let stem = filename.strip_suffix(".desktop").unwrap_or(&filename);
                        let clean_stem = stem.strip_prefix("universe-").unwrap_or(stem);

                        if !exec_stem.is_empty() && (clean_stem == exec_stem || stem == exec_stem) {
                            return Some((path, entry_icon));
                        }

                        // 3. Exact Name match only if Exec does not point to another existing binary
                        if !name_lower.is_empty() && entry_name.to_lowercase() == name_lower {
                            return Some((path, entry_icon));
                        }
                    }
                }
            }
        }
    }
    None
}

#[allow(dead_code)]
pub fn ensure_desktop_launcher(
    exec_path: &Path,
    app_name: &str,
    icon_path: Option<&str>,
) -> Option<PathBuf> {
    create_or_update_desktop_launcher(exec_path, app_name, icon_path, false, None, None, None, None).ok()
}

pub fn create_or_update_desktop_launcher(
    exec_path: &Path,
    app_name: &str,
    icon_path: Option<&str>,
    terminal: bool,
    categories: Option<&str>,
    arguments: Option<&str>,
    startup_wm_class: Option<&str>,
    custom_desktop_path: Option<&Path>,
) -> Result<PathBuf> {
    let stem = exec_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("app")
        .to_ascii_lowercase();

    let desktop_file_path = if let Some(p) = custom_desktop_path {
        p.to_path_buf()
    } else {
        let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
            BackendError::new(ErrorKind::Io, "HOME environment variable not set")
        })?;
        let app_dir = home.join(".local/share/applications");
        let _ = fs::create_dir_all(&app_dir);

        let sanitized_stem: String = stem
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let desktop_filename = format!(
            "universe-{}.desktop",
            if sanitized_stem.is_empty() {
                "app"
            } else {
                &sanitized_stem
            }
        );
        app_dir.join(desktop_filename)
    };

    let icon_line = match icon_path {
        Some(icon) if !icon.trim().is_empty() => format!("Icon={icon}\n"),
        _ => String::new(),
    };

    let exec_args = arguments.unwrap_or("%U");
    let cats = categories.unwrap_or("Utility;Application;");

    let wm_class = if let Some(wm) = startup_wm_class.filter(|s| !s.trim().is_empty()) {
        wm.to_string()
    } else if stem == "antigravity-ide" || app_name.contains("Antigravity IDE") {
        "Antigravity IDE".to_string()
    } else {
        stem.clone()
    };

    let content = format!(
        "[Desktop Entry]\n\
Type=Application\n\
Name={app_name}\n\
Exec=\"{}\" {exec_args}\n\
{}Terminal={terminal}\n\
Categories={cats}\n\
StartupWMClass={wm_class}\n\
X-Integrated-By=universe-manager\n",
        exec_path.to_string_lossy(),
        icon_line
    );

    fs::write(&desktop_file_path, content)
        .map_err(|e| BackendError::from_io("cannot write desktop entry", &desktop_file_path, e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&desktop_file_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&desktop_file_path, perms);
        }
        if let Some(parent) = desktop_file_path.parent() {
            let _ = std::process::Command::new("update-desktop-database")
                .arg(parent)
                .output();
        }
    }

    Ok(desktop_file_path)
}

pub fn remove_desktop_launcher(desktop_path: &Path) -> Result<()> {
    if desktop_path.exists() {
        fs::remove_file(desktop_path)
            .map_err(|e| BackendError::from_io("cannot remove desktop file", desktop_path, e))?;
        #[cfg(unix)]
        {
            if let Some(parent) = desktop_path.parent() {
                let _ = std::process::Command::new("update-desktop-database")
                    .arg(parent)
                    .output();
            }
        }
    }
    Ok(())
}

pub fn list_executables_in_dir(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    if !dir.is_dir() {
        if dir.is_file() && is_executable(dir) {
            results.push(dir.to_path_buf());
        }
        return results;
    }

    fn walk_execs(curr: &Path, depth: usize, results: &mut Vec<PathBuf>) {
        if depth > 3 {
            return;
        }
        if let Ok(entries) = fs::read_dir(curr) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_symlink() {
                    continue;
                }
                if p.is_dir() {
                    walk_execs(&p, depth + 1, results);
                } else if p.is_file() && is_executable(&p) && !is_helper(&p) {
                    results.push(p);
                }
            }
        }
    }

    walk_execs(dir, 0, &mut results);

    let root_stem = dir.file_name().and_then(OsStr::to_str).unwrap_or("").to_ascii_lowercase();
    results.sort_by(|a, b| {
        let a_stem = a.file_stem().and_then(OsStr::to_str).unwrap_or("").to_ascii_lowercase();
        let b_stem = b.file_stem().and_then(OsStr::to_str).unwrap_or("").to_ascii_lowercase();
        let a_match = a_stem == root_stem;
        let b_match = b_stem == root_stem;
        let a_root = a.parent() == Some(dir);
        let b_root = b.parent() == Some(dir);
        b_match.cmp(&a_match).then_with(|| b_root.cmp(&a_root)).then_with(|| a.cmp(b))
    });

    results
}

pub fn relocate_app_dir(app: &AppEntry, target_parent_dir: &Path) -> Result<AppEntry> {
    let src = Path::new(&app.install_path);
    if !src.exists() {
        return Err(BackendError::new(
            ErrorKind::NotFound,
            format!("Thư mục cài đặt nguồn không tồn tại: {}", app.install_path),
        ));
    }

    let src_canonical = fs::canonicalize(src)
        .map_err(|e| BackendError::from_io("cannot canonicalize source", src, e))?;
    let target_canonical = fs::canonicalize(target_parent_dir)
        .unwrap_or_else(|_| target_parent_dir.to_path_buf());

    if src_canonical.starts_with(&target_canonical) {
        return Err(BackendError::new(
            ErrorKind::Validation,
            "Ứng dụng đã nằm trong thư mục quản lý tập trung (~/Applications)",
        ));
    }

    let file_or_folder_name = src
        .file_name()
        .ok_or_else(|| BackendError::new(ErrorKind::InvalidArgument, "Invalid source path"))?;
    let dest = target_canonical.join(file_or_folder_name);

    if dest.exists() {
        return Err(BackendError::new(
            ErrorKind::Conflict,
            format!(
                "Thư mục/tệp đích đã tồn tại trong thư mục quản lý: {}",
                dest.display()
            ),
        ));
    }

    let _ = fs::create_dir_all(&target_canonical);

    let move_success = fs::rename(&src, &dest).is_ok();
    if !move_success {
        if src.is_dir() {
            copy_dir_recursive(&src, &dest)
                .map_err(|e| BackendError::from_io("cannot copy directory", &src, e))?;
            let _ = fs::remove_dir_all(&src);
        } else {
            fs::copy(&src, &dest)
                .map_err(|e| BackendError::from_io("cannot copy file", &src, e))?;
            let _ = fs::remove_file(&src);
        }
    }

    let old_prefix = src.to_string_lossy().into_owned();
    let new_prefix = dest.to_string_lossy().into_owned();

    let new_exec_path = app.exec_path.replace(&old_prefix, &new_prefix);
    let new_icon_path = app.icon_path.as_ref().map(|ic| ic.replace(&old_prefix, &new_prefix));

    if !app.desktop_file.trim().is_empty() {
        let dpath = Path::new(&app.desktop_file);
        if dpath.is_file() {
            if let Ok(content) = fs::read_to_string(dpath) {
                let updated_content = content
                    .replace(&app.exec_path, &new_exec_path)
                    .replace(&old_prefix, &new_prefix);
                let _ = fs::write(dpath, updated_content);
                #[cfg(unix)]
                {
                    if let Some(parent) = dpath.parent() {
                        let _ = std::process::Command::new("update-desktop-database")
                            .arg(parent)
                            .output();
                    }
                }
            }
        }
    }

    if let Some(symlink_file) = &app.symlink_file {
        let sym_path = Path::new(symlink_file);
        if sym_path.is_symlink() || sym_path.exists() {
            let _ = fs::remove_file(sym_path);
            #[cfg(unix)]
            {
                let _ = std::os::unix::fs::symlink(&new_exec_path, sym_path);
            }
        }
    }

    let mut updated_app = app.clone();
    updated_app.install_path = new_prefix;
    updated_app.exec_path = new_exec_path;
    updated_app.icon_path = new_icon_path;
    updated_app.install_type = InstallType::Moved;
    updated_app.package_type = Some("Local".to_string());
    if !updated_app.inventory_sources.iter().any(|s| s == "Applications") {
        updated_app.inventory_sources.push("Applications".to_string());
    }

    Ok(updated_app)
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if ft.is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}


pub fn scan_cli_applications(
    cancellation: &CancellationToken,
    mut progress: impl FnMut(Progress),
) -> Result<Vec<AppEntry>> {
    let mut apps = Vec::new();
    let home = match std::env::var_os("HOME").map(PathBuf::from) {
        Some(h) => h,
        None => return Ok(apps),
    };

    let mut search_dirs = Vec::new();
    search_dirs.push(home.join(".local/bin"));
    search_dirs.push(home.join(".cargo/bin"));
    search_dirs.push(home.join(".grok/bin"));
    search_dirs.push(home.join(".gemini/antigravity-ide/bin"));
    search_dirs.push(home.join(".gemini/bin"));
    search_dirs.push(home.join(".filen-cli/bin"));
    search_dirs.push(home.join(".opencode/bin"));
    search_dirs.push(home.join("go/bin"));

    // Also parse user directories from PATH
    if let Some(path_var) = std::env::var_os("PATH") {
        for p in std::env::split_paths(&path_var) {
            if p.starts_with(&home) && !search_dirs.contains(&p) {
                search_dirs.push(p);
            }
        }
    }

    let mut seen_names = std::collections::HashSet::new();
    let mut seen_targets = std::collections::HashSet::new();

    for bin_dir in search_dirs {
        cancellation.check()?;
        if !bin_dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                cancellation.check()?;
                let path = entry.path();
                let file_name = match path.file_name().and_then(OsStr::to_str) {
                    Some(name) => name,
                    None => continue,
                };
                if file_name.starts_with('.') {
                    continue;
                }
                // Skip common non-binary helpers or library extensions
                if file_name.ends_with(".py")
                    || file_name.ends_with(".config")
                    || file_name.ends_with(".js")
                    || file_name.ends_with(".d.ts")
                    || file_name.contains(".so")
                    || file_name.ends_with(".png")
                    || file_name.ends_with(".desktop")
                {
                    continue;
                }

                #[cfg(unix)]
                let is_exec = fs::metadata(&path)
                    .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
                #[cfg(not(unix))]
                let is_exec = true;

                if is_exec {
                    let canon_str = fs::canonicalize(&path)
                        .map(|c| c.to_string_lossy().into_owned())
                        .unwrap_or_else(|_| path.to_string_lossy().into_owned());

                    if seen_names.contains(file_name) || seen_targets.contains(&canon_str) {
                        continue;
                    }
                    seen_names.insert(file_name.to_string());
                    seen_targets.insert(canon_str.clone());

                    let name = file_name.to_string();
                    let source_label = format!("PATH ({})", bin_dir.to_string_lossy());
                    let resolved_target = if canon_str != path.to_string_lossy() {
                        Some(canon_str.clone())
                    } else {
                        None
                    };

                    apps.push(AppEntry {
                        id: format!("cli-{}", name),
                        name: format!("{} (CLI)", name),
                        install_type: InstallType::InPlace,
                        source_path: resolved_target,
                        install_path: bin_dir.to_string_lossy().into_owned(),
                        exec_path: path.to_string_lossy().into_owned(),
                        icon_path: None,
                        desktop_file: String::new(),
                        symlink_file: Some(path.to_string_lossy().into_owned()),
                        added_at: String::new(),
                        is_custom: Some(true),
                        start_cmd: Some(name.clone()),
                        stop_cmd: None,
                        category: Some("Development".to_string()),
                        package_type: Some("CLI".to_string()),
                        inventory_sources: vec![source_label],
                        status: None,
                        ..AppEntry::default()
                    });
                }
            }
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    progress(Progress {
        completed: apps.len() as u64,
        total: Some(apps.len() as u64),
        message: format!("Discovered {} CLI tools in PATH and user bin folders", apps.len()),
    });
    Ok(apps)
}

pub fn scan_all_applications(
    config: &ManagerConfig,
    managed_dir_opt: Option<&Path>,
    cancellation: &CancellationToken,
    mut progress: impl FnMut(Progress),
) -> Result<Vec<AppEntry>> {
    let mut all_apps = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_execs = std::collections::HashSet::new();

    // 1. Scan portable / managed apps if directory is supplied and exists
    if let Some(managed_dir) = managed_dir_opt {
        if managed_dir.is_dir() {
            if let Ok(service) = DiscoveryService::new(vec![managed_dir.to_path_buf()]) {
                if let Ok(portable_apps) = service.scan_managed(config, managed_dir, cancellation, &mut progress) {
                    for app in portable_apps {
                        if seen_ids.insert(app.id.clone()) {
                            if !app.exec_path.is_empty() {
                                seen_execs.insert(app.exec_path.clone());
                                if let Some(token) = app.exec_path.split_whitespace().next() {
                                    seen_execs.insert(token.to_string());
                                    if let Ok(canon) = fs::canonicalize(token) {
                                        seen_execs.insert(canon.to_string_lossy().into_owned());
                                    }
                                }
                                if let Ok(canon) = fs::canonicalize(&app.exec_path) {
                                    seen_execs.insert(canon.to_string_lossy().into_owned());
                                }
                            }
                            all_apps.push(app);
                        }
                    }
                }
            }
        }
    }

    // 2. Scan system applications (Flatpak, Snap, System Desktop)
    let system_apps = scan_system_applications(cancellation, &mut progress)?;
    for app in system_apps {
        if seen_ids.insert(app.id.clone()) {
            if !app.exec_path.is_empty() {
                seen_execs.insert(app.exec_path.clone());
                if let Some(token) = app.exec_path.split_whitespace().next() {
                    seen_execs.insert(token.to_string());
                    if let Ok(canon) = fs::canonicalize(token) {
                        seen_execs.insert(canon.to_string_lossy().into_owned());
                    }
                }
                if let Ok(canon) = fs::canonicalize(&app.exec_path) {
                    seen_execs.insert(canon.to_string_lossy().into_owned());
                }
            }
            all_apps.push(app);
        }
    }

    // 3. Scan user CLI / PATH applications (~/.local/bin)
    let cli_apps = scan_cli_applications(cancellation, &mut progress)?;
    for app in cli_apps {
        let cli_canon = fs::canonicalize(&app.exec_path).ok();
        let is_known_exec = !app.exec_path.is_empty()
            && (seen_execs.contains(&app.exec_path)
                || cli_canon
                    .as_ref()
                    .map(|c| seen_execs.contains(&c.to_string_lossy().into_owned()))
                    .unwrap_or(false));

        if is_known_exec {
            // MERGE: Find existing app and record symlink / CLI command
            for existing in &mut all_apps {
                let first_token = existing.exec_path.split_whitespace().next().unwrap_or("");
                let existing_canon = fs::canonicalize(first_token).ok();
                let is_match = first_token == app.exec_path
                    || existing.exec_path == app.exec_path
                    || app.source_path.as_deref() == Some(first_token)
                    || (cli_canon.is_some() && (existing_canon == cli_canon || cli_canon.as_ref().map(|c| c.to_string_lossy().into_owned()) == Some(first_token.to_string())));

                if is_match {
                    if existing.symlink_file.is_none() {
                        existing.symlink_file = app.symlink_file.clone().or_else(|| Some(app.exec_path.clone()));
                    }
                    if existing.start_cmd.is_none() {
                        existing.start_cmd = app.start_cmd.clone();
                    }
                    if !existing.inventory_sources.iter().any(|s| s.contains("CLI") || s.contains("PATH")) {
                        existing.inventory_sources.push("CLI".to_string());
                    }
                    break;
                }
            }
        } else if seen_ids.insert(app.id.clone()) {
            all_apps.push(app);
        }
    }

    Ok(all_apps)
}

