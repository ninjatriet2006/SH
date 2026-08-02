use inquire::{Password, Select};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn natural_sort(a: &Path, b: &Path) -> std::cmp::Ordering {
    let a_str = a.to_string_lossy();
    let b_str = b.to_string_lossy();
    let re = regex::Regex::new(r"(\d+)").unwrap();

    let a_parts: Vec<_> = re.split(&a_str).collect();
    let b_parts: Vec<_> = re.split(&b_str).collect();
    let a_nums: Vec<_> = re
        .find_iter(&a_str)
        .map(|m| m.as_str().parse::<u64>().unwrap_or(0))
        .collect();
    let b_nums: Vec<_> = re
        .find_iter(&b_str)
        .map(|m| m.as_str().parse::<u64>().unwrap_or(0))
        .collect();

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
                    #[cfg(unix)]
                    let is_exec = {
                        use std::os::unix::fs::PermissionsExt;
                        metadata.permissions().mode() & 0o111 != 0
                    };
                    #[cfg(not(unix))]
                    let is_exec = false;

                    // Nếu là executable trên Linux hoặc file .exe/.bat trên Windows
                    if is_exec || file_name.ends_with(".exe") || file_name.ends_with(".bat") {
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
        println!(
            "Cảnh báo: Phát hiện {} file bị từ chối quyền truy cập (Permission Denied).",
            permission_denied_files.len()
        );
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

    valid_files.sort_by(|a, b| natural_sort(a.as_path(), b.as_path()));
    valid_files
}

pub fn is_image_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        matches!(
            ext_str.as_str(),
            "jpg" | "jpeg" | "png" | "webp" | "avif" | "heic" | "bmp" | "tiff"
        )
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_image_extension_known_formats() {
        let test_cases = vec![
            ("photo.jpg", true),
            ("photo.jpeg", true),
            ("photo.png", true),
            ("photo.webp", true),
            ("photo.avif", true),
            ("photo.heic", true),
            ("photo.bmp", true),
            ("photo.tiff", true),
            ("photo.JPG", true),  // uppercase
            ("photo.PNG", true),  // uppercase
            ("photo.Jpeg", true), // mixed case
        ];
        for (name, expected) in test_cases {
            let path = Path::new(name);
            assert_eq!(
                is_image_extension(path),
                expected,
                "Failed for '{}': expected {}, got {}",
                name,
                expected,
                !expected
            );
        }
    }

    #[test]
    fn test_is_image_extension_non_images() {
        let test_cases = vec![
            ("document.pdf", false),
            ("video.mp4", false),
            ("script.rs", false),
            ("archive.zip", false),
            ("readme.txt", false),
            ("image.gif", false), // gif is NOT in the supported list
            ("image.svg", false), // svg is NOT in the supported list
            ("no_extension", false),
            (".hidden_file", false),
        ];
        for (name, expected) in test_cases {
            let path = Path::new(name);
            assert_eq!(
                is_image_extension(path),
                expected,
                "Failed for '{}': expected {}, got {}",
                name,
                expected,
                !expected
            );
        }
    }

    #[test]
    fn test_is_image_extension_no_extension() {
        let path = Path::new("README");
        assert!(!is_image_extension(path));
        let path = Path::new("/some/dir/file");
        assert!(!is_image_extension(path));
    }

    #[test]
    fn test_is_image_extension_path_with_directories() {
        let path = Path::new("/home/user/photos/sunset.png");
        assert!(is_image_extension(path));
        let path = Path::new("./relative/path/image.JPEG");
        assert!(is_image_extension(path));
    }
}
