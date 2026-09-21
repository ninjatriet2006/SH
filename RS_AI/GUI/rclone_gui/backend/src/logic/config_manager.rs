//! S2 ConfigManager: quản lý `rclone.conf` + snapshot/remote đơn lẻ.
//!
//! UNIVERSAL: Local và remote cloud đều dùng chung một `rclone.conf` INI
//! (`[remote]` + `key = value`); module này giữ nguyên ngữ nghĩa INI đó,
//! chỉ thêm snapshot khôi phục và xuất/nhập từng remote cho mọi backend.

use crate::logic::fastlane::fastlane;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// UNIVERSAL: số bản snapshot tối đa giữ lại (cũ nhất bị xoá khi vượt N).
pub const MAX_SNAPSHOTS: usize = 10;

/// Quản lý một tệp `rclone.conf` cụ thể.
///
/// UNIVERSAL: mặc định (`new`) dò đường dẫn qua `rclone config file` như
/// hành vi cũ; `with_path` ghim đường dẫn để test không cần binary rclone.
#[derive(Debug, Clone, Default)]
pub struct ConfigManager {
    config_path_override: Option<PathBuf>,
}

impl ConfigManager {
    /// Dò đường dẫn thật qua `rclone` (hành vi cũ).
    pub fn new() -> Self {
        Self {
            config_path_override: None,
        }
    }

    /// Ghim đường dẫn (dùng cho test / gọi nội bộ).
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            config_path_override: Some(path),
        }
    }

    fn resolve_path(&self) -> Result<PathBuf, String> {
        if let Some(p) = self.config_path_override.clone() {
            return Ok(p);
        }
        get_rclone_config_path().map(PathBuf::from)
    }

    fn snapshots_dir_for(conf: &Path) -> PathBuf {
        conf.parent()
            .map(|p| p.join("rclone_snapshots"))
            .unwrap_or_else(|| PathBuf::from("rclone_snapshots"))
    }

    fn snapshots_dir(&self) -> Result<PathBuf, String> {
        Ok(Self::snapshots_dir_for(&self.resolve_path()?))
    }

    /// Đọc toàn bộ nội dung config.
    pub fn read(&self) -> Result<String, String> {
        let path = self.resolve_path()?;
        // UNIVERSAL: đọc thô UTF-8, giữ nguyên thứ tự section cho mọi backend.
        fs::read_to_string(&path).map_err(|e| format!("Lỗi đọc tệp {}: {}", path.display(), e))
    }

    /// Chụp bản sao theo giờ (UNIX giây) rồi tỉa còn [`MAX_SNAPSHOTS`].
    pub fn snapshot(&self) -> Result<PathBuf, String> {
        let path = self.resolve_path()?;
        let content = fs::read(&path)
            .map_err(|e| format!("Lỗi đọc tệp {} để snapshot: {}", path.display(), e))?;
        let dir = Self::snapshots_dir_for(&path);
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Lỗi tạo thư mục {}: {}", dir.display(), e))?;
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dest = dir.join(format!("rclone-{secs}.conf"));
        fs::write(&dest, content)
            .map_err(|e| format!("Lỗi ghi snapshot {}: {}", dest.display(), e))?;
        prune_snapshots(&dir)?;
        Ok(dest)
    }

    /// Liệt kê snapshot (tên tệp, cũ → mới).
    pub fn list_snapshots(&self) -> Result<Vec<String>, String> {
        let dir = self.snapshots_dir()?;
        list_snapshot_names(&dir)
    }

    /// Khôi phục một snapshot đè lên config hiện tại (snapshot trước khi ghi).
    pub fn restore_snapshot(&self, name: &str) -> Result<(), String> {
        let safe = checked_snapshot_name(name)?;
        let path = self.resolve_path()?;
        let dir = Self::snapshots_dir_for(&path);
        let src = dir.join(&safe);
        if !src.is_file() {
            return Err(format!("Không tìm thấy snapshot '{safe}'"));
        }
        // UNIVERSAL: giữ bản hiện tại trước khi lùi về bản cũ để không mất config.
        if path.is_file() {
            self.snapshot()?;
        }
        let content = fs::read(&src)
            .map_err(|e| format!("Lỗi đọc snapshot {}: {}", src.display(), e))?;
        write_conf_file(&path, content)
    }

    /// Ghi đè toàn bộ config (snapshot bản cũ trước khi ghi).
    pub fn write_with_snapshot(&self, content: String) -> Result<(), String> {
        let path = self.resolve_path()?;
        // UNIVERSAL: snapshot bản cũ trước mỗi lần set để lùi được khi ghi sai.
        if path.is_file() {
            self.snapshot()?;
        }
        write_conf_file(&path, content.into_bytes())
    }

    /// Sắp xếp lại thứ tự các remote theo danh sách tên.
    pub fn reorder(&self, names: Vec<String>) -> Result<(), String> {
        let content = self.read()?;
        let reordered = reorder_content(&content, &names);
        self.write_with_snapshot(reordered)
    }

    /// Xuất một remote thành đoạn INI (`[name]` + các dòng key=value).
    pub fn export_remote(&self, name: &str) -> Result<String, String> {
        extract_section(&self.read()?, name)
    }

    /// Thêm một remote từ đoạn INI; trùng tên thì lỗi rõ (không ghi đè).
    pub fn import_remote_merge(&self, name: &str, ini: &str) -> Result<(), String> {
        let merged = merge_section(&self.read().unwrap_or_default(), name, ini)?;
        self.write_with_snapshot(merged)
    }
}

