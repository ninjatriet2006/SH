//! [INTEGRITY NOTES]
//! Mục đích: Tương tác với hệ thống file trên Cloud (Filen).
//! Trách nhiệm: Chứa các hàm bao bọc gọi CLI `filen` như ls, mkdir, rm, mv, cp, upload, download, cat...
//! Tương tác: Giao tiếp với CLI `filen`, sử dụng models để trả về dữ liệu chuẩn.
//!
//! [KHỐI CLOUD_FS]

use std::process::Stdio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use crate::models::*;

pub async fn list_remote_terminal(active_account: &Option<String>, path: &str) -> Result<Vec<FileItem>, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("ls").arg(path).arg("--long");
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
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
            ..Default::default()
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
                ..Default::default()
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

// Duyệt thư mục Cloud dạng Streaming
pub async fn list_remote_stream_terminal<F>(
    active_account: &Option<String>,
    path: &str,
    mut on_chunk: F,
) -> Result<(), String>
where
    F: FnMut(Vec<FileItem>) + Send + Sync,
{
    use tokio::io::AsyncBufReadExt;
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("ls").arg(path).arg("--long");
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Err(format!("Lỗi chạy tiến trình: {}", e)),
    };

    let stdout = child.stdout.take().unwrap();
    let mut reader = tokio::io::BufReader::new(stdout).lines();

    let mut buffer = Vec::new();

    if path != "/" && !path.is_empty() {
        let parent_dir = FileItem {
            name: "..".to_string(),
            is_dir: true,
            size: 0,
            mod_time: String::new(),
            ..Default::default()
        };
        buffer.push(parent_dir);
    }

    while let Ok(Some(line)) = reader.next_line().await {
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

                buffer.push(FileItem {
                    name,
                    is_dir,
                    size,
                    mod_time,
                    ..Default::default()
                });

                if buffer.len() >= 50 {
                    on_chunk(std::mem::take(&mut buffer));
                }
            }
        }
    }

    if !buffer.is_empty() {
        on_chunk(buffer);
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Tiến trình filen bị lỗi khi chạy stream".to_string());
    }

    Ok(())
}

// Tạo ảnh thu nhỏ (thumbnail) dạng Base64

pub async fn mkdir_terminal(active_account: &Option<String>, path: &str) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("mkdir").arg(path);
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Xóa file/thư mục (rm).
// CLI hỏi xác nhận 1 lần (vào Thùng rác, "Are you sure you want to delete ...? [y/N] ")
// hoặc 2 lần (--no-trash, xóa vĩnh viễn, thêm prompt "Are you sure? [y/N] ").
// Không được pre-pipe toàn bộ "y\n" vì CLI chỉ hỏi prompt thứ 2 sau khi prompt
// thứ 1 được trả lời → dùng chạy interactive đọc output và gửi "y" theo prompt.

pub async fn rm_terminal(active_account: &Option<String>, path: &str, no_trash: bool) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(rm_args(path, no_trash));
    let expected_prompts = if no_trash { 2 } else { 1 };
    let output = crate::cloud_fs::run_cmd_confirm(cmd, b"", expected_prompts, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.is_empty() {
            Err("Không xoá được mục (CLI báo lỗi).".to_string())
        } else {
            Err(err)
        }
    }
}

// Đổi tên hoặc di chuyển (mv)

pub async fn mv_terminal(active_account: &Option<String>, from: &str, to: &str) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("mv").arg(from).arg(to);
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Sao chép (cp)

pub async fn cp_terminal(active_account: &Option<String>, from: &str, to: &str) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("cp").arg(from).arg(to);
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Tải lên tệp (upload)

pub async fn upload_terminal(active_account: &Option<String>, local: &str, remote: &str) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("upload").arg(local).arg(remote);
    let output = cmd.output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Tải xuống tệp (download)

pub async fn download_terminal(active_account: &Option<String>, remote: &str, local: &str) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("download").arg(remote).arg(local);
    let output = cmd.output().await.map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Đọc nội dung file text (cat)

pub async fn cat_terminal(active_account: &Option<String>, path: &str) -> Result<String, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("cat").arg(path);
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Thêm vào mục Yêu thích (favorite)

pub async fn favorite_terminal(active_account: &Option<String>, path: &str) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("favorite").arg(path);
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Bỏ Yêu thích (unfavorite)

pub async fn unfavorite_terminal(active_account: &Option<String>, path: &str) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("unfavorite").arg(path);
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Danh sách Yêu thích

pub async fn list_favorites_terminal(active_account: &Option<String>) -> Result<Vec<FileItem>, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("favorites");
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
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
            ..Default::default()
        });
    }
    Ok(items)
}

// Danh sách thùng rác (trash)

pub async fn list_trash_terminal(active_account: &Option<String>) -> Result<Vec<FileItem>, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("trash");
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
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
                ..Default::default()
            });
        }
    }
    Ok(items)
}

