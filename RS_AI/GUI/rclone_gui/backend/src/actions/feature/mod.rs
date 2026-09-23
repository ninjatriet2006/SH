//! Nhóm `feature` — mọi thứ BIẾT năng lực rclone thì ở đây, 1 mối duy nhất.
//!
//! - `getfeature`: ánh xạ 52 cờ native (`backend features` → struct typed).
//! - `combinefeature`: tổ hợp cờ native thành đáp án (dời/copy/dọn được không).
//! - `checkcap`: cửa quyết định duy nhất (cache + fallback + trả lời JSON).
//!
//! UNIVERSAL: nhóm theo BIẾT cờ (providers), không theo DÙNG cờ (consumers).
//! Thợ thực thi (copy/move/delete/trash/rename) đứng ngoài, đọc vào như mọi
//! consumer khác — nếu gom theo "dùng cờ" thì nửa codebase sụp vào đây.

pub mod checkcap;
pub mod combinefeature;
pub mod getfeature;
