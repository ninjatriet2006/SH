/*
[INTEGRITY NOTES]
- Mục đích: Route + Cap cho copy + THỰC THI copy (nơi duy nhất gọi rclone copy).
- Trách nhiệm: Phân tuyến (Route) → chọn Cap; `execute_copy` NHẬN VÉ ĐÃ ĐÓNG DẤU
  (`TransferTicket`: src/dst/rel/mode/across/engine_flags/policy — không đọc
  engine settings), tự ráp lệnh + hứng log + tôn trọng cờ hủy + `perm::escalate`.
- Tương tác: `logic::queue` gọi `execute_copy` với vé + `should_cancel` +
  sink dòng thô (`on_log_line`); máy gọi rclone dùng chung nằm ở
  `super::rclone_stream` (copy chỉ còn thực thi + Route/Cap).
  Không AppHandle/State (worker sync, test sync).
*/
// UNIVERSAL: actions tay trắng về % — chỉ GỌI rclone (ráp lệnh + phun dòng
// log THÔ vào sink) + hủy êm qua `logic::process::terminate_gracefully` +
// Local↔Local qua `perm::escalate`. Tính % là việc DUY NHẤT của tracker,
// queue hỏi tracker rồi ghi vào Job.

use crate::actions::types::{DeleteScope, SameProvider};
use crate::logic::tracker::TransferTicket;

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

    /// Năng lực gợi ý cho từng tuyến (tắt cờ xuyên-config → giữ hành vi cũ).
    pub fn cap(self) -> Cap {
        // UNIVERSAL: đường cũ — coi như khác hãng, DiffCloud không server-side.
        self.cap_with_provider(SameProvider(false), false)
    }

    /// UNIVERSAL: năng lực theo tuyến × cùng-hãng × cờ xuyên-config; DiffCloud
    /// `server_side = same_provider && across_enabled`, các tuyến khác giữ nguyên.
    pub fn cap_with_provider(self, same: SameProvider, across_enabled: bool) -> Cap {
        let diff_server_side = same.0 && across_enabled;
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
            // UNIVERSAL: khác backend cùng hãng + bật cờ thì server-side
            // xuyên-config (`--server-side-across-configs`); khác hãng/tắt cờ
            // giữ nguyên trung chuyển qua máy.
            Self::DiffCloud => Cap {
                server_side: diff_server_side,
                sudo_fallback: false,
                support_copy: false,
                // UNIVERSAL: copy giữ nguồn nên NoTrash; Trash chỉ dùng khi copy-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_diffcloud_cap_same_vs_diff_provider() {
        // UNIVERSAL: cùng hãng + bật cờ → server-side xuyên-config.
        assert!(Route::DiffCloud.cap_with_provider(SameProvider(true), true).server_side);
        // UNIVERSAL: khác hãng / tắt cờ → giữ nguyên trung chuyển qua local.
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(false), true).server_side);
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(true), false).server_side);
        assert!(!Route::DiffCloud.cap().server_side);
    }
}

/// UNIVERSAL: thực thi copy từ VÉ ĐÃ ĐÓNG DẤU (worker sync, test sync) —
/// không đọc engine settings/AppHandle/State; phun log thô qua `on_log_line`
/// (queue đưa vào tracker để tính %), hủy hợp tác qua `should_cancel`.
/// Máy gọi rclone dùng chung nằm ở [`crate::actions::rclone_stream::exec`].
pub fn execute_copy(
    ticket: TransferTicket,
    should_cancel: impl Fn() -> bool,
    on_log_line: impl FnMut(&str),
) -> Result<(), String> {
    crate::actions::rclone_stream::exec("copyto", "cp", &ticket, should_cancel, on_log_line)
}
