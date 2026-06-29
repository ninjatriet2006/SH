use std::fs;
use std::path::{Path, PathBuf};
use dialoguer::{Select, Confirm, MultiSelect};
use crossterm::event::{self, Event, KeyCode};
use std::io::{self, Write};
use crate::detector;
use crate::app_image;
use crate::integrator::{self, IntegrationParams};
use crate::config::InstallType;

/// Helper function to check if the path points to a compressed archive file.
fn is_archive(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    name.ends_with(".zip")
        || name.ends_with(".tar.gz")
        || name.ends_with(".tgz")
        || name.ends_with(".tar.xz")
        || name.ends_with(".txz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tbz2")
        || name.ends_with(".tar")
}

/// Helper function to extract a compressed archive file to a target directory.
fn extract_archive(file_path: &Path, target_dir: &Path) -> Result<(), String> {
    let _ = fs::create_dir_all(target_dir);
    let file_name = file_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let status = if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        std::process::Command::new("tar")
            .arg("-xzf")
            .arg(file_path)
            .arg("-C")
            .arg(target_dir)
            .status()
    } else if file_name.ends_with(".tar.xz") || file_name.ends_with(".txz") {
        std::process::Command::new("tar")
            .arg("-xJf")
            .arg(file_path)
            .arg("-C")
            .arg(target_dir)
            .status()
    } else if file_name.ends_with(".tar.bz2") || file_name.ends_with(".tbz2") {
        std::process::Command::new("tar")
            .arg("-xjf")
            .arg(file_path)
            .arg("-C")
            .arg(target_dir)
            .status()
    } else if file_name.ends_with(".tar") {
        std::process::Command::new("tar")
            .arg("-xf")
            .arg(file_path)
            .arg("-C")
            .arg(target_dir)
            .status()
    } else if ext == "zip" {
        std::process::Command::new("unzip")
            .arg("-q")
            .arg(file_path)
            .arg("-d")
            .arg(target_dir)
            .status()
    } else {
        return Err(format!("Định dạng file nén không được hỗ trợ: {}", ext));
    };

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("Lệnh giải nén kết thúc với mã lỗi: {:?}", s.code())),
        Err(e) => Err(format!("Không thể chạy công cụ giải nén: {}", e)),
    }
}

/// Helper function to find the primary app folder inside the extracted directory.
fn find_app_folder(extracted_path: &Path) -> PathBuf {
    if let Ok(entries) = fs::read_dir(extracted_path) {
        let entries: Vec<_> = entries.flatten().collect();
        if entries.len() == 1 && entries[0].file_type().map(|t| t.is_dir()).unwrap_or(false) {
            return entries[0].path();
        }
    }
    extracted_path.to_path_buf()
}

struct OnlineIcon {
    app_ids: &'static [&'static str],
    url: &'static str,
    filename: &'static str,
}

const ONLINE_ICONS: &[OnlineIcon] = &[
    OnlineIcon {
        app_ids: &["telegram", "telegram-desktop", "tsetup"],
        url: "https://raw.githubusercontent.com/telegramdesktop/tdesktop/master/Telegram/Resources/art/icon256.png",
        filename: "telegram.png",
    },
    OnlineIcon {
        app_ids: &["vscode", "code", "visual-studio-code"],
        url: "https://raw.githubusercontent.com/microsoft/vscode/main/resources/linux/code.png",
        filename: "code.png",
    },
    OnlineIcon {
        app_ids: &["obsidian"],
        url: "https://raw.githubusercontent.com/linuxserver/docker-templates/master/linuxserver.io/img/obsidian-logo.png",
        filename: "obsidian.png",
    },
    OnlineIcon {
        app_ids: &["discord"],
        url: "https://raw.githubusercontent.com/flathub/com.discordapp.Discord/master/com.discordapp.Discord.svg",
        filename: "discord.svg",
    },
];

fn download_icon(url: &str, dest: &Path) -> Result<(), String> {
    // Try curl first
    let status_curl = std::process::Command::new("curl")
        .arg("-sL")
        .arg("-o")
        .arg(dest)
        .arg(url)
        .status();

    if let Ok(s) = status_curl {
        if s.success() {
            return Ok(());
        }
    }

    // Try wget as fallback
    let status_wget = std::process::Command::new("wget")
        .arg("-q")
        .arg("-O")
        .arg(dest)
        .arg(url)
        .status();

    match status_wget {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("Lệnh tải icon kết thúc với mã lỗi: {:?}", s.code())),
        Err(e) => Err(format!("Không thể chạy công cụ tải (yêu cầu curl hoặc wget): {}", e)),
    }
}

