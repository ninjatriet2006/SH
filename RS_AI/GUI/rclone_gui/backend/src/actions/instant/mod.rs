//! Nhóm Instant Actions: các thao tác tệp tức thời (mkdir, touch, rename, delete) và cỗ máy rclone instant.
//! Khác với nhóm streaming (chuyển file ngầm có tiến độ %): nhóm này phản hồi ngay lập tức qua fastlane.

pub mod mkdir;
pub mod touch;
pub mod rename;
pub mod delete;
pub mod rclone_instant;

pub use rclone_instant::{CreateKind, CreatePlan, execute_create, execute_create_sync, plan_create};
pub use mkdir::{MkdirPlan, execute_mkdir, execute_mkdir_sync, plan_mkdir};
pub use touch::{TouchPlan, execute_touch, execute_touch_sync, plan_touch};
pub use rename::{
    Cap as RenameCap, IsDir, RenamePlan, SupportRename, execute_rename, execute_rename_sync,
    plan_rename, plan_rename_for,
};
pub use delete::{
    DeletePlan, DeleteScope, EmptyDirs, Route as DeleteRoute, execute_delete, execute_delete_sync,
    execute_delete_with_empty_dirs, plan_delete,
};
pub use delete as delete_op;
