use crate::config::{AppEntry, AppStatus, Config};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn parse_desktop_file(path: &Path) -> Result<AppEntry, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Không thể đọc file: {}", e))?;

    parse_desktop_content(path, &content)
}

fn parse_desktop_content(path: &Path, content: &str) -> Result<AppEntry, String> {
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let id = filename.strip_suffix(".desktop").unwrap_or(&filename).to_string();

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
                "Name" => {
                    if name.is_empty() {
                        name = val.to_string();
                    }
                }
                "Exec" => {
                    exec = val.to_string();
                }
                "Categories" => {
                    categories_str = val.to_string();
                }
                "Icon" => {
                    icon = Some(val.to_string());
                }
                "NoDisplay" => {
                    if val.to_lowercase() == "true" {
                        no_display = true;
                    }
                }
                "Type" if val.to_lowercase() == "application" => {
                    is_application = true;
                }
                _ => {}
            }
        }
    }

    if !is_application || no_display || name.is_empty() {
        return Err("Không phải ứng dụng hiển thị được".to_string());
    }

    // Clean Exec path (remove parameters like %u, %U, %f, %F)
    let clean_exec = exec
        .split_whitespace()
        .filter(|part| !part.starts_with('%'))
        .collect::<Vec<&str>>()
        .join(" ")
        .replace(['"', '\''], "");

    // Resolve primary category
    let mut category = "Other".to_string();
    if !categories_str.is_empty() {
        let list: Vec<&str> = categories_str
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let main_categories = [
            "Network",
            "Internet",
            "Development",
            "Office",
            "Graphics",
            "AudioVideo",
            "Audio",
            "Video",
            "Multimedia",
            "Game",
            "System",
            "Utility",
            "Accessories",
            "Settings",
        ];

        let mut found = false;
        for cat in &list {
            for main_cat in &main_categories {
                if cat.to_lowercase() == main_cat.to_lowercase() {
                    category = main_cat.to_string();
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }

        if !found && !list.is_empty() {
            category = list[0].to_string();
        }
    }

    // Determine packaging type
    let path_str = path.to_string_lossy().to_string();
    let package_type = if path_str.contains("flatpak") {
        "Flatpak".to_string()
    } else if path_str.contains("snap") {
        "Snap".to_string()
    } else {
        "APT".to_string()
    };

    let real_id = id.clone();
    let mut final_id = id;
    if package_type == "Flatpak" {
        final_id = format!("{}-flatpak", real_id);
    } else if package_type == "Snap" {
        final_id = format!("{}-snap", real_id);
    }
    let inventory_source = if path_str.contains("flatpak") {
        "Flatpak"
    } else if path_str.contains("snap") {
        "Snap"
    } else if path_str.contains("/.local/share/applications") {
        "Desktop (user)"
    } else {
        "Desktop (system)"
    };

    Ok(AppEntry {
        id: final_id,
        name,
        install_type: crate::config::InstallType::InPlace,
        source_path: None,
        install_path: path.parent().unwrap_or(Path::new("")).to_string_lossy().to_string(),
        exec_path: clean_exec,
        icon_path: icon,
        desktop_file: path_str,
        symlink_file: None,
        added_at: "".to_string(),
        is_custom: Some(package_type == "Flatpak" || package_type == "Snap"),
        start_cmd: if package_type == "Flatpak" {
            Some(format!("flatpak run {}", real_id))
        } else {
            None
        },
        stop_cmd: if package_type == "Flatpak" {
            Some(format!("flatpak kill {}", real_id))
        } else {
            None
        },
        category: Some(category),
        package_type: Some(package_type),
        inventory_sources: vec![inventory_source.to_string()],
        ..Default::default()
    })
}

fn scan_managed_applications(managed_dir: &Path) -> Result<Vec<AppEntry>, SourceScanError> {
    let entries = fs::read_dir(managed_dir).map_err(|error| SourceScanError {
        source: "Applications".to_string(),
        message: error.to_string(),
    })?;
    let mut apps = Vec::new();
    let mut seen_ids = HashSet::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let app_id = folder_name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
            .replace(' ', "-");
        if seen_ids.contains(&app_id) {
            continue;
        }
        if let Ok(det) = crate::detector::detect(&path)
            && let Some(executable) = det.executables.first()
        {
            seen_ids.insert(app_id.clone());
            apps.push(AppEntry {
                id: app_id,
                name: det.suggested_name,
                install_type: crate::config::InstallType::Moved,
                install_path: path.to_string_lossy().into_owned(),
                exec_path: executable.to_string_lossy().into_owned(),
                icon_path: det.icons.first().map(|icon| icon.to_string_lossy().into_owned()),
                is_custom: Some(false),
                category: Some("Utility".to_string()),
                package_type: Some("Local".to_string()),
                inventory_sources: vec!["Applications".to_string()],
                ..Default::default()
            });
        }
    }
    Ok(apps)
}

