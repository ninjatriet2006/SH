use inquire::{Password, Select};
use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn natural_sort(a: &PathBuf, b: &PathBuf) -> std::cmp::Ordering {
    let a_str = a.to_string_lossy();
    let b_str = b.to_string_lossy();
    let re = regex::Regex::new(r"(\d+)").unwrap();
    
    let a_parts: Vec<_> = re.split(&a_str).collect();
    let b_parts: Vec<_> = re.split(&b_str).collect();
    let a_nums: Vec<_> = re.find_iter(&a_str).map(|m| m.as_str().parse::<u64>().unwrap_or(0)).collect();
    let b_nums: Vec<_> = re.find_iter(&b_str).map(|m| m.as_str().parse::<u64>().unwrap_or(0)).collect();

    let mut i = 0;
    while i < a_parts.len() && i < b_parts.len() {
        if a_parts[i] != b_parts[i] {
            return a_parts[i].cmp(b_parts[i]);
        }
        if i < a_nums.len() && i < b_nums.len() && a_nums[i] != b_nums[i] {
            return a_nums[i].cmp(&b_nums[i]);
        }
        i += 1;
    }
    a_str.cmp(&b_str)
}

pub fn scan_files() -> Vec<PathBuf> {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let current_exe = env::current_exe().unwrap_or_default();
    let mut valid_files = Vec::new();
    let mut permission_denied_files = Vec::new();

    if let Ok(entries) = fs::read_dir(&current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                // Lọc file thực thi hiện tại
                if path == current_exe {
                    continue;
                }
                
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                // Bỏ qua hidden files (.*) và file cấu hình settings.yaml
                if file_name.starts_with('.') || file_name == "settings.yaml" {
                    continue;
                }
                
                // Lọc file thực thi (Linux ELF hoặc .exe, .bat)
                if let Ok(metadata) = entry.metadata() {
                    let permissions = metadata.permissions();
                    let mode = permissions.mode();
                    // Nếu là executable trên Linux hoặc file .exe/.bat trên Windows
                    if mode & 0o111 != 0 || file_name.ends_with(".exe") || file_name.ends_with(".bat") {
                        continue;
                    }
                }

                // Kiểm tra quyền Read
                match fs::File::open(&path) {
                    Ok(_) => valid_files.push(path),
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        permission_denied_files.push(path);
                    }
                    Err(_) => valid_files.push(path), // Các lỗi khác tạm chấp nhận để xử lý sau
                }
            }
        }
    }

    if !permission_denied_files.is_empty() {
        println!("Cảnh báo: Phát hiện {} file bị từ chối quyền truy cập (Permission Denied).", permission_denied_files.len());
        let options = vec![
            "Tiếp tục và nhập mật khẩu sudo để chạy lại chương trình bằng quyền quản trị",
            "Bỏ qua các file này và xử lý các file khác",
            "Thoát chương trình",
        ];
        
        let choice = Select::new("Lựa chọn của bạn:", options).prompt();
        match choice {
            Ok(ans) if ans.starts_with("Tiếp tục") => {
                let password = Password::new("Nhập mật khẩu sudo:")
                    .without_confirmation()
                    .prompt()
                    .unwrap_or_default();
                
                // Restart lại script với quyền sudo
                let mut child = Command::new("sudo")
                    .arg("-S")
                    .arg(current_exe)
                    .stdin(Stdio::piped())
                    .spawn()
                    .expect("Không thể khởi động lại với sudo");
                
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(format!("{}\n", password).as_bytes());
                }
                let status = child.wait().expect("Lỗi khi đợi tiến trình sudo");
                std::process::exit(status.code().unwrap_or(1));
            }
            Ok(ans) if ans.starts_with("Thoát") => {
                std::process::exit(1);
            }
            _ => {
                println!("Tiếp tục bỏ qua file lỗi quyền...");
            }
        }
    }

    valid_files.sort_by(natural_sort);
    valid_files
}

pub fn is_image_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(ext_str.as_str(), "jpg" | "jpeg" | "png" | "webp" | "avif" | "heic" | "bmp" | "tiff")
    } else {
        false
    }
}
