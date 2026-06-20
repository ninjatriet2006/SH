use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;
use crate::config::AppEntry;

pub fn get_real_id(id: &str) -> &str {
    id.strip_suffix("-flatpak")
        .or_else(|| id.strip_suffix("-snap"))
        .unwrap_or(id)
}

/// Helper to robustly parse registry query line
pub fn parse_reg_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let types = ["REG_SZ", "REG_EXPAND_SZ", "REG_DWORD", "REG_BINARY", "REG_MULTI_SZ", "REG_QWORD"];
    for t in &types {
        if let Some(idx) = trimmed.find(t) {
            let before = &trimmed[..idx];
            let after = &trimmed[idx + t.len()..];
            if (before.is_empty() || before.ends_with(char::is_whitespace)) &&
               (after.is_empty() || after.starts_with(char::is_whitespace)) {
                let name = before.trim().to_string();
                let data = after.trim().to_string();
                if !name.is_empty() && !data.is_empty() {
                    return Some((name, data));
                }
            }
        }
    }
    // Fallback to splitting by multiple spaces
    let parts: Vec<&str> = trimmed.split("    ").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() >= 3 {
        return Some((parts[0].to_string(), parts[2].to_string()));
    }
    None
}

#[cfg(windows)]
pub fn find_exec_path_on_windows(app: &AppEntry) -> Option<String> {
    // 1. If the exec_path exists as a file, return it
    if Path::new(&app.exec_path).exists() {
        return Some(app.exec_path.clone());
    }

    // 2. Try clean names for registry App Paths lookup
    let clean_names = vec![
        app.name.clone(),
        app.name.replace(' ', ""),
        app.id.clone(),
        get_real_id(&app.id).to_string(),
        get_real_id(&app.id).replace(|c: char| !c.is_alphanumeric(), ""),
    ];

    let registry_bases = [
        "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\App Paths",
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\App Paths",
    ];

    for name in clean_names {
        if name.trim().is_empty() {
            continue;
        }
        let extensions = ["", ".exe", ".cmd", ".bat"];
        for ext in &extensions {
            let key_name = format!("{}{}", name, ext);
            for base in &registry_bases {
                let full_key = format!("{}\\{}", base, key_name);
                if let Ok(out) = Command::new("reg").args(&["query", &full_key, "/ve"]).output() {
                    if out.status.success() {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        for line in stdout.lines() {
                            if let Some((name, data)) = parse_reg_line(line) {
                                if name == "(Default)" && !data.is_empty() {
                                    let clean_data = data.replace('"', "").replace('\'', "");
                                    if Path::new(&clean_data).exists() {
                                        return Some(clean_data);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Try Uninstall registry subkeys
    let uninstall_bases = [
        "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "HKLM\\Software\\Wow6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
        "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
    ];

    for base in &uninstall_bases {
        if let Ok(out) = Command::new("reg").args(&["query", base]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for subkey in stdout.lines() {
                let subkey = subkey.trim();
                if subkey.is_empty() {
                    continue;
                }
                let subkey_lower = subkey.to_lowercase();
                let clean_id = get_real_id(&app.id).to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
                let clean_name = app.name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");

                if subkey_lower.contains(&clean_id) || subkey_lower.contains(&clean_name) {
                    if let Ok(val_out) = Command::new("reg").args(&["query", subkey]).output() {
                        let val_stdout = String::from_utf8_lossy(&val_out.stdout);
                        let mut install_loc = None;
                        let mut display_name_matches = false;

                        for line in val_stdout.lines() {
                            if let Some((name, data)) = parse_reg_line(line) {
                                if name == "DisplayName" {
                                    let display_lower = data.to_lowercase();
                                    if display_lower.contains(&app.name.to_lowercase()) {
                                        display_name_matches = true;
                                    }
                                } else if name == "InstallLocation" {
                                    install_loc = Some(data.replace('"', "").replace('\'', ""));
                                }
                            }
                        }

                        if display_name_matches || subkey_lower.contains(&clean_id) {
                            if let Some(loc) = install_loc {
                                if !loc.trim().is_empty() {
                                    let loc_path = Path::new(&loc);
                                    if loc_path.exists() && loc_path.is_dir() {
                                        if let Ok(entries) = fs::read_dir(loc_path) {
                                            for entry in entries.flatten() {
                                                if let Ok(meta) = entry.metadata() {
                                                    if meta.is_file() {
                                                        let path = entry.path();
                                                        if path.extension().map_or(false, |e| e.to_ascii_lowercase() == "exe") {
                                                            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                                                            if filename.contains(&clean_name) || clean_name.contains(&filename.replace(".exe", "")) {
                                                                return Some(path.to_string_lossy().to_string());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        let common_exe = loc_path.join(format!("{}.exe", app.name.replace(' ', "")));
                                        if common_exe.exists() {
                                            return Some(common_exe.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// A snapshot of all running processes in the system.
#[derive(Clone)]
pub struct ProcessSnapshot {
    pub canonical_exes: HashSet<PathBuf>,
    pub names: HashSet<String>,
    pub cmdlines: Vec<String>,
}

impl ProcessSnapshot {
    pub fn collect() -> Self {
        #[allow(unused_mut)]
        let mut canonical_exes = HashSet::new();
        let mut names = HashSet::new();
        #[allow(unused_mut)]
        let mut cmdlines = Vec::new();
        
        #[cfg(unix)]
        {
            if let Ok(entries) = fs::read_dir("/proc") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if name_str.chars().all(|c| c.is_ascii_digit()) {
                        // Method 1: Check exe link
                        let exe_link = path.join("exe");
                        if let Ok(target) = fs::read_link(&exe_link) {
                            if let Some(n) = target.file_name() {
                                names.insert(n.to_string_lossy().to_string());
                            }
                            canonical_exes.insert(target);
                        }
                        
                        // Method 2: Check cmdline
                        let cmd_file = path.join("cmdline");
                        if let Ok(cmd_bytes) = fs::read(cmd_file) {
                            let cmd_str = String::from_utf8_lossy(&cmd_bytes).replace('\0', " ");
                            if !cmd_str.trim().is_empty() {
                                cmdlines.push(cmd_str);
                            }
                        }
                    }
                }
            }
        }

        #[cfg(windows)]
        {
            // Call tasklist to get running processes on Windows
            if let Ok(out) = Command::new("tasklist").args(&["/FO", "CSV", "/NH"]).output() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    // CSV output format: "Image Name","PID","Session Name","Session#","Mem Usage"
                    let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim_matches('"')).collect();
                    if !parts.is_empty() {
                        let proc_name = parts[0].to_string();
                        names.insert(proc_name.clone());
                        // Also insert without .exe extension to make matching simpler
                        if proc_name.to_lowercase().ends_with(".exe") {
                            names.insert(proc_name[..proc_name.len() - 4].to_string());
                        }
                    }
                }
            }
        }
        
        ProcessSnapshot {
            canonical_exes,
            names,
            cmdlines,
        }
    }
    
    pub fn is_running(&self, app: &AppEntry) -> bool {
        let ptype = app.package_type.as_deref().unwrap_or("Local");
        
        if ptype == "Local" {
            let is_custom = app.is_custom.unwrap_or(false);
            let exec_path_buf = PathBuf::from(&app.exec_path);
            
            // Check path directly
            if self.canonical_exes.contains(&exec_path_buf) {
                return true;
            }
            
            let exec_name = exec_path_buf.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            
            // Check exe name
            if !exec_name.is_empty() && self.names.contains(&exec_name) {
                return true;
            }
            
            let real_id = get_real_id(&app.id);
            // Check cmdline
            for cmd_str in &self.cmdlines {
                if is_custom {
                    if cmd_str.contains(real_id) {
                        return true;
                    }
                    if let Some(ref start) = app.start_cmd {
                        let first_word = start.split_whitespace().next().unwrap_or("");
                        if !first_word.is_empty() && cmd_str.contains(first_word) {
                            if start.contains("flatpak") && start.contains(real_id) {
                                return true;
                            }
                        }
                    }
                } else {
                    if cmd_str.contains(&app.exec_path) {
                        return true;
                    }
                }
            }
            return false;
        }

        // For system package managers (APT, Flatpak, Snap, Homebrew, Winget, Scoop, Chocolatey):
        // Match name, ID, and initials case-insensitively.
        let app_name_lower = app.name.to_lowercase();
        let app_id_lower = app.id.to_lowercase();
        
        let clean_id = get_real_id(&app_id_lower).replace(|c: char| !c.is_alphanumeric(), "");
        let initials: String = app.name.split_whitespace()
            .filter_map(|w| w.chars().next())
            .collect::<String>()
            .to_lowercase();

        for name in &self.names {
            let name_lower = name.to_lowercase();
            // Ignore generic commands
            if name_lower == "winget" || name_lower == "scoop" || name_lower == "choco" || name_lower == "brew" || name_lower == "cmd" || name_lower == "powershell" {
                continue;
            }

            let clean_name = name_lower.replace(|c: char| !c.is_alphanumeric(), "");

            if name_lower == app_name_lower {
                return true;
            }
            if clean_name == clean_id {
                return true;
            }
            if clean_name.contains(&clean_id) || clean_id.contains(&clean_name) {
                if clean_name.len() > 3 && clean_id.len() > 3 {
                    return true;
                }
            }
            if initials.len() >= 3 && clean_name == initials {
                return true;
            }
            for word in app.name.split_whitespace() {
                let clean_word = word.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
                if clean_word.len() > 3 && clean_name.contains(&clean_word) {
                    if clean_word != "client" && clean_word != "player" && clean_word != "manager" && clean_word != "free" && clean_word != "open" {
                        return true;
                    }
                }
            }
        }

        for cmd_str in &self.cmdlines {
            let cmd_lower = cmd_str.to_lowercase();
            if cmd_lower.contains(&app_name_lower) || cmd_lower.contains(&clean_id) {
                return true;
            }
        }

        false
    }
}

/// Checks if the application process is currently running.
pub fn is_app_running(app: &AppEntry) -> bool {
    let snapshot = ProcessSnapshot::collect();
    snapshot.is_running(app)
}

/// Spawns the application process in the background.
pub fn start_app(app: &AppEntry) -> Result<(), String> {
    let ptype = app.package_type.as_deref().unwrap_or("Local");

    #[cfg(unix)]
    {
        if app.is_custom.unwrap_or(false) {
            if let Some(ref start_cmd) = app.start_cmd {
                Command::new("sh")
                    .arg("-c")
                    .arg(start_cmd)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|e| format!("Lỗi khởi chạy (custom command): {}", e))?;
                return Ok(());
            }
        }
        Command::new(&app.exec_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Lỗi khởi chạy: {}", e))?;
        Ok(())
    }

    #[cfg(windows)]
    {
        // 1. If it's a custom command with start_cmd, run cmd.exe /c start_cmd
        if let Some(ref start_cmd) = app.start_cmd {
            if !start_cmd.trim().is_empty() {
                Command::new("cmd")
                    .args(&["/c", start_cmd])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|e| format!("Lỗi khởi chạy: {}", e))?;
                return Ok(());
            }
        }

        // 2. Resolve real executable path on Windows
        if let Some(resolved_path) = find_exec_path_on_windows(app) {
            Command::new(resolved_path)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| format!("Lỗi khởi chạy (Resolved Path): {}", e))?;
            return Ok(());
        }

        // 3. Fallback: try cmd /c start "app name"
        let fallback_exe = if ptype != "Local" {
            app.name.replace(' ', "").to_lowercase()
        } else {
            app.exec_path.clone()
        };

        let status = Command::new("cmd")
            .args(&["/c", "start", "", &fallback_exe])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => Ok(()),
            _ => {
                Command::new(&fallback_exe)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| format!("Không tìm thấy file chạy và chạy thử lệnh cmd/direct đều thất bại: {}", e))
            }
        }
    }
}

/// Stops the application process(es).
pub fn stop_app(app: &AppEntry) -> Result<(), String> {
    #[cfg(unix)]
    {
        if app.is_custom.unwrap_or(false) {
            if let Some(ref stop_cmd) = app.stop_cmd {
                let status = Command::new("sh")
                    .arg("-c")
                    .arg(stop_cmd)
                    .status()
                    .map_err(|e| format!("Lỗi thực thi lệnh stop: {}", e))?;
                if status.success() {
                    return Ok(());
                }
            }
        }

        let exec_path_buf = PathBuf::from(&app.exec_path);
        let exec_canonical = fs::canonicalize(&exec_path_buf).ok();
        let exec_name = exec_path_buf.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let mut pids = Vec::new();
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.chars().all(|c| c.is_ascii_digit()) {
                    let mut matches = false;
                    
                    let exe_link = entry.path().join("exe");
                    if let Ok(target) = fs::read_link(&exe_link) {
                        if let Some(ref canon) = exec_canonical {
                            if &target == canon {
                                matches = true;
                            }
                        }
                        if !exec_name.is_empty() && target.to_string_lossy().contains(&exec_name) {
                            matches = true;
                        }
                    }
                    
                    let cmd_file = entry.path().join("cmdline");
                    if let Ok(cmd_bytes) = fs::read(cmd_file) {
                        let cmd_str = String::from_utf8_lossy(&cmd_bytes).replace('\0', " ");
                        if app.is_custom.unwrap_or(false) {
                            let real_id = get_real_id(&app.id);
                            if cmd_str.contains(real_id) {
                                matches = true;
                            }
                        } else {
                            if cmd_str.contains(&app.exec_path) {
                                matches = true;
                            }
                        }
                    }

                    if matches {
                        pids.push(name_str.to_string());
                    }
                }
            }
        }

        if !pids.is_empty() {
            let mut cmd = Command::new("kill");
            for pid in pids {
                cmd.arg(pid);
            }
            let _ = cmd.status();
        }
    }

    #[cfg(windows)]
    {
        if let Some(ref stop_cmd) = app.stop_cmd {
            if !stop_cmd.trim().is_empty() {
                let status = Command::new("cmd")
                    .args(&["/c", stop_cmd])
                    .status()
                    .map_err(|e| format!("Lỗi thực thi lệnh stop: {}", e))?;
                if status.success() {
                    return Ok(());
                }
            }
        }

        let snapshot = ProcessSnapshot::collect();
        let mut stopped_any = false;

        let app_name_lower = app.name.to_lowercase();
        let app_id_lower = app.id.to_lowercase();
        let clean_id = get_real_id(&app_id_lower).replace(|c: char| !c.is_alphanumeric(), "");
        let initials: String = app.name.split_whitespace()
            .filter_map(|w| w.chars().next())
            .collect::<String>()
            .to_lowercase();

        for name in &snapshot.names {
            let name_lower = name.to_lowercase();
            if name_lower == "winget" || name_lower == "scoop" || name_lower == "choco" || name_lower == "brew" || name_lower == "cmd" || name_lower == "powershell" {
                continue;
            }

            let clean_name = name_lower.replace(|c: char| !c.is_alphanumeric(), "");
            let mut matches = false;

            if name_lower == app_name_lower {
                matches = true;
            } else if clean_name == clean_id {
                matches = true;
            } else if clean_name.contains(&clean_id) || clean_id.contains(&clean_name) {
                if clean_name.len() > 3 && clean_id.len() > 3 {
                    matches = true;
                }
            } else if initials.len() >= 3 && clean_name == initials {
                matches = true;
            } else {
                for word in app.name.split_whitespace() {
                    let clean_word = word.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
                    if clean_word.len() > 3 && clean_name.contains(&clean_word) {
                        if clean_word != "client" && clean_word != "player" && clean_word != "manager" && clean_word != "free" && clean_word != "open" {
                            matches = true;
                            break;
                        }
                    }
                }
            }

            if matches {
                let target_exe = if name.to_lowercase().ends_with(".exe") {
                    name.clone()
                } else {
                    format!("{}.exe", name)
                };

                let _ = Command::new("taskkill")
                    .args(&["/F", "/IM", &target_exe])
                    .status();
                stopped_any = true;
            }
        }

        if !stopped_any {
            let clean_name = app.name.replace(' ', "");
            let _ = Command::new("taskkill")
                .args(&["/F", "/IM", &format!("{}.exe", clean_name)])
                .status();
        }
    }

    Ok(())
}

/// Restarts the application process.
pub fn restart_app(app: &AppEntry) -> Result<(), String> {
    let _ = stop_app(app);
    std::thread::sleep(std::time::Duration::from_millis(500));
    start_app(app)
}

/// Checks if the autostart launcher for this app exists.

/// Checks system updates. On Windows checks Winget, MS Store, Chocolatey, and Scoop. On Unix/Linux checks APT, Flatpak, and Snap.
#[cfg(windows)]
pub struct AppPaths {
    pub config_dir: Option<String>,
    pub data_dir: Option<String>,
    pub cache_dir: Option<String>,
    pub system_share_dir: Option<String>,
}

pub fn get_app_paths(app: &AppEntry) -> AppPaths {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let real_id = get_real_id(&app.id);
    let package_type = app.package_type.as_deref().unwrap_or("Local");

    let mut config_dir = None;
    let mut data_dir = None;
    let mut cache_dir = None;
    let mut system_share_dir = None;

    match package_type {
        "Flatpak" => {
            let base = home.join(".var/app").join(real_id);
            if base.exists() {
                config_dir = Some(base.join("config").to_string_lossy().to_string());
                data_dir = Some(base.join("data").to_string_lossy().to_string());
                cache_dir = Some(base.join("cache").to_string_lossy().to_string());
            } else {
                config_dir = Some(base.to_string_lossy().to_string());
            }
            
            // Check flatpak app files directory
            let sys_app_dir = PathBuf::from("/var/lib/flatpak/app").join(real_id);
            let user_app_dir = home.join(".local/share/flatpak/app").join(real_id);
            if sys_app_dir.exists() {
                system_share_dir = Some(sys_app_dir.to_string_lossy().to_string());
            } else if user_app_dir.exists() {
                system_share_dir = Some(user_app_dir.to_string_lossy().to_string());
            }
        }
        "Snap" => {
            let base_current = home.join("snap").join(real_id).join("current");
            let base_common = home.join("snap").join(real_id).join("common");
            
            if base_current.exists() {
                config_dir = Some(base_current.join(".config").to_string_lossy().to_string());
                data_dir = Some(base_current.to_string_lossy().to_string());
            }
            if base_common.exists() {
                cache_dir = Some(base_common.join(".cache").to_string_lossy().to_string());
                if data_dir.is_none() {
                    data_dir = Some(base_common.to_string_lossy().to_string());
                }
            }
            
            let sys_app_dir = PathBuf::from("/snap").join(real_id);
            if sys_app_dir.exists() {
                system_share_dir = Some(sys_app_dir.to_string_lossy().to_string());
            }
        }
        _ => { // APT or Local
            // 1. Config Dir
            let user_config = home.join(".config").join(real_id);
            let etc_config = PathBuf::from("/etc").join(real_id);
            if user_config.exists() {
                config_dir = Some(user_config.to_string_lossy().to_string());
            } else if etc_config.exists() {
                config_dir = Some(etc_config.to_string_lossy().to_string());
            } else {
                config_dir = Some(user_config.to_string_lossy().to_string());
            }

            // 2. Data Dir
            let user_data = home.join(".local/share").join(real_id);
            let var_lib_data = PathBuf::from("/var/lib").join(real_id);
            if user_data.exists() {
                data_dir = Some(user_data.to_string_lossy().to_string());
            } else if var_lib_data.exists() {
                data_dir = Some(var_lib_data.to_string_lossy().to_string());
            } else {
                data_dir = Some(user_data.to_string_lossy().to_string());
            }

            // 3. Cache Dir
            let user_cache = home.join(".cache").join(real_id);
            if user_cache.exists() {
                cache_dir = Some(user_cache.to_string_lossy().to_string());
            } else {
                cache_dir = Some(user_cache.to_string_lossy().to_string());
            }

            // 4. System share dir
            let sys_share = PathBuf::from("/usr/share").join(real_id);
            if sys_share.exists() {
                system_share_dir = Some(sys_share.to_string_lossy().to_string());
            }
        }
    }

    AppPaths {
        config_dir,
        data_dir,
        cache_dir,
        system_share_dir,
    }
}
