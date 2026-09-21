/*
[INTEGRITY NOTES]
- Mục đích: Liệt kê thùng rác Local (chuẩn FreeDesktop) + Remote (rclone) (S1 unify).
- Trách nhiệm: Phân tuyến (Route) → chọn nhánh Local/Remote bằng `match` + UNIVERSAL.
- Tương tác: Tầng `api::trash` bọc mỏng qua `logic::fastlane::fastlane`. Không đụng IPC/frontend.
  Helpers dùng chung (`trash_dir`, `percent_*`, `remote_type`, `trashed_only_flag`,
  `list_local_inner`) để `trash_restore`/`trash_delete` tái sử dụng.
*/

use crate::api::files::FileItem;
use crate::api::trash::TrashItemLocal;
use crate::core::rclone_caller;
use serde_json::Value;

/// Tuyến thùng rác, suy từ tên remote (`"Local"` = ổ máy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Local,
    Remote,
}

impl Route {
    /// Phân tuyến từ tên remote đã parse.
    pub fn classify(remote: &str) -> Self {
        // UNIVERSAL: ổ máy đi qua thùng rác FreeDesktop (`Trash/files` + `gio`); remote qua rclone.
        if remote == "Local" {
            Self::Local
        } else {
            Self::Remote
        }
    }
}

/// Trả về thư mục thùng rác theo chuẩn XDG.
pub(crate) fn trash_dir() -> Result<std::path::PathBuf, String> {
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        if !data_home.is_empty() {
            return Ok(std::path::PathBuf::from(data_home).join("Trash"));
        }
    }
    let home =
        std::env::var("HOME").map_err(|_| "Không xác định được biến môi trường HOME".to_string())?;
    Ok(std::path::PathBuf::from(home).join(".local/share/Trash"))
}

/// Giải mã percent-encoding (`%20` → space) trong trường `Path=` của `.trashinfo`.
pub(crate) fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Mã hoá percent-encoding cho URI `trash:///<name>` truyền vào `gio`.
/// Chỉ giữ nguyên ký tự an toàn (unreserved theo RFC 3986), còn lại escape hết.
pub(crate) fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~' | '/') {
            out.push(c);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// Backend hỗ trợ liệt kê thùng rác, kèm cờ tương ứng.
pub(crate) fn trashed_only_flag(backend_type: &str) -> Option<&'static str> {
    match backend_type {
        "drive" => Some("--drive-trashed-only"),
        "jottacloud" => Some("--jottacloud-trashed-only"),
        "pikpak" => Some("--pikpak-trashed-only"),
        _ => None,
    }
}

/// Tra `type` của một remote từ `rclone config dump`.
pub(crate) fn remote_type(remote: &str) -> Result<String, String> {
    let output = rclone_caller::run_cmd(&["config", "dump"])?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let dump: Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("Lỗi đọc cấu hình rclone: {}", e))?;

    dump.get(remote)
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Không tìm thấy remote '{}' trong cấu hình rclone.", remote))
}

/// Liệt kê toàn bộ mục trong thùng rác cục bộ, mới xoá xếp trước.
pub(crate) fn list_local_inner() -> Result<Vec<TrashItemLocal>, String> {
    let dir = trash_dir()?;
    let info_dir = dir.join("info");
    let files_dir = dir.join("files");

    // Thùng rác chưa từng được dùng → chưa có thư mục, coi như rỗng.
    let entries = match std::fs::read_dir(&info_dir) {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let info_path = entry.path();
        if info_path.extension().and_then(|s| s.to_str()) != Some("trashinfo") {
            continue;
        }

        // "a b.txt.trashinfo" → id = "a b.txt"
        let id = match info_path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        // Bỏ qua metadata mồ côi (không còn nội dung thật) để UI không hiện mục ảo.
        if !files_dir.join(&id).exists() {
            continue;
        }

        let content = std::fs::read_to_string(&info_path).unwrap_or_default();
        let mut original_path = String::new();
        let mut time_deleted = String::new();
        for line in content.lines() {
            if let Some(v) = line.strip_prefix("Path=") {
                original_path = percent_decode(v.trim());
            } else if let Some(v) = line.strip_prefix("DeletionDate=") {
                time_deleted = v.trim().to_string();
            }
        }

        let name = std::path::Path::new(&original_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&id)
            .to_string();

        items.push(TrashItemLocal {
            id,
            name,
            original_path,
            time_deleted,
        });
    }

    // Mới xoá lên đầu (DeletionDate dạng ISO nên so sánh chuỗi là đủ).
    items.sort_by(|a, b| b.time_deleted.cmp(&a.time_deleted));
    Ok(items)
}

