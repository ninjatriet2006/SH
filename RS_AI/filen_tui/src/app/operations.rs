use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct FileItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub mod_time: String,
}

pub struct Operations;

const FILEN_BIN: &str = "/home/bimatkeo/.filen-cli/bin/filen";

impl Operations {
    // Lấy đối tượng Command cấu hình sẵn cờ --data-dir dựa trên tài khoản active
    pub fn get_command(active_account: &Option<String>) -> Command {
        let mut cmd = Command::new(FILEN_BIN);
        if let Some(email) = active_account {
            if let Some(home) = dirs::home_dir() {
                let data_path = home.join(".config/filen-cli/accounts").join(email);
                cmd.arg("--data-dir").arg(data_path);
            }
        }
        cmd
    }

    // Lấy thông tin tài khoản đang hoạt động (whoami)
    #[allow(dead_code)]
    pub async fn whoami(active_account: &Option<String>) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("whoami");
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
            if let Some(date_idx) = find_date_index(line) {
                if date_idx + 22 <= line.len() {
                    let size_part = line[..date_idx].trim();
                    let mod_time = line[date_idx..date_idx + 19].to_string(); // Bỏ đi phần miligiây .xx cho gọn
                    let name = line[date_idx + 22..].trim().to_string();

                    let is_dir = size_part.is_empty();
                    let size = if is_dir {
                        0
                    } else {
                        parse_size_bytes(size_part)
                    };

                    items.push(FileItem {
                        name,
                        is_dir,
                        size,
                        mod_time,
                    });
                }
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
        if let Some(parent) = dir.parent() {
            if parent != dir {
                items.push(FileItem {
                    name: "..".to_string(),
                    is_dir: true,
                    size: 0,
                    mod_time: String::new(),
                });
            }
        }
        let read_dir = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
        for entry in read_dir {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().to_string();
                let metadata = entry.metadata().map_err(|e| e.to_string())?;
                let is_dir = metadata.is_dir();
                let size = if is_dir { 0 } else { metadata.len() };
                let mod_time = metadata.modified()
                    .ok()
                    .and_then(|t| {
                        let datetime: chrono::DateTime<chrono::Local> = t.into();
                        Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
                    })
                    .unwrap_or_else(|| "N/A".to_string());

                items.push(FileItem {
                    name,
                    is_dir,
                    size,
                    mod_time,
                });
            }
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
            if let Some(date_idx) = find_date_index(line) {
                if date_idx + 22 <= line.len() {
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
        let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
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
        let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            // Link thường hiển thị ở cuối dạng "Link: https://..." hoặc chỉ URL
            let mut link = String::new();
            for line in text.lines() {
                if line.contains("https://") {
                    if let Some(pos) = line.find("https://") {
                        link = line[pos..].trim().to_string();
                    }
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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

    // Đăng nhập tài khoản mới (thực chất chạy whoami để kích hoạt prompt và gửi input)
    pub async fn login_new(email: &str, password: &str) -> Result<(), String> {
        if let Some(home) = dirs::home_dir() {
            let data_path = home.join(".config/filen-cli/accounts").join(email);
            let mut cmd = Command::new(FILEN_BIN);
            cmd.arg("--data-dir").arg(data_path).arg("whoami");
            cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = cmd.spawn().map_err(|e| e.to_string())?;
            
            if let Some(mut stdin) = child.stdin.take() {
                // CLI hỏi Email trước:
                let _ = stdin.write_all(format!("{}\n", email).as_bytes()).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                // Tiếp theo hỏi Password:
                let _ = stdin.write_all(format!("{}\n", password).as_bytes()).await;
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                // CLI hỏi có lưu credentials hay không:
                let _ = stdin.write_all(b"Y\n").await;
            }
            
            let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
            if output.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
            }
        } else {
            Err("Không tìm thấy thư mục Home".to_string())
        }
    }

    // Đăng xuất (logout)
    pub async fn logout(active_account: &Option<String>) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("logout");
        let output = cmd.output().await.map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Xuất auth config
    pub async fn export_auth_config(active_account: &Option<String>) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("export-auth-config");
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
        let output = cmd.output().await.map_err(|e| e.to_string())?;
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
    for i in 0..=(line.len() - 10) {
        if bytes[i].is_ascii_digit() &&
           bytes[i+1].is_ascii_digit() &&
           bytes[i+2].is_ascii_digit() &&
           bytes[i+3].is_ascii_digit() &&
           bytes[i+4] == b'-' &&
           bytes[i+5].is_ascii_digit() &&
           bytes[i+6].is_ascii_digit() &&
           bytes[i+7] == b'-' &&
           bytes[i+8].is_ascii_digit() &&
           bytes[i+9].is_ascii_digit() {
            return Some(i);
        }
    }
    None
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
