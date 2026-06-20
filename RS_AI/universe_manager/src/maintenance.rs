use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};

pub fn check_system_updates() -> Result<String, String> {
    let mut result = String::new();

    // 1. Winget upgrade check
    result.push_str("=== KIỂM TRA CẬP NHẬT WINGET ===\n");
    if Command::new("winget").arg("--version").output().is_ok() {
        match Command::new("winget").args(&["upgrade", "--source", "winget"]).output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}{}", stdout, stderr);
                if combined.contains("No upgrades available") || combined.contains("No installed package found matching input criteria") {
                    result.push_str("-> Không có bản cập nhật Winget nào khả dụng.\n");
                } else {
                    let mut lines_to_print = Vec::new();
                    for line in combined.lines() {
                        let trim_line = line.trim();
                        if trim_line.is_empty() 
                            || trim_line.starts_with('-') 
                            || trim_line.starts_with('\\') 
                            || trim_line.starts_with('|') 
                            || trim_line.starts_with('/')
                        {
                            if trim_line.chars().all(|c| c == '-') && trim_line.len() > 3 {
                                lines_to_print.push(line.to_string());
                            }
                            continue;
                        }
                        lines_to_print.push(line.to_string());
                    }
                    result.push_str(&lines_to_print.join("\n"));
                    result.push_str("\n");
                }
            }
            Err(e) => {
                result.push_str(&format!("-> Lỗi khi thực thi Winget: {}\n", e));
            }
        }
    } else {
        result.push_str("-> Winget chưa được cài đặt hoặc không nằm trong PATH.\n");
    }
    result.push_str("\n");

    // 2. Microsoft Store update check
    result.push_str("=== KIỂM TRA CẬP NHẬT MICROSOFT STORE ===\n");
    if Command::new("winget").arg("--version").output().is_ok() {
        match Command::new("winget").args(&["upgrade", "--source", "msstore"]).output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}{}", stdout, stderr);
                if combined.contains("No upgrades available") || combined.contains("No installed package found matching input criteria") {
                    result.push_str("-> Không có bản cập nhật Microsoft Store nào khả dụng.\n");
                } else {
                    let mut lines_to_print = Vec::new();
                    for line in combined.lines() {
                        let trim_line = line.trim();
                        if trim_line.is_empty() 
                            || trim_line.starts_with('-') 
                            || trim_line.starts_with('\\') 
                            || trim_line.starts_with('|') 
                            || trim_line.starts_with('/')
                        {
                            if trim_line.chars().all(|c| c == '-') && trim_line.len() > 3 {
                                lines_to_print.push(line.to_string());
                            }
                            continue;
                        }
                        lines_to_print.push(line.to_string());
                    }
                    result.push_str(&lines_to_print.join("\n"));
                    result.push_str("\n");
                }
            }
            Err(e) => {
                result.push_str(&format!("-> Lỗi khi kiểm tra Microsoft Store: {}\n", e));
            }
        }
    } else {
        result.push_str("-> Winget chưa được cài đặt (không thể kiểm tra Microsoft Store).\n");
    }
    result.push_str("\n");

    // 3. Chocolatey outdated check
    result.push_str("=== KIỂM TRA CẬP NHẬT CHOCOLATEY ===\n");
    if Command::new("choco").arg("--version").output().is_ok() {
        match Command::new("choco").arg("outdated").output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}{}", stdout, stderr);
                
                if combined.contains("0 package(s) are outdated") {
                    result.push_str("-> Các gói Chocolatey đã được cập nhật đầy đủ.\n");
                } else if combined.contains("Chocolatey has determined") {
                    let mut lines_to_print = Vec::new();
                    let mut started = false;
                    for line in combined.lines() {
                        let trim_line = line.trim();
                        if trim_line.starts_with("Outdated Packages") {
                            started = true;
                            continue;
                        }
                        if started {
                            if trim_line.contains("Chocolatey has determined") {
                                lines_to_print.push(line.to_string());
                                break;
                            }
                            lines_to_print.push(line.to_string());
                        }
                    }
                    if lines_to_print.is_empty() {
                        result.push_str(&combined);
                    } else {
                        result.push_str(&lines_to_print.join("\n"));
                    }
                    result.push_str("\n");
                } else {
                    result.push_str(&combined);
                    result.push_str("\n");
                }
            }
            Err(e) => {
                result.push_str(&format!("-> Lỗi khi thực thi Chocolatey: {}\n", e));
            }
        }
    } else {
        result.push_str("-> Chocolatey chưa được cài đặt hoặc không nằm trong PATH.\n");
    }
    result.push_str("\n");

    // 4. Scoop status check
    result.push_str("=== KIỂM TRA CẬP NHẬT SCOOP ===\n");
    if Command::new("scoop").arg("--version").output().is_ok() {
        match Command::new("powershell").args(&["-NoProfile", "-Command", "scoop status"]).output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{}{}", stdout, stderr);
                if combined.contains("Everything is ok") || combined.contains("is up to date") {
                    result.push_str("-> Các ứng dụng Scoop đã được cập nhật đầy đủ.\n");
                } else {
                    result.push_str(&combined);
                    result.push_str("\n");
                }
            }
            Err(e) => {
                result.push_str(&format!("-> Lỗi khi kiểm tra Scoop: {}\n", e));
            }
        }
    } else {
        result.push_str("-> Scoop chưa được cài đặt hoặc không nằm trong PATH.\n");
    }

    Ok(result)
}

/// Checks system updates for APT, Flatpak, and Snap without requiring sudo password.
#[cfg(not(windows))]
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

