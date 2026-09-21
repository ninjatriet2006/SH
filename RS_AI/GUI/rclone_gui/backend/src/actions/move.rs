/*
[INTEGRITY NOTES]
- Mục đích: THỰC THI move (nơi duy nhất gọi rclone move).
- Trách nhiệm: `execute_move` NHẬN VÉ ĐÃ ĐÓNG DẤU (`TransferTicket` — không đọc
  engine settings), dùng chung lõi streaming `copy_op::run_streaming` với lệnh
  `moveto` + sudo fallback `mv`.
- Tương tác: `logic::queue` gọi `execute_move` như `execute_copy`. Không đụng copy/ipc/frontend.
*/
// UNIVERSAL: actions tay trắng về % — chỉ GỌI rclone + phun dòng log THÔ vào
// sink (tính % là việc DUY NHẤT của tracker, queue hỏi tracker rồi ghi Job)
// + hủy êm + escalate (xem `copy_op::run_streaming`); não Route/Cap/check_cap
// nằm ở `checkcap` (file này chỉ giữ thực thi + re-export tương thích).
// UNIVERSAL: checkcap là não chung duy nhất; move chỉ lo chạy.

pub use crate::actions::types::DeleteScope;
/// UNIVERSAL: re-export tương thích — code ngoài `move_op::{Cap,Route,...}` không vỡ.
pub use super::checkcap::{Cap, Route, SupportCopyAndDelete, SupportMove, check_cap, check_cap_with_flags};
use crate::logic::tracker::TransferTicket;

/// Loại transfer mà module này phục vụ (giữ chỗ để S2 gộp copy/move chung).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Move,
}

#[cfg(test)]
mod tests {
    use crate::actions::copy_op::{item_args, whole_args};

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
