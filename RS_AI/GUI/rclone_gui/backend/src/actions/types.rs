//! S1 shared action types: `RemoteKind` + `DeleteScope` + `EmptyDirs`.
//!
//! Dùng chung cho copy/move/delete/perm/explorer/ops; mỗi module giữ `Cap`
//! riêng (năng lực khác nhau), chỉ dedup enum dùng chung này.

/// Phân loại remote đã parse (`"Local"` = ổ máy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteKind {
    Local,
    Remote,
}

impl RemoteKind {
    /// Phân loại từ tên remote đã parse.
    pub fn classify(remote: &str) -> Self {
        match remote {
            // UNIVERSAL: ổ máy đi qua syscall/pkexec; remote đi qua lệnh rclone.
            "Local" => Self::Local,
            _ => Self::Remote,
        }
    }
}

/// Phạm vi xóa — enum dùng chung cho copy/move/delete (S2 gộp CopyAndDelete).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteScope {
    /// Xóa vào thùng rác (khôi phục được): Local qua `gio trash`,
    /// remote qua `rclone delete` (backend có trash sẽ trash thay vì xóa hẳn).
    Trash,
    /// Xóa vĩnh viễn: `purge` cho thư mục, `deletefile` cho file.
    NoTrash,
}

/// Chính sách dọn thư mục rỗng còn lại sau `rclone delete` (S1 bổ sung cờ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmptyDirs {
    /// Giữ nguyên hành vi cũ: không dọn thư mục rỗng.
    #[default]
    Keep,
    /// Chỉ dọn đúng target: `rclone rmdir target`.
    OnlyHere,
    /// Dọn đệ quy cây rỗng: `rclone rmdirs target`.
    Recursive,
}
