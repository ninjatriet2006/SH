pub mod app_state;
pub mod desktop_apps;
pub mod file_ops;
pub mod transfer;
pub mod trash_local;
pub mod trash_remote;
pub mod watcher;
/// Re-export trial S1 move action để tầng `api` dùng chung khi sẵn sàng.
pub use crate::actions::execute_move;
