/*
[INTEGRITY NOTES]
- Mục đích: Não chung DUY NHẤT cho năng lực move/copy (Route + Cap + check_cap).
- Trách nhiệm: Phân tuyến (Route) → chọn Cap; THUẦN tính toán, KHÔNG thực thi,
  KHÔNG spawn rclone transfer (chỉ đọc cờ engine + `config dump` cho cùng-hãng).
- Tương tác: UI hỏi (qua `checkfeature::query_transfer_options`) + đường chạy
  hỏi (`logic::queue`/`logic::jobs`) đều gọi `check_cap` ở đây; `move_op` chỉ
  lo chạy (`execute_move`), import Cap/Route từ đây khi cần.
*/
// UNIVERSAL: checkcap là não chung duy nhất — move chỉ lo chạy.

pub use crate::actions::types::DeleteScope;
use crate::actions::types::SameProvider;

/// Tuyến di chuyển, suy từ cặp (src_remote, dst_remote).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    LocalLocal,
    LocalCloud,
    CloudLocal,
    SameCloud,
    DiffCloud,
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
            // UNIVERSAL: khác backend cùng hãng + bật cờ thì server-side
            // xuyên-config (`--server-side-across-configs`); khác hãng/tắt cờ
            // giữ nguyên trung chuyển qua máy.
            Self::DiffCloud => Cap {
                server_side: diff_server_side,
                sudo_fallback: false,
                support_move: false,
                support_copy_and_delete: false,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
        }
    }
}

/// UNIVERSAL: check_cap là não chung duy nhất cho move/copy — UI hỏi
/// (qua `checkfeature::query_transfer_options`) + đường chạy hỏi
/// (`logic::queue`/`logic::jobs`) đều gọi đây; THUẦN tính toán, KHÔNG thực thi move.
pub fn check_cap(src: &str, dst: &str) -> Cap {
    let across_enabled = crate::settings::engine::load_engine_flags()
        .map(|f| f.switches.server_side_across)
        .unwrap_or(false);
    check_cap_with_flags(src, dst, across_enabled)
}

/// UNIVERSAL: lõi của `check_cap` với cờ xuyên-config bơm vào (để `jobs`
/// tái dùng mà không đọc đĩa lần nữa); vẫn hỏi `config dump` cho cùng-hãng.
pub fn check_cap_with_flags(src: &str, dst: &str, across_enabled: bool) -> Cap {
    let (src_remote, _) = crate::core::path::cut_remote_path(src);
    let (dst_remote, _) = crate::core::path::cut_remote_path(dst);
    let route = Route::classify(&src_remote, &dst_remote);
    let same = SameProvider(crate::actions::types::same_provider(&src_remote, &dst_remote));
    route.cap_with_provider(same, across_enabled)
}

/// UNIVERSAL: TrashCap là não chung DUY NHẤT cho thùng rác remote —
/// `trash_list`/`trash_restore`/`trash_delete` đều hỏi đây, THUẦN tính toán.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrashCap {
    /// Cờ `--<backend>-trashed-only` để nhắm đúng bản trong thùng rác.
    pub trashed_only: Option<&'static str>,
    /// Có khôi phục được (`backend untrash`) hay không.
    pub can_restore: bool,
    /// Có dọn sạch được (`cleanup`/`CleanUp`) hay không.
    pub can_cleanup: bool,
}

/// UNIVERSAL: 1 não cho năng lực thùng rác theo loại backend.
pub fn check_trash_cap(backend_type: &str) -> TrashCap {
    match backend_type {
        // UNIVERSAL: Drive đủ cả 3 (xem + khôi phục + dọn sạch).
        "drive" => TrashCap {
            trashed_only: Some("--drive-trashed-only"),
            can_restore: true,
            can_cleanup: true,
        },
        // UNIVERSAL: Jottacloud/PikPak xem + dọn sạch, không khôi phục.
        "jottacloud" => TrashCap {
            trashed_only: Some("--jottacloud-trashed-only"),
            can_restore: false,
            can_cleanup: true,
        },
        "pikpak" => TrashCap {
            trashed_only: Some("--pikpak-trashed-only"),
            can_restore: false,
            can_cleanup: true,
        },
        // UNIVERSAL: còn lại không có khái niệm thùng rác trong rclone.
        other => {
            crate::core::debug::warn(None, "checkcap/check_trash_cap", format!("backend lạ '{other}', rớt về không-trash"));
            TrashCap {
                trashed_only: None,
                can_restore: false,
                can_cleanup: false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::types::SameProvider;

    #[test]
    fn move_diffcloud_cap_same_vs_diff_provider() {
        // UNIVERSAL: cùng hãng + bật cờ → server-side xuyên-config.
        assert!(Route::DiffCloud.cap_with_provider(SameProvider(true), true).server_side);
        // UNIVERSAL: khác hãng / tắt cờ → giữ nguyên trung chuyển qua local.
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(false), true).server_side);
        assert!(!Route::DiffCloud.cap_with_provider(SameProvider(true), false).server_side);
        assert!(!Route::DiffCloud.cap().server_side);
    }

    #[test]
    fn trash_cap_covers_supported_backends() {
        // UNIVERSAL: drive đủ cả 3; jotta/pikpak thiếu restore; còn lại đều không.
        let d = check_trash_cap("drive");
        assert_eq!(d.trashed_only, Some("--drive-trashed-only"));
        assert!(d.can_restore && d.can_cleanup);
        let j = check_trash_cap("jottacloud");
        assert_eq!(j.trashed_only, Some("--jottacloud-trashed-only"));
        assert!(!j.can_restore && j.can_cleanup);
        let p = check_trash_cap("pikpak");
        assert_eq!(p.trashed_only, Some("--pikpak-trashed-only"));
        assert!(!p.can_restore && p.can_cleanup);
        for b in ["dropbox", "onedrive", "s3", ""] {
            let c = check_trash_cap(b);
            assert_eq!(c.trashed_only, None);
            assert!(!c.can_restore && !c.can_cleanup);
        }
    }
}
