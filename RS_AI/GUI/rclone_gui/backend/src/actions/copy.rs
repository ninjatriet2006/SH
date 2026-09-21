/*
[INTEGRITY NOTES]
- Mục đích: Route + Cap cho copy + THỰC THI copy (nơi duy nhất gọi rclone copy).
- Trách nhiệm: Phân tuyến (Route) → chọn Cap; `execute_copy` NHẬN VÉ ĐÃ ĐÓNG DẤU
  (`TransferTicket`: src/dst/rel/mode/across/engine_flags/policy — không đọc
  engine settings), tự ráp lệnh + hứng log + tôn trọng cờ hủy + `perm::escalate`.
- Tương tác: `logic::queue` gọi `execute_copy` với vé + `should_cancel` +
  sink dòng thô (`on_log_line`); streaming core dùng chung cho `move_op`.
  Không AppHandle/State (worker sync, test sync).
*/
// UNIVERSAL: actions tay trắng về % — chỉ GỌI rclone (ráp lệnh + phun dòng
// log THÔ vào sink) + hủy êm qua `logic::process::terminate_gracefully` +
// Local↔Local qua `perm::escalate`. Tính % là việc DUY NHẤT của tracker,
// queue hỏi tracker rồi ghi vào Job.

use crate::actions::types::{DeleteScope, SameProvider};
use crate::logic::tracker::{TransferMode, TransferTicket};

/// Tuyến sao chép, suy từ cặp (src_remote, dst_remote).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    LocalLocal,
    LocalCloud,
    CloudLocal,
    SameCloud,
    DiffCloud,
}

/// Loại transfer mà module này phục vụ (giữ chỗ để S2 gộp copy/move chung).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Copy,
}

/// Năng lực gợi ý cho từng tuyến (hiện chỉ mang tính tài liệu; chưa đổi cờ rclone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap {
    /// Có thể server-side (không tải qua máy local) hay không.
    pub server_side: bool,
    /// Có cần sudo fallback (`pkexec cp`) khi lỗi quyền hay không.
    pub sudo_fallback: bool,
    /// Backend hỗ trợ copy (`Copy`).
    pub support_copy: bool,
    /// Phạm vi xóa nguồn — copy giữ nguồn nên luôn `NoTrash` (tài liệu cho S2).
    pub delete_scope: DeleteScope,
}

/// UNIVERSAL: copy 1 bước — backend có `Copy` nên `rclone copyto` sao chép tại
/// chỗ (cùng backend thì server-side, khác backend thì trung chuyển qua local).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportCopy(pub bool);

impl Cap {
    /// Suy năng lực từ cờ backend `Copy`.
    pub fn from_backend_features(copy: bool) -> Self {
        let support_copy = SupportCopy(copy);
        Self {
            server_side: false,
            sudo_fallback: false,
            support_copy: support_copy.0,
            // UNIVERSAL: copy giữ nguồn nên không xóa gì; NoTrash tường minh để S2 gộp chung CopyAndDelete.
            delete_scope: DeleteScope::NoTrash,
        }
    }
}

impl Route {
    /// Phân tuyến từ tên remote đã parse (`"Local"` = ổ máy).
    pub fn classify(src_remote: &str, dst_remote: &str) -> Self {
        match (src_remote == "Local", dst_remote == "Local", src_remote == dst_remote) {
            // UNIVERSAL: cả hai đầu là ổ máy — rclone `copyto` + fallback `pkexec cp -r`.
            (true, true, _) => Self::LocalLocal,
            // UNIVERSAL: đẩy từ đĩa lên cloud — upload đơn thuần, không sudo.
            (true, false, _) => Self::LocalCloud,
            // UNIVERSAL: kéo từ cloud về đĩa — download đơn thuần, không sudo.
            (false, true, _) => Self::CloudLocal,
            // UNIVERSAL: cùng một remote — backend thường copy server-side nhanh.
            (false, false, true) => Self::SameCloud,
            // UNIVERSAL: khác remote cloud — phải re-upload qua băng thông máy local.
            (false, false, false) => Self::DiffCloud,
        }
    }

