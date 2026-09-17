use std::sync::LazyLock;

/// Regex phát hiện path local — biên dịch 1 lần, dùng chung mọi request
/// (trước đây `Regex::new()` chạy mỗi request: tốn CPU).
/// Đặt ở module patterns riêng để mọi analyzer/sanitizer dùng chung một chuẩn.
pub static LOCAL_PATH_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(/home/[a-zA-Z0-9_-]+|[A-Za-z]:\\[a-zA-Z0-9_\\]+)")
        .expect("LOCAL_PATH_REGEX must compile")
});
