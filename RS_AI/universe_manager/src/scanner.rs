use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;
use crate::config::{Config, AppEntry};

pub fn parse_desktop_file(path: &Path) -> Result<AppEntry, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Không thể đọc file: {}", e))?;
        
    let filename = path.file_name()
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
            if line == "[Desktop Entry]" {
                in_desktop_entry = true;
            } else {
                in_desktop_entry = false;
            }
            continue;
        }
        
        if !in_desktop_entry {
            continue;
        }
        
        if let Some(index) = line.find('=') {
            let key = line[..index].trim();
            let val = line[index+1..].trim();
            
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
                "Type" => {
                    if val.to_lowercase() == "application" {
                        is_application = true;
                    }
                }
                _ => {}
            }
        }
    }
    
    if !is_application || no_display || name.is_empty() {
        return Err("Không phải ứng dụng hiển thị được".to_string());
    }
    
    // Clean Exec path (remove parameters like %u, %U, %f, %F)
    let clean_exec = exec.split_whitespace()
        .filter(|part| !part.starts_with('%'))
        .collect::<Vec<&str>>()
        .join(" ")
        .replace('"', "")
        .replace('\'', "");
        
    // Resolve primary category
    let mut category = "Other".to_string();
    if !categories_str.is_empty() {
        let list: Vec<&str> = categories_str.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        
        let main_categories = [
            "Network", "Internet", "Development", "Office", "Graphics",
            "AudioVideo", "Audio", "Video", "Multimedia", "Game",
            "System", "Utility", "Accessories", "Settings"
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
        ..Default::default()
    })
}

