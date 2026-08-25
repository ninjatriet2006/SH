//! [INTEGRITY NOTES]
//! Mục đích: Quản lý và thực thi các luồng truyền tải dữ liệu.
//! Trách nhiệm: Xử lý hàng đợi Upload, Download, Copy, Move qua CLI `filen` hoặc `std::fs`. Parse tiến trình tiến độ.
//! Tương tác: Giao tiếp với CLI `filen`, luồng thread, mpsc channel và `models`.
//!
//! Quản lý và chạy các luồng chuyển dữ liệu: upload/download qua CLI filen
//! (pipe stdout/stderr + parse tiến trình + timeout/huỷ), copy/move Cloud qua
//! CLI `cp`/`mv`, và copy/move Local→Local qua `std::fs`.
//!
//! Toàn bộ công việc chạy trong thread riêng (không block UI); kết quả gửi về
//! app qua mpsc channel giống các thao tác async khác trong main.rs.

use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::io::AsyncReadExt;

use crate::models::{get_default_data_dir, resolve_filen_bin};

/// Loại thao tác chuyển dữ liệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Upload,
    Download,
    Copy,
    Move,
}

impl TransferKind {
    pub fn label(&self) -> &'static str {
        match self {
            TransferKind::Upload => "Tải lên",
            TransferKind::Download => "Tải xuống",
            TransferKind::Copy => "Sao chép",
            TransferKind::Move => "Di chuyển",
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            TransferKind::Upload => "⬆️",
            TransferKind::Download => "⬇️",
            TransferKind::Copy => "📋",
            TransferKind::Move => "✂️",
        }
    }
}

/// Trạng thái một transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    Queued,
    Running,
    Done,
    Error,
    Cancelled,
}

impl TransferStatus {
    pub fn label(&self) -> &'static str {
        match self {
            TransferStatus::Queued => "Chờ",
            TransferStatus::Running => "Đang chạy",
            TransferStatus::Done => "Xong",
            TransferStatus::Error => "Lỗi",
            TransferStatus::Cancelled => "Đã huỷ",
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            TransferStatus::Done | TransferStatus::Error | TransferStatus::Cancelled
        )
    }
}

/// Lỗi khi chạy transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    Timeout,
    Cancelled,
    Spawn(String),
    Failed(String),
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::Timeout => write!(f, "Quá thời gian chờ"),
            TransferError::Cancelled => write!(f, "Đã huỷ"),
            TransferError::Spawn(e) => write!(f, "Không khởi chạy được CLI: {e}"),
            TransferError::Failed(e) => write!(f, "{e}"),
        }
    }
}

/// Cập nhật tiến trình từ CLI.
#[derive(Debug, Clone, Copy)]
pub struct ProgressUpdate {
    /// 0.0..=1.0 nếu parse được; `None` = indeterminate.
    pub progress: Option<f32>,
    pub bytes_done: u64,
    pub total_bytes: u64,
}

/// Một mục chuyển dữ liệu trong danh sách.
#[derive(Debug, Clone)]
pub struct TransferItem {
    pub id: usize,
    pub kind: TransferKind,
    /// Tên hiển thị (file/thư mục đang chuyển).
    pub name: String,
    /// Đường dẫn nguồn (local hoặc cloud).
    pub src: String,
    /// Đường dẫn đích (local hoặc cloud).
    pub dst: String,
    /// Nguồn/đích có phải đường dẫn cục bộ không.
    pub src_local: bool,
    pub dst_local: bool,
    /// Sau khi thành công có xoá nguồn (dùng cho Move qua Cloud) không.
    pub cleanup_src: bool,
    pub status: TransferStatus,
    /// 0.0..=1.0 nếu có dữ liệu tiến trình, `None` = indeterminate.
    pub progress: Option<f32>,
    pub bytes_done: u64,
    pub total_bytes: u64,
    /// Thông báo (lỗi / trạng thái phụ).
    pub msg: String,
    /// Cờ huỷ (atomic, đọc bởi thread runner).
    pub cancelled: Arc<AtomicBool>,
    /// Khung nguồn/đích cần refresh khi xong.
    pub src_pane: usize,
    pub dst_pane: usize,
}