/// Helper function to read user text input in raw mode, enabling Escape key cancellation.
fn read_line_with_esc(prompt: &str, default_val: Option<&str>) -> io::Result<Option<String>> {
    let mut input = String::new();
    
    // Print instructions
    println!("  [Phím tắt: Nhập chữ | Enter để xác nhận | Esc để huỷ/thoát]");
    
    // Enable raw mode to read keys one by one
    crossterm::terminal::enable_raw_mode()?;
    
    let def_part = if let Some(def) = default_val {
        format!(" [{}]", def)
    } else {
        "".to_string()
    };
    
    print!("{}{}: ", prompt, def_part);
    io::stdout().flush()?;
    
    let result = loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Release {
                    continue;
                }
                match key.code {
                    KeyCode::Enter => {
                        let final_val = if input.trim().is_empty() {
                            default_val.unwrap_or("").to_string()
                        } else {
                            input
                        };
                        break Ok(Some(final_val));
                    }
                    KeyCode::Esc => {
                        break Ok(None);
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        print!("{}", c);
                        io::stdout().flush()?;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            // Erase last character on screen: back cursor, print space, back cursor
                            print!("\u{0008} \u{0008}");
                            io::stdout().flush()?;
                        }
                    }
                    _ => {}
                }
            }
        }
    };
    
    crossterm::terminal::disable_raw_mode()?;
    println!();
    result
}

fn is_portable_path(exec_path: &str) -> bool {
    let p = exec_path.trim().to_lowercase();
    if !p.starts_with('/') {
        return !p.starts_with("flatpak") && !p.starts_with("snap");
    }
    !(p.starts_with("/usr/bin/")
        || p.starts_with("/usr/sbin/")
        || p.starts_with("/usr/lib/")
        || p.starts_with("/bin/")
        || p.starts_with("/sbin/")
        || p.starts_with("/var/lib/flatpak/")
        || p.starts_with("/snap/")
        || p.starts_with("/var/lib/snapd/"))
}

fn parse_terminal_from_desktop(desktop_file: &str) -> Option<bool> {
    if let Ok(content) = fs::read_to_string(desktop_file) {
        for line in content.lines() {
            if line.trim().starts_with("Terminal=") {
                let val = line.split('=').nth(1).unwrap_or("").trim().to_lowercase();
                if val == "true" {
                    return Some(true);
                } else if val == "false" {
                    return Some(false);
                }
            }
        }
    }
    None
}

fn parse_paths(input: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            current.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if c.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.trim().is_empty() {
                paths.push(current.trim().to_string());
                current.clear();
            }
        } else {
            current.push(c);
        }
        i += 1;
    }
    
    if !current.trim().is_empty() {
        paths.push(current.trim().to_string());
    }
    
    paths
}

