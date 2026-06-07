use inquire::{Select, Text, MultiSelect};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use indicatif::ProgressBar;
use crate::system::context_menu;
use crate::config::ConfigManager;
use crate::core::scanner::{self, FileType, ScannedFile};
use crate::core::dispatcher::{ProcessMode, ArchiveMode};
use crate::engine::ArchiveEngine;
use crate::core::traits::Compressor;

pub async fn show_main_menu() -> anyhow::Result<()> {
    let config_mgr = ConfigManager::new();
    let _ = config_mgr.init_all_configs();

    loop {
        println!("\n=== UNI_CONV2 - CÔNG CỤ CHUYỂN ĐỔI ĐA NĂNG ===");
        let options = vec![
            "1. Bắt đầu Xử lý File (Simple TUI)",
            "2. Bắt đầu Xử lý File (Advanced Terminal GUI)",
            "3. Quản lý Config hệ thống (Sửa YAML)",
            "4. Quản lý Tích hợp Context Menu (Chuột phải OS)",
            "5. Uninstaller (Gỡ bỏ và Dọn dẹp)",
            "0. Thoát chương trình",
        ];

        let ans = Select::new("Chọn chức năng:", options).prompt();
        match ans {
            Ok("1. Bắt đầu Xử lý File (Simple TUI)") => {
                if let Err(e) = handle_file_processing(false).await {
                    println!("[❌] Có lỗi xảy ra trong quá trình xử lý: {}", e);
                }
            }
            Ok("2. Bắt đầu Xử lý File (Advanced Terminal GUI)") => {
                if let Err(e) = handle_file_processing(true).await {
                    println!("[❌] Có lỗi xảy ra trong quá trình xử lý: {}", e);
                }
            }
            Ok("3. Quản lý Config hệ thống (Sửa YAML)") => {
                let _ = handle_config_menu(&config_mgr);
            }
            Ok("4. Quản lý Tích hợp Context Menu (Chuột phải OS)") => {
                let _ = handle_context_menu_setup();
            }
            Ok("5. Uninstaller (Gỡ bỏ và Dọn dẹp)") => {
                let _ = handle_uninstaller(&config_mgr);
            }
            Ok("0. Thoát chương trình") | Err(_) => {
                println!("Đang thoát...");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_file_processing(use_advanced_tui: bool) -> anyhow::Result<()> {
    // 1. Ask input method
    let methods = vec![
        "1. Từng file cụ thể (Kéo thả vào terminal)",
        "2. Lọc theo định dạng trong toàn bộ thư mục",
    ];
    let method = Select::new("Phương thức chọn file:", methods).prompt()?;

    let mut files = Vec::new();

    if method.starts_with('1') {
        let input = Text::new("Hãy kéo thả các file vào đây rồi nhấn Enter:").prompt()?;
        let paths = scanner::parse_drag_drop_paths(&input);
        for p in paths {
            files.push(scanner::classify_file(p));
        }
    } else {
        let dir_str = Text::new("Nhập đường dẫn thư mục (để trống cho thư mục hiện tại):").prompt()?;
        let dir_path = if dir_str.trim().is_empty() {
            std::env::current_dir()?
        } else {
            PathBuf::from(shellexpand::full(&dir_str)?.to_string())
        };

        if !dir_path.exists() {
            println!("[❌] Thư mục không tồn tại!");
            return Ok(());
        }

        // MultiSelect Checkbox for file types
        let filter_options = vec![
            "Archive (zip, 7z, rar, tar...)",
            "Video (mp4, mkv, avi...)",
            "Image (jpg, png, webp...)",
            "Audio (mp3, flac, wav...)",
            "Document (docx, pdf, txt...)",
        ];
        let chosen_filters = MultiSelect::new("Chọn loại file cần quét:", filter_options).prompt()?;
        
        let mut allowed_types = vec![false; 5];
        for f in chosen_filters {
            if f.starts_with("Archive") { allowed_types[0] = true; }
            if f.starts_with("Video") { allowed_types[1] = true; }
            if f.starts_with("Image") { allowed_types[2] = true; }
            if f.starts_with("Audio") { allowed_types[3] = true; }
            if f.starts_with("Document") { allowed_types[4] = true; }
        }

        files = scanner::scan_directory(&dir_path, &allowed_types)?;
    }

    if use_advanced_tui {
        crate::ui::advanced_tui::start_advanced_tui(files)?;
    } else {
        process_selected_files(files, None).await?;
    }

    Ok(())
}

#[derive(Clone)]
struct FileItem {
    index: usize,
    label: String,
}

impl std::fmt::Display for FileItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label)
    }
}

pub async fn process_selected_files(files: Vec<ScannedFile>, context_mode: Option<&str>) -> anyhow::Result<()> {
    if files.is_empty() {
        println!("[⚠️] Không tìm thấy file hợp lệ nào để xử lý!");
        return Ok(());
    }

    let archive_engine = ArchiveEngine::new();

    // 1. Show scrollable multi-select menu for selected files
    let items: Vec<FileItem> = files.iter().enumerate().map(|(idx, f)| {
        let type_str = match f.file_type {
            FileType::Video => "Video",
            FileType::Audio => "Audio",
            FileType::Image => "Image",
            FileType::Document => "Doc",
            FileType::Archive => "Archive",
            FileType::Directory => "Folder",
            FileType::Unknown => "Unknown",
        };
        let label = format!("[{}] {}", type_str, f.path.file_name().unwrap_or_default().to_string_lossy());
        FileItem { index: idx, label }
    }).collect();

    let defaults: Vec<usize> = (0..items.len()).collect();
    let chosen_items = MultiSelect::new(
        "Xác nhận các tệp/thư mục cần xử lý (Space để Chọn/Bỏ chọn, Enter để Tiếp tục):",
        items
    )
    .with_default(&defaults)
    .with_page_size(5)
    .prompt()?;

    if chosen_items.is_empty() {
        println!("[⚠️] Bạn chưa chọn tệp nào để xử lý!");
        return Ok(());
    }

    let mut filtered_files = Vec::new();
    for item in chosen_items {
        filtered_files.push(files[item.index].clone());
    }
    let files = filtered_files;

    // Summarize files
    let mut v_count = 0;
    let mut a_count = 0;
    let mut i_count = 0;
    let mut d_count = 0;
    let mut arc_count = 0;
    let mut dir_count = 0;
    for f in &files {
        match f.file_type {
            FileType::Video => v_count += 1,
            FileType::Audio => a_count += 1,
            FileType::Image => i_count += 1,
            FileType::Document => d_count += 1,
            FileType::Archive => arc_count += 1,
            FileType::Directory => dir_count += 1,
            FileType::Unknown => {}
        }
    }
    println!("\n[📊] Báo cáo mục tiêu đã nhận diện:");
    if v_count > 0 { println!(" - Video: {} file", v_count); }
    if a_count > 0 { println!(" - Audio: {} file", a_count); }
    if i_count > 0 { println!(" - Image: {} file", i_count); }
    if d_count > 0 { println!(" - Document: {} file", d_count); }
    if arc_count > 0 { println!(" - Archive: {} file", arc_count); }
    if dir_count > 0 { println!(" - Thư mục (Directory): {} thư mục", dir_count); }

    // Global Action selection
    let actions = vec![
        "A. Nén tất cả các file đã chọn thành một file nén (.zip, .7z...)",
        "B. Đi vào xử lý chuyên sâu cho từng loại file",
    ];
    let action = Select::new("Hành động chung:", actions).prompt()?;

    if action.starts_with('A') {
        let base_name = Text::new("Nhập tên file nén đầu ra (không kèm đuôi):")
            .with_default("output")
            .prompt()?;
        
        let ext = Select::new("Chọn định dạng đuôi nén:", vec![".zip", ".7z", ".tar.gz"]).prompt()?;
        let file_name = format!("{}{}", base_name, ext);

        // Password prompt
        let has_pass = inquire::Confirm::new("Bạn có muốn đặt mật khẩu cho file nén không?")
            .with_default(false)
            .prompt()?;
        
        let password = if has_pass {
            let pass = inquire::Password::new("Nhập mật khẩu:")
                .with_display_mode(inquire::PasswordDisplayMode::Masked)
                .prompt()?;
            Some(pass)
        } else {
            None
        };
        
        // Resolve output path relative to the first input file's folder
        let parent_dir = files[0].path.parent().unwrap_or_else(|| Path::new("."));
        let out_path = parent_dir.join(file_name);

        println!("Đang thực hiện nén tất cả file/thư mục...");
        let pb = ProgressBar::new(100);
        let paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
        let res = archive_engine.compress_files(&paths, &out_path, password.as_deref(), &pb);
        if let Err(e) = res {
            println!("[❌] Nén thất bại: {}", e);
        } else {
            println!("[✅] Đã nén thành công và lưu tại {:?}", out_path);
        }
    } else {
        // Option B: Process each type
        let mut has_video = false;
        let mut has_audio = false;
        let mut has_image = false;
        let mut has_doc = false;
        let mut has_archive = false;
        let mut has_dir = false;
        for f in &files {
            match f.file_type {
                FileType::Video => has_video = true,
                FileType::Audio => has_audio = true,
                FileType::Image => has_image = true,
                FileType::Document => has_doc = true,
                FileType::Archive => has_archive = true,
                FileType::Directory => has_dir = true,
                _ => {}
            }
        }

        let mut video_mode = ProcessMode::Skip;
        let mut audio_mode = ProcessMode::Skip;
        let mut image_mode = ProcessMode::Skip;
        let mut doc_mode = ProcessMode::Skip;
        let mut archive_mode = ArchiveMode::Skip;
        let mut directory_mode = ProcessMode::Skip;

        let default_cursor = if context_mode == Some("config") { 1 } else { 0 };

        // Collect source extensions per type (to exclude from target list)
        let mut video_src_exts: Vec<String> = Vec::new();
        let mut audio_src_exts: Vec<String> = Vec::new();
        let mut image_src_exts: Vec<String> = Vec::new();
        let mut doc_src_exts: Vec<String> = Vec::new();
        for f in &files {
            let ext = f.extension.to_lowercase();
            match f.file_type {
                FileType::Video => { if !video_src_exts.contains(&ext) { video_src_exts.push(ext); } }
                FileType::Audio => { if !audio_src_exts.contains(&ext) { audio_src_exts.push(ext); } }
                FileType::Image => { if !image_src_exts.contains(&ext) { image_src_exts.push(ext); } }
                FileType::Document => { if !doc_src_exts.contains(&ext) { doc_src_exts.push(ext); } }
                _ => {}
            }
        }

        if has_video {
            let mode = Select::new(
                "Chọn cấu hình cho VIDEO:",
                vec![
                    "1. Chuyển đổi định dạng (Chọn đuôi đích)", 
                    "2. Trích xuất âm thanh (Tách nhạc sang MP3)",
                    "3. Xử lý theo Config YAML (Tùy chỉnh nâng cao)", 
                    "4. Bỏ qua"
                ]
            )
            .with_starting_cursor(default_cursor)
            .prompt()?;
            if mode.starts_with('1') {
                let all_exts = vec!["mp4", "avi", "mkv", "mov", "webm", "flv"];
                let filtered: Vec<&str> = all_exts.into_iter().filter(|e| !video_src_exts.contains(&e.to_string())).collect();
                if filtered.is_empty() {
                    println!("[⚠️] Không có định dạng đích khả dụng.");
                } else {
                    let ext = Select::new("Chọn định dạng đích cho Video:", filtered).prompt()?;
                    video_mode = ProcessMode::Default(ext.to_string());
                }
            } else if mode.starts_with('2') {
                video_mode = ProcessMode::Default("mp3".to_string());
            } else if mode.starts_with('3') {
                video_mode = ProcessMode::YamlConfig;
            }
        }

        if has_audio {
            let mode = Select::new(
                "Chọn cấu hình cho AUDIO:",
                vec!["1. Chuyển đổi định dạng (Chọn đuôi đích)", "2. Xử lý theo Config YAML (Tùy chỉnh nâng cao)", "3. Bỏ qua"]
            )
            .with_starting_cursor(default_cursor)
            .prompt()?;
            if mode.starts_with('1') {
                let all_exts = vec!["mp3", "flac", "wav", "m4a", "ogg"];
                let filtered: Vec<&str> = all_exts.into_iter().filter(|e| !audio_src_exts.contains(&e.to_string())).collect();
                if filtered.is_empty() {
                    println!("[⚠️] Không có định dạng đích khả dụng.");
                } else {
                    let ext = Select::new("Chọn định dạng đích cho Audio:", filtered).prompt()?;
                    audio_mode = ProcessMode::Default(ext.to_string());
                }
            } else if mode.starts_with('2') {
                audio_mode = ProcessMode::YamlConfig;
            }
        }

        if has_image {
            let mode = Select::new(
                "Chọn cấu hình cho IMAGE (Ảnh):",
                vec!["1. Chuyển đổi định dạng (Chọn đuôi đích)", "2. Xử lý theo Config YAML (Tùy chỉnh nâng cao)", "3. Bỏ qua"]
            )
            .with_starting_cursor(default_cursor)
            .prompt()?;
            if mode.starts_with('1') {
                let all_exts = vec!["webp", "jpg", "png", "bmp", "gif"];
                let filtered: Vec<&str> = all_exts.into_iter().filter(|e| !image_src_exts.contains(&e.to_string())).collect();
                if filtered.is_empty() {
                    println!("[⚠️] Không có định dạng đích khả dụng.");
                } else {
                    let ext = Select::new("Chọn định dạng đích cho Ảnh:", filtered).prompt()?;
                    image_mode = ProcessMode::Default(ext.to_string());
                }
            } else if mode.starts_with('2') {
                image_mode = ProcessMode::YamlConfig;
            }
        }

        if has_doc {
            let mode = Select::new(
                "Chọn cấu hình cho DOCUMENT (Tài liệu):",
                vec!["1. Chuyển đổi định dạng (Chọn đuôi đích)", "2. Xử lý theo Config YAML (Tùy chỉnh nâng cao)", "3. Bỏ qua"]
            )
            .with_starting_cursor(default_cursor)
            .prompt()?;
            if mode.starts_with('1') {
                let all_exts = vec!["pdf", "docx", "xlsx", "txt", "html"];
                let filtered: Vec<&str> = all_exts.into_iter().filter(|e| !doc_src_exts.contains(&e.to_string())).collect();
                if filtered.is_empty() {
                    println!("[⚠️] Không có định dạng đích khả dụng.");
                } else {
                    let ext = Select::new("Chọn định dạng đích cho Tài liệu:", filtered).prompt()?;
                    doc_mode = ProcessMode::Default(ext.to_string());
                }
            } else if mode.starts_with('2') {
                doc_mode = ProcessMode::YamlConfig;
            }
        }

        let mut archive_passwords: HashMap<PathBuf, String> = HashMap::new();

        if has_archive {
            let mode = Select::new(
                "Chọn hành động cho FILE NÉN (Archive):",
                vec![
                    "1. Xem danh sách tệp tin bên trong (List Contents)",
                    "2. Giải nén tại đây (Extract Here)",
                    "3. Giải nén vào thư mục riêng (Extract to <Folder>)",
                    "4. Giải nén tới một thư mục tùy ý...",
                    "5. Chuyển đổi sang định dạng nén khác (ví dụ: zip -> 7z)",
                    "6. Bỏ qua"
                ]
            ).prompt()?;
            
            if mode.starts_with("1") {
                // List files and then re-prompt archive action
                for f in &files {
                    if let FileType::Archive = f.file_type {
                        println!("\n--- Danh sách tệp trong {:?} ---", f.path.file_name().unwrap_or_default());
                        match archive_engine.list_archive_contents(&f.path) {
                            Ok(contents) => {
                                for entry in contents {
                                    println!("  - {}", entry);
                                }
                            }
                            Err(e) => println!("[❌] Không thể đọc danh sách tệp: {}", e),
                        }
                    }
                }
                println!("--------------------------------------");
                let _ = inquire::Confirm::new("Nhấn Enter để tiếp tục hành động giải nén...")
                    .with_default(true)
                    .prompt()?;
                // Default to extract to folder after listing
                archive_mode = ArchiveMode::ExtractToFolder;
            } else if mode.starts_with('2') {
                archive_mode = ArchiveMode::ExtractHere;
            } else if mode.starts_with('3') {
                archive_mode = ArchiveMode::ExtractToFolder;
            } else if mode.starts_with('4') {
                let dest = Text::new("Nhập/Kéo thả thư mục đích:").prompt()?;
                let paths = scanner::parse_drag_drop_paths(&dest);
                let final_path = if paths.is_empty() { PathBuf::from(dest) } else { paths[0].clone() };
                archive_mode = ArchiveMode::ExtractToCustom(final_path);
            } else if mode.starts_with('5') {
                let ext = Text::new("Nhập định dạng đích (ví dụ: 7z, zip):").prompt()?;
                archive_mode = ArchiveMode::ConvertFormat(ext);
            }

            // Detect & ask passwords for encrypted archives (with retry validation)
            match &archive_mode {
                ArchiveMode::ExtractHere | ArchiveMode::ExtractToFolder | ArchiveMode::ExtractToCustom(_) => {
                    for f in &files {
                        if let FileType::Archive = f.file_type {
                            if archive_engine.is_password_protected(&f.path) {
                                let fname = f.path.file_name().unwrap_or_default().to_string_lossy();
                                println!("\n[🔑] Phát hiện file nén có mật khẩu: {}", fname);
                                
                                let max_retries = 3;
                                let mut success = false;
                                for attempt in 1..=max_retries {
                                    let pass = inquire::Password::new(
                                        &format!("Nhập mật khẩu (lần {}/{}):", attempt, max_retries)
                                    )
                                    .with_display_mode(inquire::PasswordDisplayMode::Masked)
                                    .prompt()?;

                                    // Verify password with test
                                    if archive_engine.verify_password(&f.path, &pass) {
                                        println!("[✅] Mật khẩu chính xác!");
                                        archive_passwords.insert(f.path.clone(), pass);
                                        success = true;
                                        break;
                                    } else {
                                        if attempt < max_retries {
                                            println!("[❌] Sai mật khẩu! Vui lòng thử lại.");
                                        } else {
                                            println!("[❌] Sai mật khẩu {} lần liên tiếp. Bỏ qua file {}.", max_retries, fname);
                                        }
                                    }
                                }
                                if !success {
                                    // Mark this file to be skipped
                                    println!("[⚠️] File {} sẽ bị bỏ qua do không có mật khẩu hợp lệ.", fname);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if has_dir {
            let mode = Select::new(
                "Chọn hành động cho THƯ MỤC (Directory):",
                vec![
                    "1. Default (Nén sang .zip)",
                    "2. Cấu hình Config Yaml",
                    "3. Bỏ qua"
                ]
            )
            .with_starting_cursor(default_cursor)
            .prompt()?;
            if mode.starts_with('1') {
                directory_mode = ProcessMode::Default("zip".to_string());
            } else if mode.starts_with('2') {
                directory_mode = ProcessMode::YamlConfig;
            }
        }

        // Execute batch conversion!
        println!("\n🚀 Đang chạy xử lý hàng loạt...");
        crate::core::dispatcher::run_batch_processing(
            files,
            video_mode,
            audio_mode,
            image_mode,
            doc_mode,
            archive_mode,
            directory_mode,
            archive_passwords,
        ).await?
    }

    Ok(())
}

fn handle_config_menu(config_mgr: &ConfigManager) -> anyhow::Result<()> {
    loop {
        let options = vec![
            "1. Sửa config_video.yaml",
            "2. Sửa config_audio.yaml",
            "3. Sửa config_img.yaml",
            "4. Sửa config_doc.yaml",
            "5. Sửa config_archive.yaml",
            "0. Quay lại",
        ];

        let ans = Select::new("Chọn file config cần chỉnh sửa:", options).prompt();
        match ans {
            Ok("0. Quay lại") | Err(_) => break,
            Ok(opt) => {
                let filename = match opt {
                    "1. Sửa config_video.yaml" => "config_video.yaml",
                    "2. Sửa config_audio.yaml" => "config_audio.yaml",
                    "3. Sửa config_img.yaml" => "config_img.yaml",
                    "4. Sửa config_doc.yaml" => "config_doc.yaml",
                    "5. Sửa config_archive.yaml" => "config_archive.yaml",
                    _ => "",
                };
                if !filename.is_empty() {
                    let file_path = config_mgr.config_dir.join(filename);
                    open_in_editor(&file_path)?;
                }
            }
        }
    }
    Ok(())
}

fn open_in_editor(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("notepad")
            .arg(path)
            .status()?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Try gedit first for GUI, fallback to nano
        if std::process::Command::new("gedit").arg(path).status().is_err() {
            std::process::Command::new("nano").arg(path).status()?;
        }
    }
    Ok(())
}

fn handle_context_menu_setup() -> anyhow::Result<()> {
    loop {
        let options = vec![
            "1. Thêm 'UNI_CONV2 (Default)' vào Menu Chuột Phải",
            "2. Thêm 'UNI_CONV2 (Config Mode)' vào Menu Chuột Phải",
            "3. Gỡ bỏ tích hợp khỏi Menu Chuột Phải",
            "0. Quay lại",
        ];

        let ans = Select::new("Quản lý Context Menu:", options).prompt();
        match ans {
            Ok("1. Thêm 'UNI_CONV2 (Default)' vào Menu Chuột Phải") => {
                let _ = context_menu::install_nemo_action("default");
            }
            Ok("2. Thêm 'UNI_CONV2 (Config Mode)' vào Menu Chuột Phải") => {
                let _ = context_menu::install_nemo_action("config");
            }
            Ok("3. Gỡ bỏ tích hợp khỏi Menu Chuột Phải") => {
                let _ = context_menu::uninstall_all();
            }
            Ok("0. Quay lại") | Err(_) => break,
            _ => {}
        }
    }
    Ok(())
}

fn handle_uninstaller(config_mgr: &ConfigManager) -> anyhow::Result<()> {
    loop {
        let options = vec![
            "1. Dọn dẹp Config (Xóa toàn bộ YAML)",
            "2. Gỡ cài đặt Context Menu",
            "3. Gỡ sạch toàn bộ (Clean All)",
            "0. Quay lại",
        ];

        let ans = Select::new("Dọn dẹp hệ thống:", options).prompt();
        match ans {
            Ok("1. Dọn dẹp Config (Xóa toàn bộ YAML)") => {
                let _ = config_mgr.delete_all_configs();
            }
            Ok("2. Gỡ cài đặt Context Menu") => {
                let _ = context_menu::uninstall_all();
            }
            Ok("3. Gỡ sạch toàn bộ (Clean All)") => {
                let _ = config_mgr.delete_all_configs();
                let _ = context_menu::uninstall_all();
            }
            Ok("0. Quay lại") | Err(_) => break,
            _ => {}
        }
    }
    Ok(())
}
