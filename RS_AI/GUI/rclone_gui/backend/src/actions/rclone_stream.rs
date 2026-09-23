/*
[INTEGRITY NOTES]
- Mục đích: Tầng Actions — MÁY GỌI RCLONE DÙNG CHUNG (thực thi lệnh rclone +
  stream log thô + hủy êm + sudo fallback). Copy/move đều gọi đây, chỉ khác
  tên lệnh (`copyto`/`cp` vs `moveto`/`mv`).
- Khác `core::rclone_caller` (spawn nguyên thủy, không biết nghiệp vụ): máy này
  hiểu vé transfer (mode/rel/across/engine_flags/policy), ráp cờ rclone, leo
  thang quyền qua `perm`, hủy êm qua `logic::process`. Vì cần 3 tầng trên nên
  KHÔNG thể nằm ở core.
- Trách nhiệm: `exec` chạy 1 lệnh từ vé; `whole_args`/`item_args` ráp args;
  `join_child` nối path vé. Tay trắng về % (queue/tracker lo).
- Tương tác: `copy_op::execute_copy` + `move_op::execute_move` gọi `exec`.
*/

use crate::logic::tracker::{TransferMode, TransferTicket};

/// UNIVERSAL: ráp args rclone cho vé nguyên khối — đủ cờ như builder cũ:
/// `--transfers/--checkers` + `--use-json-log --stats 0.5s -v` (tần số vừa đủ
/// để stream tiến độ, không 100ms thô) + `--dry-run`/`--backup-dir` khi bật +
/// `--server-side-across-configs` khi cùng hãng (`across`) và bật cờ engine.
pub(crate) fn whole_args(
    cmd: &str,
    src: &str,
    dst: &str,
    flags: &crate::settings::engine::EngineSettings,
    server_side_across: bool,
) -> Vec<String> {
    let mut args = vec![
        cmd.to_string(),
        src.to_string(),
        dst.to_string(),
        format!("--transfers={}", flags.tuning.transfers),
        format!("--checkers={}", flags.tuning.checkers),
        "--use-json-log".to_string(),
        "--stats".to_string(),
        "0.5s".to_string(),
        "-v".to_string(),
    ];
    // UNIVERSAL: dry-run thử trước, không ghi gì lên đích.
    if flags.switches.dry_run {
        args.push("--dry-run".to_string());
    }
    // UNIVERSAL: giữ bản bị ghi đè/xoá vào backup-dir thay vì mất hẳn.
    if let Some(dir) = flags.tuning.backup_dir.as_deref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            args.push(format!("--backup-dir={dir}"));
        }
    }
    // UNIVERSAL: cùng hãng + bật cờ — server-side xuyên-config, không qua local.
    // UNIVERSAL: khác hãng / tắt cờ — giữ nguyên args cũ.
    if flags.switches.server_side_across && server_side_across {
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
    flags: &crate::settings::engine::EngineSettings,
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
    if flags.switches.dry_run {
        args.push("--dry-run".to_string());
    }
    if let Some(dir) = flags.tuning.backup_dir.as_deref() {
        let dir = dir.trim();
        if !dir.is_empty() {
            args.push(format!("--backup-dir={dir}"));
        }
    }
    args
}

/// UNIVERSAL: nối base với path tương đối của vé (`Item`); giữ `/` phân cách
/// vì remote dùng `/`; `rel` rỗng (= vé đơn lẻ) thì giữ nguyên base.
/// Gộp 2 bản trùng (`copy.rs` + `queue.rs`) về 1 mối duy nhất.
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