fn integrate_single_app(
    path_str: &str,
    mode: usize,
    initial_is_update: bool,
    initial_update_target_id: Option<String>,
    initial_default_app_name: Option<String>,
    initial_default_install_type: Option<crate::config::InstallType>,
    initial_default_symlink_name: Option<String>,
    initial_default_terminal: Option<bool>,
    initial_default_icon_path: Option<String>,
    local_apps: &[crate::config::AppEntry],
    config: &mut crate::config::Config,
) -> Result<bool, String> {
    let original_source_path = PathBuf::from(path_str);

    // 3. Handle extraction if it is an archive
    let (final_source_path, _temp_dir) = if is_archive(&original_source_path) {
        println!("\nĐang giải nén tệp tin...");
        let dir = tempfile::Builder::new().prefix("um-extract").tempdir()
            .map_err(|e| format!("Không thể tạo thư mục tạm để giải nén: {}", e))?;
        
        extract_archive(&original_source_path, dir.path())?;
        let path = find_app_folder(dir.path());
        println!("Giải nén thành công tại: {:?}", path);
        (path, Some(dir))
    } else {
        (find_app_folder(&original_source_path), None)
    };

    // 4. Perform detection
    let detection = detector::detect(&final_source_path)
        .map_err(|e| format!("Lỗi quét thông tin: {}", e))?;

    let mut name = detection.suggested_name.clone();
    let mut exec_rel_path = PathBuf::new();
    let mut icon_path = None;
    let mut comment = None;

    if detection.is_appimage {
        println!("\n[AppImage] Đang trích xuất metadata & icon từ file AppImage...");
        match app_image::extract_metadata(&final_source_path) {
            Ok((meta, extracted_icon)) => {
                if let Some(meta_name) = meta.name {
                    println!("Tìm thấy thông tin tên app: {}", meta_name);
                    name = meta_name;
                }
                if let Some(ref meta_comment) = meta.comment {
                    comment = Some(meta_comment.clone());
                }
                icon_path = extracted_icon;
                if let Some(ref path) = icon_path {
                    println!("Tìm thấy và trích xuất icon tại: {:?}", path);
                }
            }
            Err(e) => {
                println!("Cảnh báo: Không thể trích xuất metadata AppImage: {}", e);
                println!("Sẽ tiếp tục tích hợp thủ công.");
            }
        }
    } else if final_source_path.is_dir() {
        // Look for existing desktop templates inside the folder
        if !detection.desktop_templates.is_empty() {
            println!("\nTìm thấy file launcher mẫu trong thư mục:");
            let template_paths: Vec<String> = detection.desktop_templates.iter()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
                .collect();
            
            println!("  [Phím tắt: Mũi tên ↑/↓ để di chuyển | Enter để chọn | Esc để huỷ/thoát]");
            let selection_opt = Select::new()
                .with_prompt("Bạn có muốn sử dụng thông tin từ một file mẫu này? (Chọn mục cuối cùng để bỏ qua):")
                .items(&template_paths)
                .item("Bỏ qua và tự nhập thông tin")
                .default(0)
                .interact_opt()
                .map_err(|e| format!("Lỗi chọn mẫu: {}", e))?;

            let selection = match selection_opt {
                Some(s) => s,
                None => return Ok(false), // Cancelled
            };

            if selection < template_paths.len() {
                let selected_template = &detection.desktop_templates[selection];
                let meta = detector::DesktopMetadata::parse_file(selected_template);
                if let Some(meta_name) = meta.name {
                    name = meta_name;
                }
                if let Some(meta_comment) = meta.comment {
                    comment = Some(meta_comment);
                }
                // Try resolving Exec path from template
                if let Some(meta_exec) = meta.exec {
                    // Extract binary filename
                    let bin_name = meta_exec.split_whitespace().next()
                        .unwrap_or("")
                        .trim_matches('"')
                        .trim_matches('\'');
                    
                    let bin_path = Path::new(bin_name);
                    let bin_filename = bin_path.file_name().unwrap_or_default().to_string_lossy().to_string();

                    // Search in detection executables for matches
                    if let Some(found_exec) = detection.executables.iter().find(|e| {
                        e.file_name().unwrap_or_default().to_string_lossy() == bin_filename
                    }) {
                        exec_rel_path = found_exec.strip_prefix(&final_source_path)
                            .unwrap_or(found_exec)
                            .to_path_buf();
                    }
                }
                // Try resolving Icon path from template
                if let Some(meta_icon) = meta.icon {
                    // If it is an absolute path or exists, use it
                    let icon_p = Path::new(&meta_icon);
                    if icon_p.exists() {
                        icon_path = Some(icon_p.to_path_buf());
                    } else {
                        // Search in detection icons
                        let icon_filename = icon_p.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if let Some(found_icon) = detection.icons.iter().find(|i| {
                            i.file_name().unwrap_or_default().to_string_lossy().to_string().contains(&icon_filename) ||
                            i.file_name().unwrap_or_default().to_string_lossy() == icon_filename
                        }) {
                            icon_path = Some(found_icon.clone());
                        }
                    }
                }
            }
        }

        // If exec_rel_path is still empty, let's select from discovered executables
        if exec_rel_path.as_os_str().is_empty() {
            if detection.executables.is_empty() {
                // Let user type it
                let typed_exec_opt = read_line_with_esc("Không tìm thấy file chạy tự động. Nhập tên file chạy chính (ví dụ: run.sh)", None)
                    .map_err(|e| format!("Lỗi nhập liệu: {}", e))?;
                let typed_exec = match typed_exec_opt {
                    Some(val) if !val.trim().is_empty() => val,
                    _ => return Ok(false), // Cancelled
                };
                exec_rel_path = PathBuf::from(typed_exec);
            } else if detection.executables.len() == 1 {
                let found = &detection.executables[0];
                exec_rel_path = found.strip_prefix(&final_source_path)
                    .unwrap_or(found)
                    .to_path_buf();
                println!("Tự động chọn file chạy duy nhất tìm thấy: {:?}", exec_rel_path);
            } else {
                // Select from list
                let exec_options: Vec<String> = detection.executables.iter()
                    .map(|p| p.strip_prefix(&final_source_path).unwrap_or(p).to_string_lossy().to_string())
                    .collect();
                
                println!("  [Phím tắt: Mũi tên ↑/↓ để di chuyển | Enter để chọn | Esc để huỷ/thoát]");
                let selection_opt = Select::new()
                    .with_prompt("Tìm thấy nhiều file chạy khả dụng. Chọn file chạy chính:")
                    .items(&exec_options)
                    .default(0)
                    .interact_opt()
                    .map_err(|e| format!("Lỗi chọn file chạy: {}", e))?;
                
                let selection = match selection_opt {
                    Some(s) => s,
                    None => return Ok(false), // Cancelled
                };
                
                let found = &detection.executables[selection];
                exec_rel_path = found.strip_prefix(&final_source_path)
                    .unwrap_or(found)
                    .to_path_buf();
            }
        }

        // If icon_path is still empty, let's select from discovered icons
        if icon_path.is_none() && !detection.icons.is_empty() {
            let icon_options: Vec<String> = detection.icons.iter()
                .map(|p| p.strip_prefix(&final_source_path).unwrap_or(p).to_string_lossy().to_string())
                .collect();
            
            println!("  [Phím tắt: Mũi tên ↑/↓ để di chuyển | Enter để chọn | Esc để huỷ/thoát]");
            let selection_opt = Select::new()
                .with_prompt("Tìm thấy các file ảnh icon khả dụng. Chọn một icon làm ảnh đại diện:")
                .items(&icon_options)
                .item("Không sử dụng (Sử dụng icon hệ thống mặc định)")
                .default(0)
                .interact_opt()
                .unwrap_or(Some(icon_options.len()));

            let selection = match selection_opt {
                Some(s) => s,
                None => return Ok(false), // Cancelled
            };

            if selection < icon_options.len() {
                icon_path = Some(detection.icons[selection].clone());
            }
        }
    }

    let mut is_update = initial_is_update;
    let mut update_target_id = initial_update_target_id;
    let mut default_app_name = initial_default_app_name;
    let mut default_install_type = initial_default_install_type;
    let mut default_symlink_name = initial_default_symlink_name;
    let mut default_terminal = initial_default_terminal;
    let mut default_icon_path = initial_default_icon_path;

    // 5. Flow Routing (Based on mode)
    let app_name;
    let install_type;
    let create_symlink;
    let mut symlink_name = None;
    let terminal;

    if mode == 0 {
        // --- 5.1. INTEGRATE NEW APP ---
        let app_name_opt = read_line_with_esc("Nhập tên hiển thị của ứng dụng", Some(&name))
            .map_err(|e| format!("Lỗi nhập tên: {}", e))?;
        app_name = match app_name_opt {
            Some(val) if !val.trim().is_empty() => val,
            _ => return Ok(false),
        };
        
        let app_id = app_name.to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
            .replace(' ', "-");
            
        // Check duplicate name
        if let Some(existing) = local_apps.iter().find(|a| a.id == app_id) {
            println!("\nCảnh báo: Ứng dụng '{}' đã tồn tại trong danh sách.", app_name);
            println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
            let confirm_opt = Confirm::new()
                .with_prompt("Bạn có muốn chuyển sang cập nhật/ghi đè lên phiên bản cũ?")
                .default(true)
                .interact_opt()
                .map_err(|e| format!("Lỗi xác nhận: {}", e))?;
                
            match confirm_opt {
                Some(true) => {
                    default_install_type = Some(existing.install_type.clone());
                    default_symlink_name = existing.symlink_file.as_ref()
                        .and_then(|s| Path::new(s).file_name())
                        .map(|f| f.to_string_lossy().to_string());
                    default_terminal = parse_terminal_from_desktop(&existing.desktop_file).or(existing.is_custom);
                    default_icon_path = existing.icon_path.clone();
                    update_target_id = Some(existing.id.clone());
                    is_update = true;
                }
                Some(false) => {
                    return Err("Huỷ bỏ tích hợp do trùng lặp tên ứng dụng.".to_string());
                }
                None => return Ok(false),
            }
        }
    } else if mode == 1 {
        // --- 5.2. AUTOMATIC UPDATE ROUTING ---
        let temp_app_id = name.to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
            .replace(' ', "-");
            
        if let Some(existing) = local_apps.iter().find(|a| a.id == temp_app_id) {
            // Found duplicate ID -> Overwrite confirm
            println!("\nPhát hiện ứng dụng trùng khớp trong hệ thống: {} (ID: {})", existing.name, existing.id);
            println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
            let confirm_opt = Confirm::new()
                .with_prompt("Bạn có muốn thực hiện cập nhật (ghi đè) lên phiên bản cũ?")
                .default(true)
                .interact_opt()
                .map_err(|e| format!("Lỗi xác nhận: {}", e))?;
                
            match confirm_opt {
                Some(true) => {
                    default_app_name = Some(existing.name.clone());
                    default_install_type = Some(existing.install_type.clone());
                    default_symlink_name = existing.symlink_file.as_ref()
                        .and_then(|s| Path::new(s).file_name())
                        .map(|f| f.to_string_lossy().to_string());
                    default_terminal = parse_terminal_from_desktop(&existing.desktop_file).or(existing.is_custom);
                    default_icon_path = existing.icon_path.clone();
                    update_target_id = Some(existing.id.clone());
                    is_update = true;
                }
                Some(false) => {
                    return Ok(false);
                }
                None => return Ok(false),
            }
        } else {
            // Not found -> Ask to integrate as new or select existing to overwrite
            println!("\nKhông tự động tìm thấy ứng dụng nào có tên tương ứng để cập nhật.");
            println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
            let choices = vec![
                "1. Tích hợp làm ứng dụng PORTABLE MỚI",
                "2. Chọn thủ công một ứng dụng cũ để GHI ĐÈ"
            ];
            let choice_opt = Select::new()
                .with_prompt("Bạn muốn làm gì?")
                .items(&choices)
                .default(0)
                .interact_opt()
                .map_err(|e| format!("Lỗi xác nhận: {}", e))?;
                
            match choice_opt {
                Some(0) => {
                    // Proceed as new
                }
                Some(1) => {
                    // Manual select to overwrite
                    if local_apps.is_empty() {
                        println!("Không có ứng dụng cũ nào để ghi đè. Sẽ tiếp tục tạo mới.");
                    } else {
                        let app_selections: Vec<String> = local_apps.iter()
                            .map(|a| format!("{} (ID: {})", a.name, a.id))
                            .collect();
                        let selection_opt = Select::new()
                            .with_prompt("Chọn ứng dụng cũ để ghi đè:")
                            .items(&app_selections)
                            .default(0)
                            .interact_opt()
                            .map_err(|e| format!("Lỗi chọn: {}", e))?;
                        
                        if let Some(sel) = selection_opt {
                            let existing = &local_apps[sel];
                            default_app_name = Some(existing.name.clone());
                            default_install_type = Some(existing.install_type.clone());
                            default_symlink_name = existing.symlink_file.as_ref()
                                .and_then(|s| Path::new(s).file_name())
                                .map(|f| f.to_string_lossy().to_string());
                            default_terminal = parse_terminal_from_desktop(&existing.desktop_file).or(existing.is_custom);
                            default_icon_path = existing.icon_path.clone();
                            update_target_id = Some(existing.id.clone());
                            is_update = true;
                        } else {
                            return Ok(false); // Cancelled selection
                        }
                    }
                }
                _ => return Ok(false),
            }
        }
        
        let app_name_opt = read_line_with_esc("Nhập tên hiển thị của ứng dụng", Some(default_app_name.as_deref().unwrap_or(&name)))
            .map_err(|e| format!("Lỗi nhập tên: {}", e))?;
        app_name = match app_name_opt {
            Some(val) if !val.trim().is_empty() => val,
            _ => return Ok(false),
        };
    } else {
        // --- 5.3. MANUAL UPDATE VERIFICATION ---
        let target_id = update_target_id.clone().unwrap();
        let detected_app_id = name.to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
            .replace(' ', "-");
            
        if detected_app_id != target_id {
            // Warning mismatch
            println!("\nCẢNH BÁO: Tên ứng dụng mới quét được ({}) khác với ứng dụng bạn đã chọn cập nhật ({}).", name, default_app_name.as_deref().unwrap_or(""));
            println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
            let confirm_opt = Confirm::new()
                .with_prompt("Bạn có chắc chắn muốn thực hiện ghi đè cập nhật?")
                .default(false)
                .interact_opt()
                .map_err(|e| format!("Lỗi xác nhận: {}", e))?;
                
            match confirm_opt {
                Some(true) => {}
                _ => return Ok(false),
            }
        }
        app_name = default_app_name.clone().unwrap();
    }

    // --- 6. DEFINE PATH & CONFIGURATION FOR NEW OR UPDATE ---
    let default_dir = config.settings.managed_dir.clone();
    
    if !is_update {
        // New integration installation directory confirmation
        let managed_dir_opt = read_line_with_esc("Nhập thư mục lưu trữ mặc định", Some(&default_dir))
            .map_err(|e| format!("Lỗi nhập thư mục: {}", e))?;
            
        let final_managed_dir = match managed_dir_opt {
            Some(val) if !val.trim().is_empty() => val,
            _ => return Ok(false),
        };
        
        if final_managed_dir != default_dir {
            // Update and save configuration
            config.settings.managed_dir = final_managed_dir;
            let _ = config.save();
        }
    }

    // Select installation type (Moved vs InPlace)
    let install_types = vec![
        "Di chuyển (Copy ứng dụng vào thư mục quản lý tập trung ~/Applications)",
        "Tại chỗ (Giữ nguyên thư mục ứng dụng tại vị trí hiện tại)",
    ];
    let default_install_idx = match default_install_type {
        Some(InstallType::Moved) => 0,
        Some(InstallType::InPlace) => 1,
        None => 0,
    };
    println!("  [Phím tắt: Mũi tên ↑/↓ để di chuyển | Enter để chọn | Esc để huỷ/thoát]");
    let install_selection_opt = Select::new()
        .with_prompt("Chọn phương thức cài đặt/tích hợp:")
        .items(&install_types)
        .default(default_install_idx)
        .interact_opt()
        .map_err(|e| format!("Lỗi chọn kiểu cài đặt: {}", e))?;

    let install_selection = match install_selection_opt {
        Some(s) => s,
        None => return Ok(false),
    };

    install_type = if install_selection == 0 {
        InstallType::Moved
    } else {
        InstallType::InPlace
    };

    // Confirm symlink creation
    let default_symlink_val = default_symlink_name.is_some() || !is_update;
    println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
    let create_symlink_opt = Confirm::new()
        .with_prompt("Tạo link command-line (chạy nhanh trong Terminal)?")
        .default(default_symlink_val)
        .interact_opt()
        .map_err(|e| format!("Lỗi chọn symlink: {}", e))?;

    create_symlink = match create_symlink_opt {
        Some(val) => val,
        None => return Ok(false),
    };

    if create_symlink {
        let default_cmd = default_symlink_name
            .unwrap_or_else(|| app_name.to_lowercase().replace(' ', "-"));
            
        let cmd_name_opt = read_line_with_esc("Tên lệnh Terminal (CLI command)", Some(&default_cmd))
            .map_err(|e| format!("Lỗi nhập lệnh CLI: {}", e))?;
            
        let cmd_name = match cmd_name_opt {
            Some(cmd) if !cmd.trim().is_empty() => cmd,
            _ => return Ok(false),
        };
        symlink_name = Some(cmd_name);
    }

    // Confirm terminal application
    let default_term_val = default_terminal.unwrap_or(false);
    println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
    let terminal_opt = Confirm::new()
        .with_prompt("Đây là ứng dụng giao diện Console/Terminal (chạy trong Terminal)?")
        .default(default_term_val)
        .interact_opt()
        .map_err(|e| format!("Lỗi chọn terminal: {}", e))?;

    terminal = match terminal_opt {
        Some(val) => val,
        None => return Ok(false),
    };

    let app_id = app_name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "-")
        .replace(' ', "-");

    // Find the old app folder if it is an update
    let mut delete_old_dir = false;
    let mut old_app_dir = None;
    if is_update {
        let all_scanned = crate::scanner::scan_all_system_apps();
        let old_id = update_target_id.clone().unwrap_or_else(|| app_id.clone());
        
        if let Some(existing) = all_scanned.iter().find(|a| a.id == old_id) {
            let path = Path::new(&existing.exec_path);
            if let Some(parent) = path.parent() {
                if parent.exists() && is_portable_path(&existing.exec_path) {
                    old_app_dir = Some(parent.to_path_buf());
                }
            }
        }
    }

    if let Some(ref old_dir) = old_app_dir {
        let new_install_path = match install_type {
            InstallType::InPlace => final_source_path.clone(),
            InstallType::Moved => {
                let managed_dir = PathBuf::from(&config.settings.managed_dir);
                managed_dir.join(&app_id)
            }
        };

        if old_dir != &new_install_path && old_dir.canonicalize().ok() != new_install_path.canonicalize().ok() {
            println!("\nPhát hiện thư mục phiên bản cũ tại: {:?}", old_dir);
            println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
            let confirm_del = Confirm::new()
                .with_prompt("Bạn có muốn xoá sạch thư mục phiên bản cũ này không?")
                .default(false)
                .interact_opt()
                .map_err(|e| format!("Lỗi xác nhận: {}", e))?;
                
            if let Some(true) = confirm_del {
                delete_old_dir = true;
            }
        }
    }

    // 7. Resolve and verify Icon paths (Check new and old icons)
    let mut final_icon = icon_path;
    let new_icon_exists = final_icon.as_ref().map(|p| p.exists()).unwrap_or(false);
    
    if !new_icon_exists {
        if let Some(ref old_icon_str) = default_icon_path {
            let old_icon_path = Path::new(old_icon_str);
            if old_icon_path.exists() {
                let file_name = old_icon_path.file_name().unwrap_or_default();
                let temp_icon = std::env::temp_dir().join(format!(
                    "um_icon_{}_{}",
                    app_id,
                    file_name.to_string_lossy()
                ));
                if fs::copy(old_icon_path, &temp_icon).is_ok() {
                    final_icon = Some(temp_icon);
                }
            }
        }
    }

    // If still no icon found, prompt for online icon download or system icon fallback
    if final_icon.is_none() {
        let registry_match = ONLINE_ICONS.iter().find(|item| {
            item.app_ids.contains(&app_id.as_str())
        });

        let mut downloaded = false;
        if let Some(online_item) = registry_match {
            println!("\nKhông tìm thấy file icon cục bộ. Tìm thấy link tải icon trực tuyến cho ứng dụng: {}", app_name);
            println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
            let confirm_dl = Confirm::new()
                .with_prompt("Bạn có muốn tự động tải về icon trực tuyến này không?")
                .default(true)
                .interact_opt()
                .map_err(|e| format!("Lỗi xác nhận: {}", e))?;

            if let Some(true) = confirm_dl {
                let temp_dest = std::env::temp_dir().join(online_item.filename);
                println!("Đang tải icon từ: {} ...", online_item.url);
                match download_icon(online_item.url, &temp_dest) {
                    Ok(_) => {
                        println!("Tải icon thành công!");
                        final_icon = Some(temp_dest);
                        downloaded = true;
                    }
                    Err(e) => {
                        println!("Cảnh báo: Không thể tải icon từ internet: {}", e);
                    }
                }
            }
        }

        if !downloaded {
            println!("\nKhông tìm thấy file icon nào trong thư mục ứng dụng.");
            println!("  [Phím tắt: Enter để chọn mặc định | Esc để huỷ/thoát]");
            let use_sys_icon_opt = Confirm::new()
                .with_prompt(format!("Bạn có muốn sử dụng icon hệ thống (ví dụ: '{}')?", app_id))
                .default(true)
                .interact_opt()
                .map_err(|e| format!("Lỗi xác nhận: {}", e))?;

            match use_sys_icon_opt {
                Some(true) => {
                    final_icon = Some(PathBuf::from(&app_id));
                }
                Some(false) => {
                    // Let user type it
                    let typed_icon_opt = read_line_with_esc("Nhập đường dẫn file icon hoặc tên icon hệ thống (để trống nếu không dùng)", None)
                        .map_err(|e| format!("Lỗi nhập: {}", e))?;
                    if let Some(typed_icon) = typed_icon_opt {
                        if !typed_icon.trim().is_empty() {
                            final_icon = Some(PathBuf::from(typed_icon.trim()));
                        }
                    }
                }
                None => return Ok(false), // Esc pressed
            }
        }
    }

    if delete_old_dir {
        if let Some(ref old_dir) = old_app_dir {
            if old_dir.exists() && old_dir != &final_source_path {
                let _ = fs::remove_dir_all(old_dir);
            }
        }
    }

    // 8. Invoke Integration
    let params = IntegrationParams {
        name: app_name,
        source_path: final_source_path,
        install_type,
        exec_rel_path,
        icon_path: final_icon,
        create_symlink,
        symlink_name,
        categories: None,
        comment,
        terminal,
    };

    println!("\nĐang tích hợp/cập nhật ứng dụng...");
    let entry = integrator::integrate(params)?;
    println!("\n[Thành công] Ứng dụng đã được tích hợp/cập nhật thành công!");
    println!("- ID: {}", entry.id);
    println!("- File chạy chính: {}", entry.exec_path);
    println!("- Đường dẫn launcher: {}", entry.desktop_file);
    if let Some(ref sym) = entry.symlink_file {
        println!("- CLI Command: {}", sym);
    }

    Ok(true)
}

