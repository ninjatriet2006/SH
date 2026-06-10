use std::fs;
use std::env;
use std::path::PathBuf;
use crate::system::permissions::run_as_admin;

fn get_nemo_actions_dir() -> PathBuf {
    let home = env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".local/share/nemo/actions")
}

pub fn install_nemo_action(mode: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // Check if Nemo is installed
        let nemo_check = std::process::Command::new("which")
            .arg("nemo")
            .output();

        if nemo_check.is_err() || !nemo_check.unwrap().status.success() {
            println!("Không tìm thấy file manager Nemo. Bạn có muốn cài đặt Nemo? [Y/n]");
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer)?;
            if answer.trim().to_lowercase() == "y" || answer.trim().is_empty() {
                run_as_admin("apt-get install -y nemo")?;
            } else {
                anyhow::bail!("Hủy cài đặt vì thiếu Nemo.");
            }
        }

        let dir = get_nemo_actions_dir();
        fs::create_dir_all(&dir)?;

        let current_exe = env::current_exe()?;
        let exe_str = current_exe.to_string_lossy();

        let filename = format!("universal_converter_{}.nemo_action", mode);
        let filepath = dir.join(filename);

        let name = match mode {
            "default" => "Chuyển đổi với Universal Converter (Default)",
            "config" => "Chuyển đổi với Universal Converter (Config)",
            _ => "Universal Converter",
        };

        // We use x-terminal-emulator or gnome-terminal to run the command
        // Passing %F (multiple files bôi đen)
        let content = format!(
            r#"[Nemo Action]
Name={}
Comment=Chuyển đổi / Xử lý file đa luồng
Exec=gnome-terminal -- {} --context {} %F
Selection=any
Extensions=any;
"#,
            name, exe_str, mode
        );

        fs::write(filepath, content)?;
        println!("[✅] Đã cài đặt Nemo Action ({}) thành công!", mode);
    }

    #[cfg(target_os = "windows")]
    {
        let current_exe = env::current_exe()?;
        let exe_str = current_exe.to_string_lossy().replace('/', "\\");

        let (key_name, display_name) = match mode {
            "default" => ("universal_converter_default", "Chuyển đổi với Universal Converter (Default)"),
            "config" => ("universal_converter_config", "Chuyển đổi với Universal Converter (Config)"),
            _ => ("universal_converter", "Universal Converter"),
        };

        // Command to run Registry Add via run_as_admin
        // Note double quotes escaping inside registry paths
        let cmd = format!(
            "reg add \"HKCR\\*\\shell\\{}\" /ve /t REG_SZ /d \"{}\" /f && \
             reg add \"HKCR\\*\\shell\\{}\\command\" /ve /t REG_SZ /d \"\\\"{}\\\" --context {} \\\"%1\\\"\" /f",
            key_name, display_name, key_name, exe_str, mode
        );

        run_as_admin(&cmd)?;
        println!("[✅] Đã cài đặt Context Menu ({}) trên Windows Explorer thành công!", mode);
    }

    Ok(())
}

pub fn uninstall_all() -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let dir = get_nemo_actions_dir();
        let _ = fs::remove_file(dir.join("universal_converter_default.nemo_action"));
        let _ = fs::remove_file(dir.join("universal_converter_config.nemo_action"));
        println!("[✅] Đã gỡ tích hợp khỏi Nemo Action!");
    }

    #[cfg(target_os = "windows")]
    {
        let cmd = "reg delete \"HKCR\\*\\shell\\universal_converter_default\" /f && reg delete \"HKCR\\*\\shell\\universal_converter_config\" /f";
        let _ = run_as_admin(cmd);
        println!("[✅] Đã gỡ tích hợp khỏi Windows Registry!");
    }

    Ok(())
}
