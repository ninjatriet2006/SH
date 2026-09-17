use std::collections::VecDeque;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::fingerprint::LeakFinding;

/// Đường dẫn thư mục logs mặc định theo chế độ Portable:
/// Ưu tiên thư mục cha của current_exe() / logs, fallback về ./logs
pub fn get_logs_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("VPN_AI_PROXY_CONFIG_DIR") {
        return PathBuf::from(dir).join("logs");
    }
    if let Some(home) = std::env::var_os("HOME") {
        let path = PathBuf::from(home)
            .join(".config")
            .join("vpn_ai_proxy_gui")
            .join("logs");
        let _ = std::fs::create_dir_all(&path);
        return path;
    }
    PathBuf::from("logs")
}

pub fn get_traffic_log_path() -> PathBuf {
    get_logs_dir().join("traffic_log.jsonl")
}

/// Ghi một bản ghi RawTrafficLog xuống file disk log (JSONL) kèm cơ chế xoay vòng (rotation).
/// Rotation check KHÔNG chạy mỗi request (đọc cả file rất tốn kém) mà chỉ mỗi
/// ROTATE_CHECK_EVERY lần append — đủ để giữ giới hạn với chi phí O(1) amortized.
pub fn append_disk_log(log_path: &Path, log: &RawTrafficLog, max_disk_entries: usize) -> std::io::Result<()> {
    if let Some(parent) = log_path.parent() {
        create_dir_all(parent)?;
    }

    let serialized = serde_json::to_string(log).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Append entry
    {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        writeln!(file, "{}", serialized)?;
    }

    if max_disk_entries > 0 {
        static APPEND_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        const ROTATE_CHECK_EVERY: usize = 64;
        let n = APPEND_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if n % ROTATE_CHECK_EVERY == 0 {
            rotate_disk_log_if_needed(log_path, max_disk_entries)?;
        }
    }

    Ok(())
}

/// Kiểm tra và xoay vòng file log nếu số dòng vượt quá max_disk_entries + threshold
pub fn rotate_disk_log_if_needed(log_path: &Path, max_disk_entries: usize) -> std::io::Result<()> {
    if !log_path.exists() {
        return Ok(());
    }

    let file = File::open(log_path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        if let Ok(l) = line {
            if !l.trim().is_empty() {
                lines.push(l);
            }
        }
    }

    // Ngưỡng vượt: > 100 dòng hoặc khi max_disk_entries nhỏ (để test xoay vòng nhạy hơn khi max < 100)
    let threshold = if max_disk_entries < 100 { 1 } else { 100 };
    if lines.len() > max_disk_entries + threshold || (lines.len() > max_disk_entries && threshold == 1) {
        let keep_from = lines.len().saturating_sub(max_disk_entries);
        let recent_lines = &lines[keep_from..];

        let mut out_file = File::create(log_path)?;
        for l in recent_lines {
            writeln!(out_file, "{}", l)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTrafficLog {
    pub id: String,
    pub timestamp: String,
    pub route_id: String,
    pub method: String,
    pub port: u16,
    pub path: String,
    pub target_url: String,
    pub tunnel_id: String,
    pub key_used_preview: Option<String>,
    
    // Status & Timing
    pub status_code: u16,
    pub duration_ms: u64,
    pub is_streaming: bool,

    // 100% Raw Wire Data
    pub raw_request_headers: Vec<(String, String)>,
    pub raw_forwarded_headers: Vec<(String, String)>,
    pub raw_request_body: String,
    pub raw_response_headers: Vec<(String, String)>,
    pub raw_response_body: String,

    // Forensic findings for privacy awareness
    pub leaked_findings: Vec<LeakFinding>,
}

pub struct RingBufferLog {
    max_entries: AtomicUsize,
    entries: RwLock<VecDeque<RawTrafficLog>>,
}

impl RingBufferLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: AtomicUsize::new(max_entries),
            entries: RwLock::new(VecDeque::with_capacity(max_entries)),
        }
    }

    pub fn push(&self, log: RawTrafficLog) {
        let mut queue = self.entries.write();
        if queue.len() >= self.max_entries.load(Ordering::Relaxed) {
            queue.pop_front(); // Tự động dọn dẹp ghi đè bản ghi cũ nhất
        }
        queue.push_back(log);
    }

    pub fn get_all(&self) -> Vec<RawTrafficLog> {
        let queue = self.entries.read();
        queue.iter().cloned().collect()
    }

    pub fn clear(&self) {
        let mut queue = self.entries.write();
        queue.clear();
    }

    /// Đổi giới hạn entries lúc runtime (khi user đổi max_log_entries trong Settings).
    /// Trước đây setting này là no-op cho tới khi restart app vì capacity chỉ set ở new().
    pub fn set_max_entries(&self, max_entries: usize) {
        self.max_entries.store(max_entries, Ordering::Relaxed);
        let mut queue = self.entries.write();
        while queue.len() > max_entries {
            queue.pop_front();
        }
    }

    pub fn count(&self) -> usize {
        self.entries.read().len()
    }

    pub fn update_response_body(&self, id: &str, body: String) {
        let mut queue = self.entries.write();
        if let Some(entry) = queue.iter_mut().find(|e| e.id == id) {
            entry.raw_response_body = body;
        }
    }
}
