/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_chmod`/`fs_chown` thành plan thuần.
- Trách nhiệm: guard Local-only + chuẩn hóa mode/spec + chọn sudo; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `logic::file_ops::parse_remote_path`.
  Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::types::RemoteKind;
use crate::logic::file_ops::parse_remote_path;

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
}
