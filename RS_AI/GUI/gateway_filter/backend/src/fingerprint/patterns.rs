use std::sync::LazyLock;

/// Regex phát hiện path local — biên dịch 1 lần, dùng chung mọi request
/// (trước đây `Regex::new()` chạy mỗi request: tốn CPU).
/// Đặt ở module patterns riêng để mọi analyzer/sanitizer dùng chung một chuẩn.
pub static LOCAL_PATH_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?:/Users/[a-zA-Z0-9_\-\.]+|/(?:home/[a-zA-Z0-9_\-\.]+|root|tmp|etc|var)(?:/[a-zA-Z0-9_\-\.]+)*|[A-Za-z]:\\\\(?:[^"\\\r\n]+\\\\)*[^"\\\r\n]+|[A-Za-z]:[/\\](?:[^"/\\\r\n]+[/\\])*[^"/\\\r\n]+)"#
    ).expect("LOCAL_PATH_REGEX must compile")
});

/// Che path local trong body trước khi forward upstream (`mask_local_paths_in_body`).
/// Trả `Some(masked)` khi có thay thế, `None` khi body sạch (caller giữ nguyên bytes gốc).
pub fn mask_local_paths(body: &str) -> Option<String> {
    let masked = LOCAL_PATH_REGEX.replace_all(body, "[REDACTED_PATH]");
    if masked == body {
        None
    } else {
        Some(masked.into_owned())
    }
}