    /// Năng lực gợi ý cho từng tuyến (tắt cờ xuyên-config → giữ hành vi cũ).
    pub fn cap(self) -> Cap {
        // UNIVERSAL: đường cũ — coi như khác hãng, DiffCloud không server-side.
        self.cap_with_provider(SameProvider(false), false)
    }

    /// UNIVERSAL: năng lực theo tuyến × cùng-hãng × cờ xuyên-config; DiffCloud
    /// `server_side = same_provider && across_enabled`, các tuyến khác giữ nguyên.
    pub fn cap_with_provider(self, same: SameProvider, across_enabled: bool) -> Cap {
        let diff_server_side = same.0 && across_enabled;
        match self {
            // UNIVERSAL: Local↔Local đi qua syscall; server-side vô nghĩa,
            // nhưng cần sudo fallback khi dính Permission Denied.
            Self::LocalLocal => Cap {
                server_side: false,
                sudo_fallback: true,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Local→Cloud là upload; rclone tải lên thẳng backend.
            Self::LocalCloud => Cap {
                server_side: false,
                sudo_fallback: false,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Cloud→Local là download; rclone tải về thẳng đĩa.
            Self::CloudLocal => Cap {
                server_side: false,
                sudo_fallback: false,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: cùng backend có thể copy server-side, không qua local.
            Self::SameCloud => Cap {
                server_side: true,
                sudo_fallback: false,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: khác backend cùng hãng + bật cờ thì server-side
            // xuyên-config (`--server-side-across-configs`); khác hãng/tắt cờ
            // giữ nguyên trung chuyển qua máy.
            Self::DiffCloud => Cap {
                server_side: diff_server_side,
                sudo_fallback: false,
                support_copy: false,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::engine::GlobalFlags;

    #[test]
    fn copy_diffcloud_cap_same_vs_diff_provider() {
        // UNIVERSAL: cùng hãng + bật cờ → server-side xuyên-config.
        assert!(Route::DiffCloud.cap_with_provider(SameProvider(true), true).server_side);
        // UNIVERSAL: khác hãng / tắt cờ → giữ nguyên trung chuyển qua local.
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(false), true).server_side);
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(true), false).server_side);
        assert!(!Route::DiffCloud.cap().server_side);
    }

    #[test]
    fn copy_diffcloud_args_same_vs_diff_provider() {
        // UNIVERSAL: cùng hãng + bật cờ engine → có cờ xuyên-config trong args rclone.
        let on = GlobalFlags { server_side_across: true, ..GlobalFlags::default() };
        let same_args = whole_args("copyto", "A:/a", "B:/b", &on, true);
        assert!(same_args.contains(&"--server-side-across-configs".to_string()));
        // UNIVERSAL: khác hãng / tắt cờ → args giữ nguyên, không có cờ.
        let diff_args = whole_args("copyto", "A:/a", "B:/b", &on, false);
        assert!(!diff_args.contains(&"--server-side-across-configs".to_string()));
        let off = GlobalFlags::default();
        let off_args = whole_args("copyto", "A:/a", "B:/b", &off, true);
        assert!(!off_args.contains(&"--server-side-across-configs".to_string()));
    }

    #[test]
    fn whole_args_keep_legacy_behavior_by_default() {
        // UNIVERSAL: default (4/8, các cờ tắt) giữ hành vi cũ, không cờ thêm.
        let flags = GlobalFlags::default();
        let args = whole_args("copyto", "A:/a", "B:/b", &flags, false);
        assert!(args.contains(&"--transfers=4".to_string()));
        assert!(args.contains(&"--checkers=8".to_string()));
        assert!(args.contains(&"--use-json-log".to_string()));
        assert!(!args.iter().any(|a| a == "--dry-run"));
        assert!(!args.iter().any(|a| a.starts_with("--backup-dir=")));
        assert!(!args.contains(&"--server-side-across-configs".to_string()));
    }

    #[test]
    fn whole_args_append_optional_switches() {
        // UNIVERSAL: bật dry-run/backup/across thì args phải có đủ cờ.
        let flags = GlobalFlags {
            transfers: 2,
            checkers: 3,
            fast_list: false,
            server_side_across: true,
            dry_run: true,
            backup_dir: Some("/tmp/bk".to_string()),
            bulk_transfer: false,
        };
        let args = whole_args("copyto", "A:/a", "B:/b", &flags, true);
        assert!(args.contains(&"--transfers=2".to_string()));
        assert!(args.contains(&"--checkers=3".to_string()));
        assert!(args.contains(&"--dry-run".to_string()));
        assert!(args.contains(&"--backup-dir=/tmp/bk".to_string()));
        assert!(args.contains(&"--server-side-across-configs".to_string()));
    }
}

/// UNIVERSAL: ráp args rclone cho vé nguyên khối — đủ cờ như builder cũ:
/// `--transfers/--checkers` + `--use-json-log --stats 0.5s -v` (tần số vừa đủ
/// để stream tiến độ, không 100ms thô) + `--dry-run`/`--backup-dir` khi bật +
/// `--server-side-across-configs` khi cùng hãng (`across`) và bật cờ engine.
pub(crate) fn whole_args(
    cmd: &str,
    src: &str,
    dst: &str,
    flags: &crate::settings::engine::GlobalFlags,
    server_side_across: bool,
) -> Vec<String> {
    let mut args = vec![
        cmd.to_string(),
        src.to_string(),
        dst.to_string(),
        format!("--transfers={}", flags.transfers),
        format!("--checkers={}", flags.checkers),
        "--use-json-log".to_string(),
        "--stats".to_string(),
        "0.5s".to_string(),
        "-v".to_string(),
    ];
    // UNIVERSAL: dry-run thử trước, không ghi gì lên đích.
    if flags.dry_run {
        args.push("--dry-run".to_string());
    }
    // UNIVERSAL: giữ bản bị ghi đè/xoá vào backup-dir thay vì mất hẳn.
    if let Some(dir) = flags.backup_dir.as_deref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            args.push(format!("--backup-dir={dir}"));
        }
    }
    // UNIVERSAL: cùng hãng + bật cờ — server-side xuyên-config, không qua local.
    // UNIVERSAL: khác hãng / tắt cờ — giữ nguyên args cũ.
    if flags.server_side_across && server_side_across {
        args.push("--server-side-across-configs".to_string());
    }
    args
}

/// UNIVERSAL: lệnh đơn cho 1 file (`Item`) — vẫn `--use-json-log --stats 0.5s -v`
/// để stream tiến độ; giữ `dry-run`/`backup-dir` theo vé, bỏ cờ bulk
/// (transfers/checkers/across vô nghĩa với 1 file).
pub(crate) fn item_args(
    cmd: &str,
    src: &str,
    dst: &str,
    flags: &crate::settings::engine::GlobalFlags,
) -> Vec<String> {
    let mut args = vec![
        cmd.to_string(),
        src.to_string(),
        dst.to_string(),
        "--use-json-log".to_string(),
        "--stats".to_string(),
        "0.5s".to_string(),
        "-v".to_string(),
    ];
    if flags.dry_run {
        args.push("--dry-run".to_string());
    }
    if let Some(dir) = flags.backup_dir.as_deref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            args.push(format!("--backup-dir={dir}"));
        }
    }
    args
}

