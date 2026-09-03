/*
[INTEGRITY NOTES]
- Mục đích: Thùng rác phía remote (Cloud) thông qua rclone.
- Trách nhiệm: Xác định remote có hỗ trợ xem thùng rác không, liệt kê, khôi phục,
  xoá vĩnh viễn từng mục và dọn sạch toàn bộ.
- Tương tác: Gọi từ `api/trash.rs`. Dùng `core::rclone`.

Giới hạn của rclone (đã kiểm chứng bằng `rclone help flags` / `backend help`):
  * Chỉ 3 backend có cờ xem thùng rác: `drive`, `jottacloud`, `pikpak`
    (`--<backend>-trashed-only`).
  * Chỉ `drive` có lệnh khôi phục: `rclone backend untrash`.
  * `rclone cleanup` (dọn sạch) chỉ hoạt động khi backend có tính năng `CleanUp`.
Với remote ngoài danh sách trên, ta trả lỗi rõ ràng thay vì im lặng trả rỗng.
*/

use serde_json::Value;

use crate::api::files::FileItem;
use crate::core::rclone;

/// Backend hỗ trợ liệt kê thùng rác, kèm cờ tương ứng.
fn trashed_only_flag(backend_type: &str) -> Option<&'static str> {
    match backend_type {
        "drive" => Some("--drive-trashed-only"),
        "jottacloud" => Some("--jottacloud-trashed-only"),
        "pikpak" => Some("--pikpak-trashed-only"),
        _ => None,
    }
}

/// Tra `type` của một remote từ `rclone config dump`.
fn remote_type(remote: &str) -> Result<String, String> {
    let output = rclone::run_cmd(&["config", "dump"])?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let dump: Value = serde_json::from_slice(&output.stdout).map_err(|e| format!("Lỗi đọc cấu hình rclone: {}", e))?;

    dump.get(remote)
        .and_then(|v| v.get("type"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Không tìm thấy remote '{}' trong cấu hình rclone.", remote))
}

/// Tên hàm: list
/// Mô tả: Liệt kê các mục đang ở trong thùng rác của remote.
/// `FileItem.uuid` giữ đường dẫn tương đối để các thao tác sau định vị chính xác
/// (không dùng chỉ số mảng như bản cũ — dễ lệch khi danh sách thay đổi).
pub fn list(remote: &str) -> Result<Vec<FileItem>, String> {
    let backend = remote_type(remote)?;
    let flag = trashed_only_flag(&backend).ok_or_else(|| {
        format!(
            "rclone không hỗ trợ xem thùng rác cho loại '{}'. Chỉ Google Drive, Jottacloud và PikPak có tính năng này.",
            backend
        )
    })?;

    let target = format!("{}:", remote);
    let output = rclone::run_cmd(&["lsjson", &target, "--max-depth", "1", flag])?;
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

/// Tên hàm: restore
/// Mô tả: Khôi phục một mục khỏi thùng rác. Chỉ Google Drive hỗ trợ (`backend untrash`).
pub fn restore(remote: &str, path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Thiếu đường dẫn mục cần khôi phục.".to_string());
    }

    let backend = remote_type(remote)?;
    if backend != "drive" {
        return Err(format!(
            "rclone không hỗ trợ khôi phục từ thùng rác cho loại '{}'. Hiện chỉ Google Drive làm được (rclone backend untrash).",
            backend
        ));
    }

    let target = format!("{}:{}", remote, path);
    let output = rclone::run_cmd(&["backend", "untrash", &target])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Khôi phục '{}' thất bại: {}", path, output.status)
        } else {
            err
        });
    }

    // `untrash` báo số mục đã xử lý; 0 nghĩa là không tìm thấy gì trong thùng rác.
    if let Ok(res) = serde_json::from_slice::<Value>(&output.stdout) {
        if res.get("Untrashed").and_then(|v| v.as_u64()) == Some(0) {
            return Err(format!("Không tìm thấy '{}' trong thùng rác để khôi phục.", path));
        }
    }
    Ok(())
}

/// Tên hàm: delete
/// Mô tả: Xoá vĩnh viễn một mục đang ở trong thùng rác.
///
/// Cần cờ `--<backend>-trashed-only` để rclone nhắm vào bản trong thùng rác chứ
/// không phải file cùng tên đang ở ngoài — thiếu cờ này sẽ xoá nhầm file đang dùng.
pub fn delete(remote: &str, path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("Thiếu đường dẫn mục cần xoá.".to_string());
    }

    let backend = remote_type(remote)?;
    let flag = trashed_only_flag(&backend).ok_or_else(|| {
        format!(
            "rclone không hỗ trợ xoá từng mục trong thùng rác cho loại '{}'. Hãy dùng 'Dọn sạch thùng rác'.",
            backend
        )
    })?;

    let target = format!("{}:{}", remote, path);
    let is_dir = rclone::is_dir(&target).unwrap_or(false);
    let cmd = if is_dir { "purge" } else { "deletefile" };

    let output = rclone::run_cmd(&[cmd, &target, flag])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Xoá vĩnh viễn '{}' thất bại: {}", path, output.status)
        } else {
            err
        });
    }
    Ok(())
}

/// Tên hàm: empty
/// Mô tả: Dọn sạch toàn bộ thùng rác của remote bằng `rclone cleanup`.
/// Kiểm tra trước tính năng `CleanUp` để báo lỗi rõ ràng thay vì thất bại mơ hồ.
pub fn empty(remote: &str) -> Result<(), String> {
    let target = format!("{}:", remote);

    if let Ok(output) = rclone::run_cmd(&["backend", "features", &target]) {
        if output.status.success() {
            if let Ok(v) = serde_json::from_slice::<Value>(&output.stdout) {
                let can_cleanup = v
                    .get("Features")
                    .and_then(|f| f.get("CleanUp"))
                    .and_then(|b| b.as_bool())
                    .unwrap_or(true);
                if !can_cleanup {
                    return Err(format!("Remote '{}' không hỗ trợ dọn sạch thùng rác (CleanUp).", remote));
                }
            }
        }
    }

    let output = rclone::run_cmd(&["cleanup", &target])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Dọn sạch thùng rác '{}' thất bại: {}", remote, output.status)
        } else {
            err
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn operations_reject_empty_path() {
        assert!(restore("AnyRemote", "").is_err());
        assert!(delete("AnyRemote", "").is_err());
    }
}
