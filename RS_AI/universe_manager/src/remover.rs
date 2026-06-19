use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::config::{Config, InstallType};
use dialoguer::Confirm;

/// Removes launcher and symlinks but leaves application files intact.
pub fn unintegrate(app_id: &str) -> Result<(), String> {
    let mut config = Config::load();
    
    // Find the app entry
    let entry_opt = config.apps.iter().find(|a| a.id == app_id).cloned();
    
    let entry = match entry_opt {
        Some(e) => e,
        None => return Err(format!("Không tìm thấy ứng dụng với ID: {}", app_id)),
    };

    // 1. Remove Desktop Launcher file
    let desktop_path = Path::new(&entry.desktop_file);
    if desktop_path.exists() {
        let _ = fs::remove_file(desktop_path);
    }

    // 2. Remove command-line symlink
    if let Some(ref symlink_str) = entry.symlink_file {
        let symlink_path = Path::new(symlink_str);
        if symlink_path.exists() || symlink_path.is_symlink() {
            let _ = fs::remove_file(symlink_path);
        }
    }

    // 3. Update Desktop Database
    #[cfg(unix)]
    if let Some(parent) = desktop_path.parent() {
        let _ = Command::new("update-desktop-database")
            .arg(parent.to_string_lossy().as_ref())
            .status();
    }

    // Clean MIME associations on Linux
    #[cfg(unix)]
    clean_mime_associations(app_id);

    // 4. Remove from configuration
    config.remove_app(app_id);
    let _ = config.save();

    Ok(())
}

/// Helper to compress and backup a leftovers directory into a archive (.zip on Windows, .tar.gz on Linux).
pub fn backup_leftover_dir(dir_path: &Path) -> Result<PathBuf, String> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| std::env::temp_dir());
    let backup_root = cache_dir.join("universe-manager").join("backups");
    let _ = fs::create_dir_all(&backup_root);

    let folder_name = dir_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();

    #[cfg(windows)]
    let extension = "zip";
    #[cfg(not(windows))]
    let extension = "tar.gz";

    let backup_file = backup_root.join(format!("{}_{}.{}", folder_name, timestamp, extension));
    let parent = dir_path.parent().ok_or("Đường dẫn không hợp lệ")?;
    
    #[cfg(windows)]
    let status = Command::new("tar")
        .arg("-a") // auto-detect format from extension (will create zip)
        .arg("-c")
        .arg("-f")
        .arg(&backup_file)
        .arg("-C")
        .arg(parent)
        .arg(&folder_name)
        .status();

    #[cfg(not(windows))]
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&backup_file)
        .arg("-C")
        .arg(parent)
        .arg(&folder_name)
        .status();

    match status {
        Ok(s) if s.success() => Ok(backup_file),
        Ok(s) => Err(format!("Lệnh tar kết thúc với mã lỗi: {:?}", s.code())),
        Err(e) => Err(format!("Không thể chạy công cụ sao lưu: {}", e)),
    }
}

/// Helper to clean MIME type associations for Linux.
#[cfg(unix)]
pub fn clean_mime_associations(app_id: &str) {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let paths = vec![
        home.join(".config").join("mimeapps.list"),
        home.join(".local/share/applications/mimeapps.list"),
    ];

    let desktop_filename = format!("{}.desktop", app_id);

    for path in paths {
        if !path.exists() || !path.is_file() {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            let mut new_lines = Vec::new();
            let mut modified = false;

            for line in content.lines() {
                if line.contains(&desktop_filename) {
                    if let Some(eq_idx) = line.find('=') {
                        let mimetype = &line[..eq_idx];
                        let apps_list = &line[eq_idx + 1..];
                        let cleaned_apps: Vec<&str> = apps_list.split(';')
                            .filter(|app| !app.is_empty() && *app != desktop_filename.as_str())
                            .collect();
                        
                        if cleaned_apps.is_empty() {
                            modified = true;
                            continue;
                        } else {
                            let new_line = format!("{}={};", mimetype, cleaned_apps.join(";"));
                            new_lines.push(new_line);
                            modified = true;
                        }
                    }
                } else {
                    new_lines.push(line.to_string());
                }
            }

            if modified {
                let _ = fs::write(&path, new_lines.join("\n") + "\n");
            }
        }
    }
}

