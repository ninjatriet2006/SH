use inquire::{Select, Text, Confirm};
use crossterm::style::Stylize;
use crate::config::{self, Config, AppConfig, AppType};
use crate::app_manager::{self, AppStatus};

pub fn print_banner() {
    println!("{}", "==================================================".blue());
    println!("{}", "    HỆ THỐNG QUẢN LÝ ỨNG DỤNG CLOSE & START (C&S)".cyan().bold());
    println!("{}", "==================================================".blue());
}

fn press_enter_to_continue() {
    println!("\nNhấn Enter để tiếp tục...");
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
}

pub fn start_ui_loop() -> anyhow::Result<()> {
    let mut config = config::load_config();

    loop {
        // Xóa màn hình và in banner
        print!("{}[2J{}[1;1H", 27 as char, 27 as char);
        print_banner();

        let mut options = Vec::new();
        for app in &config.apps {
            let status = app_manager::check_status(app);
            let status_str = match status {
                AppStatus::Running => "● Đang chạy".green().bold(),
                AppStatus::Stopped => "○ Đang dừng".red(),
            };
            let app_type_str = match app.app_type {
                AppType::Flatpak => "Flatpak",
                AppType::System => "Hệ thống",
            };
            options.push(format!("{} - {} ({})", status_str, app.name, app_type_str));
        }
        options.push("➕ Thêm ứng dụng mới".cyan().to_string());
        options.push("🚪 Thoát".yellow().to_string());

        let choice = Select::new("Chọn ứng dụng muốn can thiệp:", options.clone()).prompt();

        match choice {
            Ok(selected_str) => {
                let selected_idx = options.iter().position(|r| r == &selected_str).unwrap();

                if selected_idx < config.apps.len() {
                    // Người dùng chọn một ứng dụng trong danh sách
                    manage_app_menu(&mut config, selected_idx)?;
                } else if selected_idx == config.apps.len() {
                    // Thêm ứng dụng mới
                    add_app_prompt(&mut config)?;
                } else {
                    // Thoát chương trình
                    println!("{}", "\nCảm ơn bạn đã sử dụng Close and Start! Tạm biệt!".green());
                    break;
                }
            }
            Err(_) => {
                // Thoát khi bấm Ctrl+C hoặc Esc
                break;
            }
        }
    }

    Ok(())
}

