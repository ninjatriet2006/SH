use std::fs;
use std::path::Path;
use std::process::Command;

use crate::config::{Config, InstallType};

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
        if let Err(e) = fs::remove_file(desktop_path) {
            println!("Cảnh báo: Không thể xoá file launcher .desktop: {}", e);
        }
    }

    // 2. Remove command-line symlink
    if let Some(ref symlink_str) = entry.symlink_file {
        let symlink_path = Path::new(symlink_str);
        if symlink_path.exists() || symlink_path.is_symlink() {
            if let Err(e) = fs::remove_file(symlink_path) {
                println!("Cảnh báo: Không thể xoá symlink command-line: {}", e);
            }
        }
    }

    // 3. Update Desktop Database
    if let Some(parent) = desktop_path.parent() {
        let _ = Command::new("update-desktop-database")
            .arg(parent.to_string_lossy().as_ref())
            .status();
    }

    // 4. Remove from configuration
    config.remove_app(app_id);
    config.save().map_err(|e| format!("Không thể cập nhật file cấu hình config.json: {}", e))?;

    Ok(())
}

/// Removes launchers, symlinks, and deletes the application folder/files safely (only for local portable apps).
pub fn uninstall_local_portable(entry: &crate::config::AppEntry) -> Result<(), String> {
    let app_id = &entry.id;
    // Store install_path and install_type before unintegrating
    let install_path_str = entry.install_path.clone();
    let install_path = Path::new(&install_path_str);
    let exec_path_str = entry.exec_path.clone();
    let exec_path = Path::new(&exec_path_str);
    let icon_path_str = entry.icon_path.clone();
    let install_type = entry.install_type.clone();

    // 1. First, perform unintegration (removes desktop and symlinks, updates config)
    unintegrate(app_id)?;

    // 2. Safety Check: Verify folder is safe to delete
    let home = dirs::home_dir().ok_or("Không thể xác định thư mục Home")?;
    
    // Safety boundaries: Do not delete home, root, or top-level directories
    if install_path == Path::new("/") || install_path == home {
        return Err("Bảo vệ an toàn: Không thể xoá thư mục gốc (root) hoặc thư mục Home!".to_string());
    }

    // Check against standard directories to avoid accidental deletion
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

    // 3. Delete files based on installation type
    match install_type {
        InstallType::Moved => {
            // Since it was moved to a managed subdirectory (e.g. ~/Applications/app_id/),
            // we can safely delete the entire folder.
            if install_path.exists() && install_path.is_dir() {
                // Double check that the install path contains the app_id as the final component to be absolutely sure
                if let Some(filename) = install_path.file_name() {
                    if filename.to_string_lossy() == *app_id {
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
            // For In-Place installations, the files are sitting in an arbitrary folder (e.g., Downloads, Documents).
            // We MUST NOT delete the entire folder, because it could contain other user files!
            // Instead, we only delete the executable binary and the icon we registered, if they exist.
            if exec_path.exists() && exec_path.is_file() {
                fs::remove_file(exec_path)
                    .map_err(|e| format!("Lỗi khi xoá file chạy ứng dụng: {}", e))?;
            }

            if let Some(ref icon_str) = icon_path_str {
                let icon_path = Path::new(icon_str);
                // Only delete the icon if it lies within the app's folder or next to it and is a file
                if icon_path.exists() && icon_path.is_file() {
                    let _ = fs::remove_file(icon_path);
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
                _ => Err(format!("Kiểu ứng dụng không hỗ trợ gỡ cài đặt: {:?}", entry.package_type)),
            }
        } else {
            Err(format!("Không tìm thấy ứng dụng với ID: {}", app_id))
        }
    }
}
