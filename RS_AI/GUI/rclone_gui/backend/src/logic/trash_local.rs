/*
[INTEGRITY NOTES]
- Mục đích: Thùng rác cục bộ theo chuẩn FreeDesktop.org Trash Specification.
- Trách nhiệm: Liệt kê / khôi phục / xoá vĩnh viễn từng mục trong `$XDG_DATA_HOME/Trash`.
- Tương tác: Gọi từ `api/trash.rs`. Dùng `gio` cho việc khôi phục (nó tự tạo lại
  thư mục gốc nếu đã bị xoá), còn liệt kê thì đọc thẳng metadata để không phụ
  thuộc định dạng đầu ra của `gio`.

Cấu trúc thùng rác chuẩn:
  $XDG_DATA_HOME/Trash/files/<name>            → nội dung thật
  $XDG_DATA_HOME/Trash/info/<name>.trashinfo   → metadata (ini)

Nội dung `.trashinfo`:
  [Trash Info]
  Path=/tmp/tt%20space/a%20b%23c%25d.txt   ← URL-encoded
  DeletionDate=2026-09-03T11:19:42
*/

use std::path::PathBuf;

use crate::api::trash::TrashItemLocal;

/// Trả về thư mục thùng rác theo chuẩn XDG.
fn trash_dir() -> Result<PathBuf, String> {
    if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
        if !data_home.is_empty() {
            return Ok(PathBuf::from(data_home).join("Trash"));
        }
    }
    let home = std::env::var("HOME").map_err(|_| "Không xác định được biến môi trường HOME".to_string())?;
    Ok(PathBuf::from(home).join(".local/share/Trash"))
}

/// Giải mã percent-encoding (`%20` → space) trong trường `Path=` của `.trashinfo`.
fn percent_decode(input: &str) -> String {
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
fn percent_encode(input: &str) -> String {
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

/// Tên hàm: list
/// Mô tả: Liệt kê toàn bộ mục trong thùng rác cục bộ, mới xoá xếp trước.
/// `id` là tên file trong `Trash/files/` — định danh duy nhất trong một thùng rác.
pub fn list() -> Result<Vec<TrashItemLocal>, String> {
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

/// Tên hàm: restore
/// Mô tả: Khôi phục một mục về vị trí gốc. Dùng `gio trash --restore` vì nó tự
/// tạo lại thư mục cha nếu đã bị xoá, và không ghi đè file đang tồn tại.
pub fn restore(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Thiếu định danh mục cần khôi phục.".to_string());
    }

    let uri = format!("trash:///{}", percent_encode(id));
    let output = std::process::Command::new("gio")
        .args(["trash", "--restore", &uri])
        .output()
        .map_err(|e| format!("Lỗi khi gọi gio: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("Khôi phục '{}' thất bại: {}", id, output.status)
        } else {
            err
        });
    }
    Ok(())
}

/// Tên hàm: delete
/// Mô tả: Xoá vĩnh viễn một mục khỏi thùng rác (bỏ cả nội dung và metadata).
pub fn delete(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Thiếu định danh mục cần xoá.".to_string());
    }
    // Chặn path traversal: `id` phải là một tên đơn, không chứa phân cách.
    if id.contains('/') || id.contains('\\') || id == "." || id == ".." {
        return Err(format!("Định danh không hợp lệ: '{}'", id));
    }

    let dir = trash_dir()?;
    let target = dir.join("files").join(id);
    let info = dir.join("info").join(format!("{}.trashinfo", id));

    if !target.exists() && !info.exists() {
        return Err(format!("Không tìm thấy '{}' trong thùng rác.", id));
    }

    if target.is_dir() {
        std::fs::remove_dir_all(&target).map_err(|e| format!("Lỗi xoá thư mục: {}", e))?;
    } else if target.exists() {
        std::fs::remove_file(&target).map_err(|e| format!("Lỗi xoá tệp: {}", e))?;
    }

    // Metadata mồ côi sẽ làm `list()` bỏ qua mục đó, nhưng vẫn nên dọn sạch.
    let _ = std::fs::remove_file(&info);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decode_handles_encoded_chars() {
        assert_eq!(percent_decode("/tmp/tt%20space/a%20b%23c%25d.txt"), "/tmp/tt space/a b#c%d.txt");
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
    fn delete_rejects_path_traversal() {
        assert!(delete("../../etc/passwd").is_err());
        assert!(delete("sub/file").is_err());
        assert!(delete("..").is_err());
        assert!(delete("").is_err());
    }
}
