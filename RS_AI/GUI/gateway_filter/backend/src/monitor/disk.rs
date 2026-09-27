use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::types::RawTrafficLog;

/// Đường dẫn thư mục logs mặc định theo chế độ Portable:
/// Ưu tiên thư mục cha của current_exe() / logs, fallback về ./logs
pub fn get_logs_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GATEWAY_FILTER_CONFIG_DIR") {
        return PathBuf::from(dir).join("logs");
    }
    if let Ok(dir) = std::env::var("VPN_AI_PROXY_CONFIG_DIR") {
        return PathBuf::from(dir).join("logs");
    }
    if let Some(home) = std::env::var_os("HOME") {
        let path = PathBuf::from(home)
            .join(".config")
            .join("gateway_filter")
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
///
/// Đây là hàm SYNC (blocking file I/O): caller trên async path PHẢI dùng
/// `spawn_disk_append` bên dưới (worker riêng trên blocking pool) thay vì gọi trực tiếp.
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

/// Worker riêng cho request path: đẩy toàn bộ I/O ghi đĩa xuống tokio blocking pool
/// (fire-and-forget), đảm bảo KHÔNG BAO GIỜ block worker thread async của HTTP handler.
pub fn spawn_disk_append(log_path: PathBuf, log: RawTrafficLog, max_disk_entries: usize) {
    tokio::task::spawn_blocking(move || {
        let _ = append_disk_log(&log_path, &log, max_disk_entries);
    });
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

/// Cập nhật trường raw_response_body của một request ID cụ thể trong file traffic_log.jsonl.
/// Dùng cho SSE Streaming khi stream kết thúc, đảm bảo log trên đĩa không bị kẹt ở trạng thái live flow.
pub fn update_disk_log_response_body(log_path: &Path, req_id: &str, final_body: &str) -> std::io::Result<()> {
    if !log_path.exists() {
        return Ok(());
    }

    let file = File::open(log_path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    let mut updated = false;

    for line in reader.lines() {
        if let Ok(l) = line {
            if !l.trim().is_empty() {
                if !updated && l.contains(req_id) {
                    if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&l) {
                        if val.get("id").and_then(|v| v.as_str()) == Some(req_id) {
                            if let Some(map) = val.as_object_mut() {
                                map.insert("raw_response_body".to_string(), serde_json::json!(final_body));
                                if let Ok(new_l) = serde_json::to_string(&val) {
                                    lines.push(new_l);
                                    updated = true;
                                    continue;
                                }
                            }
                        }
                    }
                }
                lines.push(l);
            }
        }
    }

    if updated {
        let mut out_file = File::create(log_path)?;
        for l in lines {
            writeln!(out_file, "{}", l)?;
        }
    }

    Ok(())
}

/// Đẩy tác vụ cập nhật disk log xuống tokio blocking pool để không block async handler
pub fn spawn_disk_update_response_body(log_path: PathBuf, req_id: String, final_body: String) {
    tokio::task::spawn_blocking(move || {
        let _ = update_disk_log_response_body(&log_path, &req_id, &final_body);
    });
}
