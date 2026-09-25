/*
[INTEGRITY NOTES]
- Mục đích: THỰC THI copy (nơi duy nhất gọi rclone copy).
- Trách nhiệm: `execute_copy` NHẬN VÉ ĐÃ ĐÓNG DẤU (`TransferTicket` — không đọc
  engine settings). Phân tuyến/năng lực do `feature::checkcap` lo (bản sao
  Route/Cap cũ của copy đã xoá — 1 não duy nhất, khỏi song sinh).
- Tương tác: `logic::queue` gọi `execute_copy` với vé + `should_cancel` +
  sink dòng thô (`on_log_line`); máy gọi rclone dùng chung nằm ở
  `super::rclone_stream` (copy chỉ còn thực thi).
  Không AppHandle/State (worker sync, test sync).
*/
// UNIVERSAL: actions tay trắng về % — chỉ GỌI rclone (ráp lệnh + phun dòng
// log THÔ vào sink) + hủy êm qua `logic::process::terminate_gracefully` +
// Local↔Local qua `perm::escalate`. Tính % là việc DUY NHẤT của tracker,
// queue hỏi tracker rồi ghi vào Job.

use crate::logic::tracker::TransferTicket;

/// Loại transfer mà module này phục vụ (giữ chỗ để S2 gộp copy/move chung).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Copy,
}

/// UNIVERSAL: thực thi copy từ VÉ ĐÃ ĐÓNG DẤU (worker sync, test sync) —
/// không đọc engine settings/AppHandle/State; phun log thô qua `on_log_line`
/// (queue đưa vào tracker để tính %), hủy hợp tác qua `should_cancel`.
/// Máy gọi rclone dùng chung nằm ở [`crate::actions::rclone_stream::exec`].
pub fn execute_copy(
    ticket: TransferTicket,
    should_cancel: impl Fn() -> bool,
    on_log_line: impl FnMut(&str),
) -> Result<(), String> {
    crate::actions::rclone_stream::exec("copyto", "cp", &ticket, should_cancel, on_log_line)
}