/// Liệt kê các mục đang ở trong thùng rác của remote.
/// `FileItem.uuid` giữ đường dẫn tương đối để các thao tác sau định vị chính xác.
fn list_remote_inner(remote: &str) -> Result<Vec<FileItem>, String> {
    let backend = remote_type(remote)?;
    let flag = trashed_only_flag(&backend).ok_or_else(|| {
        format!(
            "rclone không hỗ trợ xem thùng rác cho loại '{}'. Chỉ Google Drive, Jottacloud và PikPak có tính năng này.",
            backend
        )
    })?;

    let target = format!("{}:", remote);
    let output = rclone_caller::run_cmd(&["lsjson", &target, "--max-depth", "1", flag])?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let items: Vec<Value> =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("Lỗi phân tích JSON thùng rác: {}", e))?;

    let mut files: Vec<FileItem> = items
        .into_iter()
        .map(|item| {
            let path = item["Path"].as_str().unwrap_or("").to_string();
            FileItem {
                uuid: path.clone(),
                name: item["Name"].as_str().unwrap_or(&path).to_string(),
                size: item["Size"].as_i64().unwrap_or(0),
                is_dir: item["IsDir"].as_bool().unwrap_or(false),
                mod_time: item["ModTime"].as_str().unwrap_or("").to_string(),
                file_type: item["MimeType"].as_str().filter(|s| !s.is_empty()).map(String::from),
            }
        })
        .collect();

    files.sort_by(|a, b| match (b.is_dir, a.is_dir) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(files)
}

/// Liệt kê thùng rác cục bộ (đồng bộ; tầng api bọc `blocking`).
pub fn list_local() -> Result<Vec<TrashItemLocal>, String> {
    match Route::Local {
        // UNIVERSAL: Local đọc metadata `.trashinfo` thẳng, không qua `gio`.
        Route::Local => list_local_inner(),
        // UNIVERSAL: nhánh Remote không xảy ra ở hàm local — giữ để `match` đủ đầy.
        Route::Remote => Err("Tuyến Remote phải dùng `list_remote`.".to_string()),
    }
}

/// Liệt kê thùng rác remote (đồng bộ; tầng api bọc `blocking`).
pub fn list_remote(remote: &str) -> Result<Vec<FileItem>, String> {
    match Route::classify(remote) {
        // UNIVERSAL: classifier đã loại `"Local"` ở tầng api nên nhánh này là lỗi lập trình.
        Route::Local => Err("Tuyến Local phải dùng `list_local`.".to_string()),
        // UNIVERSAL: remote liệt kê qua `lsjson` + cờ `--<backend>-trashed-only`.
        Route::Remote => list_remote_inner(remote),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_classifies_local_vs_remote() {
        assert_eq!(Route::classify("Local"), Route::Local);
        assert_eq!(Route::classify("GDrive"), Route::Remote);
    }

    #[test]
    fn percent_decode_handles_encoded_chars() {
        assert_eq!(
            percent_decode("/tmp/tt%20space/a%20b%23c%25d.txt"),
            "/tmp/tt space/a b#c%d.txt"
        );
        assert_eq!(percent_decode("/plain/path.txt"), "/plain/path.txt");
        // `%` đứng cuối không đủ 2 chữ số hex → giữ nguyên, không panic.
        assert_eq!(percent_decode("abc%"), "abc%");
        assert_eq!(percent_decode("abc%zz"), "abc%zz");
    }

    #[test]
    fn percent_encode_escapes_unsafe_chars() {
        assert_eq!(percent_encode("a b.txt"), "a%20b.txt");
        assert_eq!(percent_encode("a#b%c.txt"), "a%23b%25c.txt");
        assert_eq!(percent_encode("plain-file_1.txt"), "plain-file_1.txt");
    }

    #[test]
    fn encode_decode_roundtrip() {
        for name in ["a b.txt", "tên tiếng Việt.txt", "a#b%c&d.txt", "normal.txt"] {
            assert_eq!(percent_decode(&percent_encode(name)), name);
        }
    }

    #[test]
    fn trashed_only_flag_covers_supported_backends() {
        assert_eq!(trashed_only_flag("drive"), Some("--drive-trashed-only"));
        assert_eq!(trashed_only_flag("jottacloud"), Some("--jottacloud-trashed-only"));
        assert_eq!(trashed_only_flag("pikpak"), Some("--pikpak-trashed-only"));
        // Các backend phổ biến khác không có khái niệm "xem thùng rác" trong rclone.
        assert_eq!(trashed_only_flag("dropbox"), None);
        assert_eq!(trashed_only_flag("onedrive"), None);
        assert_eq!(trashed_only_flag("s3"), None);
    }
}
