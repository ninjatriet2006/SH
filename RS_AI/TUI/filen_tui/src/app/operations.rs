use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FileItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mod_time: String,
}

pub struct Operations;

fn resolve_filen_bin() -> PathBuf {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            // 1. Kiểm tra biến môi trường
            if let Ok(val) = std::env::var("FILEN_BIN_PATH") {
                let path = PathBuf::from(val);
                if path.exists() {
                    return path;
                }
            }

            // 2. Quét trong PATH của hệ thống sử dụng crate which
            if let Ok(path) = which::which("filen") {
                return path;
            }

            // 3. Quét các đường dẫn cài đặt mặc định thông thường
            if let Some(home) = dirs::home_dir() {
                // Đường dẫn trên Unix/Linux
                let unix_path = home.join(".filen-cli/bin/filen");
                if unix_path.exists() {
                    return unix_path;
                }

                let unix_config_path = home.join(".config/filen-cli/bin/filen");
                if unix_config_path.exists() {
                    return unix_config_path;
                }

                // Đường dẫn trên Windows
                let win_path = home.join(".filen-cli\\bin\\filen.exe");
                if win_path.exists() {
                    return win_path;
                }

                let win_cmd_path = home.join(".filen-cli\\bin\\filen.cmd");
                if win_cmd_path.exists() {
                    return win_cmd_path;
                }

                // Đường dẫn Global npm trên Windows
                if cfg!(windows)
                    && let Ok(appdata) = std::env::var("APPDATA")
                {
                    let npm_path = PathBuf::from(appdata).join("npm\\filen.cmd");
                    if npm_path.exists() {
                        return npm_path;
                    }
                }
            }

            // Phương án dự phòng cuối cùng
            if cfg!(windows) {
                PathBuf::from("filen.cmd")
            } else {
                PathBuf::from("filen")
            }
        })
        .clone()
}

impl Operations {
    pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
        // Dự phòng cho hệ điều hành Windows sử dụng clip.exe
        if cfg!(target_os = "windows") {
            let child = std::process::Command::new("clip")
                .stdin(std::process::Stdio::piped())
                .spawn();
            if let Ok(mut c) = child {
                if let Some(mut stdin) = c.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = c.wait();
                return Ok(());
            }
        }

        // Các công cụ cho Unix/Linux
        let child = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn();
        if let Ok(mut c) = child {
            if let Some(mut stdin) = c.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
            return Ok(());
        }

        let child = std::process::Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn();
        if let Ok(mut c) = child {
            if let Some(mut stdin) = c.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
            return Ok(());
        }

        let child = std::process::Command::new("xsel")
            .arg("--clipboard")
            .arg("--input")
            .stdin(std::process::Stdio::piped())
            .spawn();
        if let Ok(mut c) = child {
            if let Some(mut stdin) = c.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = c.wait();
            return Ok(());
        }