/// Lấy đường dẫn `rclone.conf` qua `rclone config file` (hành vi cũ giữ nguyên).
fn get_rclone_config_path() -> Result<String, String> {
    let output = Command::new("rclone")
        .arg("config")
        .arg("file")
        .output()
        .map_err(|e| format!("Lỗi khi chạy lệnh rclone config file: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "Lệnh `rclone config file` thất bại ({}): {}",
            output.status, err
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let path = stdout
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("");

    if path.is_empty() {
        Err(format!("Không thể trích xuất đường dẫn từ output: {}", stdout))
    } else {
        Ok(path.to_string())
    }
}

fn write_conf_file(path: &Path, content: Vec<u8>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Lỗi tạo thư mục {}: {}", parent.display(), e))?;
        }
    }
    fs::write(path, content).map_err(|e| format!("Lỗi ghi tệp {}: {}", path.display(), e))
}

/// Chặn path-traversal khi chọn snapshot; chỉ cho tên tệp `.conf` đơn.
fn checked_snapshot_name(name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() || n.contains('/') || n.contains('\\') || n.contains("..") {
        return Err(format!("Tên snapshot không hợp lệ: '{name}'"));
    }
    if !n.ends_with(".conf") {
        return Err(format!("Tên snapshot phải kết thúc bằng .conf: '{name}'"));
    }
    Ok(n.to_string())
}

fn list_snapshot_names(dir: &Path) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    match fs::read_dir(dir) {
        Ok(entries) => {
            for e in entries {
                let e = e.map_err(|e| format!("Lỗi liệt kê {}: {}", dir.display(), e))?;
                let fname = e.file_name().to_string_lossy().to_string();
                if fname.ends_with(".conf") && e.path().is_file() {
                    names.push(fname);
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Lỗi liệt kê {}: {}", dir.display(), e)),
    }
    // UNIVERSAL: tên chứa timestamp tăng dần nên sort chuỗi = cũ → mới.
    names.sort();
    Ok(names)
}

/// Xoá bản cũ nhất khi vượt [`MAX_SNAPSHOTS`].
fn prune_snapshots(dir: &Path) -> Result<(), String> {
    let mut names = list_snapshot_names(dir)?;
    while names.len() > MAX_SNAPSHOTS {
        let oldest = names.remove(0);
        let p = dir.join(&oldest);
        fs::remove_file(&p).map_err(|e| format!("Lỗi tỉa snapshot {}: {}", p.display(), e))?;
    }
    Ok(())
}

/// Tách config INI thành (tên section, các dòng); phần đầu không tên giữ `None`.
///
/// UNIVERSAL: mọi backend rclone đều là section `[tên]` + dòng `key = value`,
/// nên parse dòng-thô (không chuẩn hoá) để giữ nguyên comment/khoảng trắng.
fn split_sections(content: &str) -> Vec<(Option<String>, Vec<String>)> {
    let mut sections: Vec<(Option<String>, Vec<String>)> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();
    let mut started = false;

    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') && t.len() >= 2 {
            if started || !current_lines.is_empty() || current_name.is_some() {
                sections.push((current_name.clone(), current_lines.clone()));
            }
            current_name = Some(t[1..t.len() - 1].trim().to_string());
            current_lines = vec![line.to_string()];
            started = true;
        } else {
            current_lines.push(line.to_string());
        }
    }
    if started || !current_lines.is_empty() || current_name.is_some() {
        sections.push((current_name, current_lines));
    }
    sections
}

