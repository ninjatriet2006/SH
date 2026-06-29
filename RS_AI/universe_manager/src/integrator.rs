use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink;
use chrono::Local;

use crate::config::{Config, AppEntry, InstallType};

/// Helper to recursively copy directories
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Helper to robustly move a file or folder (handles cross-device moves)
fn move_path(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Try rename first (fast)
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }

    // Fallback if renaming across filesystems
    if src.is_dir() {
        copy_dir_all(src, dst)?;
        fs::remove_dir_all(src)?;
    } else {
        fs::copy(src, dst)?;
        fs::remove_file(src)?;
    }
    Ok(())
}

pub struct IntegrationParams {
    pub name: String,
    pub source_path: PathBuf,
    pub install_type: InstallType,
    pub exec_rel_path: PathBuf, // Relative to the install folder (or empty if source is a single binary)
    pub icon_path: Option<PathBuf>, // Original icon path (relative or absolute)
    pub create_symlink: bool,
    pub symlink_name: Option<String>,
    pub categories: Option<String>,
    pub comment: Option<String>,
    pub terminal: bool,
}

pub fn integrate(params: IntegrationParams) -> Result<AppEntry, String> {
    let source_canon = params.source_path.canonicalize()
        .map_err(|e| format!("Lỗi đường dẫn gốc: {}", e))?;
    
    // 1. Generate unique App ID
    let app_id = params.name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
        .replace(' ', "-");
    
    let home = dirs::home_dir().ok_or("Không thể xác định thư mục Home")?;
    
    // Clean up existing app of same ID if any
    {
        let config = Config::load();
        if let Some(existing) = config.apps.iter().find(|a| a.id == app_id) {
            println!("[Update] Phát hiện ứng dụng '{}' đã tồn tại. Đang làm sạch phiên bản cũ...", app_id);
            
            // 1. Clean up old symlink
            if let Some(ref sym) = existing.symlink_file {
                let sym_path = Path::new(sym);
                if sym_path.exists() || sym_path.is_symlink() {
                    let _ = fs::remove_file(sym_path);
                }
            }
            
            // 2. Clean up old desktop launcher
            let desktop_path = Path::new(&existing.desktop_file);
            if desktop_path.exists() {
                let _ = fs::remove_file(desktop_path);
            }
            
            // 3. If Moved, clean up the old install folder
            if existing.install_type == InstallType::Moved {
                let old_install_path = Path::new(&existing.install_path);
                if old_install_path.exists() && old_install_path != source_canon {
                    let _ = fs::remove_dir_all(old_install_path);
                }
            }
        }
    }
    
    // 2. Determine installation paths
    let install_path = match params.install_type {
        InstallType::InPlace => {
            if source_canon.is_file() {
                source_canon.parent().unwrap_or(Path::new(".")).to_path_buf()
            } else {
                source_canon.clone()
            }
        }
        InstallType::Moved => {
            let config = Config::load();
            let managed_dir = PathBuf::from(&config.settings.managed_dir);
            managed_dir.join(&app_id)
        }
    };

    // 3. Perform move if requested
    if params.install_type == InstallType::Moved {
        let target_dest = if source_canon.is_file() {
            // It's a single file (like an AppImage), we move it to install_path/<filename>
            let filename = source_canon.file_name().ok_or("Tên file gốc không hợp lệ")?;
            install_path.join(filename)
        } else {
            // It's a directory, move the entire directory to install_path
            install_path.clone()
        };

        if source_canon != target_dest {
            if target_dest.exists() {
                // Ask for removal or handle safely, here we overwrite or remove the target first
                let _ = fs::remove_dir_all(&target_dest);
                let _ = fs::remove_file(&target_dest);
            }

            move_path(&source_canon, &target_dest)
                .map_err(|e| format!("Không thể sao chép/di chuyển ứng dụng vào thư mục quản lý: {}", e))?;
        }
    }

    // 4. Determine final executable path
    let final_exec_path = if source_canon.is_file() {
        let filename = source_canon.file_name().ok_or("Tên file không hợp lệ")?;
        install_path.join(filename)
    } else {
        install_path.join(&params.exec_rel_path)
    };

    if !final_exec_path.exists() {
        return Err(format!("Không tìm thấy file chạy tại: {:?}", final_exec_path));
    }

    // 5. Ensure executable permission
    #[cfg(unix)]
    if let Ok(metadata) = fs::metadata(&final_exec_path) {
        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        if mode & 0o111 == 0 {
            permissions.set_mode(mode | 0o111); // Add executable permission (+x)
            let _ = fs::set_permissions(&final_exec_path, permissions);
        }
    }

    // 6. Handle Icon
    let mut final_icon_path = None;
    if let Some(ref orig_icon) = params.icon_path {
        // Resolve path to icon. If we moved the directory, check if the icon was inside the source directory,
        // and if so, map it to the new install directory.
        let resolved_icon = if params.install_type == InstallType::Moved && !source_canon.is_file() {
            if let Ok(rel) = orig_icon.strip_prefix(&source_canon) {
                install_path.join(rel)
            } else {
                orig_icon.clone()
            }
        } else {
            orig_icon.clone()
        };

        if resolved_icon.exists() {
            let icon_ext = resolved_icon.extension().unwrap_or_default().to_string_lossy();
            let target_icon = install_path.join(format!("{}.{}", app_id, icon_ext));
            
            // Only copy if it's not already at the target
            if resolved_icon.canonicalize().ok() != target_icon.canonicalize().ok() {
                if let Err(e) = fs::copy(&resolved_icon, &target_icon) {
                    println!("Cảnh báo: Không thể sao chép icon: {}", e);
                    // Fallback to original icon path
                    final_icon_path = Some(resolved_icon.to_string_lossy().to_string());
                } else {
                    final_icon_path = Some(target_icon.to_string_lossy().to_string());
                }
            } else {
                final_icon_path = Some(target_icon.to_string_lossy().to_string());
            }
        } else {
            // Icon path was just suggested or doesn't exist, we save the string
            final_icon_path = Some(resolved_icon.to_string_lossy().to_string());
        }
    }

    // 7. Create desktop launcher entry
    let desktop_dir = home.join(".local").join("share").join("applications");
    let _ = fs::create_dir_all(&desktop_dir);
    let desktop_file_path = desktop_dir.join(format!("{}.desktop", app_id));

    let categories = params.categories.unwrap_or_else(|| "Utility;".to_string());
    let comment = params.comment.unwrap_or_else(|| format!("Được tích hợp bởi Universe Manager: {}", params.name));
    
    let icon_line = match &final_icon_path {
        Some(path) => format!("Icon={}\n", path),
        None => String::new(),
    };

    let desktop_content = format!(
        "[Desktop Entry]\n\
        Type=Application\n\
        Name={}\n\
        Comment={}\n\
        Exec=\"{}\" %U\n\
        Path={}\n\
        {}\
        Terminal={}\n\
        Categories={}\n\
        X-Integrated-By=universe-manager\n",
        params.name,
        comment,
        final_exec_path.to_string_lossy(),
        install_path.to_string_lossy(),
        icon_line,
        params.terminal,
        categories
    );

    fs::write(&desktop_file_path, desktop_content)
        .map_err(|e| format!("Không thể ghi file launcher .desktop: {}", e))?;

    // Set +x permission on .desktop file just in case
    #[cfg(unix)]
    if let Ok(metadata) = fs::metadata(&desktop_file_path) {
        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        permissions.set_mode(mode | 0o111);
        let _ = fs::set_permissions(&desktop_file_path, permissions);
    }

    // 8. Create symlink in ~/.local/bin/
    let mut final_symlink_path = None;
    if params.create_symlink {
        let bin_dir = home.join(".local").join("bin");
        let _ = fs::create_dir_all(&bin_dir);
        
        let symlink_name = params.symlink_name.unwrap_or_else(|| app_id.clone());
        let symlink_path = bin_dir.join(&symlink_name);

        // Remove if exists to prevent conflict
        if symlink_path.exists() || symlink_path.is_symlink() {
            let _ = fs::remove_file(&symlink_path);
        }

        if let Err(e) = symlink(&final_exec_path, &symlink_path) {
            println!("Cảnh báo: Không thể tạo link command-line tại {:?}: {}", symlink_path, e);
        } else {
            final_symlink_path = Some(symlink_path.to_string_lossy().to_string());
        }
    }

    // 9. Update Desktop Database
    let _ = Command::new("update-desktop-database")
        .arg(desktop_dir.to_string_lossy().as_ref())
        .status();

    // 10. Save to Config
    let mut config = Config::load();
    let entry = AppEntry {
        id: app_id.clone(),
        name: params.name,
        install_type: params.install_type,
        source_path: Some(source_canon.to_string_lossy().to_string()),
        install_path: install_path.to_string_lossy().to_string(),
        exec_path: final_exec_path.to_string_lossy().to_string(),
        icon_path: final_icon_path,
        desktop_file: desktop_file_path.to_string_lossy().to_string(),
        symlink_file: final_symlink_path,
        added_at: Local::now().to_rfc3339(),
        is_custom: None,
        start_cmd: None,
        stop_cmd: None,
        category: Some(categories.replace(';', "")),
        package_type: Some("Local".to_string()),
        ..Default::default()
    };

    config.add_app(entry.clone());
    config.save().map_err(|e| format!("Không thể lưu cấu hình config.json: {}", e))?;

    Ok(entry)
}
