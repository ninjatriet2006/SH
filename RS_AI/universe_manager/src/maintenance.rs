use std::process::Command;
use std::fs;



#[derive(Clone, Debug)]
pub struct UpdateEntry {
    pub id: String,
    pub name: String,
    pub current_version: String,
    pub available_version: String,
    pub source: String,
}

fn check_git_updates(updates: &mut Vec<UpdateEntry>) {
    let config = crate::config::Config::load();
    for app in config.apps {
        let install_path = std::path::Path::new(&app.install_path);
        let git_dir = install_path.join(".git");
        if git_dir.exists() && git_dir.is_dir() {
            // Check git status
            if Command::new("git").current_dir(install_path).args(&["fetch"]).status().is_ok() {
                if let Ok(out) = Command::new("git").current_dir(install_path).args(&["rev-list", "HEAD...@{u}", "--count"]).output() {
                    let count_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if let Ok(count) = count_str.parse::<u32>() {
                        if count > 0 {
                            updates.push(UpdateEntry {
                                id: app.id.clone(),
                                name: app.name.clone(),
                                current_version: "local".to_string(),
                                available_version: format!("+{} commits", count),
                                source: "git".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
}

pub fn execute_updates(entries: Vec<&UpdateEntry>) -> Result<String, String> {
    let mut result = String::new();
    let config = crate::config::Config::load();
    for entry in entries {
        result.push_str(&format!("Đang cập nhật {} qua {}...\n", entry.name, entry.source));
        let status = match entry.source.as_str() {
            "winget" | "msstore" => {
                Command::new("winget").args(&["upgrade", "--id", &entry.id, "--silent", "--accept-package-agreements", "--accept-source-agreements", "--include-unknown"]).status()
            }
            "chocolatey" => {
                Command::new("choco").args(&["upgrade", &entry.id, "-y"]).status()
            }
            "scoop" => {
                Command::new("powershell").args(&["-NoProfile", "-Command", &format!("scoop update {}", entry.id)]).status()
            }
            "apt" => {
                Command::new("sudo").args(&["apt-get", "install", "--only-upgrade", "-y", &entry.id]).status()
            }
            "flatpak" => {
                Command::new("flatpak").args(&["update", "-y", &entry.id]).status()
            }
            "snap" => {
                Command::new("sudo").args(&["snap", "refresh", &entry.id]).status()
            }
            "git" => {
                if let Some(app) = config.apps.iter().find(|a| a.id == entry.id) {
                    Command::new("git").current_dir(&app.install_path).args(&["pull", "--rebase", "--autostash"]).status()
                } else {
                    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "App not found"))
                }
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Unknown source")),
        };
        match status {
            Ok(s) if s.success() => {
                result.push_str(&format!("-> Cập nhật {} thành công!\n", entry.name));
            }
            _ => {
                result.push_str(&format!("-> Cập nhật {} thất bại!\n", entry.name));
            }
        }
    }
    Ok(result)
}


#[cfg(target_os = "windows")]
pub fn check_system_updates() -> Result<Vec<UpdateEntry>, String> {
    let mut updates = Vec::new();

    // 1. Winget & MSStore
    if Command::new("winget").arg("--version").output().is_ok() {
        if let Ok(out) = Command::new("winget").args(&["upgrade", "--include-unknown"]).output() {
            let combined = String::from_utf8_lossy(&out.stdout).to_string();
            let mut started = false;
            for line in combined.lines() {
                let tline = line.trim();
                if tline.contains("Name") && tline.contains("Id") && tline.contains("Version") {
                    started = true;
                    continue;
                }
                if started {
                    if tline.starts_with('-') || tline.is_empty() { continue; }
                    // Skip summary and warning lines
                    if tline.ends_with("upgrades available.") || tline.contains("package(s) have version numbers that cannot be determined") || tline.starts_with("The following packages") {
                        continue;
                    }
                    if tline.contains("Name") && tline.contains("Id") && tline.contains("Version") {
                        continue;
                    }

                    let parts: Vec<&str> = tline.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let source = parts.last().unwrap().to_string();
                        if parts.len() >= 5 {
                            let available = parts[parts.len()-2].to_string();
                            let current = parts[parts.len()-3].to_string();
                            let id = parts[parts.len()-4].to_string();
                            let name = parts[..parts.len()-4].join(" ");
                            updates.push(UpdateEntry {
                                id, name, current_version: current, available_version: available, source
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Chocolatey
    if Command::new("choco").arg("--version").output().is_ok() {
        if let Ok(out) = Command::new("choco").arg("outdated").output() {
            let combined = String::from_utf8_lossy(&out.stdout).to_string();
            let mut started = false;
            for line in combined.lines() {
                let tline = line.trim();
                if tline.starts_with("Outdated Packages") {
                    started = true; continue;
                }
                if started && tline.contains("Chocolatey has determined") { break; }
                if started && tline.contains('|') {
                    if tline.starts_with("Output is Id") || tline.contains("Available Version") {
                        continue;
                    }
                    let parts: Vec<&str> = tline.split('|').collect();
                    if parts.len() >= 3 {
                        updates.push(UpdateEntry {
                            id: parts[0].trim().to_string(),
                            name: parts[0].trim().to_string(),
                            current_version: parts[1].trim().to_string(),
                            available_version: parts[2].trim().to_string(),
                            source: "chocolatey".to_string(),
                        });
                    }
                }
            }
        }
    }

    // 3. Scoop
    if Command::new("scoop").arg("--version").output().is_ok() {
        if let Ok(out) = Command::new("powershell").args(&["-NoProfile", "-Command", "scoop status"]).output() {
            let combined = String::from_utf8_lossy(&out.stdout).to_string();
            let mut started = false;
            for line in combined.lines() {
                let tline = line.trim();
                if tline.starts_with("Name") && tline.contains("Version") {
                    started = true; continue;
                }
                if started && tline.starts_with('-') { continue; }
                if started && tline.is_empty() { break; }
                if started {
                    let parts: Vec<&str> = tline.split_whitespace().collect();
                    if parts.len() >= 4 && parts[2] == "->" {
                        updates.push(UpdateEntry {
                            id: parts[0].to_string(),
                            name: parts[0].to_string(),
                            current_version: parts[1].to_string(),
                            available_version: parts[3].to_string(),
                            source: "scoop".to_string(),
                        });
                    }
                }
            }
        }
    }

    check_git_updates(&mut updates);

    Ok(updates)
}


#[cfg(not(target_os = "windows"))]
pub fn check_system_updates() -> Result<Vec<UpdateEntry>, String> {
    let mut updates = Vec::new();
    
    // 1. APT Simulation check (does not require root)
    if let Ok(output) = Command::new("apt-get").arg("-s").arg("upgrade").output() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        for line in stdout.lines() {
            if line.starts_with("Inst ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let id = parts[1].to_string();
                    let current = parts[2].trim_matches('[').trim_matches(']').to_string();
                    let available = parts[3].trim_matches('(').to_string();
                    updates.push(UpdateEntry {
                        id: id.clone(),
                        name: id,
                        current_version: current,
                        available_version: available,
                        source: "apt".to_string(),
                    });
                }
            }
        }
    }

    // 2. Flatpak update check
    if let Ok(output) = Command::new("flatpak").arg("update").arg("--check").output() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut started = false;
        for line in stdout.lines() {
            let tline = line.trim();
            if tline.starts_with("Name") && tline.contains("Application ID") {
                started = true; continue;
            }
            if started && tline.is_empty() { break; }
            if started {
                let parts: Vec<&str> = tline.split_whitespace().collect();
                if parts.len() >= 4 {
                    let id = parts[parts.len()-4].to_string();
                    let name = parts[..parts.len()-4].join(" ");
                    let available = parts[parts.len()-3].to_string();
                    updates.push(UpdateEntry {
                        id: id.clone(),
                        name,
                        current_version: "unknown".to_string(),
                        available_version: available,
                        source: "flatpak".to_string(),
                    });
                }
            }
        }
    }

    // 3. Snap check
    if let Ok(output) = Command::new("snap").arg("refresh").arg("--list").output() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut started = false;
        for line in stdout.lines() {
            let tline = line.trim();
            if tline.starts_with("Name") && tline.contains("Version") {
                started = true; continue;
            }
            if started && tline.is_empty() { break; }
            if started {
                let parts: Vec<&str> = tline.split_whitespace().collect();
                if parts.len() >= 3 {
                    let id = parts[0].to_string();
                    let current_version = "unknown".to_string();
                    let available_version = parts[1].to_string();
                    updates.push(UpdateEntry {
                        id: id.clone(),
                        name: id,
                        current_version,
                        available_version,
                        source: "snap".to_string(),
                    });
                }
            }
        }
    }

    check_git_updates(&mut updates);

    Ok(updates)
}

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

