//! Nhóm Instant Actions: các thao tác biến đổi tệp (mkdir, touch, rename, delete) cho worker Job Queue.
//! Được thực thi đồng bộ, tuần tự trong worker của `logic::jobs` (không chạy qua fastlane).

pub mod mkdir;
pub mod touch;
pub mod rename;
pub mod delete;
pub mod rclone_instant;

pub use rclone_instant::{CreateKind, CreatePlan, execute_create_sync, plan_create};
pub use mkdir::{MkdirPlan, execute_mkdir_sync, plan_mkdir};
pub use touch::{TouchPlan, execute_touch_sync, plan_touch};
pub use rename::{
    Cap as RenameCap, IsDir, RenamePlan, SupportRename, execute_rename_sync,
    plan_rename, plan_rename_for,
};
pub use delete::{
    DeletePlan, DeleteScope, EmptyDirs, Route as DeleteRoute, execute_delete_sync, plan_delete,
};
pub use delete as delete_op;