/// Hàng đợi và danh sách transfer.
pub struct TransferManager {
    pub items: Vec<TransferItem>,
    /// Số transfer chạy đồng thời tối đa.
    pub max_concurrent: usize,
    /// Timeout (giây) cho transfer CLI; 0 = vô hạn.
    pub timeout_secs: u64,
    next_id: usize,
}

impl TransferManager {
    pub fn new() -> Self {
        TransferManager {
            items: Vec::new(),
            max_concurrent: 2,
            timeout_secs: 600,
            next_id: 1,
        }
    }

    /// Thêm transfer vào hàng đợi, trả về id.
    #[allow(clippy::too_many_arguments)]
    pub fn enqueue(
        &mut self,
        kind: TransferKind,
        name: String,
        src: String,
        dst: String,
        src_local: bool,
        dst_local: bool,
        cleanup_src: bool,
        src_pane: usize,
        dst_pane: usize,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(TransferItem {
            id,
            kind,
            name,
            src,
            dst,
            src_local,
            dst_local,
            cleanup_src,
            status: TransferStatus::Queued,
            progress: None,
            bytes_done: 0,
            total_bytes: 0,
            msg: String::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
            src_pane,
            dst_pane,
        });
        id
    }

    pub fn running_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.status == TransferStatus::Running)
            .count()
    }

    /// Index của mục queued đầu tiên (để khởi động đúng giới hạn concurrent).
    pub fn next_queued_idx(&self) -> Option<usize> {
        self.items
            .iter()
            .position(|i| i.status == TransferStatus::Queued)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut TransferItem> {
        self.items.iter_mut().find(|i| i.id == id)
    }

    /// Đặt cờ huỷ cho một transfer (thread runner sẽ kill child).
    pub fn cancel(&self, id: usize) {
        if let Some(item) = self.items.iter().find(|i| i.id == id) {
            item.cancelled.store(true, Ordering::Relaxed);
        }
    }

    /// Gỡ các mục đã kết thúc (Done/Error/Cancelled) khỏi danh sách.
    pub fn remove_finished(&mut self) {
        self.items.retain(|i| !i.status.is_finished());
    }

    /// Huỷ tất cả transfer đang chờ/đang chạy.
    pub fn cancel_all(&self) {
        for item in &self.items {
            item.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Runner CLI upload/download ──────────────────────────────────────────────

/// Chạy transfer upload/download bằng CLI filen. Pipe stdout+stderr để đọc dòng
/// progress (cli-progress ghi ra stderr, dùng ký tự `\r` giữa các lần render).
///
/// - `timeout_secs`: 0 = không giới hạn.
/// - `cancelled`: khi cờ được set, tiến trình con bị kill.
/// - `on_update`: gọi mỗi khi parse được dữ liệu tiến trình mới.
pub async fn run_cli_transfer_terminal(
    kind: TransferKind,
    src: &str,
    dst: &str,
    timeout_secs: u64,
    cancelled: Arc<AtomicBool>,
    on_update: impl FnMut(ProgressUpdate),
) -> Result<(), TransferError> {
    let argv = build_transfer_argv(kind, src, dst)?;
    // Unix: chạy qua `script -qec` để giả lập TTY — filen-cli (cli-progress) chỉ
    // render progress khi có TTY nên pipe trực tiếp không nhận được dữ liệu.
    let mut cmd = build_transfer_command(&argv);
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| TransferError::Spawn(e.to_string()))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| TransferError::Spawn("stdout bị đóng".to_string()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| TransferError::Spawn("stderr bị đóng".to_string()))?;

    let deadline = if timeout_secs > 0 {
        Some(
            tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs),
        )
    } else {
        None
    };

    let mut on_update = on_update;
    let mut stdout_open = true;
    let mut stderr_open = true;
    let mut stdout_buf = [0u8; 8192];
    let mut stderr_buf = [0u8; 8192];
    let mut stdout_carry = String::new();
    let mut stderr_carry = String::new();
    let mut stdout_tail = String::new();
    let mut stderr_tail = String::new();

    while stdout_open || stderr_open {
        tokio::select! {
            res = stdout.read(&mut stdout_buf), if stdout_open => {
                match res {
                    Ok(0) => stdout_open = false,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&stdout_buf[..n]);
                        stdout_tail.push_str(&chunk);
                        if stdout_tail.len() > 4096 {
                            let cut = stdout_tail.len() - 4096;
                            stdout_tail.drain(..cut);
                        }
                        handle_progress_chunk(&stdout_buf[..n], &mut stdout_carry, &mut on_update)
                    }
                    Err(_) => stdout_open = false,
                }
            }
            res = stderr.read(&mut stderr_buf), if stderr_open => {
                match res {
                    Ok(0) => stderr_open = false,
                    Ok(n) => {
                        let chunk = String::from_utf8_lossy(&stderr_buf[..n]);
                        stderr_tail.push_str(&chunk);
                        if stderr_tail.len() > 4096 {
                            let cut = stderr_tail.len() - 4096;
                            stderr_tail.drain(..cut);
                        }
                        handle_progress_chunk(&stderr_buf[..n], &mut stderr_carry, &mut on_update);
                    }
                    Err(_) => stderr_open = false,
                }
            }
            _ = cancel_signal(&cancelled) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(TransferError::Cancelled);
            }
            _ = wait_deadline(deadline) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(TransferError::Timeout);
            }
        }
    }

    // Dòng cuối chưa có ký tự xuống dòng — quét nốt.
    if let Some(upd) = parse_progress(&stdout_carry) {
        on_update(upd);
    }
    if let Some(upd) = parse_progress(&stderr_carry) {
        on_update(upd);
    }

    let status = child
        .wait()
        .await
        .map_err(|e| TransferError::Failed(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        // Khi chạy qua `script`, stdout+stderr của CLI bị gộp lên stdout của script
        // → ưu tiên stderr_tail, fallback stdout_tail nếu trống.
        let tail = if stderr_tail.trim().is_empty() {
            &stdout_tail
        } else {
            &stderr_tail
        };
        Err(TransferError::Failed(last_useful_line(tail)))
    }
}

