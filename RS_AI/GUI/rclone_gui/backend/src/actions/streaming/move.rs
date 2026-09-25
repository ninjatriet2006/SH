/*
[INTEGRITY NOTES]
- Mục đích: THỰC THI move (nơi duy nhất gọi rclone move).
- Trách nhiệm: `execute_move` NHẬN VÉ ĐÃ ĐÓNG DẤU (`TransferTicket` — không đọc
  engine settings), gọi máy rclone chung `rclone_stream::exec` với lệnh
  `moveto` + sudo fallback `mv`.
- Tương tác: `logic::queue` gọi `execute_move` như `execute_copy`. Không đụng copy/ipc/frontend.
*/
// UNIVERSAL: actions tay trắng về % — chỉ GỌI rclone + phun dòng log THÔ vào
// sink (tính % là việc DUY NHẤT của tracker, queue hỏi tracker rồi ghi Job)
// + hủy êm + escalate (xem `super::rclone_stream::exec`); não Route/Cap/check_cap
// nằm ở `checkcap` (file này chỉ giữ thực thi + re-export tương thích).
// UNIVERSAL: checkcap là não chung duy nhất; move chỉ lo chạy.

pub use crate::actions::types::DeleteScope;
/// UNIVERSAL: re-export tương thích — code ngoài `move_op::{Cap,Route,...}` không vỡ.
pub use crate::actions::checkcap::{Cap, Route, SupportCopyAndDelete, SupportMove, check_cap, check_cap_with_flags};
use crate::logic::tracker::TransferTicket;

/// Loại transfer mà module này phục vụ (giữ chỗ để S2 gộp copy/move chung).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Move,
}

/// UNIVERSAL: thực thi move từ VÉ ĐÃ ĐÓNG DẤU (worker sync, test sync) —
/// gọi máy rclone chung (`rclone moveto`, sudo fallback `pkexec mv`);
/// phun log thô qua `on_log_line` (tay trắng về %, tracker tính sau).
/// Vé bị đọc các field: `src`/`dst`, `mode` (+`rel`), `engine_flags` + `across`,
/// `policy` — chi tiết xem doc máy chung `rclone_stream::exec`.
pub fn execute_move(
    ticket: TransferTicket,
    should_cancel: impl Fn() -> bool,
    on_log_line: impl FnMut(&str),
) -> Result<(), String> {
    super::rclone_stream::exec("moveto", "mv", &ticket, should_cancel, on_log_line)
}