fn manage_app_menu(config: &mut Config, idx: usize) -> anyhow::Result<()> {
    loop {
        let app = &config.apps[idx];
        let status = app_manager::check_status(app);
        
        print!("{}[2J{}[1;1H", 27 as char, 27 as char);
        print_banner();
        println!("\nỨng dụng được chọn: {} ({})", app.name.clone().cyan().bold(), app.app_type);
        let status_str = match status {
            AppStatus::Running => "Đang chạy".green().bold(),
            AppStatus::Stopped => "Đang dừng".red(),
        };
        println!("Trạng thái hiện tại: {}", status_str);
        println!("--------------------------------------------------");

        let options = vec![
            "▶️ Khởi chạy (Run)".to_string(),
            "⏹️ Tắt ứng dụng (Kill)".to_string(),
            "🔄 Khởi động lại (Restart)".to_string(),
            "✏️ Sửa ứng dụng".to_string(),
            "❌ Xóa ứng dụng".to_string(),
            "◀️ Quay lại".to_string(),
        ];

        let choice = Select::new("Chọn hành động:", options).prompt();

        match choice {
            Ok(selected) => {
                match selected.as_str() {
                    s if s.starts_with("▶️") => {
                        println!("Đang bật {}...", app.name);
                        if let Err(e) = app_manager::start_app(app) {
                            println!("{}", format!("Lỗi khởi chạy: {}", e).red());
                            press_enter_to_continue();
                        } else {
                            println!("{}", "Gửi lệnh khởi chạy thành công!".green());
                            std::thread::sleep(std::time::Duration::from_millis(800));
                        }
                    }
                    s if s.starts_with("⏹️") => {
                        println!("Đang tắt {}...", app.name);
                        if let Err(e) = app_manager::kill_app(app) {
                            println!("{}", format!("Lỗi khi tắt: {}", e).red());
                            press_enter_to_continue();
                        } else {
                            println!("{}", "Đã gửi lệnh tắt ứng dụng!".green());
                            std::thread::sleep(std::time::Duration::from_millis(800));
                        }
                    }
                    s if s.starts_with("🔄") => {
                        println!("Đang khởi động lại {}...", app.name);
                        if let Err(e) = app_manager::restart_app(app) {
                            println!("{}", format!("Lỗi khởi động lại: {}", e).red());
                            press_enter_to_continue();
                        } else {
                            println!("{}", "Khởi động lại thành công!".green());
                            std::thread::sleep(std::time::Duration::from_millis(800));
                        }
                    }
                    s if s.starts_with("✏️") => {
                        edit_app_prompt(config, idx)?;
                        break; // Quay lại menu chính sau khi cập nhật
                    }
                    s if s.starts_with("❌") => {
                        if delete_app_prompt(config, idx)? {
                            break; // Quay lại menu chính sau khi xóa thành công
                        }
                    }
                    _ => break,
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

fn add_app_prompt(config: &mut Config) -> anyhow::Result<()> {
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    println!("{}", "➕ THÊM ỨNG DỤNG MỚI".cyan().bold());
    println!("--------------------------------------------------");

    let name = Text::new("Tên hiển thị:").prompt()?;
    if name.trim().is_empty() {
        println!("{}", "Lỗi: Tên hiển thị không được để trống!".red());
        press_enter_to_continue();
        return Ok(());
    }

    let app_types = vec!["Flatpak", "Hệ thống"];
    let app_type_choice = Select::new("Loại ứng dụng:", app_types).prompt()?;

    let app_type = match app_type_choice {
        "Flatpak" => AppType::Flatpak,
        _ => AppType::System,
    };

    let target_label = match app_type {
        AppType::Flatpak => "ID Flatpak (ví dụ: org.fcitx.Fcitx5):",
        AppType::System => "Tên tiến trình hoặc đường dẫn thực thi:",
    };

    let target = Text::new(target_label).prompt()?;
    if target.trim().is_empty() {
        println!("{}", "Lỗi: Dữ liệu đích không được để trống!".red());
        press_enter_to_continue();
        return Ok(());
    }

    let start_cmd_raw = Text::new("Lệnh khởi chạy tùy chỉnh (Để trống để tự động):").prompt()?;
    let start_cmd = if start_cmd_raw.trim().is_empty() {
        None
    } else {
        Some(start_cmd_raw)
    };

    let kill_cmd_raw = Text::new("Lệnh tắt tùy chỉnh (Để trống để tự động):").prompt()?;
    let kill_cmd = if kill_cmd_raw.trim().is_empty() {
        None
    } else {
        Some(kill_cmd_raw)
    };

    let new_app = AppConfig {
        name,
        app_type,
        target,
        start_cmd,
        kill_cmd,
    };

    config.apps.push(new_app);
    config::save_config(config)?;
    println!("{}", "Đã thêm ứng dụng mới thành công!".green());
    std::thread::sleep(std::time::Duration::from_millis(800));

    Ok(())
}

fn edit_app_prompt(config: &mut Config, idx: usize) -> anyhow::Result<()> {
    print!("{}[2J{}[1;1H", 27 as char, 27 as char);
    println!("{}", "✏️ CHỈNH SỬA ỨNG DỤNG".cyan().bold());
    println!("--------------------------------------------------");

    let app = &config.apps[idx];

    let name = Text::new("Tên hiển thị:")
        .with_default(&app.name)
        .prompt()?;
    if name.trim().is_empty() {
        println!("{}", "Lỗi: Tên hiển thị không được để trống!".red());
        press_enter_to_continue();
        return Ok(());
    }

    let app_types = vec!["Flatpak", "Hệ thống"];
    let default_type_idx = match app.app_type {
        AppType::Flatpak => 0,
        AppType::System => 1,
    };
    let app_type_choice = Select::new("Loại ứng dụng:", app_types)
        .with_starting_cursor(default_type_idx)
        .prompt()?;

    let app_type = match app_type_choice {
        "Flatpak" => AppType::Flatpak,
        _ => AppType::System,
    };

    let target_label = match app_type {
        AppType::Flatpak => "ID Flatpak (ví dụ: org.fcitx.Fcitx5):",
        AppType::System => "Tên tiến trình hoặc đường dẫn thực thi:",
    };

    let target = Text::new(target_label)
        .with_default(&app.target)
        .prompt()?;
    if target.trim().is_empty() {
        println!("{}", "Lỗi: Dữ liệu đích không được để trống!".red());
        press_enter_to_continue();
        return Ok(());
    }

    let start_cmd_raw = Text::new("Lệnh khởi chạy tùy chỉnh (Để trống để tự động):")
        .with_default(app.start_cmd.as_deref().unwrap_or(""))
        .prompt()?;
    let start_cmd = if start_cmd_raw.trim().is_empty() {
        None
    } else {
        Some(start_cmd_raw)
    };

    let kill_cmd_raw = Text::new("Lệnh tắt tùy chỉnh (Để trống để tự động):")
        .with_default(app.kill_cmd.as_deref().unwrap_or(""))
        .prompt()?;
    let kill_cmd = if kill_cmd_raw.trim().is_empty() {
        None
    } else {
        Some(kill_cmd_raw)
    };

    config.apps[idx] = AppConfig {
        name,
        app_type,
        target,
        start_cmd,
        kill_cmd,
    };

    config::save_config(config)?;
    println!("{}", "Đã cập nhật thay đổi thành công!".green());
    std::thread::sleep(std::time::Duration::from_millis(800));

    Ok(())
}

fn delete_app_prompt(config: &mut Config, idx: usize) -> anyhow::Result<bool> {
    let app = &config.apps[idx];
    let confirm = Confirm::new(&format!("Bạn có chắc chắn muốn xóa ứng dụng {} khỏi danh sách?", app.name))
        .with_default(false)
        .prompt()?;

    if confirm {
        config.apps.remove(idx);
        config::save_config(config)?;
        println!("{}", "Đã xóa ứng dụng thành công!".green());
        std::thread::sleep(std::time::Duration::from_millis(800));
        Ok(true)
    } else {
        Ok(false)
    }
}