/// Scans all applications installed on the system (APT, Flatpak, Snap, Local, Homebrew, Winget, Scoop, Chocolatey).
pub fn scan_all_system_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen_ids = HashSet::new();
    let mut seen_names = HashSet::new();
    
    let config = Config::load();
    for mut app in config.apps {
        app.package_type = Some("Local".to_string());
        if app.category.is_none() {
            app.category = Some("Utility".to_string());
        }
        seen_ids.insert(app.id.clone());
        seen_names.insert(app.name.to_lowercase());
        apps.push(app);
    }
    
    // 1.5 Scan managed_dir for Stateless Portable apps
    let managed_dir = std::path::Path::new(&config.settings.managed_dir);
    if managed_dir.exists() && managed_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(managed_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let app_id = folder_name.to_lowercase()
                        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
                        .replace(' ', "-");
                        
                    if !seen_ids.contains(&app_id) {
                        // Quick scan for main executable
                        if let Ok(det) = crate::detector::detect(&path) {
                            if !det.executables.is_empty() {
                                let exec_path = det.executables[0].to_string_lossy().to_string();
                                let name = det.suggested_name.clone();
                                let name_lower = name.to_lowercase();
                                
                                seen_ids.insert(app_id.clone());
                                seen_names.insert(name_lower);
                                apps.push(AppEntry {
                                    id: app_id,
                                    name,
                                    install_type: crate::config::InstallType::Moved,
                                    source_path: None,
                                    install_path: path.to_string_lossy().to_string(),
                                    exec_path,
                                    icon_path: det.icons.first().map(|p| p.to_string_lossy().to_string()),
                                    desktop_file: "".to_string(),
                                    symlink_file: None,
                                    added_at: "".to_string(),
                                    is_custom: Some(false),
                                    start_cmd: None,
                                    stop_cmd: None,
                                    category: Some("Utility".to_string()),
                                    package_type: Some("Local".to_string()),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 2. Scan directories containing .desktop files
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let scan_dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        home.join(".local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        home.join(".local/share/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ];
    
    for dir in scan_dirs {
        if !dir.exists() || !dir.is_dir() {
            continue;
        }
        
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "desktop").unwrap_or(false) {
                    if let Ok(app_entry) = parse_desktop_file(&path) {
                        if !seen_ids.contains(&app_entry.id) {
                            seen_ids.insert(app_entry.id.clone());
                            seen_names.insert(app_entry.name.to_lowercase());
                            apps.push(app_entry);
                        }
                    }
                }
            }
        }
    }

    // 3. Scan Homebrew (Linux/macOS)
    #[cfg(unix)]
    {
        if Command::new("brew").arg("--version").status().is_ok() {
            if let Ok(out) = Command::new("brew").args(&["list", "--formula"]).output() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let pkg = line.trim();
                    if !pkg.is_empty() {
                        let name_formatted = pkg.chars().next().unwrap().to_uppercase().collect::<String>() + &pkg[1..];
                        let name_lower = name_formatted.to_lowercase();
                        if !seen_ids.contains(pkg) && !seen_names.contains(&name_lower) {
                            seen_ids.insert(pkg.to_string());
                            seen_names.insert(name_lower);
                            apps.push(AppEntry {
                                id: pkg.to_string(),
                                name: name_formatted,
                                install_type: crate::config::InstallType::InPlace,
                                source_path: None,
                                install_path: "Homebrew Cellar".to_string(),
                                exec_path: "brew".to_string(),
                                icon_path: None,
                                desktop_file: "".to_string(),
                                symlink_file: None,
                                added_at: "".to_string(),
                                is_custom: Some(true),
                                start_cmd: None,
                                stop_cmd: None,
                                category: Some("System".to_string()),
                                package_type: Some("Homebrew".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
    }

    // 4. Scan Registry, Winget, Scoop, Choco on Windows
    #[cfg(windows)]
    {
        // 4.1 Windows Registry uninstall keys scanning
        let script = "Get-ItemProperty -Path 'HKLM:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*', 'HKLM:\\Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*', 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\*' -ErrorAction SilentlyContinue | Select-Object PSPath, PSChildName, DisplayName, DisplayVersion, Publisher, UninstallString, InstallLocation, HelpLink, URLInfoAbout, SystemComponent, ParentKeyName | ConvertTo-Json -Compress";
        if let Ok(out) = Command::new("powershell")
            .args(&["-NoProfile", "-Command", script])
            .output()
        {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut json_str = stdout.trim().to_string();
                if !json_str.is_empty() {
                    if json_str.starts_with('{') {
                        json_str = format!("[{}]", json_str);
                    }
                    
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
                    
                    if let Ok(reg_apps) = serde_json::from_str::<Vec<WinRegApp>>(&json_str) {
                        for app in reg_apps {
                            if let Some(ref name) = app.display_name {
                                if name.trim().is_empty() {
                                    continue;
                                }
                                
                                if let Some(ref sys_val) = app.system_component {
                                    let is_sys = match sys_val {
                                        serde_json::Value::Number(n) => n.as_u64() == Some(1),
                                        serde_json::Value::String(s) => s.trim() == "1",
                                        _ => false,
                                    };
                                    if is_sys {
                                        continue;
                                    }
                                }
                                
                                if let Some(ref parent) = app.parent_key_name {
                                    if !parent.trim().is_empty() {
                                        continue;
                                    }
                                }
                                
                                let ps_path = app.ps_path.as_deref().unwrap_or("");
                                let hive = if ps_path.contains("HKEY_LOCAL_MACHINE") {
                                    "Machine"
                                } else {
                                    "User"
                                };
                                let arch = if ps_path.contains("Wow6432Node") {
                                    "X86"
                                } else {
                                    "X64"
                                };
                                let key_name = app.ps_child_name.as_deref().unwrap_or("");
                                if key_name.is_empty() {
                                    continue;
                                }
                                
                                let app_id = format!("ARP\\{}\\{}\\{}", hive, arch, key_name);
                                
                                let name_lower = name.to_lowercase();
                                if seen_ids.contains(&app_id) || seen_names.contains(&name_lower) {
                                    continue;
                                }
                                seen_ids.insert(app_id.clone());
                                seen_names.insert(name_lower);
                                
                                let product_code = if key_name.starts_with('{') && key_name.ends_with('}') {
                                    Some(key_name.to_string())
                                } else {
                                    None
                                };
                                
                                let uninstall_cmd = app.uninstall_string.clone().map(|s| s.trim().to_string());
                                let about_url = app.help_link.clone()
                                    .filter(|s| !s.trim().is_empty())
                                    .or_else(|| app.url_info_about.clone())
                                    .map(|s| s.trim().to_string());
                                    
                                let reg_key_path = ps_path.find("HKEY_")
                                    .map(|idx| ps_path[idx..].to_string())
                                    .unwrap_or_else(|| ps_path.to_string());
                                
                                apps.push(AppEntry {
                                    id: app_id,
                                    name: name.trim().to_string(),
                                    install_type: crate::config::InstallType::InPlace,
                                    source_path: Some(reg_key_path.clone()),
                                    install_path: app.install_location.clone().unwrap_or_default().trim().to_string(),
                                    exec_path: "".to_string(),
                                    icon_path: None,
                                    desktop_file: "".to_string(),
                                    symlink_file: None,
                                    added_at: format!("Registry ({} {})", hive, arch),
                                    is_custom: Some(true),
                                    start_cmd: uninstall_cmd.clone(),
                                    stop_cmd: None,
                                    category: Some("System".to_string()),
                                    package_type: Some("Registry".to_string()),
                                    registry_key: Some(reg_key_path),
                                    product_code,
                                    about_url,
                                    publisher: app.publisher.clone().map(|s| s.trim().to_string()),
                                    version: app.display_version.clone().map(|s| s.trim().to_string()),
                                    uninstall_cmd,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }

        // winget
        if let Ok(out) = Command::new("winget").arg("--version").output() {
            if out.status.success() {
                if let Ok(list_out) = Command::new("winget").args(&["list"]).output() {
                    let stdout = String::from_utf8_lossy(&list_out.stdout);
                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed.starts_with("Name") || trimmed.starts_with("-") {
                            continue;
                        }
                        let parts: Vec<&str> = trimmed.split("  ").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                        if parts.len() >= 2 {
                            let pkg_id = parts[1].to_string();
                            let app_name = parts[0].to_string();
                            let name_lower = app_name.to_lowercase();
                            if !seen_ids.contains(&pkg_id) && !seen_names.contains(&name_lower) {
                                seen_ids.insert(pkg_id.clone());
                                seen_names.insert(name_lower);
                                
                                let is_msix = pkg_id.to_lowercase().starts_with("msix\\");
                                let ptype = if is_msix { "MSIX" } else { "Winget" };
                                
                                apps.push(AppEntry {
                                    id: pkg_id,
                                    name: app_name,
                                    install_type: crate::config::InstallType::InPlace,
                                    source_path: None,
                                    install_path: if is_msix { "Windows Store".to_string() } else { "Windows winget".to_string() },
                                    exec_path: "winget".to_string(),
                                    icon_path: None,
                                    desktop_file: "".to_string(),
                                    symlink_file: None,
                                    added_at: "".to_string(),
                                    is_custom: Some(true),
                                    start_cmd: None,
                                    stop_cmd: None,
                                    category: Some(if is_msix { "Store".to_string() } else { "System".to_string() }),
                                    package_type: Some(ptype.to_string()),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }

        // scoop
        if Command::new("scoop").arg("--version").status().is_ok() {
            if let Ok(out) = Command::new("scoop").args(&["list"]).output() {
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
                            let name_formatted = app_name.chars().next().unwrap().to_uppercase().collect::<String>() + &app_name[1..];
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
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }

        // choco
        if let Ok(ver_out) = Command::new("choco").arg("--version").output() {
            if ver_out.status.success() {
                let ver_str = String::from_utf8_lossy(&ver_out.stdout);
                let is_v2 = ver_str.trim().starts_with('2') || ver_str.trim().starts_with('3');
                let choco_args = if is_v2 {
                    vec!["list"]
                } else {
                    vec!["list", "-lo"]
                };
                
                if let Ok(out) = Command::new("choco").args(&choco_args).output() {
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
                            let name_formatted = app_name.chars().next().unwrap().to_uppercase().collect::<String>() + &app_name[1..];
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
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Sort alphabetically by name
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}
