//! Nhóm Streaming Actions: thực thi chuyển dữ liệu (copy, move) và cỗ máy rclone stream.
//! Quản lý tiến trình rclone với stream log JSON 0.5s, hủy êm và leo thang quyền.

#[path = "copy.rs"]
pub mod copy_op;

#[path = "move.rs"]
pub mod move_op;

pub mod rclone_stream;

pub use copy_op::execute_copy;
pub use move_op::execute_move;