/// UNIVERSAL: nối base với path tương đối của vé (`Item`); giữ `/` phân cách
/// vì remote dùng `/`; `rel` rỗng (= vé đơn lẻ) thì giữ nguyên base.
pub(crate) fn join_child(base: &str, rel: &str) -> String {
    if rel.is_empty() {
        return base.to_string();
    }
    let b = base.trim_end_matches('/');
    let r = rel.trim_start_matches('/');
    if b.is_empty() {
        format!("/{r}")
    } else {
        format!("{b}/{r}")
    }
}

/// UNIVERSAL: lõi streaming dùng chung copy/move — spawn rclone stderr piped,
/// phun từng DÒNG THÔ vào `on_log_line` (KHÔNG parse, KHÔNG tính %); poll `should_cancel` mỗi dòng → `process::terminate_gracefully(pid)`
/// rồi chờ. Xong thì `wait`; exit lỗi trả dòng cuối stderr. Local↔Local bọc
/// ngoài bằng `perm::escalate` (policy đã đóng dấu trên vé).
pub(crate) fn run_streaming(
    cmd: &str,
    sudo_action: &str,
    ticket: &TransferTicket,
    should_cancel: impl Fn() -> bool,
    mut on_log_line: impl FnMut(&str),
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    if should_cancel() {
        return Err("job cancelled".to_string());
    }
    let (src_remote, src_real) = crate::core::path::cut_remote_path(&ticket.src);
    let (dst_remote, dst_real) = crate::core::path::cut_remote_path(&ticket.dst);
    // UNIVERSAL: target rclone theo mode — Whole nguyên khối, Item nối `rel`.
    let (src_target, dst_target, sudo_args) = match ticket.mode {
        TransferMode::Whole => (
            crate::core::rclone_caller::build_target(&src_remote, &src_real),
            crate::core::rclone_caller::build_target(&dst_remote, &dst_real),
            vec![src_real.clone(), dst_real.clone()],
        ),
        TransferMode::Item => {
            let full_src = join_child(&src_real, &ticket.rel);
            let full_dst = join_child(&dst_real, &ticket.rel);
            (
                crate::core::rclone_caller::build_target(&src_remote, &full_src),
                crate::core::rclone_caller::build_target(&dst_remote, &full_dst),
                vec![full_src, full_dst],
            )
        }
    };
    let args = match ticket.mode {
        TransferMode::Whole => whole_args(
            cmd,
            &src_target,
            &dst_target,
            &ticket.engine_flags,
            ticket.across,
        ),
        TransferMode::Item => item_args(cmd, &src_target, &dst_target, &ticket.engine_flags),
    };
    let mut run = || -> Result<(), String> {
        let mut child = Command::new("rclone")
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Lỗi hệ thống khi gọi rclone: {e}"))?;
        let pid = child.id();
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "không đọc được stderr rclone".to_string())?;
        let mut tail = String::new();
        for line in BufReader::new(stderr).lines() {
            // UNIVERSAL: poll hủy mỗi dòng log (stats về mỗi 0.5s) — hủy êm để
            // rclone kịp dọn `.partial`, tuyệt đối không `child.kill()` thẳng.
            if should_cancel() {
                crate::logic::process::terminate_gracefully(pid);
                let _ = child.wait();
                return Err("job cancelled".to_string());
            }
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if line.trim().is_empty() {
                continue;
            }
            tail = line.clone();
            // UNIVERSAL: phun dòng thô cho queue/tracker xử lý — actions tay trắng về %.
            on_log_line(&line);
        }
        let status = child.wait().map_err(|e| format!("Lỗi chờ rclone: {e}"))?;
        if status.success() {
            Ok(())
        } else if tail.is_empty() {
            Err(format!("rclone {cmd} thất bại"))
        } else {
            Err(tail)
        }
    };
    // UNIVERSAL: Local↔Local rớt qua `pkexec cp/mv` khi dính lỗi quyền
    // (policy `Deny`/`AskOnce` chưa consent → `PERMISSION_CONSENT` để park).
    if src_remote == "Local" && dst_remote == "Local" {
        crate::actions::perm::escalate(ticket.policy, "Local", sudo_action, &sudo_args, run)
    } else {
        run()
    }
}

/// UNIVERSAL: thực thi copy từ VÉ ĐÃ ĐÓNG DẤU (worker sync, test sync) —
/// không đọc engine settings/AppHandle/State; phun log thô qua `on_log_line`
/// (queue đưa vào tracker để tính %), hủy hợp tác qua `should_cancel`
/// (xem [`run_streaming`]).
pub fn execute_copy(
    ticket: TransferTicket,
    should_cancel: impl Fn() -> bool,
    on_log_line: impl FnMut(&str),
) -> Result<(), String> {
    run_streaming("copyto", "cp", &ticket, should_cancel, on_log_line)
}