// ─── Build command / script wrapper (subtask 8.13) ──────────────────────────

/// Đối số CLI cho transfer upload/download (dạng Vec để dễ test và nối thành câu
/// lệnh khi chạy qua `script`).
fn build_transfer_argv(
    kind: TransferKind,
    src: &str,
    dst: &str,
) -> Result<Vec<String>, TransferError> {
    let mut argv = vec![resolve_filen_bin().to_string_lossy().to_string()];
    if let Some(data_path) = get_default_data_dir() {
        argv.push("--data-dir".to_string());
        argv.push(data_path.to_string_lossy().to_string());
    }
    match kind {
        TransferKind::Upload => {
            argv.push("upload".to_string());
            argv.push(src.to_string());
            argv.push(dst.to_string());
        }
        TransferKind::Download => {
            argv.push("download".to_string());
            argv.push(src.to_string());
            argv.push(dst.to_string());
        }
        _ => {
            return Err(TransferError::Spawn(
                "Loại transfer này không chạy bằng CLI upload/download".to_string(),
            ));
        }
    }
    Ok(argv)
}

/// Build tokio Command cho transfer.
///
/// Trên Unix ưu tiên chạy qua `script -qec "<cmd>" /dev/null` để giả lập TTY:
/// filen-cli (cli-progress) chỉ render progress khi có TTY nên pipe trực tiếp
/// không nhận được progress. `-e` giữ mã thoát của tiến trình con; `/dev/null`
/// bỏ transcript. Nếu `script` không khả thi → fallback chạy trực tiếp và UI sẽ
/// hiển thị progress indeterminate.
fn build_transfer_command(argv: &[String]) -> tokio::process::Command {
    #[cfg(unix)]
    {
        if which::which("script").is_ok() {
            let cmd_str = argv
                .iter()
                .map(|a| shell_quote(a))
                .collect::<Vec<_>>()
                .join(" ");
            let mut cmd = tokio::process::Command::new("script");
            cmd.args(["-qec", cmd_str.as_str(), "/dev/null"]);
            cmd.kill_on_drop(true);
            return cmd;
        }
    }
    let mut cmd = tokio::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd.kill_on_drop(true);
    cmd
}

