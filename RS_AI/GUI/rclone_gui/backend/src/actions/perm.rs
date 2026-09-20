/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_chmod`/`fs_chown` thành plan thuần.
- Trách nhiệm: guard Local-only + chuẩn hóa mode/spec + chọn sudo; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `logic::file_ops::parse_remote_path`.
  Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::types::RemoteKind;
use crate::logic::file_ops::parse_remote_path;
use serde::{Deserialize, Serialize};

/// S2: chính sách leo thang quyền do người dùng chốt qua dialog consent.
/// - `Deny`: không sudo, trả `PERMISSION_CONSENT` để frontend park task.
/// - `AskOnce`: giống `Deny` nhưng dialog hỏi lại mỗi lần (mặc định).
/// - `AllowSystem`: giữ hành vi cũ — tự `pkexec` khi dính lỗi quyền Local.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    Deny,
    #[default]
    AskOnce,
    AllowSystem,
}

impl Policy {
    /// UNIVERSAL: parse chuỗi snake_case từ IPC (`deny`/`ask_once`/`allow_system`).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "deny" => Some(Policy::Deny),
            "ask_once" => Some(Policy::AskOnce),
            "allow_system" => Some(Policy::AllowSystem),
            _ => None,
        }
    }

    /// UNIVERSAL: serialize về snake_case cho IPC get.
    pub fn as_str(self) -> &'static str {
        match self {
            Policy::Deny => "deny",
            Policy::AskOnce => "ask_once",
            Policy::AllowSystem => "allow_system",
        }
    }
}

/// S2: nhận diện lỗi thiếu quyền (rclone/os báo `permission denied`).
/// UNIVERSAL: so chữ thường + chấp nhận cả `access is denied` (Windows),
/// `operation not permitted` và `quyền` (thông điệp Việt).
pub fn classify_permission_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("permission denied")
        || lower.contains("access is denied")
        || lower.contains("operation not permitted")
        || lower.contains("not permitted")
        || lower.contains("quyền")
        || lower.contains("forbidden")
}

/// S2: thay `run_with_sudo_fallback` — chạy `attempt` một lần, chỉ leo thang
/// khi `classify_permission_error` đúng + remote Local.
/// - `Deny`/`AskOnce`: trả `PERMISSION_CONSENT: <gốc>` để frontend park + hỏi.
/// - `AllowSystem`: giữ hành vi cũ — thử lại qua `pkexec <action> <args>`.
pub fn escalate<F>(policy: Policy, remote: &str, action: &str, args: &[String], attempt: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    let result = attempt();
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            if remote != "Local" || !classify_permission_error(&e) {
                return Err(e);
            }
            match policy {
                // UNIVERSAL: chưa consent — trả marker để frontend park task + hiện dialog.
                Policy::Deny | Policy::AskOnce => Err(format!("PERMISSION_CONSENT: {}.", e)),
                // UNIVERSAL: đã consent hệ thống — hành vi cũ, tự `pkexec`.
                Policy::AllowSystem => run_pkexec(action, args),
            }
        }
    }
}

/// UNIVERSAL: thực thi `pkexec <action> <args>` trên Linux; giữ nguyên
/// mapping action cũ (`rm -rf`, `mkdir -p`, `mv`, `cp -r`, `chmod`...).
fn run_pkexec(action: &str, args: &[String]) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let mut cmd_args = Vec::new();
        match action {
            "rm" => {
                cmd_args.push("rm".to_string());
                cmd_args.push("-rf".to_string());
            }
            "mkdir" => {
                cmd_args.push("mkdir".to_string());
                cmd_args.push("-p".to_string());
            }
            "mv" => {
                cmd_args.push("mv".to_string());
            }
            "cp" => {
                cmd_args.push("cp".to_string());
                cmd_args.push("-r".to_string());
            }
            "chmod" => {
                cmd_args.push("chmod".to_string());
            }
            "chown" => {
                cmd_args.push("chown".to_string());
            }
            "write" => {
                return Err("Không đủ quyền ghi tệp này. Hãy đổi quyền hoặc chọn vị trí khác.".into());
            }
            _ => return Err("Hành động sudo không được hỗ trợ".into()),
        }
        for arg in args {
            cmd_args.push(arg.clone());
        }
        let output = Command::new("pkexec")
            .args(&cmd_args)
            .output()
            .map_err(|e| format!("Lỗi gọi pkexec: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).into_owned();
            if err.is_empty() {
                return Err("Thao tác pkexec bị huỷ hoặc lỗi phân quyền.".into());
            }
            return Err(err);
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (action, args);
        Err("Leo thang quyền hệ thống chỉ được hỗ trợ trên Linux.".to_string())
    }
}

/// Năng lực gợi ý cho từng loại remote (tài liệu; chưa đổi cờ hệ thống).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap {
    /// Có cho phép đổi quyền/chủ hay không (remote luôn từ chối).
    pub allowed: bool,
    /// Có cần sudo (`pkexec chmod/chown`) hay không.
    pub sudo_fallback: bool,
}

impl Cap {
    /// Năng lực gợi ý cho từng loại remote.
    pub fn of(kind: RemoteKind) -> Self {
        match kind {
            // UNIVERSAL: Local đổi mode qua syscall, rớt quyền thì `pkexec chmod`;
            // chown gần như luôn cần root nên đi thẳng `pkexec chown`.
            RemoteKind::Local => Cap {
                allowed: true,
                sudo_fallback: true,
            },
            // UNIVERSAL: remote cloud không có khái niệm mode/uid/gid POSIX.
            RemoteKind::Remote => Cap {
                allowed: false,
                sudo_fallback: false,
            },
        }
    }
}

/// Đặc tả thuần cho `chmod`: mode đã che + chuỗi octal + sudo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChmodPlan {
    pub real_path: String,
    pub safe_mode: u32,
    pub octal: String,
    pub sudo_action: &'static str,
}

/// Đặc tả thuần cho `chown`: spec `uid:gid` + sudo `pkexec chown`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChownPlan {
    pub real_path: String,
    pub spec: String,
    pub sudo_action: &'static str,
}

/// Dựng plan `chmod`; remote → lỗi Local-only. Chỉ giữ 12 bit quyền.
pub fn plan_chmod(path: &str, mode: u32) -> Result<ChmodPlan, String> {
    let (remote, real) = parse_remote_path(path);
    let kind = RemoteKind::classify(&remote);
    let cap = Cap::of(kind);
    if !cap.allowed {
        return Err(format!(
            "Không thể đổi quyền trên remote '{}' — chỉ hỗ trợ ổ Local.",
            remote
        ));
    }
    match kind {
        // UNIVERSAL: Local che 12 bit (gồm setuid/setgid/sticky), không ghi đè bit loại file.
        RemoteKind::Local => {
            if real.is_empty() {
                return Err("Thiếu đường dẫn cần đổi quyền.".to_string());
            }
            let safe_mode = mode & 0o7777;
            let octal = format!("{:o}", safe_mode);
            Ok(ChmodPlan {
                real_path: real,
                safe_mode,
                octal,
                sudo_action: "chmod",
            })
        }
        // UNIVERSAL: remote không có mode POSIX nên từ chối rõ ràng.
        RemoteKind::Remote => Err(format!(
            "Không thể đổi quyền trên remote '{}' — chỉ hỗ trợ ổ Local.",
            remote
        )),
    }
}

/// Dựng plan `chown`; remote → lỗi Local-only.
pub fn plan_chown(path: &str, uid: u32, gid: u32) -> Result<ChownPlan, String> {
    let (remote, real) = parse_remote_path(path);
    let kind = RemoteKind::classify(&remote);
    let cap = Cap::of(kind);
    if !cap.allowed {
        return Err(format!(
            "Không thể đổi chủ sở hữu trên remote '{}' — chỉ hỗ trợ ổ Local.",
            remote
        ));
    }
    match kind {
        // UNIVERSAL: Local đổi chủ qua `pkexec chown uid:gid` (luôn cần root).
        RemoteKind::Local => {
            if real.is_empty() {
                return Err("Thiếu đường dẫn cần đổi chủ sở hữu.".to_string());
            }
            Ok(ChownPlan {
                real_path: real,
                spec: format!("{}:{}", uid, gid),
                sudo_action: "chown",
            })
        }
        // UNIVERSAL: remote không có uid/gid nên từ chối rõ ràng.
        RemoteKind::Remote => Err(format!(
            "Không thể đổi chủ sở hữu trên remote '{}' — chỉ hỗ trợ ổ Local.",
            remote
        )),
    }
}

/// Thực thi `chmod`: chạy [`plan_chmod`] + syscall `set_permissions` + sudo fallback.
/// UNIVERSAL: `AllowSystem` giữ hành vi `fs_chmod` cũ (tự `pkexec chmod`);
/// `Deny`/`AskOnce` trả `PERMISSION_CONSENT` để frontend park + hỏi.
pub async fn execute_chmod(path: String, mode: u32, policy: Policy) -> Result<(), String> {
    let plan = plan_chmod(&path, mode)?;
    crate::core::task::blocking(move || {
        #[cfg(unix)]
        {
            escalate(policy, "Local", "chmod", &[plan.octal.clone(), plan.real_path.clone()], || {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&plan.real_path, std::fs::Permissions::from_mode(plan.safe_mode))
                    .map_err(|e| e.to_string())
            })
        }
        #[cfg(not(unix))]
        {
            let _ = policy;
            Err("Đổi quyền chỉ được hỗ trợ trên hệ điều hành Unix.".to_string())
        }
    })
    .await
}

/// Thực thi `chown`: chạy [`plan_chown`] + `pkexec chown uid:gid`.
/// UNIVERSAL: giữ nguyên `fs_chown_with_policy` cũ — chưa consent thì park ngay
/// (`PERMISSION_CONSENT`), không chạm pkexec; `AllowSystem` đi thẳng pkexec.
pub async fn execute_chown(path: String, uid: u32, gid: u32, policy: Policy) -> Result<(), String> {
    let plan = plan_chown(&path, uid, gid)?;
    // UNIVERSAL: chown luôn cần root — chưa consent thì park ngay, không chạm pkexec.
    if policy != Policy::AllowSystem {
        return Err(format!(
            "PERMISSION_CONSENT: đổi chủ sở hữu '{}' cần quyền root.",
            path
        ));
    }
    crate::core::task::blocking(move || {
        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("pkexec")
                .args(["chown", &plan.spec, &plan.real_path])
                .output()
                .map_err(|e| format!("Lỗi gọi pkexec: {}", e))?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Err(if err.is_empty() {
                    "Thao tác pkexec bị huỷ hoặc lỗi phân quyền.".to_string()
                } else {
                    err
                });
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err("Đổi chủ sở hữu chỉ được hỗ trợ trên Linux.".to_string())
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chmod_masks_to_12_bits() {
        let p = plan_chmod("Local::/tmp/a", 0o17_755).expect("plan");
        assert_eq!(p.safe_mode, 0o7755);
    }

    #[test]
    fn remote_chmod_chown_rejected() {
        assert!(plan_chmod("GDrive::/a", 0o644).is_err());
        assert!(plan_chown("GDrive::/a", 1000, 1000).is_err());
    }

    #[test]
    fn policy_roundtrip_snake_case() {
        assert_eq!(Policy::parse("deny"), Some(Policy::Deny));
        assert_eq!(Policy::parse("ask_once"), Some(Policy::AskOnce));
        assert_eq!(Policy::parse("allow_system"), Some(Policy::AllowSystem));
        assert!(Policy::parse("root").is_none());
        assert_eq!(Policy::default(), Policy::AskOnce);
    }

    #[test]
    fn classify_detects_permission_errors() {
        assert!(classify_permission_error("permission denied (os error 13)"));
        assert!(classify_permission_error("Access is denied"));
        assert!(classify_permission_error("Không đủ quyền ghi"));
        assert!(!classify_permission_error("directory not found"));
    }

    #[test]
    fn escalate_parks_when_not_consented() {
        for policy in [Policy::Deny, Policy::AskOnce] {
            let err =
                escalate(policy, "Local", "chmod", &[], || Err("permission denied".into())).expect_err("park");
            assert!(err.starts_with("PERMISSION_CONSENT"), "got: {err}");
        }
    }

    #[test]
    fn escalate_passes_through_non_permission_errors() {
        let err = escalate(Policy::Deny, "Local", "chmod", &[], || Err("directory not found".into()))
            .expect_err("passthrough");
        assert!(!err.contains("PERMISSION_CONSENT"));
    }
}
