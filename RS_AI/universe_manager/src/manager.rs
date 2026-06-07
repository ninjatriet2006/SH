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

/// A snapshot of all running processes in the system, collected in a single scan of /proc.
#[derive(Clone)]
pub struct ProcessSnapshot {
    pub canonical_exes: HashSet<PathBuf>,
    pub names: HashSet<String>,
    pub cmdlines: Vec<String>,
}

impl ProcessSnapshot {
    pub fn collect() -> Self {
        let mut canonical_exes = HashSet::new();
        let mut names = HashSet::new();
        let mut cmdlines = Vec::new();
        
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
        
        ProcessSnapshot {
            canonical_exes,
            names,
            cmdlines,
        }
    }
    
    pub fn is_running(&self, app: &AppEntry) -> bool {
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
        
        false
    }
}

/// Checks if the application process is currently running.
pub fn is_app_running(app: &AppEntry) -> bool {
    let is_custom = app.is_custom.unwrap_or(false);
    
    let exec_path_buf = PathBuf::from(&app.exec_path);
    let exec_canonical = fs::canonicalize(&exec_path_buf).ok();
    
    let exec_name = exec_path_buf.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.chars().all(|c| c.is_ascii_digit()) {
                        // Method 1: Check executable path symbolic link
                        let exe_link = entry.path().join("exe");
                        if let Ok(target) = fs::read_link(&exe_link) {
                            if let Some(ref canon) = exec_canonical {
                                if &target == canon {
                                    return true;
                                }
                            }
                            if !exec_name.is_empty() && target.to_string_lossy().contains(&exec_name) {
                                return true;
                            }
                        }
                        
                        // Method 2: Check cmdline arguments
                        let cmd_file = entry.path().join("cmdline");
                        if let Ok(cmd_bytes) = fs::read(cmd_file) {
                            let cmd_str = String::from_utf8_lossy(&cmd_bytes).replace('\0', " ");
                            if is_custom {
                                let real_id = get_real_id(&app.id);
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
                    }
                }
            }
        }
    }
    false
}

/// Spawns the application process in the background.
pub fn start_app(app: &AppEntry) -> Result<(), String> {
    let stdout = fs::File::create("/dev/null").map(std::process::Stdio::from).unwrap_or(std::process::Stdio::null());
    let stderr = fs::File::create("/dev/null").map(std::process::Stdio::from).unwrap_or(std::process::Stdio::null());
    
    if app.is_custom.unwrap_or(false) {
        if let Some(ref start_cmd) = app.start_cmd {
            Command::new("sh")
                .arg("-c")
                .arg(start_cmd)
                .stdout(stdout)
                .stderr(stderr)
                .spawn()
                .map_err(|e| format!("Lỗi khởi chạy (custom command): {}", e))?;
        } else {
            Command::new(&app.exec_path)
                .stdout(stdout)
                .stderr(stderr)
                .spawn()
                .map_err(|e| format!("Lỗi khởi chạy: {}", e))?;
        }
    } else {
        Command::new(&app.exec_path)
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .map_err(|e| format!("Lỗi khởi chạy: {}", e))?;
    }
    Ok(())
}

/// Stops the application process(es).
pub fn stop_app(app: &AppEntry) -> Result<(), String> {
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

    // Default fallback: Scan PIDs matching executable or config, then kill them.
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
    })
}

/// Scans all applications installed on the system (APT, Flatpak, Snap, Local).
pub fn scan_all_system_apps() -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen_ids = HashSet::new();
    
    // 1. Add locally registered portable apps first
    let config = Config::load();
    for mut app in config.apps {
        app.package_type = Some("Local".to_string());
        if app.category.is_none() {
            app.category = Some("Utility".to_string());
        }
        seen_ids.insert(app.id.clone());
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
                            apps.push(app_entry);
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

/// Scans and cleans system leftovers (APT rc packages and Flatpak unused runtimes).
pub fn clean_system_leftovers() -> Result<String, String> {
    let mut result = String::new();
    result.push_str("=== BẮT ĐẦU DỌN DẸP HỆ THỐNG (LEFTOVERS) ===\n\n");
    
    // 1. APT leftovers (purging 'rc' state packages)
    result.push_str("[1/2] Đang quét cấu hình cũ của APT (trạng thái 'rc')...\n");
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
    result.push_str("[2/2] Đang dọn dẹp Flatpak runtimes không sử dụng...\n");
    match clean_flatpak_unused() {
        Ok(out) => {
            result.push_str("-> Hoàn tất dọn dẹp Flatpak:\n");
            result.push_str(&out);
        }
        Err(e) => {
            result.push_str(&format!("-> Lỗi dọn Flatpak: {}\n", e));
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
}