// Khôi phục thùng rác (trash restore)

pub async fn trash_restore_terminal(active_account: &Option<String>, idx_1based: usize) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
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

// Xóa vĩnh viễn mục trong thùng rác (trash delete).
// CLI hỏi "Select an item to permanently delete (1-N): " (trả lời bằng index),
// sau đó hỏi xác nhận "permanently delete <name>? [y/N] ". Không được pre-pipe
// toàn bộ câu trả lời vì prompt xác nhận chỉ xuất hiện sau khi đã gửi index.

pub async fn trash_delete_terminal(active_account: &Option<String>, idx_1based: usize) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("trash").arg("delete");
    let input = format!("{}\n", idx_1based);
    let output = crate::cloud_fs::run_cmd_confirm(cmd, input.as_bytes(), 1, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Dọn sạch thùng rác (trash empty).
// CLI hỏi xác nhận 2 lần: "permanently delete all N trash items? [y/N] "
// rồi "Are you sure? [y/N] " (prompt thứ 2 chỉ hiện sau khi trả lời prompt 1).

pub async fn trash_empty_terminal(active_account: &Option<String>) -> Result<(), String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("trash").arg("empty");
    let output = crate::cloud_fs::run_cmd_confirm(cmd, b"", 2, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Tạo link công khai (links <path>)

pub async fn create_link_terminal(active_account: &Option<String>, path: &str) -> Result<String, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
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

pub async fn list_links_terminal(active_account: &Option<String>) -> Result<Vec<(String, String)>, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.arg("links");
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
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

pub async fn head_terminal(
    active_account: &Option<String>,
    file: &str,
    lines: Option<usize>,
) -> Result<String, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(head_args(file, lines));
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Đọc n dòng cuối cùng của file cloud (tail <file> [-n <lines>], mặc định 10)

pub async fn tail_terminal(
    active_account: &Option<String>,
    file: &str,
    lines: Option<usize>,
) -> Result<String, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(tail_args(file, lines));
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Thông tin chi tiết (stat <item>): thử --json trước, fallback text parse

pub async fn stat_terminal(active_account: &Option<String>, item: &str) -> Result<String, String> {
    // Lần 1: cờ --json (option toàn cục của nhóm fs)
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(stat_json_args(item));
    let json_output = match crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await {
        Ok(output) => output,
        Err(e) => return Err(e),
    };
    if json_output.status.success() {
        let trimmed = String::from_utf8_lossy(&json_output.stdout).trim().to_string();
        if !trimmed.is_empty() && trimmed.starts_with('{') {
            if let Ok(formatted) = parse_stat_json(&trimmed) {
                return Ok(formatted);
            }
            // CLI có thể in log trước đối tượng JSON → tìm ký tự '{' đầu tiên
            if let Some(start) = trimmed.find('{')
                && let Ok(formatted) = parse_stat_json(&trimmed[start..])
            {
                return Ok(formatted);
            }
        }
        return Ok(trimmed);
    }

    // Lần 2 (fallback): text parse khi CLI không hỗ trợ --json ở vị trí này
    let mut cmd2 = crate::cloud_fs::get_command(active_account);
    cmd2.args(stat_args(item));
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd2, 30).await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Ghi text vào file cloud (write <file> <content...>)

pub async fn write_file_terminal(
    active_account: &Option<String>,
    file: &str,
    content: &str,
) -> Result<(), String> {
    // Content nhiều dòng không truyền trực tiếp qua argv `write` được: CLI tokenize
    // theo \n thành nhiều lệnh → "Unknown command". → ghi file tạm rồi upload.
    if write_file_uses_temp_upload(content) {
        let tmp = write_temp_path();
        std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
        let result = crate::cloud_fs::upload_terminal(active_account, &tmp.to_string_lossy(), file).await;
        let _ = std::fs::remove_file(&tmp);
        return result;
    }
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(write_args(file, content));
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Danh sách file/thư mục dùng gần đây (recents, format giống ls --long)

pub async fn recents_terminal(active_account: &Option<String>) -> Result<Vec<FileItem>, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(recents_args());
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 30).await?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ls_long(&text))
}

// Mở Web Drive (view [path]) — trả về URL để GUI log/hiển thị, KHÔNG spawn trình duyệt.

pub async fn view(active_account: &Option<String>, path: Option<&str>) -> Result<String, String> {
    let _ = active_account; // view không cần gọi CLI (CLI sẽ tự mở trình duyệt)
    Ok(web_drive_url(path))
}

// Xuất tất cả Notes (export-notes [path])

