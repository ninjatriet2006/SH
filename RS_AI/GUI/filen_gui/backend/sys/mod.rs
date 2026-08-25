//! [INTEGRITY NOTES]
//! Mục đích: Lớp trừu tượng đặc thù HĐH (Linux và Windows) của backend.
//! Trách nhiệm: Xuất khẩu (export) đúng module tùy theo nền tảng hệ điều hành.
//! Tương tác: Giao tiếp với operations.rs

#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub mod desktop_apps;
#[cfg(unix)]
pub mod custom_actions;
#[cfg(unix)]
pub mod doc_search;
#[cfg(windows)]
pub mod windows;

#[cfg(unix)]
pub use unix::*;
#[cfg(unix)]
pub use desktop_apps::*;
#[cfg(unix)]
pub use custom_actions::*;
#[cfg(unix)]
pub use doc_search::*;
#[cfg(windows)]
pub use windows::*;
