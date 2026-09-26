/*
[INTEGRITY NOTES]
- Mục đích: THỰC THI `mkdir` (tạo thư mục).
- Trách nhiệm: `plan_mkdir` lập kế hoạch, `execute_mkdir` gọi cỗ máy `super::rclone_instant::execute_create`.
- Tương tác: Được gọi bởi `api::files_edit::fs_mkdir`.
*/

use super::rclone_instant::{CreateKind, CreatePlan, plan_create};
use crate::actions::perm::Policy;

/// Đặc tả thuần cho `mkdir`: target rclone + lệnh + sudo fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MkdirPlan {
    pub target: String,
    pub rclone_args: Vec<String>,
    pub sudo_action: Option<&'static str>,
}

impl From<CreatePlan> for MkdirPlan {
    fn from(p: CreatePlan) -> Self {
        Self {
            target: p.target,
            rclone_args: p.rclone_args,
            sudo_action: p.sudo_action,
        }
    }
}

/// Dựng plan `mkdir` từ đường dẫn `Remote::/Path`; lỗi khi đường dẫn rỗng.
pub fn plan_mkdir(path: &str) -> Result<MkdirPlan, String> {
    plan_create(CreateKind::Dir, path).map(Into::into)
}

/// Thực thi `mkdir` đồng bộ (dùng cho worker job hoặc gọi trực tiếp, không bọc fastlane).
pub fn execute_mkdir_sync(path: &str, policy: Policy) -> Result<(), String> {
    let plan = plan_create(CreateKind::Dir, path)?;
    super::rclone_instant::execute_create_sync(&plan, policy)
}

/// Thực thi `mkdir` (tương thích API async): bọc fastlane bảo vệ async runtime khỏi blocking.
pub async fn execute_mkdir(path: String, policy: Policy) -> Result<(), String> {
    crate::logic::fastlane::fastlane(move || execute_mkdir_sync(&path, policy)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mkdir_local_has_sudo_mkdir() {
        let p = plan_mkdir("/tmp/x").expect("plan");
        assert_eq!(p.rclone_args[0], "mkdir");
        assert_eq!(p.sudo_action, Some("mkdir"));
    }

    #[test]
    fn mkdir_remote_uses_rclone() {
        let p = plan_mkdir("GDrive::/folder").expect("plan");
        assert_eq!(p.rclone_args, vec!["mkdir", "GDrive:/folder"]);
        assert_eq!(p.sudo_action, None);
    }

    #[test]
    fn empty_path_returns_error() {
        assert!(plan_mkdir("").is_err());
        assert_eq!(plan_mkdir("").unwrap_err(), "Thiếu đường dẫn cần tạo.");
    }
}