fn parse_homebrew_formulae(output: &str) -> Vec<AppEntry> {
    output
        .lines()
        .map(str::trim)
        .filter(|package| !package.is_empty())
        .map(|package| AppEntry {
            id: package.to_string(),
            name: package
                .chars()
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + &package[first.len_utf8()..])
                .unwrap_or_default(),
            install_type: crate::config::InstallType::InPlace,
            install_path: "Homebrew Cellar".to_string(),
            exec_path: "brew".to_string(),
            is_custom: Some(true),
            category: Some("System".to_string()),
            package_type: Some("Homebrew".to_string()),
            inventory_sources: vec!["Homebrew".to_string()],
            ..Default::default()
        })
        .collect()
}

#[cfg(any(windows, test))]
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WinRegApp {
    #[serde(rename = "PSPath")]
    ps_path: Option<String>,
    #[serde(rename = "PSChildName")]
    ps_child_name: Option<String>,
    display_name: Option<String>,
    display_version: Option<String>,
    publisher: Option<String>,
    uninstall_string: Option<String>,
    install_location: Option<String>,
    help_link: Option<String>,
    url_info_about: Option<String>,
    system_component: Option<serde_json::Value>,
    parent_key_name: Option<String>,
}

#[cfg(any(windows, test))]
fn parse_windows_registry_json(output: &str) -> Result<Vec<AppEntry>, serde_json::Error> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let json = if trimmed.starts_with('{') {
        format!("[{trimmed}]")
    } else {
        trimmed.to_string()
    };
    let registry_apps = serde_json::from_str::<Vec<WinRegApp>>(&json)?;
    Ok(registry_apps
        .into_iter()
        .filter_map(|app| {
            let name = app.display_name?.trim().to_string();
            if name.is_empty()
                || app.system_component.as_ref().is_some_and(|value| match value {
                    serde_json::Value::Number(number) => number.as_u64() == Some(1),
                    serde_json::Value::String(text) => text.trim() == "1",
                    _ => false,
                })
                || app
                    .parent_key_name
                    .as_ref()
                    .is_some_and(|parent| !parent.trim().is_empty())
            {
                return None;
            }
            let ps_path = app.ps_path.as_deref().unwrap_or("");
            let hive = if ps_path.contains("HKEY_LOCAL_MACHINE") {
                "Machine"
            } else {
                "User"
            };
            let arch = if ps_path.contains("Wow6432Node") { "X86" } else { "X64" };
            let key_name = app.ps_child_name.as_deref().unwrap_or("");
            if key_name.is_empty() {
                return None;
            }
            let registry_key = ps_path
                .find("HKEY_")
                .map(|index| ps_path[index..].to_string())
                .unwrap_or_else(|| ps_path.to_string());
            let uninstall_cmd = app.uninstall_string.map(|value| value.trim().to_string());
            Some(AppEntry {
                id: format!("ARP\\{hive}\\{arch}\\{key_name}"),
                name,
                install_type: crate::config::InstallType::InPlace,
                source_path: Some(registry_key.clone()),
                install_path: app.install_location.unwrap_or_default().trim().to_string(),
                added_at: format!("Registry ({hive} {arch})"),
                is_custom: Some(true),
                start_cmd: uninstall_cmd.clone(),
                category: Some("System".to_string()),
                package_type: Some("Registry".to_string()),
                inventory_sources: vec!["Registry".to_string()],
                registry_key: Some(registry_key),
                product_code: (key_name.starts_with('{') && key_name.ends_with('}')).then(|| key_name.to_string()),
                about_url: app
                    .help_link
                    .filter(|value| !value.trim().is_empty())
                    .or(app.url_info_about)
                    .map(|value| value.trim().to_string()),
                publisher: app.publisher.map(|value| value.trim().to_string()),
                version: app.display_version.map(|value| value.trim().to_string()),
                uninstall_cmd,
                ..Default::default()
            })
        })
        .collect())
}

#[cfg(windows)]
fn registry_entries_from_output(
    output: Result<std::process::Output, std::io::Error>,
) -> Result<Vec<AppEntry>, SourceScanError> {
    let output = output.map_err(|error| SourceScanError {
        source: "Registry".to_string(),
        message: error.to_string(),
    })?;
    if !output.status.success() {
        return Err(SourceScanError {
            source: "Registry".to_string(),
            message: format!("PowerShell Registry scan exited with {}", output.status),
        });
    }
    parse_windows_registry_json(&String::from_utf8_lossy(&output.stdout)).map_err(|error| SourceScanError {
        source: "Registry".to_string(),
        message: format!("invalid PowerShell JSON: {error}"),
    })
}

