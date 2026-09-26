//! Trial copy/delete actions (S1): pure routing + `copyto`/`purge` caps,
//! thực thi chạy qua `logic::jobs`.
//!
//! CỜ CÒN THIẾU (jobs/queue chọn cờ, giữ nguyên hành vi cũ khi tắt):
//! - `--dry-run`, `--interactive` (`-i`), `--backup-dir` + `--suffix`
//! - `--immutable`, `--track-renames`, `--delete-empty-src-dirs`
//! - `--max-delete`, `--max-transfer`, `--cutoff-mode`, `--order-by`
//! - `--transfers` / `--checkers` tuning theo route, `--server-side-across-configs`
//! - `--no-traverse`, `--fast-list`, `--sftp-set-modtime=false` theo backend
//!
//! S1 flatten: `explorer/{list,stat,search,view}` + `ops/{create,rename}` nằm
//! thẳng dưới `actions/`; hai module `explorer`/`ops` bên dưới chỉ là alias
//! tương thích để `use crate::actions::explorer::*` / `ops::*` cũ không vỡ.

pub mod appearance;
pub mod feature;

// UNIVERSAL: đường dùng cũ giữ nguyên (`crate::actions::checkcap::...`) để
// logic/api không phải sửa — nhóm feature re-export lại tên module.
pub use feature::{checkcap, combinefeature, getfeature};

pub mod streaming;
pub use streaming::{copy_op, move_op, rclone_stream};

pub use instant::{delete, delete_op};

pub mod instant;
pub use instant::{mkdir, touch, rclone_instant};
pub use instant as create;
pub mod conflicts;
pub mod list;
// UNIVERSAL S2 mount micro: query/editor/control (api/mount_manager bọc fastlane).
pub mod mount_control;
pub mod mount_editor;
pub mod mount_query;
pub use instant::rename;
pub mod remote_view;
pub mod remote_edit;
pub mod information;
pub use information as checksize;
pub mod search;
pub use information::stat;
pub mod system;
pub mod view;
pub mod perm;
pub mod trash_delete;
pub mod trash_list;
pub mod trash_restore;
pub mod types;

pub use types::{RemoteKind, SameProvider, same_provider, same_provider_from_dump};

pub use move_op::{Cap, DeleteScope, Route, SupportCopyAndDelete, SupportMove, TransferKind};

pub use delete_op::{
    DeleteScope as SharedDeleteScope, EmptyDirs as DeleteEmptyDirs, Route as DeleteRoute,
    execute_delete_sync,
};
// UNIVERSAL: worker tên gọn cho `api::remote_manager` (gọi qua path module, không alias).
pub use checksize::{about, size};
pub use remote_edit::{create_remote, delete_remote, update_remote};
pub use remote_view::{get_providers, list_remotes};

/// Năng lực gợi ý cho explorer (tên cũ: `explorer::Cap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplorerCap {
    /// Có đọc mode/uid/gid local kèm theo hay không.
    pub with_ownership: bool,
    /// Có liệt kê đệ quy qua `lsjson` hay không.
    pub recursive_list: bool,
}

impl ExplorerCap {
    /// Năng lực gợi ý cho từng loại remote.
    pub fn of(kind: types::RemoteKind) -> Self {
        match kind {
            // UNIVERSAL: Local có thêm mode/uid/gid từ syscall kèm `size --json`.
            types::RemoteKind::Local => ExplorerCap {
                with_ownership: true,
                recursive_list: true,
            },
            // UNIVERSAL: remote chỉ có metadata rclone (`size`/`lsjson`), không ownership.
            types::RemoteKind::Remote => ExplorerCap {
                with_ownership: false,
                recursive_list: true,
            },
        }
    }
}

/// Năng lực gợi ý cho họ tạo mkdir/touch (tên cũ: `ops::Cap`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpsCap {
    /// Có cần sudo fallback (`pkexec mkdir/mv`, `File::create`) khi lỗi quyền hay không.
    pub sudo_fallback: bool,
    /// Có đi qua rclone (`mkdir`/`touch`/`moveto`) hay syscall local.
    pub via_rclone: bool,
}

impl OpsCap {
    /// Năng lực gợi ý cho từng loại remote.
    pub fn of(kind: types::RemoteKind) -> Self {
        match kind {
            // UNIVERSAL: Local tạo/đổi tên qua syscall nhưng rclone vẫn xử lý được;
            // cần sudo fallback khi dính Permission Denied.
            types::RemoteKind::Local => OpsCap {
                sudo_fallback: true,
                via_rclone: true,
            },
            // UNIVERSAL: remote tạo/đổi tên qua lệnh rclone, không sudo.
            types::RemoteKind::Remote => OpsCap {
                sudo_fallback: false,
                via_rclone: true,
            },
        }
    }
}

/// Alias tương thích: giữ đường dẫn `actions::explorer::{Cap, plan_list, ...}`
/// và `actions::explorer::{list, search, stat, view}` cũ.
pub mod explorer {
    pub use super::list;
    pub use super::search;
    pub use super::stat;
    pub use super::view;
    pub use super::ExplorerCap as Cap;
    pub use super::list::{ListPlan, execute_list, plan_list};
    pub use super::search::{SearchPlan, execute_search, plan_search};
    pub use super::stat::{StatPlan, execute_stat, plan_stat};
    pub use super::view::{ThumbnailGroup, ThumbnailPlan, ViewPlan, plan_thumbnail, plan_view_download};
}

/// Alias tương thích: giữ đường dẫn `actions::ops::{Cap, plan_mkdir, ...}`
/// và `actions::ops::{create, rename}` cũ (`rename::Cap` qua `ops::rename::Cap`).
pub mod ops {
    pub use super::create;
    pub use super::rename;
    pub use super::OpsCap as Cap;
    pub use super::create::{
        CreateKind, CreatePlan, MkdirPlan, TouchPlan, execute_create_sync,
        plan_create, plan_mkdir, plan_touch,
    };
    pub use super::rename::{IsDir, RenamePlan, SupportRename, execute_rename_sync, plan_rename, plan_rename_for};
}
