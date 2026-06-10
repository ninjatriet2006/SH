use std::process::Command;
use crate::system::permissions::run_as_admin;

#[derive(Debug)]
pub struct DepsResult {
    pub is_ok: bool,
    pub missing: Vec<String>,
}

pub async fn check_all() -> anyhow::Result<DepsResult> {
    let mut missing = Vec::new();

    // Check ffmpeg
    if !cmd_exists("ffmpeg", &["-version"]) {
        missing.push("ffmpeg".to_string());
    }

    // Check 7z
    if !cmd_exists("7z", &["--help"]) && !cmd_exists("7z", &[]) {
        missing.push("7z (p7zip)".to_string());
    }

    // Check LibreOffice (soffice)
    if !cmd_exists("soffice", &["--version"]) {
        missing.push("libreoffice (soffice)".to_string());
    }

    let is_ok = missing.is_empty();
    Ok(DepsResult { is_ok, missing })
}

fn cmd_exists(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

pub fn prompt_install(missing: &[String]) -> anyhow::Result<()> {
    println!("\n[⚠️ CẢNH BÁO] Phát hiện thiếu các thư viện hệ thống sau:");
    for dep in missing {
        println!(" - {}", dep);
    }

    #[cfg(target_os = "linux")]
    {
        println!("\nHệ thống Linux phát hiện thấy thiếu dependencies.");
        println!("Bạn có muốn tự động cài đặt qua APT không? (Yêu cầu quyền sudo) [Y/n]");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        let answer = answer.trim().to_lowercase();
        if answer.is_empty() || answer == "y" {
            println!("Đang chuẩn bị cài đặt...");
            let mut pkgs = Vec::new();
            for dep in missing {
                match dep.as_str() {
                    "ffmpeg" => pkgs.push("ffmpeg"),
                    "7z (p7zip)" => pkgs.push("p7zip-full"),
                    "libreoffice (soffice)" => pkgs.push("libreoffice"),
                    _ => {}
                }
            }
            if !pkgs.is_empty() {
                let pkg_list = pkgs.join(" ");
                let cmd = format!("apt-get update && apt-get install -y {}", pkg_list);
                if run_as_admin(&cmd).is_ok() {
                    println!("[✅] Cài đặt dependencies hoàn tất!");
                } else {
                    println!("[❌] Cài đặt thất bại. Vui lòng cài thủ công bằng lệnh: sudo apt install {}", pkg_list);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        println!("\nVui lòng cài đặt các công cụ thiếu:");
        println!("- ffmpeg: Tải từ gypa.co hoặc qua winget: `winget install FFmpeg`");
        println!("- 7z: Tải từ 7-zip.org hoặc qua winget: `winget install 7zip.7zip`");
        println!("- libreoffice: Tải từ libreoffice.org hoặc qua winget: `winget install TheDocumentFoundation.LibreOffice`");
        println!("Sau khi cài đặt xong và cấu hình Environment PATH, hãy khởi động lại công cụ.");
        std::process::exit(1);
    }

    Ok(())
}