pub async fn export_notes_terminal(
    active_account: &Option<String>,
    path: Option<&str>,
) -> Result<String, String> {
    let mut cmd = crate::cloud_fs::get_command(active_account);
    cmd.args(export_notes_args(path));
    let output = crate::cloud_fs::run_cmd_with_timeout(cmd, 120).await?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

// Đọc danh sách cặp đồng bộ từ {dataDir}/syncPairs.json

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    crate::sys::copy_to_clipboard(text)
}

// Lấy đối tượng Command cấu hình sẵn cờ --data-dir dựa trên tài khoản active

pub fn get_command(_active_account: &Option<String>) -> Command {
    let mut cmd = Command::new(resolve_filen_bin());
    if let Some(data_path) = get_default_data_dir() {
        cmd.arg("--data-dir").arg(data_path);
    }
    cmd.kill_on_drop(true);
    cmd
}

// Lấy đối tượng std::process::Command cấu hình sẵn cờ --data-dir.
// Dùng cho các tiến trình chạy lâu (server WebDAV/S3/Mount): child std không
// bị kill khi drop nên GUI có thể giữ handle trong app state để dừng sau.

pub fn get_std_command(_active_account: &Option<String>) -> std::process::Command {
    let mut cmd = std::process::Command::new(resolve_filen_bin());
    if let Some(data_path) = get_default_data_dir() {
        cmd.arg("--data-dir").arg(data_path);
    }
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

// Chạy lệnh Command interactive theo danh sách quy tắc phản hồi: đọc liên tục
// stdout+stderr, mỗi lần phát hiện prompt thì gửi response tương ứng. `initial_input`
// được ghi vào stdin ngay sau khi spawn (dùng cho prompt nhập liệu xuất hiện đầu,
// ví dụ index trong trash delete). Dùng cho rm/rm --no-trash, trash delete/empty,
// export-auth-config, export-api-key vì CLI chỉ hiện prompt tiếp theo SAU khi prompt
// trước được trả lời nên không thể pre-pipe toàn bộ câu trả lời.
pub async fn run_cmd_interactive(
    cmd: Command,
    initial_input: &[u8],
    rules: &[PromptRule],
    timeout_secs: u64,
) -> Result<std::process::Output, String> {
    let mut cmd = crate::sys::get_interactive_tokio_command(cmd);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().ok_or_else(|| "stdin bị đóng".to_string())?;
    if !initial_input.is_empty() {
        let _ = stdin.write_all(initial_input).await;
        let _ = stdin.flush().await;
    }
    let mut stdout = child.stdout.take().ok_or_else(|| "stdout bị đóng".to_string())?;
    let mut stderr = child.stderr.take().ok_or_else(|| "stderr bị đóng".to_string())?;

    let deadline =
        tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    let mut responder = PromptResponder::new(rules);
    let mut out_bytes: Vec<u8> = Vec::new();
    let mut err_bytes: Vec<u8> = Vec::new();
    let mut stdout_open = true;
    let mut stderr_open = true;
    let mut stdout_buf = [0u8; 1024];
    let mut stderr_buf = [0u8; 1024];

    while stdout_open || stderr_open {
        tokio::select! {
            res = stdout.read(&mut stdout_buf), if stdout_open => {
                match res {
                    Ok(0) => stdout_open = false,
                    Ok(n) => {
                        out_bytes.extend_from_slice(&stdout_buf[..n]);
                        let text = String::from_utf8_lossy(&stdout_buf[..n]);
                        for response in responder.feed(&text) {
                            let _ = stdin.write_all(response).await;
                            let _ = stdin.flush().await;
                        }
                    }
                    Err(_) => stdout_open = false,
                }
            }
            res = stderr.read(&mut stderr_buf), if stderr_open => {
                match res {
                    Ok(0) => stderr_open = false,
                    Ok(n) => {
                        err_bytes.extend_from_slice(&stderr_buf[..n]);
                        let text = String::from_utf8_lossy(&stderr_buf[..n]);
                        for response in responder.feed(&text) {
                            let _ = stdin.write_all(response).await;
                            let _ = stdin.flush().await;
                        }
                    }
                    Err(_) => stderr_open = false,
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(format!(
                    "Yêu cầu quá hạn (timeout {timeout_secs}s). CLI có thể đang chờ xác nhận hoặc gặp lỗi mạng."
                ));
            }
        }
    }

    let status = child.wait().await.map_err(|e| e.to_string())?;
    Ok(std::process::Output {
        status,
        stdout: out_bytes,
        stderr: err_bytes,
    })
}

// Chạy lệnh có 1..n prompt xác nhận [y/N]: mỗi lần phát hiện prompt thì gửi "y\n".
pub async fn run_cmd_confirm(
    cmd: Command,
    initial_input: &[u8],
    expected_prompts: usize,
    timeout_secs: u64,
) -> Result<std::process::Output, String> {
    let rules = [confirm_prompt_rule(expected_prompts)];
    crate::cloud_fs::run_cmd_interactive(cmd, initial_input, &rules, timeout_secs).await
}

