mod app_image;
mod autostart;
mod config;
mod detector;
pub mod installer;
mod integrator;
mod maintenance;
mod manager;
mod remover;
mod scanner;
mod tui;
mod wizard;

use clap::{Parser, Subcommand};
use config::{AppStatus, Config};

#[derive(Parser)]
#[command(name = "universe-manager")]
#[command(about = "Bộ quản lý ứng dụng, điều khiển tiến trình và tích hợp Portable Linux", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Mở giao diện TUI từng bước (Mặc định)
    Tui,

    /// Tích hợp ứng dụng portable mới vào hệ thống
    Integrate {
        /// Đường dẫn tới thư mục hoặc file AppImage/binary
        path: Option<String>,
    },

    /// Gỡ gắn kết ứng dụng (Giữ nguyên file gốc)
    Unintegrate {
        /// ID của ứng dụng cần gỡ
        id: String,
    },

    /// Gỡ cài đặt ứng dụng HOÀN TOÀN (Xoá cả file gốc)
    Uninstall {
        /// ID của ứng dụng cần xoá
        id: String,
    },

    /// Liệt kê các ứng dụng trong hệ thống
    List,

    /// Kiểm tra tình trạng khả dụng của các ứng dụng
    Check,

    /// Bắt đầu chạy ứng dụng
    Start {
        /// ID của ứng dụng
        id: String,
    },

    /// Dừng chạy ứng dụng
    Stop {
        /// ID của ứng dụng
        id: String,
    },

    /// Khởi động lại ứng dụng
    Restart {
        /// ID của ứng dụng
        id: String,
    },

    /// Bật hoặc tắt khởi động cùng hệ thống
    Autostart {
        /// ID của ứng dụng
        id: String,
        /// Kích hoạt tự khởi động
        #[arg(long)]
        enable: bool,
        /// Hủy tự khởi động
        #[arg(long)]
        disable: bool,
    },
}

