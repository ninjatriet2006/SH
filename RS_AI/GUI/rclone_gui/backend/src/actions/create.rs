/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_mkdir`/`fs_touch` (họ tạo) thành plan thuần.
- Trách nhiệm: parse → build_target → chọn sudo fallback; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `logic::file_ops::parse_remote_path`,
  `core::rclone_caller::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::ops::Cap;
use crate::actions::perm::{Policy, classify_permission_error, escalate};
use crate::actions::types::RemoteKind;
use crate::core::{rclone_caller, task};
use crate::core::rclone_caller::build_target;
use crate::logic::file_ops::parse_remote_path;

/// Đặc tả thuần cho `mkdir`: target rclone + lệnh + sudo fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MkdirPlan {
    pub target: String,
    pub rclone_args: Vec<String>,
    pub sudo_action: Option<&'static str>,
}

/// Đặc tả thuần cho `touch`: target rclone + lệnh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TouchPlan {
    pub target: String,
    pub rclone_args: Vec<String>,
    pub local_create: bool,
}

/// Dựng plan `mkdir` từ đường dẫn `Remote::/Path`; lỗi khi đường dẫn rỗng.
pub fn plan_mkdir(path: &str) -> Result<MkdirPlan, String> {
    let (remote, real) = parse_remote_path(path);
    if real.is_empty() {
        return Err("Thiếu đường dẫn cần tạo.".to_string());
    }
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    let target = build_target(&remote, &real);
    let rclone_args = vec!["mkdir".to_string(), target.clone()];
    let sudo_action = match kind {
        // UNIVERSAL: Local `mkdir -p` qua pkexec khi thiếu quyền.
        RemoteKind::Local => Some("mkdir"),
        // UNIVERSAL: remote `mkdir` thẳng, không sudo.
        RemoteKind::Remote => None,
    };
    Ok(MkdirPlan {
        target,
        rclone_args,
        sudo_action,
    })
}

/// Dựng plan `touch` từ đường dẫn `Remote::/Path`; lỗi khi đường dẫn rỗng.
pub fn plan_touch(path: &str) -> Result<TouchPlan, String> {
    let (remote, real) = parse_remote_path(path);
    if real.is_empty() {
        return Err("Thiếu đường dẫn cần tạo tệp.".to_string());
    }
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    let target = build_target(&remote, &real);
    let (rclone_args, local_create) = match kind {
        // UNIVERSAL: Local tạo file qua `File::create`, giữ `touch` làm tài liệu.
        RemoteKind::Local => (vec!["touch".to_string(), target.clone()], true),
        // UNIVERSAL: remote tạo file rỗng qua `rclone touch`.
        RemoteKind::Remote => (vec!["touch".to_string(), target.clone()], false),
    };
    Ok(TouchPlan {
        target,
        rclone_args,
        local_create,
    })
}

/// Thực thi `mkdir`: chạy [`plan_mkdir`] + `rclone mkdir` + sudo fallback.
/// UNIVERSAL: `AllowSystem` giữ hành vi `fs_mkdir` cũ (tự `pkexec mkdir -p`);
/// `Deny`/`AskOnce` trả `PERMISSION_CONSENT` để frontend park + hỏi.
pub async fn execute_mkdir(path: String, policy: Policy) -> Result<(), String> {
    let plan = plan_mkdir(&path)?;
    let (remote, real_path) = parse_remote_path(&path);
    let target = plan.target.clone();
    task::blocking(move || {
        escalate(policy, &remote, "mkdir", std::slice::from_ref(&real_path), || {
            let output = rclone_caller::run_cmd(&["mkdir", &target])?;
            if !output.status.success() {
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            } else {
                Ok(())
            }
        })
    })
    .await
}

/// Thực thi `touch`: chạy [`plan_touch`] + tạo file.
/// UNIVERSAL: Local qua `File::create`, remote qua `rclone touch`;
/// lỗi quyền khi chưa consent trả `PERMISSION_CONSENT` (cũ không sudo nên không pkexec).
pub async fn execute_touch(path: String, policy: Policy) -> Result<(), String> {
    let plan = plan_touch(&path)?;
    task::blocking(move || {
        if plan.local_create {
            std::fs::File::create(&plan.target).map(|_| ()).map_err(|e| {
                let msg = e.to_string();
                // UNIVERSAL: chưa consent thì park, không tự leo thang (bản cũ không sudo).
                if classify_permission_error(&msg) && policy != Policy::AllowSystem {
                    format!("PERMISSION_CONSENT: {}.", msg)
                } else {
                    msg
                }
            })
        } else {
            rclone_caller::spawn_cmd(&["touch", &plan.target])
        }
    })
    .await
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
}