        Err("Không tìm thấy công cụ sao chép clipboard (clip, wl-copy, xclip, xsel)".to_string())
    }

    // Lấy đối tượng Command cấu hình sẵn cờ --data-dir dựa trên tài khoản active
    pub fn get_command(_active_account: &Option<String>) -> Command {
        let mut cmd = Command::new(resolve_filen_bin());
        if let Some(data_path) = super::get_default_data_dir() {
            cmd.arg("--data-dir").arg(data_path);
        }
        cmd.kill_on_drop(true);
        cmd
    }

    // Chạy lệnh Command với một thời gian chờ (timeout) để tránh bị treo khi CLI yêu cầu nhập liệu
    pub async fn run_cmd_with_timeout(mut cmd: Command, timeout_secs: u64) -> Result<std::process::Output, String> {
        use std::process::Stdio;
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output()).await {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => {
                Err("Yêu cầu quá hạn (timeout). Có thể do tài khoản chưa đăng nhập hoặc lỗi kết nối mạng.".to_string())
            }
        }
    }

    // Lấy thông tin tài khoản đang hoạt động (whoami)
    #[allow(dead_code)]
    pub async fn whoami(active_account: &Option<String>) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("whoami");
        let output = Self::run_cmd_with_timeout(cmd, 15).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Lấy dung lượng lưu trữ (statfs) -> Trả về (đã dùng, tổng dung lượng) dưới dạng chuỗi
    pub async fn statfs(active_account: &Option<String>) -> Result<(String, String), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("statfs");
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut used = "0 B".to_string();
            let mut max = "20 GiB".to_string();
            for line in text.lines() {
                if line.contains("Used:") {
                    used = line.replace("Used:", "").trim().to_string();
                } else if line.contains("Max:") {
                    max = line.replace("Max:", "").trim().to_string();
                }
            }
            Ok((used, max))
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Duyệt thư mục Cloud (ls [path] --long)
    pub async fn list_remote(active_account: &Option<String>, path: &str) -> Result<Vec<FileItem>, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("ls").arg(path).arg("--long");
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut items = Vec::new();
        if path != "/" && !path.is_empty() {
            items.push(FileItem {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
                mod_time: String::new(),
            });
        }
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            // Định dạng: "22 B  2026-06-07 13:17:03.31  hello.txt" hoặc "  2026-06-07 13:16:45.00  test_dir"
            // Tìm chỉ số ngày tháng bằng regex đơn giản (tìm chuỗi có dạng YYYY-MM-DD)
            if let Some(date_idx) = find_date_index(line)
                && date_idx + 22 <= line.len()
            {
                let size_part = line[..date_idx].trim();
                let mod_time = line[date_idx..date_idx + 19].to_string(); // Bỏ đi phần miligiây .xx cho gọn
                let name = line[date_idx + 22..].trim().to_string();

                let is_dir = size_part.is_empty();
                let size = if is_dir { 0 } else { parse_size_bytes(size_part) };

                items.push(FileItem {
                    name,
                    is_dir,
                    size,
                    mod_time,
                });
            }
        }
        // Sắp xếp: Thư mục lên trước, sau đó sắp xếp theo tên (giữ .. ở đầu)
        items.sort_by(|a, b| {
            if a.name == ".." {
                std::cmp::Ordering::Less
            } else if b.name == ".." {
                std::cmp::Ordering::Greater
            } else {
                b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name))
            }
        });
        Ok(items)
    }

    // Duyệt thư mục cục bộ (Local)
    pub fn list_local(path: &str) -> Result<Vec<FileItem>, String> {
        let dir = Path::new(path);
        if !dir.is_dir() {
            return Err("Không phải là thư mục".to_string());
        }
        let mut items = Vec::new();
        if let Some(parent) = dir.parent()
            && parent != dir
        {
            items.push(FileItem {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
                mod_time: String::new(),
            });
        }
        let read_dir = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            let is_dir = metadata.is_dir();
            let size = if is_dir { 0 } else { metadata.len() };
            let mod_time = metadata
                .modified()
                .ok()
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Local> = t.into();
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_else(|| "N/A".to_string());

            items.push(FileItem {
                name,
                is_dir,
                size,
                mod_time,
            });
        }
        items.sort_by(|a, b| {
            if a.name == ".." {
                std::cmp::Ordering::Less
            } else if b.name == ".." {
                std::cmp::Ordering::Greater
            } else {
                b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name))
            }
        });
        Ok(items)
    }

    // Tạo thư mục mới (mkdir)
    pub async fn mkdir(active_account: &Option<String>, path: &str) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("mkdir").arg(path);
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Xóa file/thư mục (rm)
    pub async fn rm(active_account: &Option<String>, path: &str, no_trash: bool) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("rm").arg(path);
        if no_trash {
            cmd.arg("--no-trash");
        }
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Đổi tên hoặc di chuyển (mv)
    pub async fn mv(active_account: &Option<String>, from: &str, to: &str) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("mv").arg(from).arg(to);
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Sao chép (cp)
    pub async fn cp(active_account: &Option<String>, from: &str, to: &str) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("cp").arg(from).arg(to);
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Tải lên tệp (upload)
    pub async fn upload(active_account: &Option<String>, local: &str, remote: &str) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("upload").arg(local).arg(remote);
        let output = cmd.output().await.map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Tải xuống tệp (download)
    pub async fn download(active_account: &Option<String>, remote: &str, local: &str) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("download").arg(remote).arg(local);
        let output = cmd.output().await.map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Đọc nội dung file text (cat)
    pub async fn cat(active_account: &Option<String>, path: &str) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("cat").arg(path);
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Thêm vào mục Yêu thích (favorite)
    pub async fn favorite(active_account: &Option<String>, path: &str) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("favorite").arg(path);
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Bỏ Yêu thích (unfavorite)
    pub async fn unfavorite(active_account: &Option<String>, path: &str) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("unfavorite").arg(path);
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Danh sách Yêu thích
    #[allow(dead_code)]
    pub async fn list_favorites(active_account: &Option<String>) -> Result<Vec<FileItem>, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("favorites");
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut items = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            items.push(FileItem {
                name: line.to_string(),
                is_dir: false, // giả định là file, hoặc chúng ta không phân biệt được
                size: 0,
                mod_time: "N/A".to_string(),
            });
        }
        Ok(items)
    }

    // Danh sách thùng rác (trash)
    pub async fn list_trash(active_account: &Option<String>) -> Result<Vec<FileItem>, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("trash");
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut items = Vec::new();
        for line in text.lines() {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            if let Some(date_idx) = find_date_index(line)
                && date_idx + 22 <= line.len()
            {
                let size_part = line[..date_idx].trim();
                let mod_time = line[date_idx..date_idx + 19].to_string();
                let name = line[date_idx + 22..].trim().to_string();
                let is_dir = size_part.is_empty();
                let size = if is_dir { 0 } else { parse_size_bytes(size_part) };
                items.push(FileItem {
                    name,
                    is_dir,
                    size,
                    mod_time,
                });
            }
        }
        Ok(items)
    }

    // Khôi phục thùng rác (trash restore)
    pub async fn trash_restore(active_account: &Option<String>, idx_1based: usize) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("trash").arg("restore");
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            let input_data = format!("{}\n", idx_1based);
            let _ = stdin.write_all(input_data.as_bytes()).await;
        }
        let output = match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => {
                return Err(
                    "Yêu cầu quá hạn (timeout). Có thể do tài khoản chưa đăng nhập hoặc lỗi kết nối.".to_string(),
                );
            }
        };
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Xóa vĩnh viễn mục trong thùng rác (trash delete)
    #[allow(dead_code)]
    pub async fn trash_delete(active_account: &Option<String>, idx_1based: usize) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("trash").arg("delete");
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            let input_data = format!("{}\n", idx_1based);
            let _ = stdin.write_all(input_data.as_bytes()).await;
        }
        let output = match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => {
                return Err(
                    "Yêu cầu quá hạn (timeout). Có thể do tài khoản chưa đăng nhập hoặc lỗi kết nối.".to_string(),
                );
            }
        };
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Dọn sạch thùng rác (trash empty)
    pub async fn trash_empty(active_account: &Option<String>) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("trash").arg("empty");
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Tạo link công khai (links <path>)
    pub async fn create_link(active_account: &Option<String>, path: &str) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("links").arg(path);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            // Khi tạo link mới cho path chưa có link, CLI sẽ hỏi:
            // "Public link doesn't exist. Create it? (Y/N): "
            // Chúng ta gửi "Y\n" để xác nhận tạo
            let _ = stdin.write_all(b"Y\n").await;
        }
        let output = match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => {
                return Err(
                    "Yêu cầu quá hạn (timeout 30s). Có thể do tài khoản chưa đăng nhập hoặc lỗi kết nối.".to_string(),
                );
            }
        };
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            // Link thường hiển thị ở cuối dạng "Link: https://..." hoặc chỉ URL
            let mut link = String::new();
            for line in text.lines() {
                if line.contains("https://")
                    && let Some(pos) = line.find("https://")
                {
                    link = line[pos..].trim().to_string();
                }
            }
            if link.is_empty() {
                Ok(text.trim().to_string())
            } else {
                Ok(link)
            }
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Danh sách public links
    #[allow(dead_code)]
    pub async fn list_links(active_account: &Option<String>) -> Result<Vec<(String, String)>, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("links");
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut items = Vec::new();
        // Định dạng ra: "/some_file  https://filen.io/d/..."
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(pos) = line.find("https://") {
                let path = line[..pos].trim().to_string();
                let url = line[pos..].trim().to_string();
                items.push((path, url));
            }
        }
        Ok(items)
    }

    // Đăng nhập tài khoản mới (dùng stdbuf để bỏ đệm và giao tiếp với tiến trình con tương tác)
    // Đăng nhập tài khoản mới (dùng stdbuf để bỏ đệm và giao tiếp với tiến trình con tương tác)
    pub async fn login_new(
        email: &str,
        password: &str,
        twofa_code: Option<&str>,
        keep_logged: &str,
        tx: Option<tokio::sync::mpsc::UnboundedSender<super::AppEvent>>,
    ) -> Result<(), String> {
        let log = |msg: String| {
            if let Some(ref tx) = tx {
                let _ = tx.send(super::AppEvent::LoginLog(msg));
            }
        };

        if let Some(data_path) = super::get_default_data_dir() {
            std::fs::create_dir_all(&data_path).map_err(|e| e.to_string())?;

            // Xóa session cũ nếu có để tránh việc CLI cố đọc cấu hình hỏng và tự động báo lỗi crash decryption
            let keep_file = data_path.join(".filen-cli-keep-me-logged-in");
            let creds_file = data_path.join(".filen-cli-credentials");
            if keep_file.exists() {
                let _ = std::fs::remove_file(keep_file);
            }
            if creds_file.exists() {
                let _ = std::fs::remove_file(creds_file);
            }

            let bin = resolve_filen_bin();
            let mut cmd = if cfg!(windows) {
                log(format!("=== Khởi chạy tiến trình CLI: `{}` ===", bin.display()));
                Command::new(&bin)
            } else {
                log(format!(
                    "=== Khởi chạy tiến trình CLI: `{}` qua `stdbuf` ===",
                    bin.display()
                ));
                let mut c = Command::new("stdbuf");
                c.arg("-o0").arg("-e0").arg(&bin);
                c
            };
            cmd.kill_on_drop(true);
            cmd.arg("--data-dir").arg(&data_path).arg("whoami");
            cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

            let mut child = cmd.spawn().map_err(|e| e.to_string())?;
            let mut stdout = child.stdout.take().unwrap();
            let mut stderr = child.stderr.take().unwrap();
            let mut stdin = child.stdin.take().unwrap();

            let mut email_sent = false;
            let mut pass_sent = false;
            let mut code_sent = false;
            let mut keep_logged_sent = false;

            let mut accumulated = String::new();
            let mut stdout_buf = [0u8; 1024];
            let mut stderr_buf = [0u8; 1024];

            loop {
                tokio::select! {
                    res = stdout.read(&mut stdout_buf) => {
                        match res {
                            Ok(0) => break, // stdout EOF
                            Ok(n) => {
                                let text = String::from_utf8_lossy(&stdout_buf[..n]);
                                accumulated.push_str(&text);
                                for line in text.lines() {
                                    if !line.trim().is_empty() {
                                        log(format!("<- CLI: {}", line.trim()));
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    res = stderr.read(&mut stderr_buf) => {
                        match res {
                            Ok(0) => {}, // stderr EOF
                            Ok(n) => {
                                let text = String::from_utf8_lossy(&stderr_buf[..n]);
                                accumulated.push_str(&text);
                                for line in text.lines() {
                                    if !line.trim().is_empty() {
                                        log(format!("<- CLI (err): {}", line.trim()));
                                    }
                                }
                            }
                            Err(_) => {},
                        }
                    }
                }

                let acc_lower = accumulated.to_lowercase();

                // 1. Gửi Email khi CLI nhắc
                if !email_sent && acc_lower.contains("email:") {
                    log(format!("-> Gửi địa chỉ Email: {}", email));
                    let _ = stdin.write_all(format!("{}\n", email).as_bytes()).await;
                    let _ = stdin.flush().await;
                    email_sent = true;
                    accumulated.clear();
                }
                // 2. Gửi Password khi CLI nhắc
                else if email_sent && !pass_sent && acc_lower.contains("password:") {
                    log("-> Gửi Mật khẩu: [********]".to_string());
                    let _ = stdin.write_all(format!("{}\n", password).as_bytes()).await;
                    let _ = stdin.flush().await;
                    pass_sent = true;
                    accumulated.clear();
                }
                // 3. Sau khi gửi Password, phản hồi các bước tiếp theo
                else if pass_sent {
                    // Nếu tài khoản bật 2FA và CLI hỏi mã
                    if acc_lower.contains("2fa code") || acc_lower.contains("recovery key") {
                        if let Some(code) = twofa_code {
                            if !code_sent {
                                log(format!("-> Gửi mã xác thực 2FA: {}", code));
                                let _ = stdin.write_all(format!("{}\n", code).as_bytes()).await;
                                let _ = stdin.flush().await;
                                code_sent = true;
                                accumulated.clear();
                            }
                        } else {
                            log("=== CLI yêu cầu mã 2FA. Đang tạm dừng để hiển thị màn hình nhập TOTP ===".to_string());
                            let _ = child.kill().await;
                            return Err("2FA_REQUIRED".to_string());
                        }
                    }
                    // Nếu CLI hỏi lưu phiên đăng nhập (Keep me logged in?)
                    else if acc_lower.contains("keep me logged in") || acc_lower.contains("save credentials") {
                        if !keep_logged_sent {
                            log(format!("-> Gửi Duy trì đăng nhập: {}", keep_logged));
                            let response = format!("{}\n", keep_logged);
                            let _ = stdin.write_all(response.as_bytes()).await;
                            let _ = stdin.flush().await;
                            keep_logged_sent = true;
                            accumulated.clear();
                        }
                    }
                    // Nếu CLI báo thông tin sai
                    else if acc_lower.contains("invalid credentials") {
                        log("=== CLI báo thông tin đăng nhập không chính xác ===".to_string());
                        let _ = child.kill().await;
                        return Err("Email hoặc Mật khẩu không chính xác. Vui lòng kiểm tra lại.".to_string());
                    }
                }
            }

            let status = child.wait().await.map_err(|e| e.to_string())?;
            if status.success() {
                log("=== Đăng nhập thành công! Phiên làm việc đã được lưu ===".to_string());
                Ok(())
            } else {
                let err_lower = accumulated.to_lowercase();
                let err_msg = if err_lower.contains("invalid credentials") {
                    "Email hoặc Mật khẩu không chính xác. Vui lòng kiểm tra lại.".to_string()
                } else if !accumulated.trim().is_empty() {
                    accumulated.trim().to_string()
                } else {
                    "Đăng nhập thất bại. Vui lòng thử lại.".to_string()
                };
                log(format!("=== LỖI: CLI thoát với lỗi: {} ===", err_msg));
                Err(err_msg)
            }
        } else {
            Err("Không tìm thấy thư mục Home".to_string())
        }
    }

    // Đăng xuất (logout)
    pub async fn logout(active_account: &Option<String>) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("logout");
        match tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output()).await {
            Ok(Ok(output)) => {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
                }
            }
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err("Quá thời gian chờ đăng xuất (timeout 10s).".to_string()),
        }
    }

    // Xuất auth config
    pub async fn export_auth_config(active_account: &Option<String>) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("export-auth-config");
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"I am aware of the risks\n").await;
            let _ = stdin.flush().await;
        }

        let output = match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => return Err("Quá thời gian chờ xuất cấu hình (timeout 30s).".to_string()),
        };

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Xuất API Key
    pub async fn export_api_key(active_account: &Option<String>) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("export-api-key");
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;

        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(b"y\n").await;
            let _ = stdin.flush().await;
        }

        let output = match tokio::time::timeout(std::time::Duration::from_secs(30), child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e.to_string()),
            Err(_) => return Err("Quá thời gian chờ xuất API Key (timeout 30s).".to_string()),
        };

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