/// UNIVERSAL: GỌI RCLONE 1 lệnh từ vé đã đóng dấu + stream log thô — spawn
/// rclone stderr piped, phun từng DÒNG THÔ vào `on_log_line` (KHÔNG parse,
/// KHÔNG tính %); poll `should_cancel` mỗi dòng → `process::terminate_gracefully(pid)`
/// rồi chờ. Xong thì `wait`; exit lỗi trả dòng cuối stderr. Local↔Local bọc
/// ngoài bằng `perm::escalate` (policy đã đóng dấu trên vé).
pub(crate) fn exec(
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
            // UNIVERSAL: dòng lỗi đọc → warn rồi bỏ qua như cũ.
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    crate::core::debug::warn(None, "rclone_stream/exec", format!("bỏ dòng log lỗi: {e}"));
                    continue;
                }
            };
            // UNIVERSAL: guard sớm, phẳng else lồng.
            if line.trim().is_empty() {
                continue;
            }
            tail = line.clone();
            // UNIVERSAL: phun dòng thô cho queue/tracker xử lý — actions tay trắng về %.
            on_log_line(&line);
        }
        let status = child.wait().map_err(|e| format!("Lỗi chờ rclone: {e}"))?;
        // UNIVERSAL: match phẳng tail, giữ nguyên câu lỗi cũ.
        match (status.success(), tail.is_empty()) {
            (true, _) => Ok(()),
            (false, true) => Err(format!("rclone {cmd} thất bại")),
            (false, false) => Err(tail),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::engine::{EngineSettings, EngineSwitches, EngineTuning};

    #[test]
    fn child_path_join_keeps_slash_sep() {
        // UNIVERSAL: nối base + rel vé con, giữ `/` vì remote dùng `/`.
        assert_eq!(join_child("/a/b", "c/d.txt"), "/a/b/c/d.txt");
        assert_eq!(join_child("/a/b/", "/c.txt"), "/a/b/c.txt");
        assert_eq!(join_child("", "c.txt"), "/c.txt");
    }

    #[test]
    fn copy_diffcloud_args_same_vs_diff_provider() {
        // UNIVERSAL: cùng hãng + bật cờ engine → có cờ xuyên-config trong args rclone.
        let on = EngineSettings {
            switches: EngineSwitches { server_side_across: true, ..Default::default() },
            ..Default::default()
        };
        let same_args = whole_args("copyto", "A:/a", "B:/b", &on, true);
        assert!(same_args.contains(&"--server-side-across-configs".to_string()));
        // UNIVERSAL: khác hãng / tắt cờ → args giữ nguyên, không có cờ.
        let diff_args = whole_args("copyto", "A:/a", "B:/b", &on, false);
        assert!(!diff_args.contains(&"--server-side-across-configs".to_string()));
        let off = EngineSettings::default();
        let off_args = whole_args("copyto", "A:/a", "B:/b", &off, true);
        assert!(!off_args.contains(&"--server-side-across-configs".to_string()));
    }

    #[test]
    fn whole_args_keep_legacy_behavior_by_default() {
        // UNIVERSAL: default (4/8, các cờ tắt) giữ hành vi cũ, không cờ thêm.
        let flags = EngineSettings::default();
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
        let flags = EngineSettings {
            switches: EngineSwitches {
                fast_list: false,
                server_side_across: true,
                dry_run: true,
                bulk_transfer: false,
            },
            tuning: EngineTuning {
                transfers: 2,
                checkers: 3,
                backup_dir: Some("/tmp/bk".to_string()),
            },
        };
        let args = whole_args("copyto", "A:/a", "B:/b", &flags, true);
        assert!(args.contains(&"--transfers=2".to_string()));
        assert!(args.contains(&"--checkers=3".to_string()));
        assert!(args.contains(&"--dry-run".to_string()));
        assert!(args.contains(&"--backup-dir=/tmp/bk".to_string()));
        assert!(args.contains(&"--server-side-across-configs".to_string()));
    }

    #[test]
    fn move_diffcloud_args_same_vs_diff_provider() {
        // UNIVERSAL: cùng hãng + bật cờ engine → có cờ xuyên-config trong args rclone.
        let on = EngineSettings {
            switches: EngineSwitches { server_side_across: true, ..Default::default() },
            ..Default::default()
        };
        let same_args = whole_args("moveto", "A:/a", "B:/b", &on, true);
        assert!(same_args.contains(&"--server-side-across-configs".to_string()));
        // UNIVERSAL: khác hãng / tắt cờ → args giữ nguyên, không có cờ.
        let diff_args = whole_args("moveto", "A:/a", "B:/b", &on, false);
        assert!(!diff_args.contains(&"--server-side-across-configs".to_string()));
        let off = EngineSettings::default();
        let off_args = whole_args("moveto", "A:/a", "B:/b", &off, true);
        assert!(!off_args.contains(&"--server-side-across-configs".to_string()));
    }

    #[test]
    fn move_item_args_stay_single_with_json_log() {
        // UNIVERSAL: vé từng món là lệnh đơn nhưng vẫn json-log để stream tiến độ.
        let args = item_args("moveto", "A:/a", "B:/b", &EngineSettings::default());
        assert_eq!(&args[0..3], &["moveto", "A:/a", "B:/b"]);
        assert!(args.contains(&"--use-json-log".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("--transfers=")));
    }
}