/// Finds orphaned application configurations/data on the system.
pub fn find_orphaned_leftovers() -> Vec<PathBuf> {
    let all_scanned = crate::manager::scan_all_system_apps();
    let mut active_names = std::collections::HashSet::new();
    let mut active_ids = std::collections::HashSet::new();

    for app in all_scanned {
        active_ids.insert(app.id.clone());
        let real_id = app.id.strip_suffix("-flatpak")
            .or_else(|| app.id.strip_suffix("-snap"))
            .unwrap_or(&app.id);
        active_ids.insert(real_id.to_string());
        active_names.insert(app.name.to_lowercase());
        
        for word in app.name.split_whitespace() {
            let clean_word = word.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "");
            if clean_word.len() > 2 {
                active_names.insert(clean_word);
            }
        }
    }

    let mut whitelist = std::collections::HashSet::new();
    let common_wl = &[
        "applications", "icons", "themes", "pulse", "dconf", "systemd", "mime", 
        "menus", "gnome-session", "flatpak", "universe-manager", "crossterm", 
        "dbus-1", "microsoft", "windows", "packages", "temp", "desktop", "adobe",
        "google", "mozilla", "nvidia", "intel", "amd", "git", "cargo", "rustup",
        "npm", "node", "python", "pip", "downloads", "documents", "pictures",
        "music", "videos", "autostart", "kde.org", "qt5ct", "qt6ct", "fontconfig",
        "trash", "keyrings", "sounds", "mesa_shader_cache", "thumbnails"
    ];
    for &wl in common_wl {
        whitelist.insert(wl.to_string());
    }

    let mut leftovers = Vec::new();
    #[cfg(unix)]
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    
    let mut dirs_to_scan = Vec::new();
    #[cfg(unix)]
    {
        dirs_to_scan.push(home.join(".config"));
        dirs_to_scan.push(home.join(".local/share"));
        dirs_to_scan.push(home.join(".cache"));
        dirs_to_scan.push(home.join(".var/app"));
        dirs_to_scan.push(home.join("snap"));
    }
    
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            dirs_to_scan.push(PathBuf::from(appdata));
        }
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            dirs_to_scan.push(PathBuf::from(localappdata));
        }
        if let Ok(programdata) = std::env::var("PROGRAMDATA") {
            dirs_to_scan.push(PathBuf::from(programdata));
        }
    }

    for scan_dir in dirs_to_scan {
        if !scan_dir.exists() || !scan_dir.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&scan_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        let dir_path = entry.path();
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        let folder_lower = folder_name.to_lowercase();

                        if whitelist.contains(&folder_lower) {
                            continue;
                        }

                        let mut is_matched = false;
                        if active_ids.contains(&folder_name) || active_ids.contains(&folder_lower) {
                            is_matched = true;
                        }
                        
                        if !is_matched {
                            for active_name in &active_names {
                                if folder_lower == *active_name || (active_name.len() > 3 && folder_lower.contains(active_name)) {
                                    is_matched = true;
                                    break;
                                }
                            }
                        }

                        if !is_matched {
                            leftovers.push(dir_path);
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        let registry_base = "HKCU\\Software";
        let output = Command::new("reg")
            .args(&["query", registry_base])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.starts_with(registry_base) {
                    if let Some(key_name) = line.split('\\').last() {
                        let key_lower = key_name.to_lowercase();
                        if whitelist.contains(&key_lower) {
                            continue;
                        }
                        let mut is_matched = false;
                        if active_ids.contains(key_name) || active_ids.contains(&key_lower) {
                            is_matched = true;
                        }
                        if !is_matched {
                            for active_name in &active_names {
                                if key_lower == *active_name || (active_name.len() > 3 && key_lower.contains(active_name)) {
                                    is_matched = true;
                                    break;
                                }
                            }
                        }
                        if !is_matched {
                            leftovers.push(PathBuf::from(line));
                        }
                    }
                }
            }
        }
    }

    leftovers.sort();
    leftovers.dedup();
    leftovers
}