fn render_sections(sections: &[(Option<String>, Vec<String>)]) -> String {
    let mut out = String::new();
    for (_, lines) in sections {
        out.push_str(&lines.join("\n"));
        out.push('\n');
    }
    out
}

/// Sắp xếp section theo `names`; preamble + phần thừa giữ nguyên vị trí tương đối.
fn reorder_content(content: &str, names: &[String]) -> String {
    let mut sections = split_sections(content);
    let mut ordered: Vec<(Option<String>, Vec<String>)> = Vec::new();
    if let Some(pos) = sections.iter().position(|s| s.0.is_none()) {
        ordered.push(sections.remove(pos));
    }
    for name in names {
        if let Some(pos) = sections
            .iter()
            .position(|s| s.0.as_deref() == Some(name.as_str()))
        {
            ordered.push(sections.remove(pos));
        }
    }
    ordered.extend(sections);
    render_sections(&ordered)
}

/// Lấy đúng đoạn `[name]` (kèm header); thiếu thì lỗi rõ tên remote.
fn extract_section(content: &str, name: &str) -> Result<String, String> {
    let want = name.trim();
    if want.is_empty() {
        return Err("Tên remote không được rỗng".to_string());
    }
    for (sec_name, lines) in split_sections(content) {
        if sec_name.as_deref() == Some(want) {
            let mut out = lines.join("\n");
            out.push('\n');
            return Ok(out);
        }
    }
    Err(format!("Không tìm thấy remote '{want}'"))
}

/// Nối thêm một remote mới; đã tồn tại → lỗi rõ, không ghi đè.
///
/// UNIVERSAL: `ini` chỉ là các dòng `key = value` (có/không kèm header của
/// chính remote đó); chứa header lạ → từ chối để tránh nhập nhầm remote khác.
fn merge_section(content: &str, name: &str, ini: &str) -> Result<String, String> {
    let want = name.trim();
    if want.is_empty() || want.contains('[') || want.contains(']') || want.contains('\n') {
        return Err(format!("Tên remote không hợp lệ: '{name}'"));
    }
    for (sec_name, _) in split_sections(content) {
        if sec_name.as_deref() == Some(want) {
            return Err(format!("Remote '{want}' đã tồn tại, không ghi đè"));
        }
    }
    let body = ini.trim();
    if body.is_empty() {
        return Err("Nội dung INI của remote không được rỗng".to_string());
    }
    for line in body.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            let inner = t[1..t.len() - 1].trim();
            if inner != want {
                return Err(format!(
                    "Đoạn INI chứa section lạ '{inner}', chỉ được nhập remote '{want}'"
                ));
            }
        }
    }
    // Bỏ header trùng tên nếu người dùng dán kèm `[name]`.
    let stripped: Vec<&str> = body
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.starts_with('[') && t.ends_with(']') && t[1..t.len() - 1].trim() == want)
        })
        .collect();
    if stripped.iter().all(|l| l.trim().is_empty()) {
        return Err("Nội dung INI của remote không được rỗng".to_string());
    }
    let mut out = content.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&format!("[{want}]\n"));
    out.push_str(&stripped.join("\n"));
    out.push('\n');
    Ok(out)
}

// ---- Tên lệnh IPC cũ giữ nguyên (đổi ruột sang ConfigManager) ----

/// IPC cũ: đọc toàn bộ config.
pub async fn get_config_content() -> Result<String, String> {
    let mgr = ConfigManager::new();
    fastlane(move || mgr.read()).await
}

