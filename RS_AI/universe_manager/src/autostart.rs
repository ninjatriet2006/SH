use std::fs;
use std::path::{Path, PathBuf};
use crate::config::AppEntry;
use std::process::Command;

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
                if let Some((name, exec)) = crate::manager::parse_reg_line(trimmed) {
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
                if let Some((name, exec)) = crate::manager::parse_reg_line(trimmed) {
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