/// Removes launchers, symlinks, and deletes the application folder/files safely (only for local portable apps).
pub fn uninstall_local_portable(entry: &crate::config::AppEntry) -> Result<(), String> {
    let app_id = &entry.id;
    let install_path_str = entry.install_path.clone();
    let install_path = Path::new(&install_path_str);
    let exec_path_str = entry.exec_path.clone();
    let exec_path = Path::new(&exec_path_str);
    let icon_path_str = entry.icon_path.clone();
    let install_type = entry.install_type.clone();

    // 1. First, perform unintegration (removes desktop and symlinks, updates config)
    let _ = unintegrate(app_id);

    // 2. Scan and clean leftovers for this specific application
    let mut leftover_dirs = Vec::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let real_id = app_id.strip_suffix("-flatpak")
        .or_else(|| app_id.strip_suffix("-snap"))
        .unwrap_or(app_id);

    #[cfg(unix)]
    {
        leftover_dirs.push(home.join(".config").join(real_id));
        leftover_dirs.push(home.join(".local/share").join(real_id));
        leftover_dirs.push(home.join(".cache").join(real_id));
        leftover_dirs.push(home.join(".var/app").join(real_id));
        leftover_dirs.push(home.join("snap").join(real_id));
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            leftover_dirs.push(PathBuf::from(appdata).join(real_id));
        }
        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            leftover_dirs.push(PathBuf::from(localappdata).join(real_id));
        }
    }

    for p in leftover_dirs {
        if p.exists() && p.is_dir() {
            println!("Phát hiện thư mục cấu hình rác: {:?}", p);
            let confirm_del = Confirm::new()
                .with_prompt("Bạn có muốn dọn dẹp thư mục cấu hình này không?")
                .default(true)
                .interact()
                .unwrap_or(false);
            if confirm_del {
                if let Ok(backup_file) = backup_leftover_dir(&p) {
                    println!("Đã sao lưu cấu hình tại: {:?}", backup_file);
                }
                let _ = fs::remove_dir_all(&p);
            }
        }
    }

    #[cfg(windows)]
    {
        let reg_keys = vec![
            format!("HKCU\\Software\\{}", real_id),
            format!("HKCU\\Software\\{}", entry.name),
        ];
        for key in reg_keys {
            let check = Command::new("reg")
                .args(&["query", &key])
                .output();
            if let Ok(out) = check {
                if out.status.success() {
                    println!("Phát hiện Registry Key rác: {}", key);
                    let confirm_del = Confirm::new()
                        .with_prompt("Bạn có muốn xoá Registry Key này không?")
                        .default(true)
                        .interact()
                        .unwrap_or(false);
                    if confirm_del {
                        let _ = Command::new("reg")
                            .args(&["delete", &key, "/f"])
                            .status();
                    }
                }
            }
        }
    }

    // 3. Safety Check: Verify folder is safe to delete
    if install_path == Path::new("/") || install_path == home {
        return Err("Bảo vệ an toàn: Không thể xoá thư mục gốc (root) hoặc thư mục Home!".to_string());
    }

    let downloads = home.join("Downloads");
    let documents = home.join("Documents");
    let desktop = home.join("Desktop");
    let pictures = home.join("Pictures");
    let music = home.join("Music");
    let videos = home.join("Videos");

    if install_path == downloads || install_path == documents || install_path == desktop 
       || install_path == pictures || install_path == music || install_path == videos {
        return Err(format!(
            "Bảo vệ an toàn: Thư mục ứng dụng trỏ tới thư mục hệ thống: {:?}. Sẽ không xoá thư mục này.", 
            install_path
        ));
    }

    // 4. Delete files based on installation type
    match install_type {
        InstallType::Moved => {
            if install_path.exists() && install_path.is_dir() {
                if let Some(filename) = install_path.file_name() {
                    if filename.to_string_lossy() == *app_id {
                        // Backup before delete
                        if let Ok(backup_file) = backup_leftover_dir(install_path) {
                            println!("Đã sao lưu thư mục cài đặt tại: {:?}", backup_file);
                        }
                        fs::remove_dir_all(install_path)
                            .map_err(|e| format!("Lỗi khi xoá thư mục ứng dụng: {}", e))?;
                    } else {
                        return Err(format!(
                            "Bảo vệ an toàn: Thư mục ứng dụng '{:?}' không khớp với App ID '{}'. Không thể xoá.",
                            install_path, app_id
                        ));
                    }
                }
            }
        }
        InstallType::InPlace => {
            // Check if executable resides in a dedicated folder
            let exe_parent = exec_path.parent();
            if let Some(parent_path) = exe_parent {
                if parent_path != home 
                   && parent_path != downloads 
                   && parent_path != documents 
                   && parent_path != desktop
                   && parent_path != Path::new("/")
                   && parent_path.exists()
                {
                    println!("\nPhát hiện ứng dụng In-Place nằm trong thư mục riêng biệt: {:?}", parent_path);
                    let confirm_del = Confirm::new()
                        .with_prompt("Bạn có muốn xoá toàn bộ thư mục chứa ứng dụng này không?")
                        .default(false)
                        .interact()
                        .unwrap_or(false);

                    if confirm_del {
                        if let Ok(backup_file) = backup_leftover_dir(parent_path) {
                            println!("Đã sao lưu thư mục ứng dụng tại: {:?}", backup_file);
                        }
                        let _ = fs::remove_dir_all(parent_path);
                    } else {
                        if exec_path.exists() && exec_path.is_file() {
                            let _ = fs::remove_file(exec_path);
                        }
                        if let Some(ref icon_str) = icon_path_str {
                            let icon_path = Path::new(icon_str);
                            if icon_path.exists() && icon_path.is_file() {
                                let _ = fs::remove_file(icon_path);
                            }
                        }
                    }
                } else {
                    if exec_path.exists() && exec_path.is_file() {
                        let _ = fs::remove_file(exec_path);
                    }
                    if let Some(ref icon_str) = icon_path_str {
                        let icon_path = Path::new(icon_str);
                        if icon_path.exists() && icon_path.is_file() {
                            let _ = fs::remove_file(icon_path);
                        }
                    }
                }
            } else {
                if exec_path.exists() && exec_path.is_file() {
                    let _ = fs::remove_file(exec_path);
                }
            }
        }
    }

    Ok(())
}

