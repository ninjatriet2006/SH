//! Nhóm Instant Actions: các thao tác tệp tức thời (mkdir, touch, rename, delete) và cỗ máy rclone instant.
//! Khác với nhóm streaming (chuyển file ngầm có tiến độ %): nhóm này phản hồi ngay lập tức qua fastlane.

pub mod mkdir;
pub mod touch;
pub mod rename;
pub mod delete;
pub mod rclone_instant;

pub use rclone_instant::{CreateKind, CreatePlan, execute_create, plan_create};
pub use mkdir::{MkdirPlan, execute_mkdir, plan_mkdir};
pub use touch::{TouchPlan, execute_touch, plan_touch};
pub use rename::{Cap as RenameCap, IsDir, RenamePlan, SupportRename, execute_rename, plan_rename, plan_rename_for};
pub use delete::{
    DeletePlan, DeleteScope, EmptyDirs, Route as DeleteRoute, execute_delete, execute_delete_sync,
    execute_delete_with_empty_dirs, plan_delete,
};
pub use delete as delete_op;
