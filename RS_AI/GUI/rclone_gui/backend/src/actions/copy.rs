/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc logic `fs_copy` + `run_transfer_task(..., "copyto", ...)` thành `execute_copy`.
- Trách nhiệm: Phân tuyến (Route) → chọn Cap → chạy `copyto`; `fs_copy` cũ giữ nguyên hành vi.
- Tương tác: Gọi `logic::file_ops::parse_remote_path`, `core::rclone::build_target`,
  `logic::{transfer, file_ops}`. Dùng chung `delete_op::DeleteScope`. Không đụng move/ipc/frontend.
*/

use crate::actions::types::DeleteScope;
use crate::logic::app_state::AppState;

/// Tuyến sao chép, suy từ cặp (src_remote, dst_remote).
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
    Copy,
}

/// Năng lực gợi ý cho từng tuyến (hiện chỉ mang tính tài liệu; chưa đổi cờ rclone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cap {
    /// Có thể server-side (không tải qua máy local) hay không.
    pub server_side: bool,
    /// Có cần sudo fallback (`pkexec cp`) khi lỗi quyền hay không.
    pub sudo_fallback: bool,
    /// Backend hỗ trợ copy (`Copy`).
    pub support_copy: bool,
    /// Phạm vi xóa nguồn — copy giữ nguồn nên luôn `NoTrash` (tài liệu cho S2).
    pub delete_scope: DeleteScope,
}

/// UNIVERSAL: copy 1 bước — backend có `Copy` nên `rclone copyto` sao chép tại
/// chỗ (cùng backend thì server-side, khác backend thì trung chuyển qua local).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportCopy(pub bool);

impl Cap {
    /// Suy năng lực từ cờ backend `Copy`.
    pub fn from_backend_features(copy: bool) -> Self {
        let support_copy = SupportCopy(copy);
        Self {
            server_side: false,
            sudo_fallback: false,
            support_copy: support_copy.0,
            // UNIVERSAL: copy giữ nguồn nên không xóa gì; NoTrash tường minh để S2 gộp chung CopyAndDelete.
            delete_scope: DeleteScope::NoTrash,
        }
    }
}

impl Route {
    /// Phân tuyến từ tên remote đã parse (`"Local"` = ổ máy).
    pub fn classify(src_remote: &str, dst_remote: &str) -> Self {
        match (src_remote == "Local", dst_remote == "Local", src_remote == dst_remote) {
            // UNIVERSAL: cả hai đầu là ổ máy — rclone `copyto` + fallback `pkexec cp -r`.
            (true, true, _) => Self::LocalLocal,
            // UNIVERSAL: đẩy từ đĩa lên cloud — upload đơn thuần, không sudo.
            (true, false, _) => Self::LocalCloud,
            // UNIVERSAL: kéo từ cloud về đĩa — download đơn thuần, không sudo.
            (false, true, _) => Self::CloudLocal,
            // UNIVERSAL: cùng một remote — backend thường copy server-side nhanh.
            (false, false, true) => Self::SameCloud,
            // UNIVERSAL: khác remote cloud — phải re-upload qua băng thông máy local.
            (false, false, false) => Self::DiffCloud,
        }
    }

    /// Năng lực gợi ý cho từng tuyến.
    pub fn cap(self) -> Cap {
        match self {
            // UNIVERSAL: Local↔Local đi qua syscall; server-side vô nghĩa,
            // nhưng cần sudo fallback khi dính Permission Denied.
            Self::LocalLocal => Cap {
                server_side: false,
                sudo_fallback: true,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Local→Cloud là upload; rclone tải lên thẳng backend.
            Self::LocalCloud => Cap {
                server_side: false,
                sudo_fallback: false,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Cloud→Local là download; rclone tải về thẳng đĩa.
            Self::CloudLocal => Cap {
                server_side: false,
                sudo_fallback: false,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: cùng backend có thể copy server-side, không qua local.
            Self::SameCloud => Cap {
                server_side: true,
                sudo_fallback: false,
                support_copy: true,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: khác backend phải trung chuyển qua máy (trừ khi
            // `--server-side-across-configs` được bật — cờ còn thiếu, xem mod.rs).
            Self::DiffCloud => Cap {
                server_side: false,
                sudo_fallback: false,
                support_copy: false,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
        }
    }
}

/// Trial S1: thực thi copy qua `rclone copyto`, cùng hành vi với `api::files::fs_copy`.
///
/// Không đổi cờ rclone; chỉ thay if/else bằng `match` trên [`Route`].
pub async fn execute_copy(
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
    let _kind = TransferKind::Copy;
    let _cap = route.cap();
    // UNIVERSAL: copy giữ nguồn nên NoTrash tường minh — khớp ngữ nghĩa copyto.
    let _delete_scope = _cap.delete_scope;

    let src_target = rclone::build_target(&src_remote, &src_real);
    let dst_target = rclone::build_target(&dst_remote, &dst_real);

    let result = transfer::run_transfer_task(app_handle, state, "copyto", src_target, dst_target, task_id).await;

    match route {
        // UNIVERSAL: Local→Local — `copyto` thất bại do quyền thì thử `pkexec cp -r`.
        Route::LocalLocal => match result {
            Ok(()) => Ok(()),
            Err(e) => {
                file_ops::run_with_sudo_fallback("Local", "cp", &[src_real.clone(), dst_real.clone()], || Err(e))
            }
        },
        // UNIVERSAL: Local→Cloud — upload, lỗi trả thẳng về UI (có progress task).
        Route::LocalCloud => result,
        // UNIVERSAL: Cloud→Local — download, lỗi trả thẳng về UI (có progress task).
        Route::CloudLocal => result,
        // UNIVERSAL: cùng cloud — copy server-side, lỗi trả thẳng về UI.
        Route::SameCloud => result,
        // UNIVERSAL: khác cloud — trung chuyển qua local, lỗi trả thẳng về UI.
        Route::DiffCloud => result,
    }
}
