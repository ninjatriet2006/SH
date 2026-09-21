/*
[INTEGRITY NOTES]
- Mục đích: Route + Cap cho move + THỰC THI move (nơi duy nhất gọi rclone move).
- Trách nhiệm: Phân tuyến (Route) → chọn Cap; `execute_move` NHẬN VÉ ĐÃ ĐÓNG DẤU
  (`TransferTicket` — không đọc engine settings), dùng chung lõi streaming
  `copy_op::run_streaming` với lệnh `moveto` + sudo fallback `mv`.
- Tương tác: `logic::queue` gọi `execute_move` như `execute_copy`. Không đụng copy/ipc/frontend.
*/
// UNIVERSAL: actions tay trắng về % — chỉ GỌI rclone + phun dòng log THÔ vào
// sink (tính % là việc DUY NHẤT của tracker, queue hỏi tracker rồi ghi Job)
// + hủy êm + escalate (xem `copy_op::run_streaming`); file này chỉ giữ
// Route/Cap + `execute_move`.

pub use crate::actions::types::DeleteScope;
use crate::actions::types::SameProvider;
use crate::logic::tracker::TransferTicket;

/// Tuyến di chuyển, suy từ cặp (src_remote, dst_remote).
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
    Move,
}

/// Năng lực gợi ý cho từng tuyến (hiện chỉ mang tính tài liệu; chưa đổi cờ rclone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap {
    /// Có thể server-side (không tải qua máy local) hay không.
    pub server_side: bool,
    /// Có cần sudo fallback (`pkexec mv`) khi lỗi quyền hay không.
    pub sudo_fallback: bool,
    /// Backend hỗ trợ move-native (rclone `Move`/`DirMove`).
    pub support_move: bool,
    /// Backend hỗ trợ fallback copy + purge (`Copy` + `Purge`).
    pub support_copy_and_delete: bool,
    /// Phạm vi xóa nguồn khi fallback.
    pub delete_scope: DeleteScope,
}

/// UNIVERSAL: move-native 1 bước — backend có `Move` hoặc `DirMove` nên
/// `rclone moveto` đổi tên tại chỗ, không cần tải lại dữ liệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportMove(pub bool);

/// UNIVERSAL: copy + purge fallback 2 bước — backend thiếu move-native nên
/// phải `Copy` rồi `Purge` nguồn; chỉ khả dụng khi cả hai cờ đều bật.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportCopyAndDelete(pub bool);

impl Cap {
    /// Suy năng lực từ cờ backend `Move`/`DirMove`/`Copy`/`Purge`.
    pub fn from_backend_features(mv: bool, dir_mv: bool, copy: bool, purge: bool) -> Self {
        let support_move = SupportMove(mv || dir_mv);
        let support_copy_and_delete = SupportCopyAndDelete(copy && purge);
        Self {
            server_side: false,
            sudo_fallback: false,
            support_move: support_move.0,
            support_copy_and_delete: support_copy_and_delete.0,
            // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa;
            // Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
            delete_scope: DeleteScope::NoTrash,
        }
    }
}

impl Route {
    /// Phân tuyến từ tên remote đã parse (`"Local"` = ổ máy).
    pub fn classify(src_remote: &str, dst_remote: &str) -> Self {
        match (src_remote == "Local", dst_remote == "Local", src_remote == dst_remote) {
            // UNIVERSAL: cả hai đầu là ổ máy — rclone `moveto` + fallback `pkexec mv`.
            (true, true, _) => Self::LocalLocal,
            // UNIVERSAL: đẩy từ đĩa lên cloud — upload đơn thuần, không sudo.
            (true, false, _) => Self::LocalCloud,
            // UNIVERSAL: kéo từ cloud về đĩa — download đơn thuần, không sudo.
            (false, true, _) => Self::CloudLocal,
            // UNIVERSAL: cùng một remote — backend thường move server-side nhanh.
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
            // UNIVERSAL: Local↔Local đi qua syscall/rename; server-side vô nghĩa,
            // nhưng cần sudo fallback khi dính Permission Denied.
            Self::LocalLocal => Cap {
                server_side: false,
                sudo_fallback: true,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Local→Cloud là upload; rclone tải lên thẳng backend.
            Self::LocalCloud => Cap {
                server_side: false,
                sudo_fallback: false,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Cloud→Local là download; rclone tải về thẳng đĩa.
            Self::CloudLocal => Cap {
                server_side: false,
                sudo_fallback: false,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: cùng backend có thể rename server-side, không qua local.
            Self::SameCloud => Cap {
                server_side: true,
                sudo_fallback: false,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: khác backend cùng hãng + bật cờ thì server-side
            // xuyên-config (`--server-side-across-configs`); khác hãng/tắt cờ
            // giữ nguyên trung chuyển qua máy.
            Self::DiffCloud => Cap {
                server_side: diff_server_side,
                sudo_fallback: false,
                support_move: false,
                support_copy_and_delete: false,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::copy_op::{item_args, whole_args};

    #[test]
    fn move_diffcloud_cap_same_vs_diff_provider() {
        // UNIVERSAL: cùng hãng + bật cờ → server-side xuyên-config.
        assert!(Route::DiffCloud.cap_with_provider(SameProvider(true), true).server_side);
        // UNIVERSAL: khác hãng / tắt cờ → giữ nguyên trung chuyển qua local.
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(false), true).server_side);
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(true), false).server_side);
        assert!(!Route::DiffCloud.cap().server_side);
    }

    #[test]
    fn move_diffcloud_args_same_vs_diff_provider() {
        use crate::settings::engine::GlobalFlags;
        // UNIVERSAL: cùng hãng + bật cờ engine → có cờ xuyên-config trong args rclone.
        let on = GlobalFlags { server_side_across: true, ..GlobalFlags::default() };
        let same_args = whole_args("moveto", "A:/a", "B:/b", &on, true);
        assert!(same_args.contains(&"--server-side-across-configs".to_string()));
        // UNIVERSAL: khác hãng / tắt cờ → args giữ nguyên, không có cờ.
        let diff_args = whole_args("moveto", "A:/a", "B:/b", &on, false);
        assert!(!diff_args.contains(&"--server-side-across-configs".to_string()));
        let off = GlobalFlags::default();
        let off_args = whole_args("moveto", "A:/a", "B:/b", &off, true);
        assert!(!off_args.contains(&"--server-side-across-configs".to_string()));
    }

    #[test]
    fn move_item_args_stay_single_with_json_log() {
        // UNIVERSAL: vé từng món là lệnh đơn nhưng vẫn json-log để stream tiến độ.
        use crate::settings::engine::GlobalFlags;
        let args = item_args("moveto", "A:/a", "B:/b", &GlobalFlags::default());
        assert_eq!(&args[0..3], &["moveto", "A:/a", "B:/b"]);
        assert!(args.contains(&"--use-json-log".to_string()));
        assert!(!args.iter().any(|a| a.starts_with("--transfers=")));
    }
}

/// UNIVERSAL: thực thi move từ VÉ ĐÃ ĐÓNG DẤU (worker sync, test sync) —
/// chung lõi streaming với copy (`rclone moveto`, sudo fallback `pkexec mv`);
/// phun log thô qua `on_log_line` (tay trắng về %, tracker tính sau).
pub fn execute_move(
    ticket: TransferTicket,
    should_cancel: impl Fn() -> bool,
    on_log_line: impl FnMut(&str),
) -> Result<(), String> {
    super::copy_op::run_streaming("moveto", "mv", &ticket, should_cancel, on_log_line)
}