/// Quote một đối số cho shell (dùng khi nối argv thành câu lệnh `script -c`).
fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

/// Chờ cho đến khi cờ huỷ được set (kiểm tra mỗi 100ms).
async fn cancel_signal(cancelled: &AtomicBool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
    }
}

/// Chờ đến deadline; nếu không có deadline thì không bao giờ resolve.
async fn wait_deadline(deadline: Option<tokio::time::Instant>) {
    if let Some(d) = deadline {
        tokio::time::sleep_until(d).await;
    } else {
        std::future::pending::<()>().await;
    }
}

/// Gộp chunk vào carry, xử lý các đoạn render progress hoàn chỉnh và giữ lại
/// phần chưa đủ cho lần sau.
///
/// CLI render nhiều segment progress trong 1 chunk, phân tách bằng `ESC[1G`
/// (cursor-home) chứ không phải `\r`/`\n`. Khi có `ESC[1G`, chỉ lấy segment
/// CUỐI cùng (phần trăm mới nhất); nếu segment cuối không parse được thì ưu
/// tiên % cao nhất trong các segment đã thấy. Nếu không có `ESC[1G` thì dùng
/// logic cũ (tách theo `\r`/`\n`).
fn handle_progress_chunk(
    chunk: &[u8],
    carry: &mut String,
    on_update: &mut impl FnMut(ProgressUpdate),
) {
    carry.push_str(&String::from_utf8_lossy(chunk));
    // Phòng hờ: carry không chứa separator trong thời gian dài → cắt bớt đầu.
    if carry.len() > 16_384 {
        let cut = carry.len() - 4096;
        carry.drain(..cut);
    }

    const CURSOR_HOME: &str = "\x1b[1G";
    if let Some(_pos) = carry.find(CURSOR_HOME) {
        let segments: Vec<&str> = carry.split(CURSOR_HOME).collect();
        // segments[0] là text trước ESC[1G đầu tiên (log cũ, không phải progress).
        // Thử segment cuối trước (mới nhất), fallback sang % cao nhất.
        let mut chosen = parse_progress(segments[segments.len() - 1]);
        if chosen.is_none() {
            let mut best: Option<ProgressUpdate> = None;
            for seg in segments.iter().skip(1) {
                if let Some(upd) = parse_progress(seg) {
                    let better = match (best.as_ref(), upd.progress) {
                        (Some(b), Some(p)) => b.progress.unwrap_or(0.0) < p,
                        (None, _) => true,
                        _ => false,
                    };
                    if better {
                        best = Some(upd);
                    }
                }
            }
            chosen = best;
        }
        if let Some(upd) = chosen {
            on_update(upd);
        }
        // Giữ lại phần sau ESC[1G cuối cùng (render đang dở) cho lần sau.
        let keep_from = carry
            .rfind(CURSOR_HOME)
            .map(|p| p + CURSOR_HOME.len())
            .unwrap_or(0);
        *carry = carry[keep_from..].trim_start_matches(['\r', '\n']).to_string();
        return;
    }

    if let Some(pos) = carry.rfind(['\r', '\n']) {
        let (done, rest) = carry.split_at(pos);
        for line in done.split(['\r', '\n']) {
            if let Some(upd) = parse_progress(line) {
                on_update(upd);
            }
        }
        *carry = rest.trim_start_matches(['\r', '\n']).to_string();
    }
}

/// Parse một đoạn render của progress bar. Ưu tiên dạng `NN%`; nếu không có thì
/// thử `X <unit> / Y <unit>` để tính theo bytes. Không parse được → None.
fn parse_progress(segment: &str) -> Option<ProgressUpdate> {
    let (bytes_done, total_bytes) = parse_bytes_fraction(segment);
    if let Some(pct) = parse_percent(segment) {
        return Some(ProgressUpdate {
            progress: Some(pct),
            bytes_done,
            total_bytes,
        });
    }
    if total_bytes > 0 {
        return Some(ProgressUpdate {
            progress: Some((bytes_done as f32 / total_bytes as f32).clamp(0.0, 1.0)),
            bytes_done,
            total_bytes,
        });
    }
    None
}