#[cfg(any(windows, test))]
fn parse_winget_list(output: &str) -> Vec<AppEntry> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("Name") && !line.starts_with('-'))
        .filter_map(|line| {
            let columns: Vec<&str> = line
                .split("  ")
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .collect();
            let (name, id) = (*columns.first()?, *columns.get(1)?);
            let is_msix = id.to_lowercase().starts_with("msix\\");
            Some(AppEntry {
                id: id.to_string(),
                name: name.to_string(),
                install_type: crate::config::InstallType::InPlace,
                install_path: if is_msix { "Windows Store" } else { "Windows winget" }.to_string(),
                exec_path: "winget".to_string(),
                is_custom: Some(true),
                category: Some(if is_msix { "Store" } else { "System" }.to_string()),
                package_type: Some(if is_msix { "MSIX" } else { "Winget" }.to_string()),
                inventory_sources: vec!["Winget".to_string()],
                ..Default::default()
            })
        })
        .collect()
}

fn source_name(app: &AppEntry) -> String {
    match app.package_type.as_deref() {
        Some("MSIX" | "Winget") => "Winget".to_string(),
        Some("Local") => "Applications".to_string(),
        Some("APT") if app.desktop_file.contains("/.local/share/applications") => "Desktop (user)".to_string(),
        Some("APT") => "Desktop (system)".to_string(),
        Some(package_type) => package_type.to_string(),
        None => "Local".to_string(),
    }
}

fn normalize_identity_part(value: &str) -> String {
    value.trim().to_lowercase()
}

fn normalized_path(app: &AppEntry) -> Option<String> {
    [&app.install_path, &app.exec_path]
        .into_iter()
        .find(|path| Path::new(path).is_absolute())
        .map(|path| path.replace('\\', "/").trim_end_matches('/').to_lowercase())
}

/// Identity supplied by an inventory backend. Canonical IDs remain namespaced;
/// aliases only remove decorations belonging to that same backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoryIdentity {
    pub canonical: String,
    pub aliases: Vec<String>,
}

pub fn inventory_identity(app: &AppEntry) -> InventoryIdentity {
    let backend = app.package_type.as_deref().unwrap_or("Local").to_lowercase();
    let raw_id = app.id.trim();
    let backend_id = match backend.as_str() {
        "flatpak" => raw_id.strip_suffix("-flatpak").unwrap_or(raw_id),
        "snap" => raw_id.strip_suffix("-snap").unwrap_or(raw_id),
        "msix" => raw_id
            .strip_prefix("MSIX\\")
            .or_else(|| raw_id.strip_prefix("msix\\"))
            .unwrap_or(raw_id)
            .trim_start_matches('\\'),
        _ => raw_id,
    };
    let normalized_id = normalize_identity_part(backend_id);
    let canonical = if backend == "registry" {
        app.product_code
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("registry:product-code:{}", normalize_identity_part(value)))
            .unwrap_or_else(|| format!("registry:{normalized_id}"))
    } else if backend == "local" {
        normalized_path(app)
            .map(|path| format!("local:path:{path}"))
            .unwrap_or_else(|| format!("local:{normalized_id}"))
    } else {
        format!("{backend}:{normalized_id}")
    };

    let mut aliases = vec![canonical.clone()];
    let name = normalize_identity_part(&app.name);
    if !name.is_empty() {
        if let Some(publisher) = app
            .publisher
            .as_deref()
            .map(normalize_identity_part)
            .filter(|value| !value.is_empty())
        {
            aliases.push(format!("metadata:name:{name}:publisher:{publisher}"));
        }
        if let Some(path) = normalized_path(app) {
            aliases.push(format!("metadata:name:{name}:path:{path}"));
        }
    }
    aliases.sort();
    aliases.dedup();
    InventoryIdentity { canonical, aliases }
}

fn entry_quality(app: &AppEntry) -> usize {
    usize::from(Path::new(&app.exec_path).is_file()) * 8
        + usize::from(Path::new(&app.desktop_file).is_file()) * 4
        + usize::from(app.package_type.as_deref() == Some("Local")) * 2
        + usize::from(app.desktop_file.contains("/.local/share/applications"))
}

fn merge_entry(existing: &mut AppEntry, mut incoming: AppEntry) {
    if existing.inventory_sources.is_empty() {
        existing.inventory_sources.push(source_name(existing));
    }
    if incoming.inventory_sources.is_empty() {
        incoming.inventory_sources.push(source_name(&incoming));
    }
    for source in incoming.inventory_sources.iter().cloned() {
        if !existing.inventory_sources.contains(&source) {
            existing.inventory_sources.push(source);
        }
    }
    existing.inventory_sources.sort();

    if entry_quality(&incoming) > entry_quality(existing) {
        let sources = std::mem::take(&mut existing.inventory_sources);
        *existing = incoming;
        existing.inventory_sources = sources;
    } else {
        if existing.icon_path.is_none() {
            existing.icon_path = incoming.icon_path;
        }
        if existing.category.is_none() {
            existing.category = incoming.category;
        }
        if existing.version.is_none() {
            existing.version = incoming.version;
        }
        if existing.publisher.is_none() {
            existing.publisher = incoming.publisher;
        }
    }
}

