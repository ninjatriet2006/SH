/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_rename` (`moveto` cùng remote) thành plan thuần.
- Trách nhiệm: phân tuyến Route (dùng chung checkcap::Route) × IsDir → chọn SupportRename
  từ cờ backend Move/DirMove → dựng `moveto` + sudo fallback; ghi log qua `core::debug`.
- Tương tác: Gọi `crate::actions::checkcap::Route`, `core::path::cut_remote_path`,
  `core::rclone_caller::build_target`, `logic::fastlane`, `actions::perm::escalate`, `core::debug`.
*/

use crate::actions::perm::{Policy, escalate};
use crate::actions::types::RemoteKind;
use crate::core::rclone_caller;
use crate::logic::fastlane;
use crate::core::rclone_caller::build_target;
use crate::core::path::cut_remote_path;

/// Tuyến đổi tên dùng chung từ `feature::checkcap` (1 não duy nhất, khỏi song sinh).
pub use crate::actions::checkcap::Route;

/// Phân biệt file/dir để chọn cờ backend `Move` hay `DirMove`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsDir {
    File,
    Dir,
}

/// UNIVERSAL: rename-native 1 bước — backend có `Move` (file) hoặc `DirMove` (dir)
/// nên `rclone moveto` đổi tên tại chỗ, không cần tải lại dữ liệu.
/// Vỏ bool cho `plan_rename_for`; suy từ cờ thật bằng `combinefeature::move_splitter`
/// (rename chính là moveto cùng remote — không tự suy tại đây để khỏi song sinh).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportRename(pub bool);

/// Năng lực gợi ý cho từng tuyến đổi tên.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap {
    /// Có thể server-side (không tải qua máy local) hay không.
    pub server_side: bool,
    /// Có cần sudo fallback (`pkexec mv`) khi lỗi quyền hay không.
    pub sudo_fallback: bool,
    /// Backend hỗ trợ rename-native (`Move` cho file / `DirMove` cho dir).
    pub support_rename: bool,
}

impl Cap {
    /// Suy năng lực đổi tên từ tuyến Route và cờ hỗ trợ rename.
    pub fn for_route(route: Route, support: SupportRename) -> Self {
        Cap {
            server_side: matches!(route, Route::SameCloud),
            sudo_fallback: matches!(route, Route::LocalLocal),
            support_rename: support.0,
        }
    }
}

/// Đặc tả thuần cho `rename` (`moveto` cùng remote).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePlan {
    pub src_target: String,
    pub dst_target: String,
    pub rclone_args: Vec<String>,
    pub sudo_action: Option<&'static str>,
}

/// Dựng plan `rename` đầy đủ theo tuyến × loại nguồn × cờ backend.
/// Cross-remote (Route khác cùng-remote) → lỗi rõ để `move` xử lý.
pub fn plan_rename_for(
    old_path: &str,
    new_path: &str,
    is_dir: IsDir,
    support: SupportRename,
) -> Result<RenamePlan, String> {
    let (src_remote, src_real) = cut_remote_path(old_path);
    let (dst_remote, dst_real) = cut_remote_path(new_path);
    if src_real.is_empty() || dst_real.is_empty() {
        return Err("Thiếu đường dẫn nguồn hoặc đích khi đổi tên.".to_string());
    }
    let route = Route::classify(&src_remote, &dst_remote);
    let kind = RemoteKind::classify(&src_remote);
    let cap = Cap::for_route(route, support);
    if !cap.support_rename {
        let what = match is_dir {
            // UNIVERSAL: file cần backend `Move`.
            IsDir::File => "Move",
            // UNIVERSAL: thư mục cần backend `DirMove`.
            IsDir::Dir => "DirMove",
        };
        return Err(format!("Backend không hỗ trợ đổi tên (thiếu {}).", what));
    }
    match route {
        // UNIVERSAL: cross-remote phải đi qua `move` (re-upload), rename từ chối rõ.
        Route::LocalCloud | Route::CloudLocal | Route::DiffCloud => {
            return Err("Đổi tên chỉ hỗ trợ cùng remote; khác remote hãy dùng move.".to_string());
        }
        // UNIVERSAL: cùng remote (kể cả Local↔Local) đổi tên tại chỗ qua `moveto`.
        Route::LocalLocal | Route::SameCloud => {}
    }
    let src_target = build_target(&src_remote, &src_real);
    let dst_target = build_target(&dst_remote, &dst_real);
    let rclone_args = vec![
        "moveto".to_string(),
        src_target.clone(),
        dst_target.clone(),
    ];
    let sudo_action = match kind {
        // UNIVERSAL: Local `moveto` rớt quyền thì thử `pkexec mv`.
        RemoteKind::Local => Some("mv"),
        // UNIVERSAL: remote `moveto` server-side, không sudo.
        RemoteKind::Remote => None,
    };
    Ok(RenamePlan {
        src_target,
        dst_target,
        rclone_args,
        sudo_action,
    })
}

/// Dựng plan `rename` (`moveto`) cùng remote; khác remote → lỗi để `move` xử lý.
/// Mặc định file + backend hỗ trợ rename (giữ hành vi S1 cũ).
pub fn plan_rename(old_path: &str, new_path: &str) -> Result<RenamePlan, String> {
    plan_rename_for(old_path, new_path, IsDir::File, SupportRename(true))
}

/// Thực thi `rename`: chạy [`plan_rename`] + `rclone moveto` + sudo fallback.
/// UNIVERSAL: `AllowSystem` giữ hành vi `fs_rename` cũ (tự `pkexec mv`);
/// `Deny`/`AskOnce` trả `PERMISSION_CONSENT` để frontend park + hỏi.
pub async fn execute_rename(old_path: String, new_path: String, policy: Policy) -> Result<(), String> {
    let plan = plan_rename(&old_path, &new_path)?;
    let (remote, old_real) = cut_remote_path(&old_path);
    let (_, new_real) = cut_remote_path(&new_path);
    let (src, dst) = (plan.src_target.clone(), plan.dst_target.clone());

    crate::core::debug::info(
        None,
        "actions/instant/rename",
        format!("BẮT ĐẦU Rename | '{}' -> '{}'", src, dst),
    );
    let start = std::time::Instant::now();

    let res = fastlane::fastlane(move || {
        escalate(policy, &remote, "mv", &[old_real.clone(), new_real.clone()], || {
            let output = rclone_caller::run_cmd(&["moveto", &src, &dst])?;
            if !output.status.success() {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            } else {
                Ok(())
            }
        })
    })
    .await;

    match &res {
        Ok(()) => {
            crate::core::debug::info(
                None,
                "actions/instant/rename",
                format!("XONG Rename | '{}' -> '{}' ({:.2?})", plan.src_target, plan.dst_target, start.elapsed()),
            );
        }
        Err(e) => {
            crate::core::debug::error(
                None,
                "actions/instant/rename",
                format!("LỖI Rename | '{}' -> '{}' | err={} ({:.2?})", plan.src_target, plan.dst_target, e, start.elapsed()),
            );
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_cross_remote_rejected() {
        assert!(plan_rename("Local::/a", "GDrive::/a").is_err());
    }

    #[test]
    fn rename_dir_needs_dirmove_flag() {
        // UNIVERSAL: đáp án rename-native suy bằng `combinefeature::move_splitter`
        // (không tự suy tại đây để khỏi song sinh bảng chân lý).
        use crate::actions::feature::combinefeature::move_splitter;
        use crate::actions::feature::getfeature::parse_feature_flags;
        use serde_json::json;
        let no_dirmove = parse_feature_flags(&json!({"Move": true, "DirMove": false}));
        assert!(plan_rename_for(
            "GDrive::/a",
            "GDrive::/b",
            IsDir::Dir,
            SupportRename(move_splitter(&no_dirmove, true)),
        )
        .is_err());
        let full = parse_feature_flags(&json!({"Move": true, "DirMove": true}));
        assert!(plan_rename_for(
            "GDrive::/a",
            "GDrive::/b",
            IsDir::Dir,
            SupportRename(move_splitter(&full, true)),
        )
        .is_ok());
    }
}