/// IPC cũ: ghi đè config (tự snapshot bản cũ, giữ tối đa N=10).
pub async fn set_config_content(content: String) -> Result<(), String> {
    let mgr = ConfigManager::new();
    fastlane(move || mgr.write_with_snapshot(content)).await
}

/// IPC cũ: sắp xếp lại remote.
pub async fn reorder_config(names: Vec<String>) -> Result<(), String> {
    let mgr = ConfigManager::new();
    fastlane(move || mgr.reorder(names)).await
}

/// IPC mới: liệt kê snapshot (cũ → mới).
pub async fn list_config_snapshots() -> Result<Vec<String>, String> {
    let mgr = ConfigManager::new();
    fastlane(move || mgr.list_snapshots()).await
}

/// IPC mới: khôi phục snapshot.
pub async fn restore_config_snapshot(name: String) -> Result<(), String> {
    let mgr = ConfigManager::new();
    fastlane(move || mgr.restore_snapshot(&name)).await
}

/// IPC mới: xuất một remote thành đoạn INI.
pub async fn export_config_remote(name: String) -> Result<String, String> {
    let mgr = ConfigManager::new();
    fastlane(move || mgr.export_remote(&name)).await
}

/// IPC mới: nhập thêm một remote từ đoạn INI.
pub async fn import_config_remote(name: String, ini: String) -> Result<(), String> {
    let mgr = ConfigManager::new();
    fastlane(move || mgr.import_remote_merge(&name, &ini)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_conf(name: &str) -> PathBuf {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "rclone_cfg_test_{}_{}_{}.conf",
            name,
            std::process::id(),
            secs
        ))
    }

    #[test]
    fn reorder_keeps_preamble_and_unknown_tail() {
        // UNIVERSAL: preamble/comment + remote lạ backend vẫn giữ sau reorder.
        let content = "# comment\n[a]\ntype = s3\n[b]\ntype = drive\n";
        let out = reorder_content(content, &["b".to_string(), "a".to_string()]);
        assert!(out.find("[b]").unwrap() < out.find("[a]").unwrap());
        assert!(out.starts_with("# comment"));
    }

    #[test]
    fn export_missing_remote_errors_with_name() {
        // UNIVERSAL: thiếu remote phải báo đúng tên cho mọi backend.
        let err = extract_section("[a]\ntype = s3\n", "ghost").expect_err("must fail");
        assert!(err.contains("ghost"), "unexpected: {err}");
    }

    #[test]
    fn import_duplicate_name_fails_clearly() {
        // UNIVERSAL: nhập trùng tên không được ghi đè âm thầm.
        let err = merge_section("[a]\ntype = s3\n", "a", "type = drive")
            .expect_err("duplicate must fail");
        assert!(err.contains("'a'"), "unexpected: {err}");
    }

    #[test]
    fn import_appends_new_remote_section() {
        // UNIVERSAL: remote mới nối thêm, remote cũ giữ nguyên nội dung.
        let out = merge_section("[a]\ntype = s3\n", "b", "type = drive").expect("merge");
        assert!(out.contains("[a]") && out.contains("[b]") && out.contains("type = drive"));
    }

    #[test]
    fn snapshot_restore_roundtrip_with_temp_conf() {
        // UNIVERSAL: set → snapshot → sửa → restore phải về đúng bản cũ.
        let path = tmp_conf("roundtrip");
        let _ = fs::remove_file(&path);
        let mgr = ConfigManager::with_path(path.clone());
        mgr.write_with_snapshot("[a]\ntype = s3\n".to_string())
            .expect("write");
        mgr.write_with_snapshot("[a]\ntype = drive\n".to_string())
            .expect("rewrite");
        let snaps = mgr.list_snapshots().expect("list");
        assert!(!snaps.is_empty());
        mgr.restore_snapshot(&snaps[0]).expect("restore");
        let back = mgr.read().expect("read");
        assert!(back.contains("[a]"));
        let _ = fs::remove_file(&path);
        let dir = path.parent().unwrap().join("rclone_snapshots");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_snapshot_path_traversal() {
        let err = checked_snapshot_name("../evil.conf").expect_err("must fail");
        assert!(err.contains("không hợp lệ"));
    }
}
