use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;
use crate::config::{Config, AppEntry};

fn get_real_id(id: &str) -> &str {
    id.strip_suffix("-flatpak")
        .or_else(|| id.strip_suffix("-snap"))
        .unwrap_or(id)
}

/// Helper to robustly parse registry query line
fn parse_reg_line(line: &str) -> Option<(String, String)> {
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
fn find_exec_path_on_windows(app: &AppEntry) -> Option<String> {
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
pub fn is_autostart_enabled(app: &AppEntry) -> bool {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
    let autostart_file = home.join(".config").join("autostart").join(format!("{}.desktop", app.id));
    autostart_file.exists()
}

/// Configures the app to run at system startup.
pub fn enable_autostart(app: &AppEntry) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Không xác định được thư mục Home")?;
    let autostart_dir = home.join(".config").join("autostart");
    if !autostart_dir.exists() {
        fs::create_dir_all(&autostart_dir)
            .map_err(|e| format!("Lỗi tạo thư mục autostart: {}", e))?;
    }

    let dest = autostart_dir.join(format!("{}.desktop", app.id));
    let src = Path::new(&app.desktop_file);
    
    if src.exists() {
        fs::copy(src, &dest)
            .map_err(|e| format!("Lỗi copy file cấu hình vào autostart: {}", e))?;
    } else {
        let categories = "Utility;";
        let comment = format!("Tự động khởi động cùng hệ thống: {}", app.name);
        let icon_line = match &app.icon_path {
            Some(path) => format!("Icon={}\n", path),
            None => String::new(),
        };
        let content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name={}\n\
            Comment={}\n\
            Exec=\"{}\"\n\
            Path={}\n\
            {}\
            Terminal=false\n\
            Categories={}\n",
            app.name,
            comment,
            app.exec_path,
            app.install_path,
            icon_line,
            categories
        );
        fs::write(&dest, content)
            .map_err(|e| format!("Lỗi ghi file autostart: {}", e))?;
    }
    Ok(())
}

/// Disables system startup run.
pub fn disable_autostart(app: &AppEntry) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("Không xác định được thư mục Home")?;
    let autostart_file = home.join(".config").join("autostart").join(format!("{}.desktop", app.id));
    if autostart_file.exists() {
        fs::remove_file(autostart_file)
            .map_err(|e| format!("Lỗi xoá tệp autostart: {}", e))?;
    }
    Ok(())
}

/// Helper function to parse a system .desktop file.
fn parse_desktop_file(path: &Path) -> Result<AppEntry, String> {
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
    
    // 1. Add locally registered portable apps first
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

/// Checks system updates for APT, Flatpak, and Snap without requiring sudo password.
pub fn check_system_updates() -> Result<String, String> {
    let mut result = String::new();
    
    // 1. APT Simulation check (does not require root)
    result.push_str("=== KIỂM TRA CẬP NHẬT APT ===\n");
    match Command::new("apt-get")
        .arg("-s")
        .arg("upgrade")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let upgradable = stdout.lines()
                .filter(|l| l.contains("Inst ") || l.contains("Conf "))
                .count();
            if upgradable > 0 {
                result.push_str(&format!("-> Có {} gói có sẵn bản nâng cấp thông qua APT.\n", upgradable));
                // Show first 5 packages
                let pkgs: Vec<&str> = stdout.lines()
                    .filter(|l| l.contains("Inst "))
                    .take(5)
                    .map(|l| l.split_whitespace().nth(1).unwrap_or(""))
                    .collect();
                result.push_str(&format!("   Các gói đề xuất: {}\n", pkgs.join(", ")));
            } else {
                result.push_str("-> Hệ thống APT đã được cập nhật đầy đủ.\n");
            }
        }
        Err(e) => {
            result.push_str(&format!("-> Không thể kiểm tra APT: {}\n", e));
        }
    }
    result.push_str("\n");

    // 2. Flatpak update check
    result.push_str("=== KIỂM TRA CẬP NHẬT FLATPAK ===\n");
    match Command::new("flatpak")
        .arg("update")
        .arg("--check")
        .output()
    {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let combined = format!("{}{}", stdout, stderr);
            
            if combined.contains("Nothing to do") || combined.trim().is_empty() {
                result.push_str("-> Các ứng dụng Flatpak đã được cập nhật đầy đủ.\n");
            } else {
                result.push_str("-> Tìm thấy bản cập nhật Flatpak khả dụng:\n");
                // Print first 5 non-empty lines
                let lines: Vec<&str> = combined.lines().filter(|l| !l.trim().is_empty()).take(5).collect();
                for l in lines {
                    result.push_str(&format!("   {}\n", l));
                }
            }
        }
        Err(e) => {
            result.push_str(&format!("-> Không thể kiểm tra Flatpak: {}\n", e));
        }
    }
    result.push_str("\n");

    // 3. Snap check
    result.push_str("=== KIỂM TRA CẬP NHẬT SNAP ===\n");
    match Command::new("snap")
        .arg("refresh")
        .arg("--list")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            if output.status.success() {
                let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
                if lines.len() > 1 {
                    result.push_str(&format!("-> Có {} gói Snap có thể nâng cấp.\n", lines.len() - 1));
                    for l in lines.iter().skip(1).take(5) {
                        result.push_str(&format!("   {}\n", l));
                    }
                } else {
                    result.push_str("-> Toàn bộ gói Snap đã được cập nhật.\n");
                }
            } else {
                let err_msg = if stderr.contains("no updates") {
                    "-> Toàn bộ gói Snap đã được cập nhật."
                } else {
                    "-> Snap daemon không phản hồi hoặc không có cập nhật."
                };
                result.push_str(&format!("{}\n", err_msg));
            }
        }
        Err(e) => {
            result.push_str(&format!("-> Không thể kiểm tra Snap: {}\n", e));
        }
    }

    Ok(result)
}

