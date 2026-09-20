/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc logic `fs_move` + `run_transfer_task(..., "moveto", ...)` thành `execute_move`.
- Trách nhiệm: Phân tuyến (Route) → chọn Cap → chạy `moveto`; `fs_move` cũ giữ nguyên hành vi.
- Tương tác: Gọi `logic::file_ops::parse_remote_path`, `core::rclone::build_target`,
  `logic::{transfer, file_ops}`. Không đụng copy/ipc/frontend.
*/

pub use crate::actions::types::DeleteScope;
use crate::logic::app_state::AppState;

/// Tuyến di chuyển, suy từ cặp (src_remote, dst_remote).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    LocalLocal,
    LocalCloud,
    CloudLocal,
    SameCloud,
    DiffCloud,
}

/// Loại transfer mà module này phục vụ (giữ chỗ để S2 gộp copy/move chung).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Move,
}

/// Năng lực gợi ý cho từng tuyến (hiện chỉ mang tính tài liệu; chưa đổi cờ rclone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap {
    /// Có thể server-side (không tải qua máy local) hay không.
    pub server_side: bool,
    /// Có cần sudo fallback (`pkexec mv`) khi lỗi quyền hay không.
    pub sudo_fallback: bool,
    /// Backend hỗ trợ move-native (rclone `Move`/`DirMove`).
    pub support_move: bool,
    /// Backend hỗ trợ fallback copy + purge (`Copy` + `Purge`).
    pub support_copy_and_delete: bool,
    /// Phạm vi xóa nguồn khi fallback.
    pub delete_scope: DeleteScope,
}

/// UNIVERSAL: move-native 1 bước — backend có `Move` hoặc `DirMove` nên
/// `rclone moveto` đổi tên tại chỗ, không cần tải lại dữ liệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportMove(pub bool);

/// UNIVERSAL: copy + purge fallback 2 bước — backend thiếu move-native nên
/// phải `Copy` rồi `Purge` nguồn; chỉ khả dụng khi cả hai cờ đều bật.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportCopyAndDelete(pub bool);

impl Cap {
    /// Suy năng lực từ cờ backend `Move`/`DirMove`/`Copy`/`Purge`.
    pub fn from_backend_features(mv: bool, dir_mv: bool, copy: bool, purge: bool) -> Self {
        let support_move = SupportMove(mv || dir_mv);
        let support_copy_and_delete = SupportCopyAndDelete(copy && purge);
        Self {
            server_side: false,
            sudo_fallback: false,
            support_move: support_move.0,
            support_copy_and_delete: support_copy_and_delete.0,
            // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa;
            // Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
            delete_scope: DeleteScope::NoTrash,
        }
    }
}

impl Route {
    /// Phân tuyến từ tên remote đã parse (`"Local"` = ổ máy).
    pub fn classify(src_remote: &str, dst_remote: &str) -> Self {
        match (src_remote == "Local", dst_remote == "Local", src_remote == dst_remote) {
            // UNIVERSAL: cả hai đầu là ổ máy — rclone `moveto` + fallback `pkexec mv`.
            (true, true, _) => Self::LocalLocal,
            // UNIVERSAL: đẩy từ đĩa lên cloud — upload đơn thuần, không sudo.
            (true, false, _) => Self::LocalCloud,
            // UNIVERSAL: kéo từ cloud về đĩa — download đơn thuần, không sudo.
            (false, true, _) => Self::CloudLocal,
            // UNIVERSAL: cùng một remote — backend thường move server-side nhanh.
            (false, false, true) => Self::SameCloud,
            // UNIVERSAL: khác remote cloud — phải re-upload qua băng thông máy local.
            (false, false, false) => Self::DiffCloud,
        }
    }

    /// Năng lực gợi ý cho từng tuyến.
    pub fn cap(self) -> Cap {
        match self {
            // UNIVERSAL: Local↔Local đi qua syscall/rename; server-side vô nghĩa,
            // nhưng cần sudo fallback khi dính Permission Denied.
            Self::LocalLocal => Cap {
                server_side: false,
                sudo_fallback: true,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Local→Cloud là upload; rclone tải lên thẳng backend.
            Self::LocalCloud => Cap {
                server_side: false,
                sudo_fallback: false,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Cloud→Local là download; rclone tải về thẳng đĩa.
            Self::CloudLocal => Cap {
                server_side: false,
                sudo_fallback: false,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: cùng backend có thể rename server-side, không qua local.
            Self::SameCloud => Cap {
                server_side: true,
                sudo_fallback: false,
                support_move: true,
                support_copy_and_delete: true,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: khác backend phải trung chuyển qua máy (trừ khi
            // `--server-side-across-configs` được bật — cờ còn thiếu, xem mod.rs).
            Self::DiffCloud => Cap {
                server_side: false,
                sudo_fallback: false,
                support_move: false,
                support_copy_and_delete: false,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
        }
    }
}

/// Trial S1: thực thi move qua `rclone moveto`, cùng hành vi với `api::files::fs_move`.
///
/// Không đổi cờ rclone; chỉ thay if/else bằng `match` trên [`Route`].
pub async fn execute_move(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    src: String,
    dst: String,
    task_id: Option<u32>,
) -> Result<(), String> {
    use crate::core::rclone;
    use crate::logic::{file_ops, transfer};

    let (src_remote, src_real) = file_ops::parse_remote_path(&src);
    let (dst_remote, dst_real) = file_ops::parse_remote_path(&dst);
    let route = Route::classify(&src_remote, &dst_remote);
    let _kind = TransferKind::Move;
    let _cap = route.cap();
    // UNIVERSAL: fallback NoTrash tường minh — purge nguồn khớp ngữ nghĩa moveto; Trash chỉ dùng khi move-an-toàn.
    let _delete_scope = _cap.delete_scope;

    let src_target = rclone::build_target(&src_remote, &src_real);
    let dst_target = rclone::build_target(&dst_remote, &dst_real);

    let result = transfer::run_transfer_task(app_handle, state, "moveto", src_target, dst_target, task_id).await;

    match route {
        // UNIVERSAL: Local→Local — `moveto` thất bại do quyền thì thử `pkexec mv`.
        Route::LocalLocal => match result {
            Ok(()) => Ok(()),
            Err(e) => {
                file_ops::run_with_sudo_fallback("Local", "mv", &[src_real.clone(), dst_real.clone()], || Err(e))
            }
        },
        // UNIVERSAL: Local→Cloud — upload, lỗi trả thẳng về UI (có progress task).
        Route::LocalCloud => result,
        // UNIVERSAL: Cloud→Local — download, lỗi trả thẳng về UI (có progress task).
        Route::CloudLocal => result,
        // UNIVERSAL: cùng cloud — move server-side, lỗi trả thẳng về UI.
        Route::SameCloud => result,
        // UNIVERSAL: khác cloud — trung chuyển qua local, lỗi trả thẳng về UI.
        Route::DiffCloud => result,
    }
}
