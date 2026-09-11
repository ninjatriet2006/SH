//! [INTEGRITY NOTES]
//! Mục đích: Khai báo module gốc cho thư viện backend.
//! Trách nhiệm: Liên kết và phơi bày (expose) tất cả các module con trong thư mục `backend`.
//! Tương tác: Điểm truy cập chung cho ứng dụng chính (hoặc các component khác).

pub mod auth;
pub mod cloud_fs;
pub mod local_fs;
pub mod models;
pub mod sync;
pub mod sys;
pub mod transfer;