/// Executes Flatpak leftover cleaning directly.
pub fn clean_flatpak_unused() -> Result<String, String> {
    match Command::new("flatpak")
        .arg("uninstall")
        .arg("--unused")
        .arg("-y")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(format!("Flatpak clean-up:\n{}{}", stdout, stderr))
        }
        Err(e) => Err(format!("Lỗi dọn Flatpak: {}", e)),
    }
}

/// Scans and cleans system leftovers (APT rc packages, Flatpak unused, and orphaned config/data folders).
pub fn clean_system_leftovers() -> Result<String, String> {
    let mut result = String::new();
    result.push_str("=== BẮT ĐẦU DỌN DẸP HỆ THỐNG (LEFTOVERS) ===\n\n");
    
    // 1. APT leftovers (purging 'rc' state packages)
    result.push_str("[1/3] Đang quét cấu hình cũ của APT (trạng thái 'rc')...\n");
    match Command::new("dpkg")
        .arg("-l")
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut rc_pkgs = Vec::new();
            for line in stdout.lines() {
                if line.starts_with("rc ") {
                    if let Some(pkg) = line.split_whitespace().nth(1) {
                        rc_pkgs.push(pkg.to_string());
                    }
                }
            }
            
            if !rc_pkgs.is_empty() {
                result.push_str(&format!("-> Phát hiện {} gói cấu hình thừa: {}\n", rc_pkgs.len(), rc_pkgs.join(", ")));
                result.push_str("-> Đang thực thi gỡ bỏ cấu hình thừa (yêu cầu quyền sudo nếu cần)...\n");
                
                let mut cmd = Command::new("sudo");
                cmd.arg("apt-get").arg("purge").arg("-y");
                for pkg in &rc_pkgs {
                    cmd.arg(pkg);
                }
                
                match cmd.status() {
                    Ok(status) => {
                           if status.success() {
                               result.push_str("-> Dọn dẹp cấu hình APT thừa thành công!\n");
                           } else {
                               result.push_str(&format!("-> Lệnh dọn dẹp kết thúc với lỗi: {:?}\n", status.code()));
                           }
                    }
                    Err(e) => {
                        result.push_str(&format!("-> Lỗi khi chạy lệnh gỡ: {}\n", e));
                    }
                }
            } else {
                result.push_str("-> Không tìm thấy cấu hình cũ (trạng thái 'rc') nào cần dọn dẹp.\n");
            }
        }
        Err(e) => {
            result.push_str(&format!("-> Không thể kiểm tra dpkg: {}\n", e));
        }
    }
    result.push_str("\n");
    
    // 2. Flatpak unused runtimes
    result.push_str("[2/3] Đang dọn dẹp Flatpak runtimes không sử dụng...\n");
    match clean_flatpak_unused() {
        Ok(out) => {
            result.push_str("-> Hoàn tất dọn dẹp Flatpak:\n");
            result.push_str(&out);
        }
        Err(e) => {
            result.push_str(&format!("-> Lỗi dọn Flatpak: {}\n", e));
        }
    }
    result.push_str("\n");

    // 3. Scan for orphaned configuration/data directories (Heuristics Leftovers)
    result.push_str("[3/3] Đang quét thư mục cấu hình & dữ liệu rác (Orphaned Leftovers)...\n");
    let orphans = crate::remover::find_orphaned_leftovers();
    if orphans.is_empty() {
        result.push_str("-> Không tìm thấy thư mục cấu hình rác nào trên hệ thống.\n");
    } else {
        result.push_str(&format!("-> Phát hiện {} thư mục rác/registry key khả nghi:\n", orphans.len()));
        for (i, p) in orphans.iter().enumerate() {
            result.push_str(&format!("  [{}] {:?}\n", i + 1, p));
        }
        result.push_str("\n* Lưu ý: Tiến trình dọn dẹp sẽ thực hiện sao lưu trước khi xoá.\n");
        
        println!("{}", result);
        result.clear();

        let confirm_all = dialoguer::Confirm::new()
            .with_prompt("Bạn có muốn dọn dẹp TẤT CẢ các thư mục rác này không?")
            .default(false)
            .interact()
            .unwrap_or(false);

        if confirm_all {
            let mut cleaned_count = 0;
            for p in &orphans {
                println!("Đang dọn dẹp: {:?}", p);
                if p.is_dir() {
                    if let Ok(backup_file) = crate::remover::backup_leftover_dir(p) {
                        println!("  - Đã sao lưu dự phòng tại: {:?}", backup_file);
                    }
                    if fs::remove_dir_all(p).is_ok() {
                        cleaned_count += 1;
                    }
                } else {
                    #[cfg(windows)]
                    {
                        let path_str = p.to_string_lossy().to_string();
                        if path_str.starts_with("HKEY_") || path_str.starts_with("HK") {
                            if Command::new("reg").args(&["delete", &path_str, "/f"]).status().is_ok() {
                                cleaned_count += 1;
                            }
                        }
                    }
                }
            }
            result.push_str(&format!("-> Đã dọn dẹp thành công {}/{} thư mục/đăng ký rác!\n", cleaned_count, orphans.len()));
        } else {
            let selections: Vec<String> = orphans.iter().map(|p| format!("{:?}", p)).collect();
            let multi_select = dialoguer::MultiSelect::new()
                .with_prompt("Chọn các thư mục/đăng ký bạn muốn xoá (Nhấn Space để chọn, Enter để chạy):")
                .items(&selections)
                .interact_opt()
                .unwrap_or(None);

            if let Some(choices) = multi_select {
                if choices.is_empty() {
                    result.push_str("-> Bỏ qua dọn dẹp thư mục rác.\n");
                } else {
                    let mut cleaned_count = 0;
                    for choice in choices {
                        let p = &orphans[choice];
                        println!("Đang dọn dẹp: {:?}", p);
                        if p.is_dir() {
                            if let Ok(backup_file) = crate::remover::backup_leftover_dir(p) {
                                println!("  - Đã sao lưu dự phòng tại: {:?}", backup_file);
                            }
                            if fs::remove_dir_all(p).is_ok() {
                                cleaned_count += 1;
                            }
                        } else {
                            #[cfg(windows)]
                            {
                                let path_str = p.to_string_lossy().to_string();
                                if path_str.starts_with("HKEY_") || path_str.starts_with("HK") {
                                    if Command::new("reg").args(&["delete", &path_str, "/f"]).status().is_ok() {
                                        cleaned_count += 1;
                                    }
                                }
                            }
                        }
                    }
                    result.push_str(&format!("-> Đã dọn dẹp thành công {}/{} mục rác được chọn!\n", cleaned_count, orphans.len()));
                }
            } else {
                result.push_str("-> Huỷ bỏ dọn dẹp rác.\n");
            }
        }
    }
    
    Ok(result)
}

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutostartEntry {
    pub name: String,
    pub exec: String,
    pub location: String,
    pub is_enabled: bool,
}