/// Tìm phần trăm trong text: số đứng ngay trước ký tự `%`, trả về 0.0..=1.0.
fn parse_percent(text: &str) -> Option<f32> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b'%' {
            continue;
        }
        let mut start = i;
        let mut seen_digit = false;
        while start > 0 {
            let c = bytes[start - 1];
            if c.is_ascii_digit() {
                start -= 1;
                seen_digit = true;
            } else if c == b'.' && seen_digit {
                start -= 1;
            } else {
                break;
            }
        }
        if seen_digit
            && let Ok(v) = text[start..i].parse::<f32>()
        {
            return Some((v / 100.0).clamp(0.0, 1.0));
        }
    }
    None
}

/// Tìm cặp "X <unit> / Y <unit>" trong text, trả về (bytes_done, total).
fn parse_bytes_fraction(text: &str) -> (u64, u64) {
    let segments: Vec<&str> = text.split(" / ").collect();
    if segments.len() < 2 {
        return (0, 0);
    }
    let done = parse_byte_size(segments[segments.len() - 2].trim());
    let total = parse_byte_size(segments[segments.len() - 1].trim());
    (done, total)
}

/// Parse kích thước có đơn vị ("6.1 MiB") ở bất kỳ vị trí nào trong chuỗi.
/// Trả về match *cuối* cùng (value/total luôn đứng cuối dòng progress).
/// Không hiểu → 0.
fn parse_byte_size(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut last = 0u64;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let num_start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let num_end = i;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let unit_start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            let unit = &s[unit_start..i];
            let mult = match unit.to_uppercase().as_str() {
                "B" => 1.0,
                "KB" | "KIB" => 1024.0,
                "MB" | "MIB" => 1024.0 * 1024.0,
                "GB" | "GIB" => 1024.0 * 1024.0 * 1024.0,
                "TB" | "TIB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
                _ => 0.0,
            };
            if mult > 0.0
                && let Ok(num) = s[num_start..num_end].parse::<f64>()
            {
                last = (num * mult) as u64;
            }
        } else {
            i += 1;
        }
    }
    last
}

/// Dòng cuối không rỗng trong phần đuôi stderr (dùng làm thông báo lỗi).
fn last_useful_line(tail: &str) -> String {
    for line in tail.lines().rev() {
        let line = line.trim();
        if !line.is_empty() {
            return line.to_string();
        }
    }
    "CLI thoát với mã lỗi".to_string()
}

// ─── Copy/move Local→Local (đồng bộ, chạy trong thread) ─────────────────────

/// Sao chép file hoặc thư mục Local→Local.
pub fn copy_local(src: &str, dst: &str) -> Result<(), String> {
    let s = Path::new(src);
    let d = Path::new(dst);
    if s.is_dir() {
        copy_dir_recursive(s, d)
    } else {
        std::fs::copy(s, d).map(|_| ()).map_err(|e| e.to_string())
    }
}

/// Di chuyển file/thư mục Local→Local bằng rename; nếu khác filesystem thì
/// copy + xoá nguồn.
pub fn move_local(src: &str, dst: &str) -> Result<(), String> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    crate::local_fs::copy_local(src, dst, false)?;
    delete_local_path(src)?;
    Ok(())
}