pub fn run_wizard(initial_path: Option<String>) -> Result<bool, String> {
    let mut config = crate::config::Config::load();
    
    // Load all system apps and filter to find portable ones
    let all_scanned_apps = crate::scanner::scan_all_system_apps();
    let local_apps: Vec<_> = all_scanned_apps.into_iter()
        .filter(|a| {
            a.package_type.as_deref().unwrap_or("Local") == "Local"
        })
        .collect();

    let mut integrated_any = false;

    let source_str_opt = if let Some(path) = initial_path {
        Some(path)
    } else {
        let choices = vec![
            "1. Nhập đường dẫn thủ công (Thư mục hoặc tệp nén)",
            "2. Tự động quét và tìm kiếm ứng dụng chưa tích hợp (Downloads, Desktop)",
        ];
        println!("\n  [Phím tắt: Mũi tên ↑/↓ để di chuyển | Enter để chọn | Esc để huỷ/thoát]");
        let choice_opt = Select::new()
            .with_prompt("CHỌN PHƯƠNG THỨC TÍCH HỢP / CẬP NHẬT ỨNG DỤNG:")
            .items(&choices)
            .default(0)
            .interact_opt()
            .map_err(|e| format!("Lỗi chọn: {}", e))?;

        match choice_opt {
            Some(0) => {
                let mut path_result = None;
                loop {
                    let prompt_msg = "Nhập đường dẫn thư mục ứng dụng hoặc tệp chạy".to_string();
                    let path_opt = read_line_with_esc(&prompt_msg, None)
                        .map_err(|e| format!("Lỗi nhập liệu: {}", e))?;
                    match path_opt {
                        Some(p) => {
                            let parsed = parse_paths(&p);
                            if parsed.is_empty() {
                                println!("Vui lòng nhập đường dẫn hợp lệ.");
                                continue;
                            }
                            let mut all_exist = true;
                            for path_str in &parsed {
                                if !Path::new(path_str).exists() {
                                    println!("Đường dẫn không tồn tại: {}! Vui lòng nhập lại.", path_str);
                                    all_exist = false;
                                    break;
                                }
                            }
                            if all_exist {
                                path_result = Some(p);
                                break;
                            }
                        }
                        None => break, // Esc
                    }
                }
                path_result
            }
            Some(1) => {
                println!("\nĐang quét ổ đĩa tìm kiếm ứng dụng di động chưa tích hợp...");
                let discovered = detector::scan_for_unintegrated_apps(&config);
                if discovered.is_empty() {
                    println!("Không tìm thấy ứng dụng di động chưa tích hợp nào trong các thư mục mặc định.");
                    println!("Nhấn Enter để tiếp tục...");
                    let mut buf = String::new();
                    let _ = io::stdin().read_line(&mut buf);
                    None
                } else {
                    let selections: Vec<String> = discovered.iter()
                        .map(|p| p.to_string_lossy().to_string())
                        .collect();
                    
                    println!("\n  [Phím tắt: Mũi tên ↑/↓ | Space để tích chọn | Enter để chạy | Esc để huỷ]");
                    let chosen_opts = MultiSelect::new()
                        .with_prompt("Tìm thấy các ứng dụng chưa tích hợp. Hãy tích chọn ứng dụng bạn muốn thêm:")
                        .items(&selections)
                        .interact_opt()
                        .map_err(|e| format!("Lỗi chọn: {}", e))?;
                    
                    if let Some(choices) = chosen_opts {
                        if choices.is_empty() {
                            None
                        } else {
                            let paths_str = choices.iter()
                                .map(|&idx| format!("\"{}\"", discovered[idx].to_string_lossy()))
                                .collect::<Vec<String>>()
                                .join(" ");
                            Some(paths_str)
                        }
                    } else {
                        None
                    }
                }
            }
            _ => None,
        }
    };

    let source_str = match source_str_opt {
        Some(s) => s,
        None => return Ok(false), // Cancelled
    };

    let parsed_paths = parse_paths(&source_str);
    for path_str in parsed_paths {
        println!("\n==================================================");
        println!("Đang xử lý tích hợp: {}", path_str);
        println!("==================================================");

        match integrate_single_app(
            &path_str,
            1, // auto update mode (will figure out new vs update automatically)
            false, // initial is_update
            None, // update_target_id
            None,
            None,
            None,
            None,
            None,
            &local_apps,
            &mut config,
        ) {
            Ok(true) => {
                integrated_any = true;
            }
            Ok(false) => {
                println!("Tích hợp bị huỷ bởi người dùng cho đường dẫn '{}'.", path_str);
            }
            Err(e) => {
                println!("Lỗi tích hợp đường dẫn '{}': {}", path_str, e);
            }
        }
    }

    Ok(integrated_any)
}
