//! Vị trí data: XDG chuẩn, tương thích setup cũ.
//!
//! Thực trạng cũ: mọi path (`config.json`, `./auths`, `./data/*`) đều tương đối
//! theo CWD — chạy app từ đâu là data rớt ở đó (desktop launcher → `$HOME`,
//! terminal → thư mục hiện tại), restart từ chỗ khác là "mất" hết setting.
//!
//! Quy tắc mới:
//! - Chuẩn: config → `$XDG_CONFIG_HOME/universal-api` (fallback
//!   `~/.config/universal-api`); data → `$XDG_DATA_HOME/universal-api`
//!   (fallback `~/.local/share/universal-api`).
//! - Legacy: nếu CWD đã có `config.json` (setup đang chạy, vd thư mục release)
//!   thì GIỮ NGUYÊN hành vi cũ (tương đối theo CWD) để không strand data.
//! - Env/abs path do user chỉ định rõ luôn thắng, không resolve lại.

use std::path::{Path, PathBuf};

const APP_DIR: &str = "universal-api";

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

/// `$XDG_CONFIG_HOME` hoặc `~/.config`. None khi không xác định được HOME.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
    {
        return Some(xdg.join(APP_DIR));
    }
    home_dir().map(|h| h.join(".config").join(APP_DIR))
}

/// `$XDG_DATA_HOME` hoặc `~/.local/share`. None khi không xác định được HOME.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
    {
        return Some(xdg.join(APP_DIR));
    }
    home_dir().map(|h| h.join(".local").join("share").join(APP_DIR))
}

/// Thư mục `~/.cockpit_tools` để tương thích hoàn toàn dữ liệu với Cockpit gốc.
pub fn cockpit_dir() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".cockpit_tools"))
}

/// True khi CWD có `config.json` → setup legacy đang hoạt động, giữ nguyên.
pub fn legacy_cwd_mode() -> bool {
    std::env::current_dir()
        .map(|cwd| cwd.join("config.json").is_file())
        .unwrap_or(false)
}

/// Base cho path tương đối: legacy → CWD; chuẩn → data dir (fallback CWD nếu
/// không xác định được HOME — không bao giờ trả None để caller khỏi branch).
pub fn data_base() -> PathBuf {
    if legacy_cwd_mode() {
        return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    }
    data_dir().unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    })
}

/// File config đọc/ghi: legacy → `./config.json`; chuẩn → XDG config dir.
pub fn config_file_path() -> PathBuf {
    if legacy_cwd_mode() {
        return PathBuf::from("config.json");
    }
    match config_dir() {
        Some(dir) => dir.join("config.json"),
        None => PathBuf::from("config.json"),
    }
}

/// Resolve path data/auth: absolute giữ nguyên; tương đối join vào base.
/// Bỏ tiền tố `./` và `data/` đầu để XDG không lồng `data/data/`.
pub fn resolve_data_path(base: &Path, raw: &str) -> PathBuf {
    let t = raw.trim();
    if t.is_empty() {
        return base.to_path_buf();
    }
    let p = Path::new(t);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    let mut rel = t.trim_start_matches("./");
    // Chỉ strip "data/" khi base KHÔNG phải CWD-legacy (tránh đổi nghĩa setup cũ).
    if !legacy_cwd_mode() {
        rel = rel.trim_start_matches("data/");
    }
    if rel.is_empty() {
        return base.to_path_buf();
    }
    base.join(rel)
}

/// Đảm bảo thư mục cha tồn tại (cho save config/settings/db).
pub fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {} failed: {e}", parent.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_keeps_absolute() {
        let base = Path::new("/x/y");
        assert_eq!(resolve_data_path(base, "/a/b.json"), Path::new("/a/b.json"));
    }

    #[test]
    fn resolve_strips_dot_slash() {
        let base = Path::new("/x/y");
        // Không phụ thuộc legacy flag: "./auths" luôn join thẳng.
        let p = resolve_data_path(base, "./auths");
        assert_eq!(p, base.join("auths"));
    }

    #[test]
    fn resolve_empty_goes_base() {
        let base = Path::new("/x/y");
        assert_eq!(resolve_data_path(base, "  "), base);
    }
}