/// Produces one stable record per application while retaining every reporting source.
pub fn merge_inventory(entries: Vec<AppEntry>) -> Vec<AppEntry> {
    struct MergedEntry {
        app: AppEntry,
        aliases: HashSet<String>,
    }

    let mut merged = Vec::<MergedEntry>::new();
    for mut entry in entries {
        if entry.inventory_sources.is_empty() {
            entry.inventory_sources.push(source_name(&entry));
        }
        let aliases: HashSet<String> = inventory_identity(&entry).aliases.into_iter().collect();
        let matching: Vec<usize> = merged
            .iter()
            .enumerate()
            .filter_map(|(index, existing)| (!existing.aliases.is_disjoint(&aliases)).then_some(index))
            .collect();
        if let Some((&target, rest)) = matching.split_first() {
            merge_entry(&mut merged[target].app, entry);
            merged[target].aliases.extend(aliases);
            for &index in rest.iter().rev() {
                let duplicate = merged.remove(index);
                merge_entry(&mut merged[target].app, duplicate.app);
                merged[target].aliases.extend(duplicate.aliases);
            }
        } else {
            merged.push(MergedEntry { app: entry, aliases });
        }
    }
    let mut apps: Vec<AppEntry> = merged.into_iter().map(|entry| entry.app).collect();
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceScanError {
    pub source: String,
    pub message: String,
}

#[derive(Debug)]
pub struct InventoryScanReport {
    pub entries: Vec<AppEntry>,
    pub source_errors: Vec<SourceScanError>,
}

#[derive(Debug)]
pub struct RescanReport {
    pub records: Vec<(AppEntry, AppStatus)>,
    pub discovered: usize,
    pub retained_broken: usize,
    pub retained_unverified: usize,
    pub merged_collisions: usize,
    pub source_errors: Vec<SourceScanError>,
}

pub fn reconcile_inventory_report(previous: &[(AppEntry, AppStatus)], scan: InventoryScanReport) -> RescanReport {
    let InventoryScanReport { entries, source_errors } = scan;
    let input_count = entries.len();
    let current = merge_inventory(entries);
    let merged_collisions = input_count.saturating_sub(current.len());
    let discovered = current.len();
    let current_aliases: HashSet<String> = current
        .iter()
        .flat_map(|entry| inventory_identity(entry).aliases)
        .collect();
    let failed_sources: HashSet<&str> = source_errors.iter().map(|error| error.source.as_str()).collect();
    let mut records: Vec<(AppEntry, AppStatus)> = current
        .into_iter()
        .map(|entry| {
            let status = entry.check_status();
            (entry, status)
        })
        .collect();
    let mut retained_broken = 0;
    let mut retained_unverified = 0;

    for (entry, old_status) in previous {
        let identity = inventory_identity(entry);
        if identity.aliases.iter().all(|alias| !current_aliases.contains(alias)) {
            let source_failed = if entry.inventory_sources.is_empty() {
                let source = source_name(entry);
                failed_sources.contains(source.as_str())
            } else {
                entry
                    .inventory_sources
                    .iter()
                    .any(|source| failed_sources.contains(source.as_str()))
            };
            if source_failed {
                records.push((entry.clone(), old_status.clone()));
                retained_unverified += 1;
                continue;
            }
            let mut issues = match old_status {
                AppStatus::Broken(issues) | AppStatus::Degraded(issues) => issues.clone(),
                AppStatus::Healthy => Vec::new(),
            };
            issues.push("Ứng dụng không còn được tìm thấy trong lần quét mới".to_string());
            records.push((entry.clone(), AppStatus::Broken(issues)));
            retained_broken += 1;
        }
    }
    records.sort_by_key(|(entry, _)| entry.name.to_lowercase());
    RescanReport {
        records,
        discovered,
        retained_broken,
        retained_unverified,
        merged_collisions,
        source_errors,
    }
}

pub fn rescan_applications(previous: &[(AppEntry, AppStatus)]) -> Result<RescanReport, String> {
    Ok(reconcile_inventory_report(previous, scan_all_system_apps_report()))
}

/// Scans all applications installed on the system (APT, Flatpak, Snap, Local, Homebrew, Winget, Scoop, Chocolatey).
pub fn scan_all_system_apps() -> Vec<AppEntry> {
    merge_inventory(scan_all_system_apps_report().entries)
}

pub fn scan_all_system_apps_report() -> InventoryScanReport {
    let mut apps = Vec::new();
    let mut source_errors = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_names = std::collections::HashSet::new();

    let config = Config::load();
    for mut app in config.apps {
        app.package_type = Some("Local".to_string());
        if app.inventory_sources.is_empty() {
            app.inventory_sources.push("Applications".to_string());
        }
        if app.category.is_none() {
            app.category = Some("Utility".to_string());
        }
        seen_ids.insert(app.id.clone());
        seen_names.insert(app.name.to_lowercase());
        apps.push(app);
    }

    // 1.5 Scan managed_dir for Stateless Portable apps
    // De-duplicate only inside each source; cross-source collisions must reach merge_inventory.
    seen_ids.clear();
    seen_names.clear();
    let managed_dir = std::path::Path::new(&config.settings.managed_dir);
    if managed_dir.exists() && managed_dir.is_dir() {
        match scan_managed_applications(managed_dir) {
            Ok(entries) => apps.extend(entries),
            Err(error) => source_errors.push(error),
        }
    }

    // 2. Scan directories containing .desktop files
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let scan_dirs = vec![
        home.join(".local/share/applications"),
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        home.join(".local/share/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];

    for dir in scan_dirs {
        seen_ids.clear();
        seen_names.clear();
        if !dir.exists() || !dir.is_dir() {
            continue;
        }

        let source = if dir.to_string_lossy().contains("flatpak") {
            "Flatpak"
        } else if dir.to_string_lossy().contains("snap") {
            "Snap"
        } else if dir.to_string_lossy().contains("/.local/share/applications") {
            "Desktop (user)"
        } else {
            "Desktop (system)"
        };
        match fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && path.extension().map(|e| e == "desktop").unwrap_or(false)
                        && let Ok(app_entry) = parse_desktop_file(&path)
                        && !seen_ids.contains(&app_entry.id)
                    {
                        seen_ids.insert(app_entry.id.clone());
                        seen_names.insert(app_entry.name.to_lowercase());
                        apps.push(app_entry);
                    }
                }
            }
            Err(error) => source_errors.push(SourceScanError {
                source: source.to_string(),
                message: error.to_string(),
            }),
        }
    }

    // 3. Scan Homebrew (Linux/macOS)
    #[cfg(unix)]
    {
        if Command::new("brew")
            .arg("--version")
            .status()
            .is_ok_and(|status| status.success())
        {
            match Command::new("brew").args(["list", "--formula"]).output() {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    apps.extend(parse_homebrew_formulae(&stdout));
                }
                Ok(out) => source_errors.push(SourceScanError {
                    source: "Homebrew".to_string(),
                    message: format!("brew list exited with {}", out.status),
                }),
                Err(error) => source_errors.push(SourceScanError {
                    source: "Homebrew".to_string(),
                    message: error.to_string(),
                }),
            }
        }
    }

    // 4. Scan Registry, Winget, Scoop, Choco on Windows
    #[cfg(windows)]
    {
        // 4.1 Windows Registry uninstall keys scanning
        let script = "Get-ItemProperty -Path 'HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*', 'HKLM:\\Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*', 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' -ErrorAction SilentlyContinue | Select-Object PSPath, PSChildName, DisplayName, DisplayVersion, Publisher, UninstallString, InstallLocation, HelpLink, URLInfoAbout, SystemComponent, ParentKeyName | ConvertTo-Json -Compress";
        match registry_entries_from_output(
            Command::new("powershell")
                .args(["-NoProfile", "-Command", script])
                .output(),
        ) {
            Ok(entries) => apps.extend(entries),
            Err(error) => source_errors.push(error),
        }

        // winget
        seen_ids.clear();
        seen_names.clear();
        match Command::new("winget").arg("--version").output() {
            Ok(out) if out.status.success() => match Command::new("winget").args(&["list"]).output() {
                Ok(list_out) if list_out.status.success() => {
                    let stdout = String::from_utf8_lossy(&list_out.stdout);
                    apps.extend(parse_winget_list(&stdout));
                }
                Ok(out) => source_errors.push(SourceScanError {
                    source: "Winget".to_string(),
                    message: format!("winget list exited with {}", out.status),
                }),
                Err(error) => source_errors.push(SourceScanError {
                    source: "Winget".to_string(),
                    message: error.to_string(),
                }),
            },
            Ok(out) => source_errors.push(SourceScanError {
                source: "Winget".to_string(),
                message: format!("winget --version exited with {}", out.status),
            }),
            Err(error) => source_errors.push(SourceScanError {
                source: "Winget".to_string(),
                message: error.to_string(),
            }),
        }

        // scoop
        seen_ids.clear();
        seen_names.clear();
        match Command::new("scoop").arg("--version").status() {
            Ok(status) if status.success() => match Command::new("scoop").args(&["list"]).output() {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let mut in_apps = false;
                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("Installed apps:") {
                            in_apps = true;
                            continue;
                        }
                        if in_apps && !trimmed.is_empty() {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if !parts.is_empty() {
                                let app_name = parts[0];
                                let name_formatted =
                                    app_name.chars().next().unwrap().to_uppercase().collect::<String>()
                                        + &app_name[1..];
                                let name_lower = name_formatted.to_lowercase();
                                if !seen_ids.contains(app_name) && !seen_names.contains(&name_lower) {
                                    seen_ids.insert(app_name.to_string());
                                    seen_names.insert(name_lower);
                                    apps.push(AppEntry {
                                        id: app_name.to_string(),
                                        name: name_formatted,
                                        install_type: crate::config::InstallType::InPlace,
                                        source_path: None,
                                        install_path: "Scoop Apps".to_string(),
                                        exec_path: "scoop".to_string(),
                                        icon_path: None,
                                        desktop_file: "".to_string(),
                                        symlink_file: None,
                                        added_at: "".to_string(),
                                        is_custom: Some(true),
                                        start_cmd: None,
                                        stop_cmd: None,
                                        category: Some("System".to_string()),
                                        package_type: Some("Scoop".to_string()),
                                        inventory_sources: vec!["Scoop".to_string()],
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
                Ok(out) => source_errors.push(SourceScanError {
                    source: "Scoop".to_string(),
                    message: format!("scoop list exited with {}", out.status),
                }),
                Err(error) => source_errors.push(SourceScanError {
                    source: "Scoop".to_string(),
                    message: error.to_string(),
                }),
            },
            Ok(status) => source_errors.push(SourceScanError {
                source: "Scoop".to_string(),
                message: format!("scoop --version exited with {status}"),
            }),
            Err(error) => source_errors.push(SourceScanError {
                source: "Scoop".to_string(),
                message: error.to_string(),
            }),
        }

        // choco
        seen_ids.clear();
        seen_names.clear();
        match Command::new("choco").arg("--version").output() {
            Ok(ver_out) if ver_out.status.success() => {
                let ver_str = String::from_utf8_lossy(&ver_out.stdout);
                let is_v2 = ver_str.trim().starts_with('2') || ver_str.trim().starts_with('3');
                let choco_args = if is_v2 { vec!["list"] } else { vec!["list", "-lo"] };

                match Command::new("choco").args(&choco_args).output() {
                    Ok(out) if out.status.success() => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        for line in stdout.lines() {
                            let trimmed = line.trim();
                            if trimmed.is_empty()
                                || trimmed.to_lowercase().starts_with("chocolatey v")
                                || trimmed.to_lowercase().contains("packages installed")
                            {
                                continue;
                            }

                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 2 {
                                if !parts[1].chars().next().map_or(false, |c| c.is_ascii_digit()) {
                                    continue;
                                }

                                let app_name = parts[0];
                                let name_formatted =
                                    app_name.chars().next().unwrap().to_uppercase().collect::<String>()
                                        + &app_name[1..];
                                let name_lower = name_formatted.to_lowercase();
                                if !seen_ids.contains(app_name) && !seen_names.contains(&name_lower) {
                                    seen_ids.insert(app_name.to_string());
                                    seen_names.insert(name_lower);
                                    apps.push(AppEntry {
                                        id: app_name.to_string(),
                                        name: name_formatted,
                                        install_type: crate::config::InstallType::InPlace,
                                        source_path: None,
                                        install_path: "Chocolatey lib".to_string(),
                                        exec_path: "choco".to_string(),
                                        icon_path: None,
                                        desktop_file: "".to_string(),
                                        symlink_file: None,
                                        added_at: "".to_string(),
                                        is_custom: Some(true),
                                        start_cmd: None,
                                        stop_cmd: None,
                                        category: Some("System".to_string()),
                                        package_type: Some("Chocolatey".to_string()),
                                        inventory_sources: vec!["Chocolatey".to_string()],
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                    Ok(out) => source_errors.push(SourceScanError {
                        source: "Chocolatey".to_string(),
                        message: format!("choco list exited with {}", out.status),
                    }),
                    Err(error) => source_errors.push(SourceScanError {
                        source: "Chocolatey".to_string(),
                        message: error.to_string(),
                    }),
                }
            }
            Ok(out) => source_errors.push(SourceScanError {
                source: "Chocolatey".to_string(),
                message: format!("choco --version exited with {}", out.status),
            }),
            Err(error) => source_errors.push(SourceScanError {
                source: "Chocolatey".to_string(),
                message: error.to_string(),
            }),
        }
    }

    InventoryScanReport {
        entries: apps,
        source_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InstallType;
    use std::collections::BTreeSet;

    fn app(id: &str, name: &str, package_type: &str, source: &str, root: &Path) -> AppEntry {
        AppEntry {
            id: id.to_string(),
            name: name.to_string(),
            install_type: InstallType::InPlace,
            install_path: root.to_string_lossy().into_owned(),
            exec_path: root.join("missing-exec").to_string_lossy().into_owned(),
            desktop_file: root.join("missing.desktop").to_string_lossy().into_owned(),
            package_type: Some(package_type.to_string()),
            inventory_sources: vec![source.to_string()],
            ..Default::default()
        }
    }

    fn snapshot(root: &Path) -> BTreeSet<PathBuf> {
        walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .map(|entry| entry.path().strip_prefix(root).unwrap_or(entry.path()).to_path_buf())
            .collect()
    }

    #[test]
    fn merges_same_desktop_id_reported_by_user_and_system_scanners() {
        let temp = tempfile::tempdir().expect("tempdir");
        let user = app(
            "org.example.Editor",
            "Editor",
            "APT",
            "Desktop (user)",
            &temp.path().join("user"),
        );
        let system = app(
            "org.example.Editor",
            "Editor",
            "APT",
            "Desktop (system)",
            &temp.path().join("system"),
        );
        let merged = merge_inventory(vec![user, system]);
        assert_eq!(merged.len(), 1);
        for source in ["Desktop (user)", "Desktop (system)"] {
            assert!(merged[0].inventory_sources.iter().any(|value| value == source));
        }
    }

    #[test]
    fn does_not_merge_same_id_or_name_without_matching_fallback_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let flatpak = app(
            "code-flatpak",
            "Code",
            "Flatpak",
            "Flatpak",
            &temp.path().join("flatpak"),
        );
        let snap = app("code-snap", "Code", "Snap", "Snap", &temp.path().join("snap"));
        let homebrew = app(
            "code",
            "Different App",
            "Homebrew",
            "Homebrew",
            &temp.path().join("brew"),
        );
        assert_eq!(merge_inventory(vec![flatpak, snap, homebrew]).len(), 3);
    }

    #[test]
    fn canonical_ids_preserve_separators() {
        let temp = tempfile::tempdir().expect("tempdir");
        let separated = app("a-b", "A-B", "Scoop", "Scoop", &temp.path().join("a-b"));
        let compact = app("ab", "AB", "Scoop", "Scoop", &temp.path().join("ab"));
        assert_ne!(
            inventory_identity(&separated).canonical,
            inventory_identity(&compact).canonical
        );
        assert_eq!(merge_inventory(vec![separated, compact]).len(), 2);
    }

    #[test]
    fn backend_aliases_merge_decorated_ids_only_within_backend() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first = app(
            "org.example.Editor-flatpak",
            "Editor",
            "Flatpak",
            "Flatpak",
            temp.path(),
        );
        let second = app(
            "org.example.Editor",
            "Editor Renamed",
            "Flatpak",
            "Flatpak",
            temp.path(),
        );
        assert_eq!(merge_inventory(vec![first, second]).len(), 1);
    }

    #[test]
    fn parses_realistic_desktop_flatpak_and_snap_launchers_without_cross_backend_collision() {
        let desktop = parse_desktop_content(
            Path::new("/usr/share/applications/org.example.Editor.desktop"),
            include_str!("../tests/fixtures/editor.desktop"),
        )
        .expect("desktop fixture");
        let flatpak = parse_desktop_content(
            Path::new("/var/lib/flatpak/exports/share/applications/org.example.Editor.desktop"),
            include_str!("../tests/fixtures/flatpak-editor.desktop"),
        )
        .expect("flatpak fixture");
        let snap = parse_desktop_content(
            Path::new("/var/lib/snapd/desktop/applications/org.example.Editor.desktop"),
            include_str!("../tests/fixtures/snap-editor.desktop"),
        )
        .expect("snap fixture");

        assert_eq!(desktop.id, "org.example.Editor");
        assert_eq!(desktop.exec_path, "/opt/example/editor");
        assert_eq!(flatpak.id, "org.example.Editor-flatpak");
        assert_eq!(flatpak.start_cmd.as_deref(), Some("flatpak run org.example.Editor"));
        assert_eq!(snap.id, "org.example.Editor-snap");
        let merged = merge_inventory(vec![desktop, flatpak, snap]);
        assert_eq!(merged.len(), 3);
        assert_eq!(
            merged
                .iter()
                .map(|entry| inventory_identity(entry).canonical)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "apt:org.example.editor".to_string(),
                "flatpak:org.example.editor".to_string(),
                "snap:org.example.editor".to_string(),
            ])
        );
    }

    #[cfg(unix)]
    #[test]
    fn scans_realistic_applications_directory_and_merges_matching_desktop_metadata() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let app_dir = temp.path().join("Example Editor");
        fs::create_dir(&app_dir).expect("application directory");
        let executable = app_dir.join("example-editor");
        fs::write(&executable, b"#!/bin/sh\n").expect("executable fixture");
        let mut permissions = fs::metadata(&executable).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("executable permissions");

        let applications = scan_managed_applications(temp.path()).expect("applications scan");
        assert_eq!(applications.len(), 1);
        assert_eq!(applications[0].id, "example-editor");
        assert_eq!(applications[0].inventory_sources, ["Applications"]);
        assert_eq!(
            inventory_identity(&applications[0]).canonical,
            format!("local:path:{}", app_dir.display()).to_lowercase()
        );
    }

    #[test]
    fn parses_homebrew_fixture_and_keeps_backend_identity_separate() {
        let entries = parse_homebrew_formulae(include_str!("../tests/fixtures/brew-list.txt"));
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].id, "example-editor");
        assert_eq!(entries[1].name, "Example-editor");
        assert_eq!(inventory_identity(&entries[1]).canonical, "homebrew:example-editor");
        let desktop = parse_desktop_content(
            Path::new("/usr/share/applications/example-editor.desktop"),
            include_str!("../tests/fixtures/editor.desktop"),
        )
        .expect("desktop fixture");
        assert_eq!(merge_inventory(vec![entries[1].clone(), desktop]).len(), 2);
    }

    #[test]
    fn parses_registry_and_winget_fixtures_with_exact_backend_identities() {
        let registry =
            parse_windows_registry_json(include_str!("../tests/fixtures/registry.json")).expect("registry fixture");
        assert_eq!(registry.len(), 2);
        assert_eq!(registry[0].id, "ARP\\Machine\\X64\\{A1B2-C3D4}");
        assert_eq!(registry[0].publisher.as_deref(), Some("Example Corp"));
        assert_eq!(
            inventory_identity(&registry[0]).canonical,
            "registry:product-code:{a1b2-c3d4}"
        );
        assert_eq!(merge_inventory(registry).len(), 1);

        let winget = parse_winget_list(include_str!("../tests/fixtures/winget-list.txt"));
        assert_eq!(winget.len(), 2);
        assert_eq!(inventory_identity(&winget[0]).canonical, "winget:example.editor");
        assert_eq!(inventory_identity(&winget[1]).canonical, "msix:microsoft.terminal");
        assert_eq!(merge_inventory(winget).len(), 2);
    }

    #[test]
    fn registry_json_error_marks_old_registry_record_unverified() {
        let error = parse_windows_registry_json("not json").expect_err("invalid JSON");
        let old = app("legacy", "Legacy", "Registry", "Registry", Path::new("/missing"));
        let report = reconcile_inventory_report(
            &[(old, AppStatus::Healthy)],
            InventoryScanReport {
                entries: Vec::new(),
                source_errors: vec![SourceScanError {
                    source: "Registry".to_string(),
                    message: format!("invalid PowerShell JSON: {error}"),
                }],
            },
        );
        assert_eq!(report.retained_unverified, 1);
        assert_eq!(report.retained_broken, 0);
    }

    #[test]
    fn rescan_retains_broken_record_without_touching_roots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("keep.txt");
        fs::write(&marker, b"keep").expect("write marker");
        let old = app("missing", "Missing", "Local", "Applications", temp.path());
        let before = snapshot(temp.path());
        let report = reconcile_inventory_report(
            &[(old, AppStatus::Healthy)],
            InventoryScanReport {
                entries: Vec::new(),
                source_errors: Vec::new(),
            },
        );
        let after = snapshot(temp.path());
        assert_eq!(report.retained_broken, 1);
        assert!(matches!(report.records[0].1, AppStatus::Broken(_)));
        assert_eq!(before, after);
        assert_eq!(fs::read(marker).expect("read marker"), b"keep");
    }

    #[test]
    fn source_error_returns_partial_and_does_not_mass_mark_broken() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old = app("org.example.App-flatpak", "Example", "Flatpak", "Flatpak", temp.path());
        let report = reconcile_inventory_report(
            &[(old, AppStatus::Healthy)],
            InventoryScanReport {
                entries: vec![app("other", "Other", "Homebrew", "Homebrew", temp.path())],
                source_errors: vec![SourceScanError {
                    source: "Flatpak".to_string(),
                    message: "service unavailable".to_string(),
                }],
            },
        );
        assert_eq!(report.source_errors.len(), 1);
        assert_eq!(report.retained_unverified, 1);
        assert_eq!(report.retained_broken, 0);
        assert!(
            report
                .records
                .iter()
                .any(|(entry, status)| { entry.name == "Example" && *status == AppStatus::Healthy })
        );
    }

    #[test]
    fn windows_source_errors_retain_records_as_unverified() {
        let temp = tempfile::tempdir().expect("tempdir");
        let previous = vec![
            (
                app("MSIX\\Example", "Store App", "MSIX", "Winget", temp.path()),
                AppStatus::Healthy,
            ),
            (
                app("scoop-app", "Scoop App", "Scoop", "Scoop", temp.path()),
                AppStatus::Healthy,
            ),
            (
                app("choco-app", "Chocolatey App", "Chocolatey", "Chocolatey", temp.path()),
                AppStatus::Healthy,
            ),
        ];
        let source_errors = ["Winget", "Scoop", "Chocolatey"]
            .into_iter()
            .map(|source| SourceScanError {
                source: source.to_string(),
                message: "command failed".to_string(),
            })
            .collect();
        let report = reconcile_inventory_report(
            &previous,
            InventoryScanReport {
                entries: Vec::new(),
                source_errors,
            },
        );
        assert_eq!(report.retained_unverified, 3);
        assert_eq!(report.retained_broken, 0);
        assert!(report.records.iter().all(|(_, status)| *status == AppStatus::Healthy));
    }
}
