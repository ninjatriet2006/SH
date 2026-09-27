//! Sổ chẩn đoán backend (module `core::debug`, file `backend.log`).
//!
//! Khác với log chuyển file của tracker (dòng `--use-json-log` của rclone → % tiến
//! độ trong `logic::tracker`): module này chỉ ghi chẩn đoán nội bộ backend.
//!
//! Giữ `eprintln!` để log vẫn thấy khi chạy dev; file append nằm trong thư
//! mục config của app (`backend.log`). Frontend đọc qua API `get_backend_log`.
//! Tuyệt đối không truyền thẳng/emit event lên frontend.

use serde::Serialize;
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Mức log dùng chung cho backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    /// Chuỗi ngắn dùng cho dòng log text.
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return PathBuf::from(dir).join("rclone_gui");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("rclone_gui")
}

/// Đường dẫn file log (`<config_dir>/backend.log`).
pub fn log_file_path() -> PathBuf {
    config_dir().join("backend.log")
}

/// UNIVERSAL: đọc toàn bộ `backend.log` cho DebugView xem LỊCH SỬ.
/// Kích thước có trần nhờ cơ chế xoay vòng. Thiếu file → chuỗi rỗng
/// (chưa có log, không phải lỗi).
pub fn read_log() -> Result<String, String> {
    let path = log_file_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(format!("Lỗi đọc nhật ký {}: {}", path.display(), e)),
    }
}

/// UNIVERSAL: dọn rác log ở RANH GIỚI job — nếu `backend.log` vượt `max_bytes`
/// thì đổi tên thành `backend.log.old` (đè đời cũ, giữ đúng 2 đời) rồi để lần
/// ghi kế tạo file mới. Gọi sau khi một job kết thúc để nhóm log không bị xé lẻ.
/// Mọi lỗi IO đều bỏ qua (dọn log không được phép làm hỏng luồng chính).
pub fn rotate_if_oversized(max_bytes: u64) {
    rotate_path_if_oversized(&log_file_path(), max_bytes);
}

/// UNIVERSAL: lõi xoay vòng theo đường dẫn cho sẵn (tách khỏi `log_file_path`
/// để test trên file tạm, không đụng `backend.log` thật). Mọi lỗi IO bỏ qua.
fn rotate_path_if_oversized(path: &std::path::Path, max_bytes: u64) {
    let Ok(meta) = std::fs::metadata(path) else {
        return; // Chưa có file hoặc không đọc được → không cần xoay.
    };
    if meta.len() <= max_bytes {
        return;
    }
    let old = path.with_extension("log.old");
    let _ = std::fs::rename(path, &old);
}

/// Ghi một dòng log: file append (best-effort) + `eprintln!`.
pub fn log(level: Level, tag: &str, message: impl AsRef<str>) {
    let msg = message.as_ref();
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{timestamp}][{}][{}] {}", level, tag, msg);
    eprintln!("{line}");
    // File append: mọi lỗi đều bỏ qua để log không bao giờ làm crash app.
    let path = log_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{line}");
    }
}

/// Xoá toàn bộ nội dung file log (dành cho nút Xoá Log trên giao diện).
pub fn clear_log() -> Result<(), String> {
    let path = log_file_path();
    if path.exists() {
        std::fs::write(&path, b"").map_err(|e| format!("Lỗi xoá nhật ký {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Shortcut cho [`log`] với [`Level::Info`].
pub fn info(tag: &str, message: impl AsRef<str>) {
    log(Level::Info, tag, message);
}

/// Shortcut cho [`log`] với [`Level::Warn`].
pub fn warn(tag: &str, message: impl AsRef<str>) {
    log(Level::Warn, tag, message);
}

/// Shortcut cho [`log`] với [`Level::Error`].
pub fn error(tag: &str, message: impl AsRef<str>) {
    log(Level::Error, tag, message);
}

/// UNIVERSAL: định dạng thời lượng gọn cho dòng nhật ký vòng đời job:
/// dưới 1 giây ghi mili-giây (`820ms`), từ 1 giây ghi giây một chữ số (`4.2s`).
pub fn human_elapsed(d: std::time::Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{}ms", d.as_millis())
    } else {
        format!("{:.1}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_labels_match_wire_format() {
        assert_eq!(Level::Info.as_str(), "INFO");
        assert_eq!(Level::Warn.as_str(), "WARN");
        assert_eq!(Level::Error.as_str(), "ERROR");
        assert_eq!(Level::Warn.to_string(), "WARN");
    }

    #[test]
    fn log_file_lives_in_app_config_dir() {
        assert_eq!(log_file_path().file_name().and_then(|n| n.to_str()), Some("backend.log"));
    }

    #[test]
    fn log_without_handle_does_not_panic() {
        // Ghi file + eprintln, không emit event.
        log(Level::Info, "test", "hello");
    }

    #[test]
    fn human_elapsed_switches_unit_at_one_second() {
        // UNIVERSAL: dưới 1s ghi ms, từ 1s ghi giây 1 chữ số.
        assert_eq!(human_elapsed(std::time::Duration::from_millis(820)), "820ms");
        assert_eq!(human_elapsed(std::time::Duration::from_millis(4200)), "4.2s");
    }

    #[test]
    fn rotate_only_when_over_threshold() {
        // UNIVERSAL: test trên file tạm riêng (không đụng backend.log thật).
        let base = std::env::temp_dir().join(format!("rclone_gui_rotate_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&base);
        let path = base.join("backend.log");
        let old = path.with_extension("log.old");
        let _ = std::fs::remove_file(&old);
        std::fs::write(&path, b"0123456789").expect("seed log");
        // Ngưỡng lớn hơn kích thước → không xoay.
        rotate_path_if_oversized(&path, 1_000);
        assert!(path.is_file(), "dưới ngưỡng phải giữ nguyên");
        // Ngưỡng nhỏ hơn kích thước → xoay sang .old.
        rotate_path_if_oversized(&path, 5);
        assert!(old.is_file(), "vượt ngưỡng phải tạo .old");
        assert!(!path.is_file(), "vượt ngưỡng thì .log đã đổi tên");
        let _ = std::fs::remove_dir_all(&base);
    }
}
