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

// Xác định thư mục dữ liệu mặc định của filen-cli (.filen-cli hoặc .config/filen-cli)
// Copy từ TUI/filen_tui/src/app/mod.rs để filen_gui độc lập hoàn toàn với filen_tui.
pub fn get_default_data_dir() -> Option<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        let dot_filen = home.join(".filen-cli");
        if dot_filen.is_dir() {
            Some(dot_filen)
        } else {
            Some(home.join(".config/filen-cli"))
        }
    } else {
        None
    }
}

// ─── Lưu trữ danh sách tài khoản (đăng nhập nhanh) ───────────────────────────
// Cấu trúc copy từ TUI/filen_tui (StoredAccount/AccountConfig), nhưng lưu dạng
// JSON (accounts.json) trong data dir để filen_gui không cần thêm serde_yaml.

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredAccount {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AccountConfig {
    pub accounts: Vec<StoredAccount>,
}

/// Đường dẫn file chứa danh sách tài khoản đã lưu (accounts.json trong data dir).
pub fn accounts_file_path() -> Option<PathBuf> {
    get_default_data_dir().map(|dir| dir.join("accounts.json"))
}

/// Nạp danh sách tài khoản đã lưu để đăng nhập nhanh.
pub fn load_stored_accounts() -> Vec<StoredAccount> {
    if let Some(path) = accounts_file_path()
        && let Ok(content) = std::fs::read_to_string(&path)
        && let Ok(config) = serde_json::from_str::<AccountConfig>(&content)
    {
        config.accounts
    } else {
        Vec::new()
    }
}

/// Lưu danh sách tài khoản (file JSON, permission 0600 trên Unix).
pub fn save_stored_accounts(accounts: &[StoredAccount]) -> Result<(), String> {
    let Some(path) = accounts_file_path() else {
        return Err("Không tìm thấy thư mục Home".to_string());
    };
    let parent = path
        .parent()
        .ok_or_else(|| "Đường dẫn thư mục dữ liệu không hợp lệ".to_string())?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let config = AccountConfig {
        accounts: accounts.to_vec(),
    };
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// Sự kiện dùng cho luồng login (giữ tương thích tối thiểu với bản TUI;
// filen_gui hiện chỉ dùng list_local nên các hàm còn lại để nguyên cho khớp bản gốc).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum AppEvent {
    LoginLog(String),
}

pub(crate) fn resolve_filen_bin() -> PathBuf {
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

            // 2.5. Quét bin của các trình quản lý Node (nvm/volta/bun/npm-global,
            // ~/.local/bin) — GUI chạy từ desktop/file-manager thường không có
            // PATH của nvm nên `which` fail; ưu tiên bản có mtime mới nhất.
            if let Some(home) = dirs::home_dir() {
                let mut node_bins = scan_node_bins(&home);
                node_bins.sort_by_key(|p| {
                    std::fs::metadata(p)
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                });
                if let Some(best) = node_bins.last() {
                    return best.clone();
                }
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

/// Quét các vị trí cài đặt phổ biến để tìm binary `filen`:
/// - `<home>/.nvm/versions/node/<ver>/bin/filen` (mỗi thư mục con là 1 version)
/// - `<home>/.volta/bin/filen`, `<home>/.bun/bin/filen`,
///   `<home>/.npm-global/bin/filen`, `<home>/.local/bin/filen`
///
/// CHỈ chấp nhận file có đúng tên `filen` — không nhặt nhầm binary khác
/// (ví dụ `agy`/`codex` cùng nằm trong `~/.local/bin`).
fn scan_node_bins(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for base in [
        home.join(".nvm/versions/node"),
        home.join(".volta/bin"),
        home.join(".bun/bin"),
        home.join(".local/bin"),
        home.join(".npm-global/bin"),
    ] {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let bin = if p.is_dir() {
                // Thư mục version của nvm: <ver>/bin/filen
                p.join("bin").join("filen")
            } else if name == "filen" {
                p
            } else {
                // File khác tên "filen" (agy, codex, ...) → bỏ qua.
                continue;
            };
            if bin.is_file() {
                out.push(bin);
            }
        }
    }
    out
}

#[allow(dead_code)] // Các ops Cloud sẽ được gọi từ GUI ở phase 3.
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
    async fn run_cmd_interactive(
        cmd: Command,
        initial_input: &[u8],
        rules: &[PromptRule],
        timeout_secs: u64,
    ) -> Result<std::process::Output, String> {
        // Unix: chạy qua stdbuf để bỏ đệm output (prompt phải xuất hiện ngay khi
        // được ghi; nếu bị đệm thì sẽ không phát hiện được prompt và phải timeout).
        #[cfg(unix)]
        let mut cmd = {
            if which::which("stdbuf").is_ok() {
                let program = cmd.as_std().get_program().to_os_string();
                let args: Vec<std::ffi::OsString> =
                    cmd.as_std().get_args().map(ToOwned::to_owned).collect();
                let mut c = Command::new("stdbuf");
                c.arg("-o0").arg("-e0").arg(&program).args(&args);
                c
            } else {
                cmd
            }
        };

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
    async fn run_cmd_confirm(
        cmd: Command,
        initial_input: &[u8],
        expected_prompts: usize,
        timeout_secs: u64,
    ) -> Result<std::process::Output, String> {
        let rules = [confirm_prompt_rule(expected_prompts)];
        Self::run_cmd_interactive(cmd, initial_input, &rules, timeout_secs).await
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

    // Xóa file/thư mục (rm).
    // CLI hỏi xác nhận 1 lần (vào Thùng rác, "Are you sure you want to delete ...? [y/N] ")
    // hoặc 2 lần (--no-trash, xóa vĩnh viễn, thêm prompt "Are you sure? [y/N] ").
    // Không được pre-pipe toàn bộ "y\n" vì CLI chỉ hỏi prompt thứ 2 sau khi prompt
    // thứ 1 được trả lời → dùng chạy interactive đọc output và gửi "y" theo prompt.
    pub async fn rm(active_account: &Option<String>, path: &str, no_trash: bool) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.args(rm_args(path, no_trash));
        let expected_prompts = if no_trash { 2 } else { 1 };
        let output = Self::run_cmd_confirm(cmd, b"", expected_prompts, 30).await?;
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

    // Xóa vĩnh viễn mục trong thùng rác (trash delete).
    // CLI hỏi "Select an item to permanently delete (1-N): " (trả lời bằng index),
    // sau đó hỏi xác nhận "permanently delete <name>? [y/N] ". Không được pre-pipe
    // toàn bộ câu trả lời vì prompt xác nhận chỉ xuất hiện sau khi đã gửi index.
    #[allow(dead_code)]
    pub async fn trash_delete(active_account: &Option<String>, idx_1based: usize) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("trash").arg("delete");
        let input = format!("{}\n", idx_1based);
        let output = Self::run_cmd_confirm(cmd, input.as_bytes(), 1, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Dọn sạch thùng rác (trash empty).
    // CLI hỏi xác nhận 2 lần: "permanently delete all N trash items? [y/N] "
    // rồi "Are you sure? [y/N] " (prompt thứ 2 chỉ hiện sau khi trả lời prompt 1).
    pub async fn trash_empty(active_account: &Option<String>) -> Result<(), String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("trash").arg("empty");
        let output = Self::run_cmd_confirm(cmd, b"", 2, 30).await?;
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
    pub async fn login_new(
        email: &str,
        password: &str,
        twofa_code: Option<&str>,
        keep_logged: &str,
        tx: Option<tokio::sync::mpsc::UnboundedSender<AppEvent>>,
    ) -> Result<(), String> {
        let log = |msg: String| {
            if let Some(ref tx) = tx {
                let _ = tx.send(AppEvent::LoginLog(msg));
            }
        };

        if let Some(data_path) = get_default_data_dir() {
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

    // Xuất auth config.
    // CLI hỏi lần lượt: (1) xác nhận overwrite ".filen-cli-auth-config" [y/N] nếu
    // file tồn tại, (2) yêu cầu gõ đúng cụm "I am aware of the risks",
    // (3) "Choose an export location: [1] data directory, [2] here:". Các prompt này
    // xuất hiện tuần tự nên phải chạy interactive (không pre-pipe được).
    pub async fn export_auth_config(active_account: &Option<String>) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("export-auth-config");
        let rules = [
            confirm_prompt_rule(1),
            PromptRule {
                matcher: looks_like_risks_prompt,
                response: b"I am aware of the risks\n",
                max: 1,
            },
            PromptRule {
                matcher: looks_like_export_location_prompt,
                response: b"1\n",
                max: 1,
            },
        ];
        let output = Self::run_cmd_interactive(cmd, b"", &rules, 30).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Xuất API Key.
    // CLI hỏi xác nhận "Proceed? (y/N) " trước khi in API key → chạy interactive
    // để gửi "y\n" đúng lúc prompt xuất hiện.
    pub async fn export_api_key(active_account: &Option<String>) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.arg("export-api-key");
        let output = Self::run_cmd_confirm(cmd, b"", 1, 30).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // ─── Phase 6: bổ sung ops theo audit filen help (subtask 6.1→6.10) ─────────

    // Đọc n dòng đầu tiên của file cloud (head <file> [-n <lines>], mặc định 10)
    pub async fn head(
        active_account: &Option<String>,
        file: &str,
        lines: Option<usize>,
    ) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.args(head_args(file, lines));
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Đọc n dòng cuối cùng của file cloud (tail <file> [-n <lines>], mặc định 10)
    pub async fn tail(
        active_account: &Option<String>,
        file: &str,
        lines: Option<usize>,
    ) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.args(tail_args(file, lines));
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Thông tin chi tiết (stat <item>): thử --json trước, fallback text parse
    pub async fn stat(active_account: &Option<String>, item: &str) -> Result<String, String> {
        // Lần 1: cờ --json (option toàn cục của nhóm fs)
        let mut cmd = Self::get_command(active_account);
        cmd.args(stat_json_args(item));
        let json_output = match Self::run_cmd_with_timeout(cmd, 30).await {
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
        let mut cmd2 = Self::get_command(active_account);
        cmd2.args(stat_args(item));
        let output = Self::run_cmd_with_timeout(cmd2, 30).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Ghi text vào file cloud (write <file> <content...>)
    pub async fn write_file(
        active_account: &Option<String>,
        file: &str,
        content: &str,
    ) -> Result<(), String> {
        // Content nhiều dòng không truyền trực tiếp qua argv `write` được: CLI tokenize
        // theo \n thành nhiều lệnh → "Unknown command". → ghi file tạm rồi upload.
        if write_file_uses_temp_upload(content) {
            let tmp = write_temp_path();
            std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
            let result = Self::upload(active_account, &tmp.to_string_lossy(), file).await;
            let _ = std::fs::remove_file(&tmp);
            return result;
        }
        let mut cmd = Self::get_command(active_account);
        cmd.args(write_args(file, content));
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Danh sách file/thư mục dùng gần đây (recents, format giống ls --long)
    pub async fn recents(active_account: &Option<String>) -> Result<Vec<FileItem>, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.args(recents_args());
        let output = Self::run_cmd_with_timeout(cmd, 30).await?;
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
    pub async fn export_notes(
        active_account: &Option<String>,
        path: Option<&str>,
    ) -> Result<String, String> {
        let mut cmd = Self::get_command(active_account);
        cmd.args(export_notes_args(path));
        let output = Self::run_cmd_with_timeout(cmd, 120).await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Đọc danh sách cặp đồng bộ từ {dataDir}/syncPairs.json
    pub fn sync_pairs() -> Result<Vec<SyncPair>, String> {
        let path = sync_pairs_path().ok_or_else(|| "Không tìm thấy thư mục dữ liệu".to_string())?;
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Không đọc được syncPairs.json ({}): {e}", path.display()))?;
        parse_sync_pairs_json(&content)
    }

    // Chạy sync (sync <locations...> [--continuous])
    pub async fn sync(
        active_account: &Option<String>,
        locations: &[String],
        continuous: bool,
    ) -> Result<(), String> {
        if locations.is_empty() {
            return Err("Không có cặp đồng bộ nào để chạy.".to_string());
        }
        let mut cmd = Self::get_command(active_account);
        cmd.args(sync_args(locations, continuous));
        let output = Self::run_cmd_with_timeout(cmd, 60).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    // Chạy sync 1 lần cho cặp local:remote (dạng `/local:/cloud`)
    pub async fn sync_once(
        active_account: &Option<String>,
        local: &str,
        remote: &str,
    ) -> Result<(), String> {
        Self::sync(active_account, &[format!("{local}:{remote}")], false).await
    }

    // Chạy sync 1 lần cho một pair đã đọc từ syncPairs.json
    pub async fn sync_pair_once(
        active_account: &Option<String>,
        pair: &SyncPair,
    ) -> Result<(), String> {
        Self::sync(active_account, &[sync_pair_arg(pair)], false).await
    }
}

// ─── Phase 6: cấu trúc state cho server child (webdav/webdav-proxy/s3/mount) ──
// Chỉ tham khảo cấu trúc WebDavServerState/S3ServerState ở TUI/filen_tui/src/app/mod.rs;
// code dưới đây được viết mới hoàn toàn cho filen_gui.

/// Định dạng một cặp đồng bộ trong syncPairs.json
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SyncPair {
    pub local: String,
    pub remote: String,
    #[serde(rename = "syncMode", default)]
    pub sync_mode: String,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(rename = "disableLocalTrash", default)]
    pub disable_local_trash: Option<bool>,
    #[serde(default)]
    pub ignore: Option<Vec<String>>,
    #[serde(rename = "excludeDotFiles", default)]
    pub exclude_dot_files: Option<bool>,
}

/// Trạng thái server WebDAV (gồm cả chế độ proxy: user/pass bỏ trống).
#[derive(Debug)]
#[allow(dead_code)]
pub struct WebDavServerState {
    pub running: bool,
    pub user: String,
    pub pass: String,
    pub port: String,
    pub https: bool,
    pub child: Option<tokio::process::Child>,
    pub logs: Vec<String>,
}

/// Trạng thái server S3.
#[derive(Debug)]
#[allow(dead_code)]
pub struct S3ServerState {
    pub running: bool,
    pub access_key: String,
    pub secret_key: String,
    pub port: String,
    pub https: bool,
    pub child: Option<tokio::process::Child>,
    pub logs: Vec<String>,
}

/// Trạng thái mount FUSE (network drive).
#[derive(Debug)]
#[allow(dead_code)]
pub struct MountState {
    pub running: bool,
    pub child: Option<tokio::process::Child>,
    pub mount_point: String,
    pub note: String,
}

impl Default for WebDavServerState {
    fn default() -> Self {
        Self {
            running: false,
            user: "admin".to_string(),
            pass: "admin123".to_string(),
            port: "8080".to_string(),
            https: false,
            child: None,
            logs: Vec::new(),
        }
    }
}

impl Default for S3ServerState {
    fn default() -> Self {
        Self {
            running: false,
            access_key: "s3key".to_string(),
            secret_key: "s3secret".to_string(),
            port: "9000".to_string(),
            https: false,
            child: None,
            logs: Vec::new(),
        }
    }
}

impl Default for MountState {
    fn default() -> Self {
        Self {
            running: false,
            child: None,
            mount_point: default_mount_point(),
            note: mount_fuse_note(),
        }
    }
}

#[allow(dead_code)] // Server state được dùng ở phase 7 (GUI), hiện chỉ có test dùng.
impl WebDavServerState {
    pub fn new() -> Self {
        Self::default()
    }

    // Chạy server WebDAV (filen webdav ...)
    pub async fn start(&mut self, active_account: &Option<String>) -> Result<(), String> {
        if self.running {
            return Err("Server WebDAV đang chạy rồi.".to_string());
        }
        let mut cmd = Operations::get_command(active_account);
        cmd.args(webdav_args(&self.user, &self.pass, &self.port, self.https));
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = cmd
            .spawn()
            .map_err(|e| format!("Không khởi động được server WebDAV: {e}"))?;
        self.child = Some(child);
        self.running = true;
        self.logs
            .push(format!("Đã khởi chạy WebDAV trên cổng {}.", self.port));
        Ok(())
    }

    // Chạy server WebDAV chế độ proxy (filen webdav-proxy ...)
    pub async fn start_proxy(&mut self, active_account: &Option<String>) -> Result<(), String> {
        if self.running {
            return Err("Server WebDAV proxy đang chạy rồi.".to_string());
        }
        let mut cmd = Operations::get_command(active_account);
        cmd.args(webdav_proxy_args(&self.port, self.https));
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = cmd
            .spawn()
            .map_err(|e| format!("Không khởi động được server WebDAV proxy: {e}"))?;
        self.child = Some(child);
        self.running = true;
        self.logs
            .push(format!("Đã khởi chạy WebDAV proxy trên cổng {}.", self.port));
        Ok(())
    }

    // Dừng server WebDAV (an toàn khi chưa chạy)
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.running = false;
        self.logs.push("Máy chủ WebDAV đã dừng.".to_string());
        Ok(())
    }
}

#[allow(dead_code)] // Server state được dùng ở phase 7 (GUI), hiện chỉ có test dùng.
impl S3ServerState {
    pub fn new() -> Self {
        Self::default()
    }

    // Chạy server S3 (filen s3 ...)
    pub async fn start(&mut self, active_account: &Option<String>) -> Result<(), String> {
        if self.running {
            return Err("Server S3 đang chạy rồi.".to_string());
        }
        let mut cmd = Operations::get_command(active_account);
        cmd.args(s3_args(
            &self.access_key,
            &self.secret_key,
            &self.port,
            self.https,
        ));
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = cmd
            .spawn()
            .map_err(|e| format!("Không khởi động được server S3: {e}"))?;
        self.child = Some(child);
        self.running = true;
        self.logs
            .push(format!("Đã khởi chạy S3 trên cổng {}.", self.port));
        Ok(())
    }

    // Dừng server S3 (an toàn khi chưa chạy)
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.running = false;
        self.logs.push("Máy chủ S3 đã dừng.".to_string());
        Ok(())
    }
}

#[allow(dead_code)] // Mount state được dùng ở phase 7 (GUI), hiện chỉ có test dùng.
impl MountState {
    pub fn new() -> Self {
        Self::default()
    }

    // Mount network drive (filen mount [mount point]); trả về ghi chú FUSE.
    pub async fn start(
        &mut self,
        active_account: &Option<String>,
        mount_point: Option<&str>,
    ) -> Result<String, String> {
        if self.running {
            return Err("Mount đang chạy rồi.".to_string());
        }
        let mut cmd = Operations::get_command(active_account);
        cmd.args(mount_args(mount_point));
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let child = cmd
            .spawn()
            .map_err(|e| format!("Không mount được (hãy kiểm tra FUSE): {e}"))?;
        let point = mount_point
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .unwrap_or_else(default_mount_point);
        self.child = Some(child);
        self.running = true;
        self.mount_point = point.clone();
        self.note = mount_fuse_note();
        Ok(format!("{}\nMount point: {}", self.note, point))
    }

    // Unmount (dừng child)
    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.running = false;
        Ok(())
    }
}

// ─── Phase 6: helpers thuần (dễ unit test, không gọi CLI thật) ────────────────

// Trả về danh sách đối số sau tên binary (bỏ --data-dir) cho từng lệnh con.

fn head_args(file: &str, lines: Option<usize>) -> Vec<String> {
    let mut args = vec!["head".to_string(), file.to_string()];
    if let Some(n) = lines {
        args.push("-n".to_string());
        args.push(n.to_string());
    }
    args
}

fn tail_args(file: &str, lines: Option<usize>) -> Vec<String> {
    let mut args = vec!["tail".to_string(), file.to_string()];
    if let Some(n) = lines {
        args.push("-n".to_string());
        args.push(n.to_string());
    }
    args
}

fn stat_args(item: &str) -> Vec<String> {
    vec!["stat".to_string(), item.to_string()]
}

fn stat_json_args(item: &str) -> Vec<String> {
    vec!["--json".to_string(), "stat".to_string(), item.to_string()]
}

fn write_args(file: &str, content: &str) -> Vec<String> {
    vec!["write".to_string(), file.to_string(), content.to_string()]
}

fn rm_args(path: &str, no_trash: bool) -> Vec<String> {
    let mut args = vec!["rm".to_string(), path.to_string()];
    if no_trash {
        args.push("--no-trash".to_string());
    }
    args
}

// ─── Phát hiện/đếm prompt xác nhận (rm / rm --no-trash / trash / export) ──────

/// Kiểm tra xem văn bản tích luỹ có chứa dấu hiệu prompt xác nhận của filen-cli
/// hay không. Dựa trên output thật (`app.promptYesNo`): prompt luôn kết thúc bằng
/// "[y/N] " hoặc "[Y/n] " (không có `\n` vì CLI đang chờ input). Dùng mẫu có
/// dấu ngoặc để tránh false-positive với text chứa chữ "are you sure" lẻ.
fn looks_like_confirm_prompt(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("[y/n]") || lower.contains("(y/n)")
}

/// Prompt yêu cầu gõ đúng cụm "I am aware of the risks" (export-auth-config).
fn looks_like_risks_prompt(text: &str) -> bool {
    text.to_lowercase().contains("i am aware of the risks")
}

/// Prompt chọn vị trí export auth config: "Choose an export location: [1] ... [2] ..."
fn looks_like_export_location_prompt(text: &str) -> bool {
    text.to_lowercase().contains("choose an export location")
}

/// Một quy tắc phản hồi: khi output tích luỹ khớp `matcher` và chưa trả lời đủ
/// `max` lần thì gửi `response` vào stdin.
#[derive(Clone, Copy)]
struct PromptRule {
    matcher: fn(&str) -> bool,
    response: &'static [u8],
    max: usize,
}

/// Quy tắc trả lời "y\n" cho prompt xác nhận [y/N] (tối đa `max` lần).
fn confirm_prompt_rule(max: usize) -> PromptRule {
    PromptRule {
        matcher: looks_like_confirm_prompt,
        response: b"y\n",
        max,
    }
}

/// Bộ phản hồi các prompt trong luồng output (tách thuần để dễ unit test).
struct PromptResponder {
    rules: Vec<(PromptRule, usize)>,
    buf: String,
}

impl PromptResponder {
    fn new(rules: &[PromptRule]) -> Self {
        Self {
            rules: rules.iter().map(|r| (*r, 0)).collect(),
            buf: String::new(),
        }
    }

    /// Feed một đoạn output mới; trả về các response cần gửi stdin khi vừa phát
    /// hiện prompt (bộ đệm được xoá để không phản hồi lại prompt cũ).
    fn feed(&mut self, chunk: &str) -> Vec<&'static [u8]> {
        self.buf.push_str(chunk);
        if self.buf.len() > 16_384 {
            let cut = self.buf.len() - 2048;
            self.buf.drain(..cut);
        }
        let mut responses = Vec::new();
        for (rule, count) in self.rules.iter_mut() {
            if *count < rule.max && (rule.matcher)(&self.buf) {
                *count += 1;
                self.buf.clear();
                responses.push(rule.response);
            }
        }
        responses
    }
}

/// Content nhiều dòng không truyền trực tiếp qua argv của `write` được vì CLI
/// tokenize theo \n (mỗi dòng thành một lệnh) → dùng file tạm + upload.
fn write_file_uses_temp_upload(content: &str) -> bool {
    content.contains('\n')
}

/// Đường dẫn file tạm dùng cho `write` multi-line (nằm trong std::env::temp_dir).
fn write_temp_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "filen_gui_write_{}_{}.tmp",
        std::process::id(),
        nanos
    ))
}

fn recents_args() -> Vec<String> {
    vec!["recents".to_string()]
}

fn export_notes_args(path: Option<&str>) -> Vec<String> {
    let mut args = vec!["export-notes".to_string()];
    if let Some(p) = path {
        let p = p.trim();
        if !p.is_empty() {
            args.push(p.to_string());
        }
    }
    args
}

fn sync_args(locations: &[String], continuous: bool) -> Vec<String> {
    let mut args = vec!["sync".to_string()];
    args.extend(locations.iter().cloned());
    if continuous {
        args.push("--continuous".to_string());
    }
    args
}

// Các hàm build đối số được dùng bởi GUI (phase 7) và unit test.

pub fn webdav_args(user: &str, pass: &str, port: &str, https: bool) -> Vec<String> {
    let mut args = vec![
        "webdav".to_string(),
        "--w-user".to_string(),
        user.to_string(),
        "--w-password".to_string(),
        pass.to_string(),
        "--w-port".to_string(),
        port.to_string(),
    ];
    if https {
        args.push("--w-https".to_string());
    }
    args
}

#[allow(dead_code)]
pub fn webdav_proxy_args(port: &str, https: bool) -> Vec<String> {
    let mut args = vec!["webdav-proxy".to_string(), "--w-port".to_string(), port.to_string()];
    if https {
        args.push("--w-https".to_string());
    }
    args
}

pub fn s3_args(access_key: &str, secret_key: &str, port: &str, https: bool) -> Vec<String> {
    let mut args = vec![
        "s3".to_string(),
        "--s3-access-key-id".to_string(),
        access_key.to_string(),
        "--s3-secret-access-key".to_string(),
        secret_key.to_string(),
        "--s3-port".to_string(),
        port.to_string(),
    ];
    if https {
        args.push("--s3-https".to_string());
    }
    args
}

pub fn mount_args(mount_point: Option<&str>) -> Vec<String> {
    let mut args = vec!["mount".to_string()];
    if let Some(mp) = mount_point {
        let mp = mp.trim();
        if !mp.is_empty() {
            args.push(mp.to_string());
        }
    }
    args
}

/// Đối số CLI cho một pair (dùng alias nếu có, ngược lại `local:remote`).
#[allow(dead_code)]
pub fn sync_pair_arg(pair: &SyncPair) -> String {
    if let Some(alias) = &pair.alias
        && !alias.trim().is_empty()
    {
        return alias.clone();
    }
    format!("{}:{}", pair.local, pair.remote)
}

/// Đường dẫn tới syncPairs.json trong data dir.
#[allow(dead_code)]
pub fn sync_pairs_path() -> Option<PathBuf> {
    get_default_data_dir().map(|dir| dir.join("syncPairs.json"))
}

// Parse output dạng `ls --long` (cũng là format của `recents`) thành Vec<FileItem>
fn parse_ls_long(text: &str) -> Vec<FileItem> {
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
    items
}

// Parse output `stat --json` (dung sai key khác nhau của các phiên bản CLI) thành
// text dễ đọc. Trả Err khi không phải JSON object hợp lệ.
fn parse_stat_json(text: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("Không parse được JSON stat: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| "JSON stat không phải object".to_string())?;

    let get_str = |keys: &[&str]| -> Option<String> {
        keys.iter()
            .find_map(|k| obj.get(*k))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let get_bool = |keys: &[&str]| -> Option<bool> {
        keys.iter()
            .find_map(|k| obj.get(*k))
            .and_then(|v| v.as_bool())
    };

    let name = get_str(&["name", "Name"]).unwrap_or_default();
    let path = get_str(&["path", "Path"]).unwrap_or_default();
    let mime = get_str(&["mime", "mimeType", "MIME"]).unwrap_or_default();
    let modified = get_str(&["modified", "modifiedAt", "lastModified", "ModificationTime"])
        .unwrap_or_default();

    let file_type = if let Some(t) = get_str(&["type", "kind", "Type"]) {
        t.to_lowercase()
    } else if get_bool(&["isDirectory", "isDir"]).unwrap_or(false) {
        "directory".to_string()
    } else {
        "file".to_string()
    };

    let size = if let Some(s) = obj.get("size").and_then(|v| v.as_str()) {
        parse_size_bytes(s)
    } else {
        obj.get("size")
            .and_then(|v| v.as_u64())
            .or_else(|| obj.get("size").and_then(|v| v.as_f64()).map(|f| f as u64))
            .unwrap_or(0)
    };

    let mut lines = Vec::new();
    if !name.is_empty() {
        lines.push(format!("Name: {name}"));
    }
    if !path.is_empty() {
        lines.push(format!("Path: {path}"));
    }
    lines.push(format!("Type: {file_type}"));
    lines.push(format!("Size: {size} bytes"));
    if !mime.is_empty() {
        lines.push(format!("MIME: {mime}"));
    }
    if !modified.is_empty() {
        lines.push(format!("Modified: {modified}"));
    }
    Ok(lines.join("\n"))
}

// Parse syncPairs.json thành danh sách pair
fn parse_sync_pairs_json(text: &str) -> Result<Vec<SyncPair>, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

// URL Web Drive cho một path cloud (view trả về URL này, không spawn trình duyệt)
pub fn web_drive_url(path: Option<&str>) -> String {
    let base = "https://drive.filen.io";
    match path {
        Some(p) => {
            let p = p.trim().trim_start_matches('/');
            if p.is_empty() {
                base.to_string()
            } else {
                format!("{base}/{}", p)
            }
        }
        None => base.to_string(),
    }
}

// Mount point mặc định của filen-cli: phải nằm trong thư mục home trên Linux
// ("Cannot mount to a directory outside of your home directory").
pub fn default_mount_point() -> String {
    dirs::home_dir()
        .map(|home| home.join(".filen-drive").to_string_lossy().to_string())
        .unwrap_or_else(|| "/tmp/filen".to_string())
}

// Ghi chú yêu cầu FUSE/WinFSP theo hệ điều hành cho lệnh mount
pub fn mount_fuse_note() -> String {
    if cfg!(target_os = "linux") {
        "Mount yêu cầu FUSE3 (https://github.com/libfuse/libfuse). Cài đặt thêm gói fuse3 nếu chưa có. Trên Linux mount point phải nằm trong thư mục home.".to_string()
    } else if cfg!(target_os = "windows") {
        "Mount yêu cầu WinFSP (https://winfsp.dev/rel) hoặc WinFUSE.".to_string()
    } else if cfg!(target_os = "macos") {
        "Mount yêu cầu FUSE-T (https://www.fuse-t.org) hoặc macFUSE (https://osxfuse.github.io).".to_string()
    } else {
        "Mount yêu cầu FUSE3 (https://github.com/libfuse/libfuse).".to_string()
    }
}

// Tìm index của chuỗi có dạng YYYY-MM-DD
#[allow(dead_code)]
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
#[allow(dead_code)]
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

    // ─── get_default_data_dir ────────────────────────────────────────────────────

    #[test]
    fn test_get_default_data_dir_returns_something_when_home_exists() {
        // Nếu có home dir thì hàm luôn trả về Some (thư mục .filen-cli hoặc .config/filen-cli)
        if let Some(dir) = get_default_data_dir() {
            assert!(
                dir.ends_with(".filen-cli") || dir.ends_with("filen-cli"),
                "unexpected data dir: {}",
                dir.display()
            );
        }
    }

    // ─── Phase 6: head_args / tail_args ─────────────────────────────────────────

    #[test]
    fn test_head_args_default_lines() {
        assert_eq!(head_args("hello.txt", None), vec!["head", "hello.txt"]);
        assert_eq!(tail_args("hello.txt", None), vec!["tail", "hello.txt"]);
    }

    #[test]
    fn test_head_args_custom_lines() {
        assert_eq!(
            head_args("log.txt", Some(5)),
            vec!["head", "log.txt", "-n", "5"]
        );
        assert_eq!(
            tail_args("log.txt", Some(20)),
            vec!["tail", "log.txt", "-n", "20"]
        );
    }

    #[test]
    fn test_head_args_edge_cases() {
        // n = 0 vẫn truyền đối số -n 0 (CLI sẽ in 0 dòng)
        assert_eq!(head_args("f.txt", Some(0)), vec!["head", "f.txt", "-n", "0"]);
        // file rỗng
        assert_eq!(head_args("", None), vec!["head", ""]);
        // file chứa khoảng trắng
        assert_eq!(
            head_args("my file.txt", Some(1)),
            vec!["head", "my file.txt", "-n", "1"]
        );
    }

    // ─── Phase 6: stat_args / stat_json_args ────────────────────────────────────

    #[test]
    fn test_stat_args_basic() {
        assert_eq!(stat_args("/folder/file.txt"), vec!["stat", "/folder/file.txt"]);
        assert_eq!(
            stat_json_args("/folder/file.txt"),
            vec!["--json", "stat", "/folder/file.txt"]
        );
    }

    // ─── Phase 6: write_args ────────────────────────────────────────────────────

    #[test]
    fn test_write_args_normal_content() {
        assert_eq!(
            write_args("/tmp/note.txt", "hello world"),
            vec!["write", "/tmp/note.txt", "hello world"]
        );
    }

    #[test]
    fn test_write_args_empty_content() {
        // Ghi file rỗng: vẫn truyền đối số content rỗng
        assert_eq!(write_args("/tmp/empty.txt", ""), vec!["write", "/tmp/empty.txt", ""]);
    }

    #[test]
    fn test_write_args_multiline_content() {
        let content = "line1\nline2\nline3";
        let args = write_args("/tmp/multi.txt", content);
        assert_eq!(args[0], "write");
        assert_eq!(args[1], "/tmp/multi.txt");
        assert_eq!(args[2], "line1\nline2\nline3");
    }

    // ─── Phase 8: rm_args / write_file helpers ─────────────────────────────────

    #[test]
    fn test_rm_args_normal() {
        assert_eq!(rm_args("/a/b.txt", false), vec!["rm", "/a/b.txt"]);
    }

    #[test]
    fn test_rm_args_no_trash() {
        assert_eq!(
            rm_args("/a/b.txt", true),
            vec!["rm", "/a/b.txt", "--no-trash"]
        );
    }

    // ─── Phase 8.11: phát hiện/đếm prompt xác nhận (rm / rm --no-trash) ────────

    #[test]
    fn test_looks_like_confirm_prompt_detects_real_output() {
        // Output thật của filen-cli `app.promptConfirm` (không có \n vì CLI chờ input)
        assert!(looks_like_confirm_prompt("Are you sure you want to delete /a.txt? [y/N] "));
        assert!(looks_like_confirm_prompt(
            "Are you sure you want to permanently delete /a.txt? [y/N] "
        ));
        assert!(looks_like_confirm_prompt("Are you sure? [y/N] "));
        // Prompt cũng có thể xuất hiện lẫn với log trước đó trong cùng buffer
        assert!(looks_like_confirm_prompt("Deleting...\nAre you sure you want to delete /a.txt? [y/N] "));
    }

    #[test]
    fn test_looks_like_confirm_prompt_rejects_normal_output() {
        assert!(!looks_like_confirm_prompt(""));
        assert!(!looks_like_confirm_prompt("Deleted /a.txt"));
        assert!(!looks_like_confirm_prompt("No such file or directory: /a.txt"));
        assert!(!looks_like_confirm_prompt("Uploading raw.bin [=====] 100% | 1 MiB / 1 MiB"));
        assert!(!looks_like_confirm_prompt("y/N without brackets"));
    }

    #[test]
    fn test_looks_like_risks_prompt_detects_real_output() {
        assert!(looks_like_risks_prompt(
            "You are about to export a Filen CLI auth config,\nwhich is a file containing your unencrypted credentials.\nType \"I am aware of the risks\" to proceed: "
        ));
        assert!(!looks_like_risks_prompt(
            "Saved auth config to /home/user/.filen-cli/.filen-cli-auth-config"
        ));
    }

    #[test]
    fn test_looks_like_export_location_prompt_detects_real_output() {
        assert!(looks_like_export_location_prompt(
            "Choose an export location: [1] data directory, [2] here:"
        ));
        assert!(!looks_like_export_location_prompt("Invalid input, please choose \"1\" or \"2\""));
    }

    #[test]
    fn test_prompt_responder_single_confirm() {
        let mut r = PromptResponder::new(&[confirm_prompt_rule(1)]);
        assert_eq!(
            r.feed("Are you sure you want to delete /a.txt? [y/N] "),
            vec![&b"y\n"[..]]
        );
        // Output còn lại không phản hồi thêm (bộ đệm đã được xoá sau khi phát hiện)
        assert!(r.feed("Deleted /a.txt\n").is_empty());
    }

    #[test]
    fn test_prompt_responder_two_confirms_no_trash() {
        let mut r = PromptResponder::new(&[confirm_prompt_rule(2)]);
        // rm --no-trash: prompt 1
        assert_eq!(
            r.feed("Are you sure you want to permanently delete /a.txt? [y/N] "),
            vec![&b"y\n"[..]]
        );
        // prompt 2 chỉ xuất hiện sau khi prompt 1 được trả lời
        assert_eq!(r.feed("Are you sure? [y/N] "), vec![&b"y\n"[..]]);
        // prompt ngoài dự kiến (vượt max) → không trả lời
        assert!(r.feed("Are you sure? [y/N] ").is_empty());
    }

    #[test]
    fn test_prompt_responder_no_prompt() {
        let mut r = PromptResponder::new(&[confirm_prompt_rule(1)]);
        assert!(r.feed("").is_empty());
        assert!(r.feed("No such file or directory: /a.txt\n").is_empty());
    }

    #[test]
    fn test_prompt_responder_prompt_split_across_chunks() {
        // Prompt bị cắt giữa chừng giữa 2 chunk → chỉ phản hồi khi đủ dấu hiệu
        let mut r = PromptResponder::new(&[confirm_prompt_rule(1)]);
        assert!(r.feed("Are you sure you want to delete /a.txt").is_empty());
        assert!(r.feed("? [y/N").is_empty());
        assert_eq!(r.feed("] "), vec![&b"y\n"[..]]);
    }

    #[test]
    fn test_prompt_responder_export_auth_config_flow() {
        // Trình tự prompt thật của `filen export-auth-config` khi file đã tồn tại:
        // overwrite [y/N] → "I am aware of the risks" → "Choose an export location".
        let rules = [
            confirm_prompt_rule(1),
            PromptRule {
                matcher: looks_like_risks_prompt,
                response: b"I am aware of the risks\n",
                max: 1,
            },
            PromptRule {
                matcher: looks_like_export_location_prompt,
                response: b"1\n",
                max: 1,
            },
        ];
        let mut r = PromptResponder::new(&rules);
        assert_eq!(
            r.feed("Are you sure you want to overwrite .filen-cli-auth-config? [y/N] "),
            vec![&b"y\n"[..]]
        );
        assert_eq!(
            r.feed("You are about to export a Filen CLI auth config,\nwhich is a file containing your unencrypted credentials.\nType \"I am aware of the risks\" to proceed: "),
            vec![&b"I am aware of the risks\n"[..]]
        );
        assert_eq!(
            r.feed("Choose an export location: [1] data directory, [2] here:"),
            vec![&b"1\n"[..]]
        );
        assert!(r.feed("Saved auth config to /home/user/.filen-cli/.filen-cli-auth-config").is_empty());
    }

    #[test]
    fn test_write_file_uses_temp_upload_single_line() {
        assert!(!write_file_uses_temp_upload(""));
        assert!(!write_file_uses_temp_upload("hello world"));
        assert!(!write_file_uses_temp_upload("single line with spaces"));
    }

    #[test]
    fn test_write_file_uses_temp_upload_multiline() {
        assert!(write_file_uses_temp_upload("line1\nline2"));
        assert!(write_file_uses_temp_upload("a\nb\nc\n"));
        assert!(write_file_uses_temp_upload("\n"));
    }

    #[test]
    fn test_write_temp_path_in_temp_dir() {
        let p = write_temp_path();
        assert!(p.starts_with(std::env::temp_dir()));
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(name.starts_with("filen_gui_write_"), "name: {name}");
        // Hai lần gọi không trùng (đuôi timestamp).
        assert_ne!(p, write_temp_path());
    }

    // ─── Phase 6: recents_args / export_notes_args ──────────────────────────────

    #[test]
    fn test_recents_args() {
        assert_eq!(recents_args(), vec!["recents"]);
    }

    #[test]
    fn test_export_notes_args_default() {
        assert_eq!(export_notes_args(None), vec!["export-notes"]);
        assert_eq!(export_notes_args(Some("   ")), vec!["export-notes"]);
    }

    #[test]
    fn test_export_notes_args_with_path() {
        assert_eq!(
            export_notes_args(Some("/home/user/notes")),
            vec!["export-notes", "/home/user/notes"]
        );
    }

    // ─── Phase 6: sync_args ─────────────────────────────────────────────────────

    #[test]
    fn test_sync_args_single_no_continuous() {
        let locs = vec!["/local:/cloud".to_string()];
        assert_eq!(sync_args(&locs, false), vec!["sync", "/local:/cloud"]);
    }

    #[test]
    fn test_sync_args_multiple_continuous() {
        let locs = vec!["/a:/b".to_string(), "/c:/d".to_string()];
        assert_eq!(
            sync_args(&locs, true),
            vec!["sync", "/a:/b", "/c:/d", "--continuous"]
        );
    }

    #[test]
    fn test_sync_args_empty_locations() {
        assert_eq!(sync_args(&[], true), vec!["sync", "--continuous"]);
    }

    // ─── Phase 6: webdav_args / webdav_proxy_args / s3_args ────────────────────

    #[test]
    fn test_webdav_args_default() {
        let args = webdav_args("admin", "admin123", "8080", false);
        assert_eq!(
            args,
            vec![
                "webdav",
                "--w-user",
                "admin",
                "--w-password",
                "admin123",
                "--w-port",
                "8080",
            ]
        );
    }

    #[test]
    fn test_webdav_args_https() {
        let args = webdav_args("u", "p", "8443", true);
        assert!(args.contains(&"--w-https".to_string()));
    }

    #[test]
    fn test_webdav_proxy_args() {
        let args = webdav_proxy_args("8080", false);
        // Proxy mode không cần user/password
        assert_eq!(args, vec!["webdav-proxy", "--w-port", "8080"]);
        assert!(!args.iter().any(|a| a == "--w-user"));
        assert!(!args.iter().any(|a| a == "--w-password"));
    }

    #[test]
    fn test_webdav_proxy_args_https() {
        assert!(webdav_proxy_args("443", true).contains(&"--w-https".to_string()));
    }

    #[test]
    fn test_s3_args_default() {
        let args = s3_args("s3key", "s3secret", "9000", false);
        assert_eq!(
            args,
            vec![
                "s3",
                "--s3-access-key-id",
                "s3key",
                "--s3-secret-access-key",
                "s3secret",
                "--s3-port",
                "9000",
            ]
        );
    }

    #[test]
    fn test_s3_args_https() {
        assert!(s3_args("k", "s", "9001", true).contains(&"--s3-https".to_string()));
    }

    // ─── Phase 6: mount_args / mount_fuse_note ─────────────────────────────────

    #[test]
    fn test_mount_args_default() {
        assert_eq!(mount_args(None), vec!["mount"]);
        assert_eq!(mount_args(Some("   ")), vec!["mount"]);
    }

    #[test]
    fn test_mount_args_with_point() {
        assert_eq!(mount_args(Some("/tmp/filen")), vec!["mount", "/tmp/filen"]);
    }

    #[test]
    fn test_mount_fuse_note_non_empty() {
        let note = mount_fuse_note();
        assert!(!note.is_empty());
        #[cfg(target_os = "linux")]
        assert!(note.contains("FUSE3"));
        #[cfg(target_os = "windows")]
        assert!(note.contains("WinFSP"));
        #[cfg(target_os = "macos")]
        assert!(note.contains("FUSE-T") || note.contains("macFUSE"));
    }

    // ─── Phase 6: web_drive_url ─────────────────────────────────────────────────

    #[test]
    fn test_web_drive_url_default() {
        assert_eq!(web_drive_url(None), "https://drive.filen.io");
        assert_eq!(web_drive_url(Some("/")), "https://drive.filen.io");
        assert_eq!(web_drive_url(Some("")), "https://drive.filen.io");
    }

    #[test]
    fn test_web_drive_url_with_path() {
        assert_eq!(
            web_drive_url(Some("/folder/file.txt")),
            "https://drive.filen.io/folder/file.txt"
        );
        // Không bắt buộc gạch chéo đầu
        assert_eq!(
            web_drive_url(Some("folder")),
            "https://drive.filen.io/folder"
        );
    }

    // ─── Phase 6: parse_ls_long (format recents/ls --long) ─────────────────────

    #[test]
    fn test_parse_ls_long_file_and_dir() {
        let sample = "22 B  2026-06-07 13:17:03.31  hello.txt\n  2026-06-07 13:16:45.00  test_dir\n";
        let items = parse_ls_long(sample);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "hello.txt");
        assert!(!items[0].is_dir);
        assert_eq!(items[0].size, 22);
        assert_eq!(items[0].mod_time, "2026-06-07 13:17:03");
        assert_eq!(items[1].name, "test_dir");
        assert!(items[1].is_dir);
        assert_eq!(items[1].size, 0);
    }

    #[test]
    fn test_parse_ls_long_empty_and_garbage() {
        assert!(parse_ls_long("").is_empty());
        assert!(parse_ls_long("\n\n   \n").is_empty());
        // Dòng không có ngày tháng bị bỏ qua
        assert!(parse_ls_long("not a listing line").is_empty());
    }

    #[test]
    fn test_parse_ls_long_mixed_lines() {
        let sample = "garbage line\n1.5 KiB  2024-01-01 10:00:00.00  big.bin\n";
        let items = parse_ls_long(sample);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "big.bin");
        assert_eq!(items[0].size, 1536);
    }

    // ─── Phase 6: parse_stat_json ───────────────────────────────────────────────

    #[test]
    fn test_parse_stat_json_file() {
        let json = r#"{
            "name": "hello.txt",
            "path": "/hello.txt",
            "type": "file",
            "size": 12345,
            "mime": "text/plain",
            "modified": "2026-06-07T13:17:03.000Z"
        }"#;
        let text = parse_stat_json(json).unwrap();
        assert!(text.contains("Name: hello.txt"));
        assert!(text.contains("Path: /hello.txt"));
        assert!(text.contains("Type: file"));
        assert!(text.contains("Size: 12345 bytes"));
        assert!(text.contains("MIME: text/plain"));
    }

    #[test]
    fn test_parse_stat_json_directory_via_is_directory() {
        let json = r#"{"name": "docs", "isDirectory": true, "size": 0}"#;
        let text = parse_stat_json(json).unwrap();
        assert!(text.contains("Type: directory"));
        assert!(text.contains("Size: 0 bytes"));
    }

    #[test]
    fn test_parse_stat_json_size_as_string_with_unit() {
        let json = r#"{"name": "a.bin", "type": "file", "size": "1.5 KiB"}"#;
        let text = parse_stat_json(json).unwrap();
        assert!(text.contains("Size: 1536 bytes"));
    }

    #[test]
    fn test_parse_stat_json_invalid() {
        assert!(parse_stat_json("not json").is_err());
        assert!(parse_stat_json("[]").is_err()); // không phải object
    }

    // ─── Phase 6: parse_sync_pairs_json / sync_pair_arg / sync_pairs_path ───────

    #[test]
    fn test_parse_sync_pairs_json_full_fields() {
        let json = r#"[
            {
                "local": "/home/user/Documents",
                "remote": "/Documents",
                "syncMode": "twoWay",
                "alias": "docs",
                "disableLocalTrash": true,
                "excludeDotFiles": true,
                "ignore": ["*.tmp"]
            }
        ]"#;
        let pairs = parse_sync_pairs_json(json).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].local, "/home/user/Documents");
        assert_eq!(pairs[0].remote, "/Documents");
        assert_eq!(pairs[0].sync_mode, "twoWay");
        assert_eq!(pairs[0].alias.as_deref(), Some("docs"));
        assert_eq!(pairs[0].disable_local_trash, Some(true));
        assert_eq!(pairs[0].exclude_dot_files, Some(true));
        assert_eq!(pairs[0].ignore.as_deref(), Some(&["*.tmp".to_string()][..]));
    }

    #[test]
    fn test_parse_sync_pairs_json_minimal_fields() {
        let json = r#"[{"local": "/a", "remote": "/b"}]"#;
        let pairs = parse_sync_pairs_json(json).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].sync_mode, "");
        assert_eq!(pairs[0].alias, None);
    }

    #[test]
    fn test_parse_sync_pairs_json_empty_array() {
        let pairs = parse_sync_pairs_json("[]").unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_parse_sync_pairs_json_errors() {
        assert!(parse_sync_pairs_json("not json").is_err());
        assert!(parse_sync_pairs_json(r#"[{"remote": "/b"}]"#).is_err()); // thiếu local
    }

    #[test]
    fn test_sync_pair_arg() {
        let pair_with_alias = SyncPair {
            local: "/a".to_string(),
            remote: "/b".to_string(),
            sync_mode: String::new(),
            alias: Some("docs".to_string()),
            disable_local_trash: None,
            ignore: None,
            exclude_dot_files: None,
        };
        assert_eq!(sync_pair_arg(&pair_with_alias), "docs");

        let pair_no_alias = SyncPair {
            alias: None,
            ..pair_with_alias.clone()
        };
        assert_eq!(sync_pair_arg(&pair_no_alias), "/a:/b");
    }

    #[test]
    fn test_sync_pairs_path_ends_with_sync_pairs_json() {
        if let Some(path) = sync_pairs_path() {
            assert_eq!(
                path.file_name().and_then(|n| n.to_str()),
                Some("syncPairs.json")
            );
        }
    }

    // ─── Phase 6: server child state (webdav/s3/mount) ──────────────────────────

    #[test]
    fn test_webdav_server_state_defaults() {
        let state = WebDavServerState::default();
        assert!(!state.running);
        assert_eq!(state.user, "admin");
        assert_eq!(state.pass, "admin123");
        assert_eq!(state.port, "8080");
        assert!(!state.https);
        assert!(state.child.is_none());
        assert!(state.logs.is_empty());
    }

    #[tokio::test]
    async fn test_webdav_server_start_when_running_is_error() {
        let mut state = WebDavServerState {
            running: true,
            ..WebDavServerState::default()
        };
        let res = state.start(&None).await;
        assert!(res.is_err());
        // vẫn giữ nguyên running
        assert!(state.running);
    }

    #[tokio::test]
    async fn test_webdav_server_stop_when_not_running_is_ok() {
        let mut state = WebDavServerState::default();
        let res = state.stop().await;
        assert!(res.is_ok());
        assert!(!state.running);
    }

    #[tokio::test]
    async fn test_s3_server_start_when_running_is_error() {
        let mut state = S3ServerState {
            running: true,
            ..S3ServerState::default()
        };
        let res = state.start(&None).await;
        assert!(res.is_err());
        assert!(state.running);
    }

    #[tokio::test]
    async fn test_s3_server_stop_when_not_running_is_ok() {
        let mut state = S3ServerState::default();
        assert!(state.stop().await.is_ok());
        assert!(!state.running);
    }

    #[test]
    fn test_s3_server_state_defaults() {
        let state = S3ServerState::default();
        assert_eq!(state.access_key, "s3key");
        assert_eq!(state.secret_key, "s3secret");
        assert_eq!(state.port, "9000");
        assert!(!state.running);
    }

    #[test]
    fn test_mount_state_defaults() {
        let state = MountState::default();
        assert!(!state.running);
        assert_eq!(state.mount_point, default_mount_point());
        assert!(!state.note.is_empty());
    }

    #[tokio::test]
    async fn test_mount_start_when_running_is_error() {
        let mut state = MountState {
            running: true,
            ..MountState::default()
        };
        let res = state.start(&None, Some("/tmp/filen")).await;
        assert!(res.is_err());
        assert!(state.running);
    }

    #[tokio::test]
    async fn test_mount_stop_when_not_running_is_ok() {
        let mut state = MountState::default();
        assert!(state.stop().await.is_ok());
        assert!(!state.running);
    }

    #[test]
    fn test_default_mount_point_is_absolute_and_in_home() {
        let mp = default_mount_point();
        assert!(mp.starts_with('/'));
        // Linux: filen-cli chỉ cho mount trong home → mặc định phải nằm trong home
        if let Some(home) = dirs::home_dir() {
            assert!(mp.starts_with(&home.to_string_lossy().to_string()));
        }
    }

    #[test]
    fn test_scan_node_bins_ignores_foreign_binaries() {
        // Tạo home giả: .local/bin chứa "filen" và "agy" (binary khác, mới hơn).
        let tmp = std::env::temp_dir().join(format!(
            "filen_gui_scan_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let local_bin = tmp.join(".local/bin");
        std::fs::create_dir_all(&local_bin).unwrap();
        std::fs::write(local_bin.join("filen"), b"#!/bin/sh\n").unwrap();
        std::fs::write(local_bin.join("agy"), b"go-binary\n").unwrap();
        // .nvm/versions/node/v20.20.2/bin/filen
        let nvm_bin = tmp.join(".nvm/versions/node/v20.20.2/bin");
        std::fs::create_dir_all(&nvm_bin).unwrap();
        std::fs::write(nvm_bin.join("filen"), b"#!/usr/bin/env node\n").unwrap();

        let found = scan_node_bins(&tmp);
        let names: Vec<String> = found
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert_eq!(found.len(), 2, "chỉ có 2 file tên đúng 'filen': {names:?}");
        assert!(
            names.iter().any(|p| p.contains(".local/bin/filen")),
            "phải tìm thấy ~/.local/bin/filen: {names:?}"
        );
        assert!(
            names.iter().any(|p| p.contains(".nvm/versions/node/v20.20.2/bin/filen")),
            "phải tìm thấy nvm filen: {names:?}"
        );
        assert!(
            names.iter().all(|p| !p.contains("agy")),
            "không được nhặt nhầm agy: {names:?}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