pub fn scan_global_autostart() -> Vec<AutostartEntry> {
    let mut entries = Vec::new();

    #[cfg(unix)]
    {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let dirs = vec![
            home.join(".config").join("autostart"),
            PathBuf::from("/etc/xdg/autostart"),
        ];

        for dir in dirs {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }

            if let Ok(files) = fs::read_dir(dir) {
                for file in files.flatten() {
                    let path = file.path();
                    if path.is_file() && path.extension().map_or(false, |e| e == "desktop") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let mut name = None;
                            let mut exec = None;
                            let mut enabled = true;

                            for line in content.lines() {
                                let trimmed = line.trim();
                                if trimmed.starts_with("Name=") {
                                    name = Some(trimmed[5..].to_string());
                                } else if trimmed.starts_with("Exec=") {
                                    exec = Some(trimmed[5..].to_string());
                                } else if trimmed.starts_with("X-GNOME-Autostart-enabled=false") 
                                       || trimmed.starts_with("Hidden=true") {
                                    enabled = false;
                                }
                            }

                            if let (Some(n), Some(e)) = (name, exec) {
                                entries.push(AutostartEntry {
                                    name: n,
                                    exec: e,
                                    location: path.to_string_lossy().to_string(),
                                    is_enabled: enabled,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // 1. Registry HKCU Run
        let hkcu_run = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        if let Ok(out) = Command::new("reg").args(&["query", hkcu_run]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with(hkcu_run) || trimmed.is_empty() {
                    continue;
                }
                if let Some((name, exec)) = parse_reg_line(trimmed) {
                    entries.push(AutostartEntry {
                        name,
                        exec,
                        location: "Registry (HKCU)".to_string(),
                        is_enabled: true,
                    });
                }
            }
        }

        // 2. Registry HKLM Run
        let hklm_run = "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        if let Ok(out) = Command::new("reg").args(&["query", hklm_run]).output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with(hklm_run) || trimmed.is_empty() {
                    continue;
                }
                if let Some((name, exec)) = parse_reg_line(trimmed) {
                    entries.push(AutostartEntry {
                        name,
                        exec,
                        location: "Registry (HKLM)".to_string(),
                        is_enabled: true,
                    });
                }
            }
        }

        // 3. Startup Folder
        if let Ok(appdata) = std::env::var("APPDATA") {
            let startup_dir = PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup");
            
            if startup_dir.exists() && startup_dir.is_dir() {
                if let Ok(files) = fs::read_dir(startup_dir) {
                    for file in files.flatten() {
                        let path = file.path();
                        if path.is_file() {
                            let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            entries.push(AutostartEntry {
                                name: filename.clone(),
                                exec: path.to_string_lossy().to_string(),
                                location: "Startup Folder".to_string(),
                                is_enabled: true,
                            });
                        }
                    }
                }
            }
        }
    }

    entries
}

pub fn remove_autostart_entry(entry: &AutostartEntry) -> Result<(), String> {
    #[cfg(unix)]
    {
        let path = Path::new(&entry.location);
        if path.exists() && path.is_file() {
            fs::remove_file(path).map_err(|e| format!("Lỗi xoá file: {}", e))?;
        }
        Ok(())
    }

    #[cfg(windows)]
    {
        if entry.location.contains("Registry (HKCU)") {
            let status = Command::new("reg")
                .args(&["delete", "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", &entry.name, "/f"])
                .status()
                .map_err(|e| format!("Lỗi gọi reg: {}", e))?;
            if status.success() { Ok(()) } else { Err("Không thể xoá registry key".to_string()) }
        } else if entry.location.contains("Registry (HKLM)") {
            let status = Command::new("reg")
                .args(&["delete", "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", "/v", &entry.name, "/f"])
                .status()
                .map_err(|e| format!("Lỗi gọi reg: {}", e))?;
            if status.success() { Ok(()) } else { Err("Không thể xoá registry key (yêu cầu quyền Admin)".to_string()) }
        } else if entry.location.contains("Startup Folder") {
            let path = Path::new(&entry.exec);
            if path.exists() && path.is_file() {
                fs::remove_file(path).map_err(|e| format!("Lỗi xoá file: {}", e))?;
            }
            Ok(())
        } else {
            Err("Không hỗ trợ vị trí khởi động này".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_scan() {
        let apps = scan_all_system_apps();
        println!("TOTAL APPS: {}", apps.len());
        for app in apps {
            if app.name.contains("Fcitx") || app.id.contains("fcitx") || app.package_type == Some("Flatpak".to_string()) {
                println!("=== FOUND FLATPAK OR FCITX APP ===");
                println!("APP: id={}, name={}, package_type={:?}, desktop_file={}", app.id, app.name, app.package_type, app.desktop_file);
            }
        }
    }

    #[test]
    fn test_parse_reg_line() {
        // Test standard REG_SZ value
        let line1 = "    Mozilla-Firefox-3080    REG_SZ    \"C:\\Program Files\\firefox.exe\" -os-autostart";
        let res1 = parse_reg_line(line1);
        assert_eq!(res1, Some(("Mozilla-Firefox-3080".to_string(), "\"C:\\Program Files\\firefox.exe\" -os-autostart".to_string())));

        // Test value name with spaces
        let line2 = "    Free Download Manager    REG_SZ    \"C:\\Program Files\\fdm.exe\" --hidden";
        let res2 = parse_reg_line(line2);
        assert_eq!(res2, Some(("Free Download Manager".to_string(), "\"C:\\Program Files\\fdm.exe\" --hidden".to_string())));

        // Test REG_EXPAND_SZ value
        let line3 = "    OneDrive    REG_EXPAND_SZ    %USERPROFILE%\\AppData\\Local\\Microsoft\\OneDrive\\OneDrive.exe /background";
        let res3 = parse_reg_line(line3);
        assert_eq!(res3, Some(("OneDrive".to_string(), "%USERPROFILE%\\AppData\\Local\\Microsoft\\OneDrive\\OneDrive.exe /background".to_string())));

        // Test fallback split
        let line4 = "    Enpass    REG_SZ    C:\\Enpass.exe";
        let res4 = parse_reg_line(line4);
        assert_eq!(res4, Some(("Enpass".to_string(), "C:\\Enpass.exe".to_string())));
    }

    #[test]
    fn test_is_running_package_manager() {
        let mut names = HashSet::new();
        names.insert("Enpass".to_string());
        names.insert("Cloudflare WARP".to_string());
        names.insert("fdm".to_string());
        names.insert("Battle.net".to_string());
        names.insert("firefox".to_string());

        let snapshot = ProcessSnapshot {
            canonical_exes: HashSet::new(),
            names,
            cmdlines: Vec::new(),
        };

        // App 1: Cloudflare Warp
        let app1 = AppEntry {
            id: "Cloudflare.Warp".to_string(),
            name: "Cloudflare One Client".to_string(),
            install_type: crate::config::InstallType::InPlace,
            source_path: None,
            install_path: "Windows winget".to_string(),
            exec_path: "winget".to_string(),
            icon_path: None,
            desktop_file: "".to_string(),
            symlink_file: None,
            added_at: "".to_string(),
            is_custom: Some(true),
            start_cmd: None,
            stop_cmd: None,
            category: Some("System".to_string()),
            package_type: Some("Winget".to_string()),
            ..Default::default()
        };
        assert!(snapshot.is_running(&app1));

        // App 2: Enpass
        let app2 = AppEntry {
            id: "enpass".to_string(),
            name: "Enpass".to_string(),
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
        };
        assert!(snapshot.is_running(&app2));

        // App 3: Free Download Manager
        let app3 = AppEntry {
            id: "free-download-manager".to_string(),
            name: "Free Download Manager".to_string(),
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
        };
        assert!(snapshot.is_running(&app3));

        // App 4: Mozilla Firefox
        let app4 = AppEntry {
            id: "Mozilla.Firefox".to_string(),
            name: "Mozilla Firefox".to_string(),
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
        };
        assert!(snapshot.is_running(&app4));
    }

    #[test]
    #[cfg(windows)]
    fn test_find_exec_path_on_windows() {
        let app = AppEntry {
            id: "Notepad++".to_string(),
            name: "Notepad++".to_string(),
            install_type: crate::config::InstallType::InPlace,
            source_path: None,
            install_path: "Windows winget".to_string(),
            exec_path: "winget".to_string(),
            icon_path: None,
            desktop_file: "".to_string(),
            symlink_file: None,
            added_at: "".to_string(),
            is_custom: Some(true),
            start_cmd: None,
            stop_cmd: None,
            category: Some("System".to_string()),
            package_type: Some("Winget".to_string()),
            ..Default::default()
        };
        let resolved = find_exec_path_on_windows(&app);
        assert!(resolved.is_some(), "Should resolve Notepad++ executable path on Windows!");
        let path = resolved.unwrap();
        assert!(path.to_lowercase().contains("notepad++.exe"));
        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    #[cfg(windows)]
    fn test_registry_scanning() {
        let apps = scan_all_system_apps();
        let registry_apps: Vec<_> = apps.iter().filter(|a| a.package_type.as_deref() == Some("Registry")).collect();
        println!("Found {} registry apps", registry_apps.len());
        if !registry_apps.is_empty() {
            let app = &registry_apps[0];
            println!("Sample registry app: Name={}, ID={}, Key={:?}, UninstallCmd={:?}, Publisher={:?}, Version={:?}", 
                app.name, app.id, app.registry_key, app.uninstall_cmd, app.publisher, app.version);
            assert!(!app.name.is_empty());
            assert!(app.id.starts_with("ARP\\"));
        }
    }
}