/// Xoá file hoặc thư mục local (dùng khi Move qua Cloud).
pub fn delete_local_path(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if p.is_dir() {
        std::fs::remove_dir_all(p).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(p).map_err(|e| e.to_string())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_percent_basic() {
        assert_eq!(
            parse_percent(
                "Uploading a.txt [██████░░] 60% | 1.2 MB/s | ETA: 0:05 | 6.1 MiB / 10.2 MiB"
            ),
            Some(0.6)
        );
        assert_eq!(parse_percent("0%"), Some(0.0));
        assert_eq!(parse_percent("100%"), Some(1.0));
        assert_eq!(parse_percent("không có phần trăm"), None);
        assert_eq!(parse_percent("50% | ETA: 0:01"), Some(0.5));
    }

    #[test]
    fn test_parse_byte_size() {
        assert_eq!(parse_byte_size("1024 B"), 1024);
        assert_eq!(parse_byte_size("1.5 MiB"), (1.5 * 1024.0 * 1024.0) as u64);
        assert_eq!(parse_byte_size("2 GiB"), 2 * 1024 * 1024 * 1024);
        assert_eq!(parse_byte_size("abc"), 0);
        assert_eq!(parse_byte_size(""), 0);
    }

    #[test]
    fn test_parse_bytes_fraction() {
        let (d, t) = parse_bytes_fraction(
            "Uploading a.txt [bar] 60% | 1.2 MB/s | ETA: 0:05 | 6.1 MiB / 10.2 MiB",
        );
        assert_eq!(d, (6.1 * 1024.0 * 1024.0) as u64);
        assert_eq!(t, (10.2 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn test_parse_progress_none_when_no_data() {
        assert!(parse_progress("Đang chuẩn bị...").is_none());
        assert!(parse_progress("Uploaded successfully").is_none());
    }

    #[test]
    fn test_parse_progress_uses_percent() {
        let upd = parse_progress("Uploading x [███] 33% | 1 B / 3 B").unwrap();
        assert_eq!(upd.progress, Some(0.33));
    }

    #[test]
    fn test_parse_progress_bytes_fallback() {
        let upd = parse_progress("Downloading y [bar] | 5 MiB / 10 MiB").unwrap();
        assert!((upd.progress.unwrap() - 0.5).abs() < 0.001);
        assert_eq!(upd.bytes_done, (5.0 * 1024.0 * 1024.0) as u64);
        assert_eq!(upd.total_bytes, (10.0 * 1024.0 * 1024.0) as u64);
    }

    // ─── Phase 8.11: parse progress tách theo ESC[1G (cursor-home) ─────────────

    #[test]
    fn test_handle_progress_chunk_ansi_cursor_home_takes_latest_percent() {
        // Mẫu ANSI gộp thật: nhiều render trong 1 chunk, phân tách bằng ESC[1G
        // (chứ không phải \r). Phải lấy segment CUỐI (100%).
        let mut carry = String::new();
        let mut updates: Vec<ProgressUpdate> = Vec::new();
        // Format thật của filen-cli: `[{bar}] {percentage}% | {speed} | ETA: ... | {value} / {total}`
        let sample = concat!(
            "\x1b[1GUploading raw.bin [-----] 0% | N/A | ETA: -- | 0 B / 10 MiB\x1b[0K",
            "\x1b[1GUploading raw.bin [=====] 100% | 5 MiB/s | ETA: 0s | 10 MiB / 10 MiB\x1b[0K",
        );
        handle_progress_chunk(sample.as_bytes(), &mut carry, &mut |u| updates.push(u));
        assert_eq!(updates.len(), 1, "chỉ nên báo 1 lần với % mới nhất");
        assert_eq!(updates[0].progress, Some(1.0));
        assert_eq!(updates[0].bytes_done, (10.0 * 1024.0 * 1024.0) as u64);
        assert_eq!(updates[0].total_bytes, (10.0 * 1024.0 * 1024.0) as u64);
        // Carry giữ lại render cuối để quét tiếp khi có chunk mới.
        assert!(carry.contains("100%"));
    }

    #[test]
    fn test_handle_progress_chunk_ansi_multi_chunk() {
        let mut carry = String::new();
        let mut updates: Vec<ProgressUpdate> = Vec::new();
        let chunk1 = "\x1b[1GUploading raw.bin [-----] 0% | N/A | ETA: -- | 0 B / 10 MiB\x1b[0K";
        let chunk2 = "\x1b[1GUploading raw.bin [=====] 100% | 5 MiB/s | ETA: 0s | 10 MiB / 10 MiB\x1b[0K";
        handle_progress_chunk(chunk1.as_bytes(), &mut carry, &mut |u| updates.push(u));
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].progress, Some(0.0));
        handle_progress_chunk(chunk2.as_bytes(), &mut carry, &mut |u| updates.push(u));
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[1].progress, Some(1.0));
    }

    #[test]
    fn test_handle_progress_chunk_ansi_last_segment_unclear_falls_back_highest() {
        // Segment cuối không chứa % rõ ràng (bị cắt giữa render) → fallback % cao nhất.
        let mut carry = String::new();
        let mut updates: Vec<ProgressUpdate> = Vec::new();
        let sample = concat!(
            "\x1b[1GUploading a.txt [====] 40% | 4 MiB / 10 MiB\x1b[0K",
            "\x1b[1GUploading a.txt [=====] 100% | 10 MiB / 10 MiB\x1b[0K",
            "\x1b[1GUploading a.txt [===",
        );
        handle_progress_chunk(sample.as_bytes(), &mut carry, &mut |u| updates.push(u));
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].progress, Some(1.0));
    }

    #[test]
    fn test_handle_progress_chunk_crlf_separated() {
        // Khi không có ESC[1G vẫn giữ logic cũ (tách theo \r).
        let mut carry = String::new();
        let mut updates: Vec<ProgressUpdate> = Vec::new();
        let sample = "Downloading a [bar] 0%\rDownloading a [bar] 50%\r";
        handle_progress_chunk(sample.as_bytes(), &mut carry, &mut |u| updates.push(u));
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].progress, Some(0.0));
        assert_eq!(updates[1].progress, Some(0.5));
    }

    // ─── Phase 8: shell_quote / build_transfer_argv (subtask 8.13) ────────────

    #[test]
    fn test_shell_quote_simple() {
        assert_eq!(shell_quote("filen"), "'filen'");
        assert_eq!(shell_quote("hello world"), "'hello world'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn test_shell_quote_escapes_single_quote() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
        assert_eq!(shell_quote("with 'quotes' inside"), "'with '\\''quotes'\\'' inside'");
    }

    #[test]
    fn test_build_transfer_argv_upload() {
        let argv = build_transfer_argv(TransferKind::Upload, "/local/a.txt", "/Cloud").unwrap();
        assert_eq!(argv[argv.len() - 3], "upload");
        assert_eq!(argv[argv.len() - 2], "/local/a.txt");
        assert_eq!(argv[argv.len() - 1], "/Cloud");
        // --data-dir được thêm vào khi có home dir.
        if get_default_data_dir().is_some() {
            assert!(argv.windows(2).any(|w| w[0] == "--data-dir"));
        }
    }

    #[test]
    fn test_build_transfer_argv_download() {
        let argv = build_transfer_argv(TransferKind::Download, "/Cloud/a.txt", "/local").unwrap();
        assert_eq!(argv[argv.len() - 3], "download");
        assert_eq!(argv[argv.len() - 2], "/Cloud/a.txt");
        assert_eq!(argv[argv.len() - 1], "/local");
    }

    #[test]
    fn test_build_transfer_argv_unsupported_kind() {
        assert!(build_transfer_argv(TransferKind::Copy, "/a", "/b").is_err());
        assert!(build_transfer_argv(TransferKind::Move, "/a", "/b").is_err());
    }

    #[test]
    fn test_transfer_manager_queue_and_cancel() {
        let mut m = TransferManager::new();
        let id1 = m.enqueue(
            TransferKind::Upload,
            "a".to_string(),
            "/a".to_string(),
            "/dst".to_string(),
            true,
            false,
            false,
            0,
            1,
        );
        let id2 = m.enqueue(
            TransferKind::Download,
            "b".to_string(),
            "/b".to_string(),
            "/c".to_string(),
            false,
            true,
            false,
            0,
            1,
        );
        assert_ne!(id1, id2);
        assert_eq!(m.running_count(), 0);
        assert_eq!(m.next_queued_idx(), Some(0));
        m.items[0].status = TransferStatus::Running;
        assert_eq!(m.running_count(), 1);
        m.cancel(id2);
        assert!(m.items[1].cancelled.load(Ordering::Relaxed));
        m.items[0].status = TransferStatus::Done;
        m.items[1].status = TransferStatus::Cancelled;
        m.remove_finished();
        assert!(m.items.is_empty());
    }
}
