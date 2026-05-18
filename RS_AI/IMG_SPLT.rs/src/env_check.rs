use inquire::{Select, Password};
use regex::Regex;
use std::io::Write;
use std::process::{Command, Stdio};
use std::str;

pub fn resize_terminal() {
    // Thử gửi escape sequence trước
    print!("\x1B[8;35;110t");
    let _ = std::io::stdout().flush();
}

pub fn check_ffmpeg() {
    let output = Command::new("ffmpeg").arg("-version").output();
    let is_ok = match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = str::from_utf8(&out.stdout).unwrap_or("");
                let re = Regex::new(r"ffmpeg version (\d+)").unwrap();
                if let Some(caps) = re.captures(stdout) {
                    if let Ok(version) = caps[1].parse::<u32>() {
                        version >= 4
                    } else {
                        true // Regex matched but parsing failed, assume true or check further
                    }
                } else {
                    true // Custom builds might not have a clean number, assume OK if it runs
                }
            } else {
                false
            }
        }
        Err(_) => false,
    };

    if !is_ok {
        println!("Cảnh báo: Không tìm thấy 'ffmpeg' hoặc phiên bản quá cũ (< 4.0).");
        let options = vec![
            "Cài đặt/Cập nhật dependencies mới nhất (yêu cầu quyền sudo)",
            "Bỏ qua cảnh báo và chạy tiếp (Run anyway)",
            "Thoát chương trình",
        ];
        
        let choice = Select::new("Vui lòng chọn hành động:", options).prompt();
        match choice {
            Ok(ans) if ans.starts_with("Cài đặt") => {
                let password = Password::new("Nhập mật khẩu sudo:")
                    .without_confirmation()
                    .prompt()
                    .unwrap_or_default();
                
                let mut child = Command::new("sudo")
                    .arg("-S")
                    .arg("apt-get")
                    .arg("update")
                    .stdin(Stdio::piped())
                    .spawn()
                    .expect("Không thể khởi động sudo");
                
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(format!("{}\n", password).as_bytes());
                }
                let _ = child.wait();
                
                let mut child2 = Command::new("sudo")
                    .arg("-S")
                    .arg("apt-get")
                    .arg("install")
                    .arg("-y")
                    .arg("ffmpeg")
                    .stdin(Stdio::piped())
                    .spawn()
                    .expect("Không thể khởi động sudo");
                
                if let Some(mut stdin) = child2.stdin.take() {
                    let _ = stdin.write_all(format!("{}\n", password).as_bytes());
                }
                let _ = child2.wait();
            }
            Ok(ans) if ans.starts_with("Thoát") => {
                std::process::exit(1);
            }
            _ => {
                println!("Tiếp tục chạy mặc dù có cảnh báo...");
            }
        }
    }
    
    // Check ffprobe
    if Command::new("ffprobe").arg("-version").output().is_err() {
        println!("Cảnh báo: Không tìm thấy 'ffprobe'.");
    }
}
