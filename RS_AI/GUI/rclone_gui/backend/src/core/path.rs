//! UNIVERSAL S2: cắt chuỗi `Remote::/path` thành `(remote, path)`.
//! Tách nguyên văn từ `logic::file_ops::parse_remote_path` (giữ hành vi).

/// Tên hàm: cut_remote_path
/// Mô tả: Bóc tách chuỗi "GDrive::/Documents" thành remote ("GDrive") và đường dẫn ("/Documents").
pub fn cut_remote_path(full_path: &str) -> (String, String) {
    if let Some(idx) = full_path.find("::") {
        let remote = full_path[..idx].to_string();
        let path = full_path[idx + 2..].to_string();
        (remote, path)
    } else {
        ("Local".to_string(), full_path.to_string())
    }
}

/// UNIVERSAL: alias tương thích cho đường gọi `parse_remote_path` cũ.
#[deprecated(note = "dùng `cut_remote_path`")]
pub fn parse_remote_path(full_path: &str) -> (String, String) {
    cut_remote_path(full_path)
}

/// Giải mã percent-encoding (`%20` → space) trong trường `Path=` của `.trashinfo`.
/// Dời nguyên từ `actions::trash_list` (UNIVERSAL: thùng rác + `gio` dùng chung).
pub fn percent_decode(input: &str) -> String {
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
/// Dời nguyên từ `actions::trash_list` (UNIVERSAL: thùng rác + `gio` dùng chung).
pub fn percent_encode(input: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cut_remote_path_local() {
        let (remote, path) = cut_remote_path("/home/user/Documents");
        assert_eq!(remote, "Local");
        assert_eq!(path, "/home/user/Documents");
    }

    #[test]
    fn test_cut_remote_path_cloud() {
        let (remote, path) = cut_remote_path("GDrive::/Work/Project");
        assert_eq!(remote, "GDrive");
        assert_eq!(path, "/Work/Project");
    }

    #[test]
    fn test_cut_remote_path_empty() {
        let (remote, path) = cut_remote_path("");
        assert_eq!(remote, "Local");
        assert_eq!(path, "");
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
}
