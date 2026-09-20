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

/// UNIVERSAL: cùng hãng provider — hai remote khác tên nhưng cùng `type`
/// trong `rclone config dump` (vd: hai `drive`) nên rclone có thể đi
/// `--server-side-across-configs`; `Local` luôn `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SameProvider(pub bool);

/// UNIVERSAL: suy cùng-hãng từ JSON `rclone config dump` đã parse (thuần để test).
pub fn same_provider_from_dump(src: &str, dst: &str, dump: &serde_json::Value) -> bool {
    match (src == "Local", dst == "Local", src == dst) {
        // UNIVERSAL: dính ổ máy hoặc cùng tên remote — không phải xuyên-config.
        (true, _, _) | (_, true, _) | (_, _, true) => false,
        // UNIVERSAL: khác tên — cùng hãng khi cả hai có `type` giống nhau.
        (false, false, false) => match (
            dump.get(src).and_then(|v| v.get("type")).and_then(|v| v.as_str()),
            dump.get(dst).and_then(|v| v.get("type")).and_then(|v| v.as_str()),
        ) {
            // UNIVERSAL: cùng type (vd: drive/drive) → server-side xuyên-config được.
            (Some(a), Some(b)) => a == b,
            // UNIVERSAL: thiếu type / khác type → trung chuyển qua local như cũ.
            _ => false,
        },
    }
}

/// UNIVERSAL: đọc `type` hai remote qua `rclone config dump`; lỗi / thiếu type
/// trả `false` để giữ hành vi cũ (trung chuyển qua local).
pub fn same_provider(src: &str, dst: &str) -> bool {
    match (src == "Local", dst == "Local", src == dst) {
        // UNIVERSAL: dính ổ máy hoặc cùng tên — không cần gọi rclone.
        (true, _, _) | (_, true, _) | (_, _, true) => false,
        (false, false, false) => {
            let output = match crate::core::rclone::run_cmd(&["config", "dump"]) {
                Ok(o) => o,
                // UNIVERSAL: rclone lỗi — giữ hành vi cũ, coi như khác hãng.
                Err(_) => return false,
            };
            if !output.status.success() {
                return false;
            }
            match serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                Ok(dump) => same_provider_from_dump(src, dst, &dump),
                // UNIVERSAL: JSON hỏng — giữ hành vi cũ.
                Err(_) => false,
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn same_provider_true_for_same_type_diff_name() {
        // UNIVERSAL: khác tên + cùng type drive → xuyên-config được.
        let dump = json!({"A": {"type": "drive"}, "B": {"type": "drive"}});
        assert!(same_provider_from_dump("A", "B", &dump));
        assert!(SameProvider(same_provider_from_dump("A", "B", &dump)).0);
    }

    #[test]
    fn same_provider_false_for_diff_type_or_local() {
        // UNIVERSAL: khác type → giữ hành vi cũ (trung chuyển qua local).
        let dump = json!({"A": {"type": "drive"}, "B": {"type": "s3"}});
        assert!(!same_provider_from_dump("A", "B", &dump));
        // UNIVERSAL: dính Local / cùng tên / thiếu remote → false.
        assert!(!same_provider_from_dump("Local", "A", &dump));
        assert!(!same_provider_from_dump("A", "A", &dump));
        assert!(!same_provider_from_dump("A", "Missing", &dump));
    }
}
