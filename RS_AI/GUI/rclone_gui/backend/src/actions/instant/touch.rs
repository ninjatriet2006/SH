/*
[INTEGRITY NOTES]
- Mục đích: THỰC THI `touch` (tạo tệp rỗng).
- Trách nhiệm: `plan_touch` lập kế hoạch, `execute_touch` gọi cỗ máy `super::rclone_instant::execute_create`.
- Tương tác: Được gọi bởi `api::files_edit::fs_touch`.
*/

use super::rclone_instant::{CreateKind, CreatePlan, plan_create};
use crate::actions::perm::Policy;

/// Đặc tả thuần cho `touch`: target rclone + lệnh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TouchPlan {
    pub target: String,
    pub rclone_args: Vec<String>,
    pub local_create: bool,
}

impl From<CreatePlan> for TouchPlan {
    fn from(p: CreatePlan) -> Self {
        Self {
            target: p.target,
            rclone_args: p.rclone_args,
            local_create: p.local_create,
        }
    }
}

/// Dựng plan `touch` từ đường dẫn `Remote::/Path`; lỗi khi đường dẫn rỗng.
pub fn plan_touch(path: &str) -> Result<TouchPlan, String> {
    plan_create(CreateKind::File, path).map(Into::into)
}

/// Thực thi `touch` đồng bộ (dùng cho worker job hoặc gọi trực tiếp, không bọc fastlane).
pub fn execute_touch_sync(path: &str, policy: Policy) -> Result<(), String> {
    let plan = plan_create(CreateKind::File, path)?;
    super::rclone_instant::execute_create_sync(&plan, policy)
}

/// Thực thi `touch` (tương thích API async): chuyển giao thực thi đồng bộ [`execute_touch_sync`].
/// UNIVERSAL: Local qua `File::create`, remote qua `rclone touch`;
/// lỗi quyền khi chưa consent trả `PERMISSION_CONSENT`.
pub async fn execute_touch(path: String, policy: Policy) -> Result<(), String> {
    execute_touch_sync(&path, policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_local_is_syscall() {
        let p = plan_touch("Local::/tmp/test.txt").expect("plan");
        assert!(p.local_create);
        assert_eq!(p.rclone_args, vec!["touch", "/tmp/test.txt"]);
    }

    #[test]
    fn touch_remote_uses_rclone() {
        let p = plan_touch("GDrive::/file.txt").expect("plan");
        assert!(!p.local_create);
        assert_eq!(p.rclone_args, vec!["touch", "GDrive:/file.txt"]);
    }

    #[test]
    fn empty_path_returns_error() {
        assert!(plan_touch("").is_err());
        assert_eq!(plan_touch("").unwrap_err(), "Thiếu đường dẫn cần tạo tệp.");
    }
}