fn main() {
    #[cfg(target_os = "linux")]
    use std::io::IsTerminal;
    let cli = Cli::parse();

    #[cfg(target_os = "linux")]
    let is_tui = matches!(&cli.command, None | Some(Commands::Tui));

    #[cfg(target_os = "linux")]
    if is_tui
        && !std::io::stdout().is_terminal()
        && std::env::var("UNIVERSE_MANAGER_WRAPPED").is_err()
        && let Ok(exe) = std::env::current_exe()
    {
        let terminals = [
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "x-terminal-emulator",
            "xterm",
        ];
        for term in terminals {
            if std::process::Command::new(term)
                .arg("--")
                .arg(&exe)
                .env("UNIVERSE_MANAGER_WRAPPED", "1")
                .spawn()
                .is_ok()
                || std::process::Command::new(term)
                    .arg("-e")
                    .arg(&exe)
                    .env("UNIVERSE_MANAGER_WRAPPED", "1")
                    .spawn()
                    .is_ok()
            {
                return;
            }
        }
    }

    match cli.command {
        None | Some(Commands::Tui) => {
            if let Err(e) = tui::run_tui() {
                eprintln!("Lỗi khởi động giao diện TUI: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Integrate { path }) => {
            if let Err(e) = wizard::run_wizard(path) {
                eprintln!("Lỗi tích hợp: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Unintegrate { id }) => {
            println!("Đang gỡ gắn kết ứng dụng '{}'...", id);
            match remover::unintegrate(&id) {
                Ok(_) => println!("Thành công: Đã gỡ gắn kết (vẫn giữ thư mục ứng dụng)."),
                Err(e) => {
                    eprintln!("Lỗi: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::Uninstall { id }) => {
            println!("CẢNH BÁO NGUY HIỂM: Đang gỡ cài đặt HOÀN TOÀN ứng dụng '{}'...", id);
            match remover::uninstall(&id) {
                Ok(_) => println!("Thành công: Đã gỡ cài đặt hoàn toàn ứng dụng và xoá thư mục gốc."),
                Err(e) => {
                    eprintln!("Lỗi: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(Commands::List) => {
            let config = Config::load();
            if config.apps.is_empty() {
                println!("Không có ứng dụng nào trong hệ thống.");
                return;
            }

            println!(
                "{:<25} {:<20} {:<12} Trạng thái chạy",
                "App ID", "Tên ứng dụng", "Kiểu cài"
            );
            println!("{}", "-".repeat(80));
            for app in config.apps {
                let type_str = match app.install_type {
                    config::InstallType::InPlace => "In-Place",
                    config::InstallType::Moved => "Moved",
                };
                let status_str = if manager::is_app_running(&app) {
                    "RUNNING"
                } else {
                    "STOPPED"
                };
                println!("{:<25} {:<20} {:<12} {}", app.id, app.name, type_str, status_str);
            }
        }
        Some(Commands::Check) => {
            let config = Config::load();
            if config.apps.is_empty() {
                println!("Không có ứng dụng nào để kiểm tra.");
                return;
            }

            println!("Đang quét trạng thái khả dụng của tất cả ứng dụng...");
            println!("{}", "-".repeat(80));
            for app in config.apps {
                print!("- App: {:<20} (ID: {:<15}) -> ", app.name, app.id);
                match app.check_status() {
                    AppStatus::Healthy => {
                        println!("\x1b[32m[HOẠT ĐỘNG TỐT]\x1b[0m");
                    }
                    AppStatus::Degraded(issues) => {
                        println!("\x1b[33m[LỖI NHẸ / CẢNH BÁO]\x1b[0m");
                        for issue in issues {
                            println!("   ! {}", issue);
                        }
                    }
                    AppStatus::Broken(issues) => {
                        println!("\x1b[31m[HỎNG / BỊ XOÁ HOẶC DI CHUYỂN]\x1b[0m");
                        for issue in issues {
                            println!("   ✖ {}", issue);
                        }
                    }
                }
            }
        }
        Some(Commands::Start { id }) => {
            let config = Config::load();
            if let Some(app) = config.apps.iter().find(|a| a.id == id) {
                println!("Đang khởi chạy '{}'...", app.name);
                match manager::start_app(app) {
                    Ok(_) => println!("Thành công: Ứng dụng đã được khởi chạy ngầm."),
                    Err(e) => {
                        eprintln!("Lỗi khởi chạy: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Không tìm thấy ứng dụng với ID: {}", id);
                std::process::exit(1);
            }
        }
        Some(Commands::Stop { id }) => {
            let config = Config::load();
            if let Some(app) = config.apps.iter().find(|a| a.id == id) {
                println!("Đang tắt ứng dụng '{}'...", app.name);
                match manager::stop_app(app) {
                    Ok(_) => println!("Thành công: Đã gửi tín hiệu dừng tiến trình."),
                    Err(e) => {
                        eprintln!("Lỗi khi tắt: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Không tìm thấy ứng dụng với ID: {}", id);
                std::process::exit(1);
            }
        }
        Some(Commands::Restart { id }) => {
            let config = Config::load();
            if let Some(app) = config.apps.iter().find(|a| a.id == id) {
                println!("Đang khởi động lại ứng dụng '{}'...", app.name);
                match manager::restart_app(app) {
                    Ok(_) => println!("Thành công: Ứng dụng đã được khởi động lại."),
                    Err(e) => {
                        eprintln!("Lỗi khi khởi động lại: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!("Không tìm thấy ứng dụng với ID: {}", id);
                std::process::exit(1);
            }
        }
        Some(Commands::Autostart { id, enable, disable }) => {
            let config = Config::load();
            if let Some(app) = config.apps.iter().find(|a| a.id == id) {
                if enable {
                    println!("Đang cấu hình '{}' tự khởi động cùng hệ thống...", app.name);
                    match autostart::enable_autostart(app) {
                        Ok(_) => println!("Thành công: Đã bật tự động khởi động."),
                        Err(e) => {
                            eprintln!("Lỗi: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else if disable {
                    println!("Đang tắt tự khởi động của '{}'...", app.name);
                    match autostart::disable_autostart(app) {
                        Ok(_) => println!("Thành công: Đã huỷ tự động khởi động."),
                        Err(e) => {
                            eprintln!("Lỗi: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    let status = if autostart::is_autostart_enabled(app) {
                        "ĐÃ BẬT"
                    } else {
                        "ĐÃ TẮT"
                    };
                    println!("Cấu hình tự khởi động cùng hệ thống của '{}': {}", app.name, status);
                }
            } else {
                eprintln!("Không tìm thấy ứng dụng với ID: {}", id);
                std::process::exit(1);
            }
        }
    }
}