/// Removes launchers, symlinks, and deletes the application folder/files safely.
pub fn uninstall(app_id: &str) -> Result<(), String> {
    let config = Config::load();
    let entry_opt = config.apps.iter().find(|a| a.id == app_id).cloned();

    if let Some(entry) = entry_opt {
        uninstall_local_portable(&entry)
    } else {
        let system_apps = crate::manager::scan_all_system_apps();
        if let Some(entry) = system_apps.iter().find(|a| a.id == app_id) {
            match entry.package_type.as_deref() {
                Some("Flatpak") => {
                    let real_id = app_id.strip_suffix("-flatpak").unwrap_or(app_id);
                    let status = Command::new("flatpak")
                        .arg("uninstall")
                        .arg("-y")
                        .arg(real_id)
                        .status()
                        .map_err(|e| format!("Lỗi thực thi flatpak uninstall: {}", e))?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Lệnh flatpak trả về mã lỗi: {:?}", status.code()))
                    }
                }
                Some("Snap") => {
                    let real_id = app_id.strip_suffix("-snap").unwrap_or(app_id);
                    let status = Command::new("sudo")
                        .arg("snap")
                        .arg("remove")
                        .arg(real_id)
                        .status()
                        .map_err(|e| format!("Lỗi thực thi snap remove: {}", e))?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Lệnh snap remove trả về mã lỗi: {:?}", status.code()))
                    }
                }
                Some("APT") => {
                    let status = Command::new("sudo")
                        .arg("apt-get")
                        .arg("purge")
                        .arg("-y")
                        .arg(app_id)
                        .status()
                        .map_err(|e| format!("Lỗi thực thi apt-get purge: {}", e))?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Lệnh apt-get purge trả về mã lỗi: {:?}", status.code()))
                    }
                }
                Some("Homebrew") => {
                    let status = Command::new("brew")
                        .arg("uninstall")
                        .arg(app_id)
                        .status()
                        .map_err(|e| format!("Lỗi thực thi brew uninstall: {}", e))?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Lệnh brew uninstall trả về mã lỗi: {:?}", status.code()))
                    }
                }
                Some("Winget") | Some("MSIX") => {
                    let status = Command::new("winget")
                        .arg("uninstall")
                        .arg("--id")
                        .arg(app_id)
                        .arg("-h") // Silent uninstall
                        .status()
                        .map_err(|e| format!("Lỗi thực thi winget uninstall: {}", e))?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Lệnh winget uninstall trả về mã lỗi: {:?}", status.code()))
                    }
                }
                Some("Scoop") => {
                    let status = Command::new("scoop")
                        .arg("uninstall")
                        .arg(app_id)
                        .status()
                        .map_err(|e| format!("Lỗi thực thi scoop uninstall: {}", e))?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Lệnh scoop uninstall trả về mã lỗi: {:?}", status.code()))
                    }
                }
                Some("Chocolatey") => {
                    let status = Command::new("choco")
                        .arg("uninstall")
                        .arg(app_id)
                        .arg("-y")
                        .status()
                        .map_err(|e| format!("Lỗi thực thi choco uninstall: {}", e))?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!("Lệnh choco uninstall trả về mã lỗi: {:?}", status.code()))
                    }
                }
                Some("Registry") => {
                    if let Some(ref uninstall_cmd) = entry.uninstall_cmd {
                        let status = Command::new("cmd")
                            .args(&["/c", uninstall_cmd])
                            .status()
                            .map_err(|e| format!("Lỗi thực thi lệnh gỡ cài đặt Registry: {}", e))?;
                        if status.success() {
                            Ok(())
                        } else {
                            Err(format!("Lệnh gỡ cài đặt trả về mã lỗi: {:?}", status.code()))
                        }
                    } else {
                        Err("Không tìm thấy lệnh gỡ cài đặt (UninstallString)".to_string())
                    }
                }
                _ => Err(format!("Kiểu ứng dụng không hỗ trợ gỡ cài đặt: {:?}", entry.package_type)),
            }
        } else {
            Err(format!("Không tìm thấy ứng dụng với ID: {}", app_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_backup_leftover_dir() {
        let dir = tempdir().unwrap();
        let test_sub_dir = dir.path().join("my_app_config");
        fs::create_dir(&test_sub_dir).unwrap();
        
        let file_path = test_sub_dir.join("config.json");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{{\"theme\": \"dark\"}}").unwrap();

        let backup_res = backup_leftover_dir(&test_sub_dir);
        assert!(backup_res.is_ok(), "Backup should succeed: {:?}", backup_res.err());
        let backup_path = backup_res.unwrap();
        assert!(backup_path.exists());
        
        // Clean up the backup file
        let _ = fs::remove_file(backup_path);
    }
}
