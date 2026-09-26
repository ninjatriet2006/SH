/*
[INTEGRITY NOTES]
- Mục đích: Đo dung lượng remote/hệ thống qua `rclone about --json`.
- Trách nhiệm:
  + `AboutPlan` & `plan_about`: Lập kế hoạch thuần túy (chuẩn hóa target, route, kiểm tra hỗ trợ cờ `About`).
  + `execute_about` / `about`: Thực thi gọi rclone, ghi log chẩn đoán qua `core::debug`.
- Tương tác: Được gọi bởi `api::remote_manager`, không đụng frontend.
*/

use crate::actions::types::RemoteKind;
use serde_json::Value;

/// Kế hoạch đo dung lượng thuần túy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutPlan {
    pub remote: String,
    pub route: RemoteKind,
    pub target: String,
    pub allowed: bool,
}

/// Kiểm tra xem remote có hỗ trợ cờ About hay không qua cache feature flags.
pub fn about_allowed(flags: Option<&crate::actions::feature::getfeature::BackendFeatures>) -> bool {
    flags.map(|f| f.about).unwrap_or(true)
}

/// Chuẩn hoá target và lập kế hoạch đo dung lượng thuần túy.
pub fn plan_about(input: &str) -> Result<AboutPlan, String> {
    let info = super::resolve_target(input)?;
    let cached = crate::actions::feature::checkcap::backend_features_cached(&info.remote);
    let allowed = about_allowed(cached.as_ref());

    Ok(AboutPlan {
        remote: info.remote,
        route: info.route,
        target: info.target,
        allowed,
    })
}

/// Đo dung lượng hệ thống/remote qua `rclone about --json`.
pub fn execute_about(input: &str) -> Result<Value, String> {
    let plan = plan_about(input)?;
    if !plan.allowed {
        return Err(format!("{} không hỗ trợ xem dung lượng (About).", plan.remote));
    }
    super::run_json_query("about", &plan.target)
}

/// Alias tương thích.
pub fn about(remote: &str) -> Result<Value, String> {
    execute_about(remote)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rclone_present() -> bool {
        crate::core::rclone_caller::run_cmd(&["version"])
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn seed_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rclone_gui_about_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("gieo thu muc");
        std::fs::write(dir.join("a.txt"), b"12345").expect("gieo file");
        dir
    }

    #[test]
    fn plan_about_resolves_local_keyword_to_root() {
        let plan = plan_about("Local").expect("plan");
        assert_eq!(plan.route, RemoteKind::Local);
        assert_eq!(plan.target, "/");

        let plan_colon = plan_about("Local:").expect("plan");
        assert_eq!(plan_colon.target, "/");
    }

    #[test]
    fn plan_about_resolves_local_path() {
        let plan = plan_about("Local::/home/user").expect("plan");
        assert_eq!(plan.route, RemoteKind::Local);
        assert_eq!(plan.target, "/home/user");

        let plan_raw = plan_about("/var/log").expect("plan");
        assert_eq!(plan_raw.route, RemoteKind::Local);
        assert_eq!(plan_raw.target, "/var/log");
    }

    #[test]
    fn plan_about_expands_tilde() {
        let plan = plan_about("~/MyDocs").expect("plan");
        assert_eq!(plan.route, RemoteKind::Local);
        assert!(!plan.target.starts_with('~'));
        assert!(plan.target.ends_with("/MyDocs"));

        let plan_colon = plan_about("Local::~/MyDocs").expect("plan");
        assert_eq!(plan_colon.route, RemoteKind::Local);
        assert!(!plan_colon.target.starts_with('~'));
        assert!(plan_colon.target.ends_with("/MyDocs"));
    }

    #[test]
    fn plan_about_resolves_cloud_remote_colon() {
        let plan = plan_about("GDrive").expect("plan");
        assert_eq!(plan.route, RemoteKind::Remote);
        assert_eq!(plan.target, "GDrive:");

        let plan2 = plan_about("GDrive:").expect("plan");
        assert_eq!(plan2.target, "GDrive:");

        let plan3 = plan_about("GDrive::/Docs").expect("plan");
        assert_eq!(plan3.target, "GDrive:/Docs");
    }

    #[test]
    fn plan_about_empty_returns_error() {
        assert!(plan_about("").is_err());
        assert!(plan_about("   ").is_err());
    }

    #[test]
    fn about_allowed_decides_from_flag_or_unknown() {
        use crate::actions::feature::getfeature::parse_feature_flags;
        use serde_json::json;
        assert!(about_allowed(None));
        let no_about = parse_feature_flags(&json!({"About": false}));
        assert!(!about_allowed(Some(&no_about)));
        let yes_about = parse_feature_flags(&json!({"About": true}));
        assert!(about_allowed(Some(&yes_about)));
    }

    #[test]
    fn live_about_local_reports_disk_numbers() {
        if !rclone_present() {
            return;
        }
        let dir = seed_dir("test");
        let v = about(&dir.to_string_lossy()).expect("about thu muc gieo");
        let total = v.get("total").and_then(|x| x.as_u64()).unwrap_or(0);
        let used = v.get("used").and_then(|x| x.as_u64()).unwrap_or(0);
        assert!(total > 0, "total phải dương");
        assert!(used <= total, "used không vượt total");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
