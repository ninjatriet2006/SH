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
}