// Tìm index của chuỗi có dạng YYYY-MM-DD
fn find_date_index(line: &str) -> Option<usize> {
    if line.len() < 10 {
        return None;
    }
    // Tìm mẫu: 4 chữ số, 1 dấu gạch, 2 chữ số, 1 dấu gạch, 2 chữ số
    let bytes = line.as_bytes();
    (0..=(line.len() - 10)).find(|&i| {
        bytes[i].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && bytes[i + 4] == b'-'
            && bytes[i + 5].is_ascii_digit()
            && bytes[i + 6].is_ascii_digit()
            && bytes[i + 7] == b'-'
            && bytes[i + 8].is_ascii_digit()
            && bytes[i + 9].is_ascii_digit()
    })
}

// Phân tích chuỗi dung lượng thành số byte (ví dụ: "22 B" -> 22, "1.5 KiB" -> 1536)
fn parse_size_bytes(size_str: &str) -> u64 {
    let parts: Vec<&str> = size_str.split_whitespace().collect();
    if parts.is_empty() {
        return 0;
    }
    let num_val: f64 = parts[0].parse().unwrap_or(0.0);
    if parts.len() < 2 {
        return num_val as u64;
    }
    let unit = parts[1].to_uppercase();
    let multiplier: f64 = match unit.as_str() {
        "B" => 1.0,
        "KB" | "KIB" => 1024.0,
        "MB" | "MIB" => 1024.0 * 1024.0,
        "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
        "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    (num_val * multiplier) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── FileItem ────────────────────────────────────────────────────────────────

    #[test]
    fn test_file_item_creation() {
        let item = FileItem {
            name: "hello.txt".to_string(),
            is_dir: false,
            size: 1024,
            mod_time: "2024-06-07 13:17:03".to_string(),
        };
        assert_eq!(item.name, "hello.txt");
        assert!(!item.is_dir);
        assert_eq!(item.size, 1024);
        assert_eq!(item.mod_time, "2024-06-07 13:17:03");
    }

    #[test]
    fn test_file_item_directory() {
        let item = FileItem {
            name: "my_folder".to_string(),
            is_dir: true,
            size: 0,
            mod_time: "2024-06-07 13:16:45".to_string(),
        };
        assert_eq!(item.name, "my_folder");
        assert!(item.is_dir);
        assert_eq!(item.size, 0);
    }

    #[test]
    fn test_file_item_debug_and_clone() {
        let item = FileItem {
            name: "a.txt".to_string(),
            is_dir: false,
            size: 42,
            mod_time: "".to_string(),
        };
        // Debug trait
        let debug_str = format!("{:?}", item);
        assert!(debug_str.contains("a.txt"));
        // Clone trait
        let cloned = item.clone();
        assert_eq!(item, cloned);
    }

    #[test]
    fn test_file_item_partial_eq() {
        let a = FileItem {
            name: "f1".to_string(),
            is_dir: false,
            size: 100,
            mod_time: "t1".to_string(),
        };
        let b = FileItem {
            name: "f1".to_string(),
            is_dir: false,
            size: 100,
            mod_time: "t1".to_string(),
        };
        assert_eq!(a, b);

        let c = FileItem {
            name: "f2".to_string(),
            ..a.clone()
        };
        assert_ne!(a, c);
    }

    // ─── find_date_index ─────────────────────────────────────────────────────────

    #[test]
    fn test_find_date_index_valid_date_at_start() {
        // Date at the very beginning
        let line = "2024-01-15 13:17:03.31  hello.txt";
        assert_eq!(find_date_index(line), Some(0));
    }

    #[test]
    fn test_find_date_index_valid_date_with_prefix() {
        // Date preceded by spaces/size (same as `ls --long` output)
        let line = "22 B  2026-06-07 13:17:03.31  hello.txt";
        let idx = find_date_index(line);
        assert!(idx.is_some());
        // Should point to the start of "2026-06-07"
        let date_str = &line[idx.unwrap()..idx.unwrap() + 10];
        assert_eq!(date_str, "2026-06-07");
    }

    #[test]
    fn test_find_date_index_valid_date_no_prefix() {
        // Directory entry: no size prefix, just date
        let line = "  2026-06-07 13:16:45.00  test_dir";
        let idx = find_date_index(line);
        assert!(idx.is_some());
    }

    #[test]
    fn test_find_date_index_invalid_no_date() {
        assert_eq!(find_date_index(""), None);
        assert_eq!(find_date_index("no digits here"), None);
        assert_eq!(find_date_index("abc-xyz-123"), None);
        assert_eq!(find_date_index("2024/01/15"), None); // wrong separator
    }

    #[test]
    fn test_find_date_index_too_short() {
        assert_eq!(find_date_index("short"), None);
        assert_eq!(find_date_index("123456789"), None); // 9 chars, need at least 10
    }

    #[test]
    fn test_find_date_index_date_embedded_in_text() {
        // Date in the middle
        let line = "prefix_2024-12-25_suffix";
        let idx = find_date_index(line);
        assert!(idx.is_some());
        assert_eq!(&line[idx.unwrap()..idx.unwrap() + 10], "2024-12-25");
    }

    #[test]
    fn test_find_date_index_multiple_dates_returns_first() {
        let line = "2023-01-01 some text 2024-02-02 more";
        let idx = find_date_index(line).unwrap();
        // Should return the first occurrence
        assert_eq!(&line[idx..idx + 10], "2023-01-01");
    }

    #[test]
    fn test_find_date_index_partial_digits() {
        // Must be exactly YYYY-MM-DD with dashes
        assert_eq!(find_date_index("2024-01-1"), None); // missing final digit
        assert_eq!(find_date_index("2024-1-15"), None); // missing leading zero
        assert_eq!(find_date_index("abcd-ef-gh"), None); // letters
    }

    #[test]
    fn test_find_date_index_edge_positions() {
        // Date at the very end of string (exactly at len-10)
        let line = "something 1999-12-31";
        assert_eq!(find_date_index(line), Some(10));
        assert_eq!(&line[10..20], "1999-12-31");

        // Date occupying the entire string
        let line = "2000-01-01";
        assert_eq!(find_date_index(line), Some(0));
    }

    // ─── parse_size_bytes ────────────────────────────────────────────────────────

    #[test]
    fn test_parse_size_bytes_bytes() {
        assert_eq!(parse_size_bytes("22 B"), 22);
        assert_eq!(parse_size_bytes("0 B"), 0);
        assert_eq!(parse_size_bytes("1024 B"), 1024);
    }

    #[test]
    fn test_parse_size_bytes_kib() {
        assert_eq!(parse_size_bytes("1 KiB"), 1024);
        assert_eq!(parse_size_bytes("1.5 KiB"), 1536);
        assert_eq!(parse_size_bytes("2 KiB"), 2048);
    }

    #[test]
    fn test_parse_size_bytes_mib() {
        assert_eq!(parse_size_bytes("1 MiB"), 1024 * 1024);
        assert_eq!(parse_size_bytes("2.5 MiB"), (2.5 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn test_parse_size_bytes_gib() {
        assert_eq!(parse_size_bytes("1 GiB"), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_bytes_empty_or_invalid() {
        assert_eq!(parse_size_bytes(""), 0);
        assert_eq!(parse_size_bytes("xyz"), 0);
        assert_eq!(parse_size_bytes("abc B"), 0); // non-numeric prefix
    }

    #[test]
    fn test_parse_size_bytes_no_unit() {
        // When there's only a number, return it as-is
        assert_eq!(parse_size_bytes("42"), 42);
    }

    #[test]
    fn test_parse_size_bytes_uppercase_units() {
        // Should handle both KB/KIB, MB/MIB, GB/GIB
        assert_eq!(parse_size_bytes("1 KB"), 1024);
        assert_eq!(parse_size_bytes("1 MB"), 1024 * 1024);
        assert_eq!(parse_size_bytes("1 GB"), 1024 * 1024 * 1024);
    }
}
