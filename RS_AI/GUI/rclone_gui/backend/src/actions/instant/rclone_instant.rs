/*
[INTEGRITY NOTES]
- Mục đích: Tầng Actions — CỖ MÁY THỰC THI TỨC THỜI (Instant Engine).
- Trách nhiệm:
  + `CreateKind` & `CreatePlan`: Mô hình dữ liệu chuẩn hóa cho thao tác tạo.
  + `plan_create`: Lập kế hoạch thuần túy (parse path, gắn năng lực OpsCap, chọn rclone/syscall).
  + `execute_create_sync`: Cỗ máy thực thi chung đồng bộ cho worker job, leo thang quyền (escalate),
    bắt lỗi rclone/syscall và ghi nhận nhật ký chẩn đoán qua `core::debug`.
- Tương tác: `actions::instant::{mkdir, touch}` gọi `execute_create_sync`. Không chạy qua fastlane.
*/

use crate::actions::OpsCap;
use crate::actions::perm::{Policy, classify_permission_error, escalate};
use crate::actions::types::RemoteKind;
use crate::core::path::cut_remote_path;
use crate::core::rclone_caller;

/// Phân loại đối tượng tạo: thư mục (`Dir`) hoặc tệp rỗng (`File`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateKind {
    Dir,
    File,
}

/// Kế hoạch chuẩn hóa cho thao tác tạo thư mục / tệp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePlan {
    pub kind: CreateKind,
    pub remote: String,
    pub real_path: String,
    pub target: String,
    pub rclone_args: Vec<String>,
    pub sudo_action: Option<&'static str>,
    pub local_create: bool,
}

/// Dựng plan tạo (`Dir` hoặc `File`) từ đường dẫn `Remote::/Path`.
pub fn plan_create(kind: CreateKind, path: &str) -> Result<CreatePlan, String> {
    let (remote, real_path) = cut_remote_path(path);
    if real_path.is_empty() {
        let msg = match kind {
            CreateKind::Dir => "Thiếu đường dẫn cần tạo.",
            CreateKind::File => "Thiếu đường dẫn cần tạo tệp.",
        };
        return Err(msg.to_string());
    }
    let remote_kind = RemoteKind::classify(&remote);
    let cap = OpsCap::of(remote_kind);
    let target = rclone_caller::build_target(&remote, &real_path);

    let (rclone_args, sudo_action, local_create) = match (kind, remote_kind) {
        (CreateKind::Dir, RemoteKind::Local) => (
            vec!["mkdir".to_string(), target.clone()],
            if cap.sudo_fallback { Some("mkdir") } else { None },
            false,
        ),
        (CreateKind::Dir, RemoteKind::Remote) => (
            vec!["mkdir".to_string(), target.clone()],
            None,
            false,
        ),
        (CreateKind::File, RemoteKind::Local) => (
            vec!["touch".to_string(), target.clone()],
            None,
            true,
        ),
        (CreateKind::File, RemoteKind::Remote) => (
            vec!["touch".to_string(), target.clone()],
            None,
            false,
        ),
    };

    Ok(CreatePlan {
        kind,
        remote,
        real_path,
        target,
        rclone_args,
        sudo_action,
        local_create,
    })
}

/// Cỗ máy thực thi chung cho họ tạo (`mkdir` / `touch`):
/// - Chạy đồng bộ trực tiếp (dùng cho worker job hoặc gọi trực tiếp, không bọc fastlane).
/// - Đồng nhất quản lý quyền: Local `File::create` hoặc rclone qua `perm::escalate`.
/// - Ghi nhận nhật ký bắt đầu/kết thúc/lỗi vào `core::debug` (hiển thị trên DebugView).
pub fn execute_create_sync(plan: &CreatePlan, policy: Policy) -> Result<(), String> {
    let kind_str = match plan.kind {
        CreateKind::Dir => "Dir",
        CreateKind::File => "File",
    };
    let target = &plan.target;
    let remote = &plan.remote;
    let real_path = &plan.real_path;
    let sudo_action = plan.sudo_action.unwrap_or("mkdir");
    let is_local_create = plan.local_create;
    let rclone_args = &plan.rclone_args;

    crate::core::debug::info(
        "actions/instant",
        format!("BẮT ĐẦU {} | target='{}'", kind_str, target),
    );
    let start = std::time::Instant::now();

    let res = if is_local_create {
        std::fs::File::create(target).map(|_| ()).map_err(|e| {
            let msg = e.to_string();
            if classify_permission_error(&msg) && policy != Policy::AllowSystem {
                format!("PERMISSION_CONSENT: {}.", msg)
            } else {
                msg
            }
        })
    } else {
        let args_ref: Vec<&str> = rclone_args.iter().map(|s| s.as_str()).collect();
        escalate(policy, remote, sudo_action, std::slice::from_ref(real_path), || {
            let output = rclone_caller::run_cmd(&args_ref)?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).into_owned());
            }
            Ok(())
        })
    };

    match &res {
        Ok(()) => {
            crate::core::debug::info(
                "actions/instant",
                format!("XONG {} | target='{}' ({:.2?})", kind_str, plan.target, start.elapsed()),
            );
        }
        Err(e) => {
            crate::core::debug::error(
                "actions/instant",
                format!("LỖI {} | target='{}' | err={} ({:.2?})", kind_str, plan.target, e, start.elapsed()),
            );
        }
    }

    res
}

