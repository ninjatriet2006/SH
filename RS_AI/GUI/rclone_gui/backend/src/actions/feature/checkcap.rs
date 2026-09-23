/*
[INTEGRITY NOTES]
- Mục đích: Não chung DUY NHẤT cho năng lực move/copy (Route + Cap + check_cap).
- Trách nhiệm: Phân tuyến (Route) → hỏi cờ backend thật (cache) → chọn Cap;
  THUẦN quyết định, KHÔNG thực thi, KHÔNG spawn transfer. `config dump` cho
  cùng-hãng + `backend features` cho cờ native đều đi qua cache/không thì fallback.
- Tương tác: UI hỏi + đường chạy hỏi (`logic::queue`/`logic::jobs`) đều gọi
  `check_cap`/`query_transfer_options` ở đây; `move_op` chỉ lo chạy.
*/
// UNIVERSAL: checkcap là não chung duy nhất — move chỉ lo chạy.

pub use crate::actions::types::DeleteScope;
use crate::actions::feature::getfeature::{BackendFeatures, query_feature_flags};
use crate::actions::types::SameProvider;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// UNIVERSAL: TTL cache cờ backend — hỏi 1 lần/remote rồi dùng lại, tránh
/// spawn `backend features` mỗi vé con (đợt tối ưu queue đã chứng minh N spawn
/// là tự sát). Hết hạn thì hỏi lại để bắt kịp remote đổi cấu hình.
const FEATURE_CACHE_TTL: Duration = Duration::from_secs(300);

/// UNIVERSAL: cache cờ theo tên remote (chỉ giữ lần hỏi THÀNH CÔNG; lỗi thì
/// không ghi để lần sau thử lại, rớt về fallback route-cứng).
fn feature_cache() -> &'static Mutex<HashMap<String, (BackendFeatures, Instant)>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<String, (BackendFeatures, Instant)>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// UNIVERSAL: cờ native của một remote qua cache — `"Local"` và remote lỗi
/// trả `None` ngay (không spawn), để tầng quyết định rớt về fallback.
pub(crate) fn backend_features_cached(remote: &str) -> Option<BackendFeatures> {
    if remote == "Local" || remote.is_empty() {
        return None;
    }
    if let Some((f, at)) = feature_cache().lock().ok().and_then(|c| c.get(remote).copied()) {
        if at.elapsed() < FEATURE_CACHE_TTL {
            return Some(f);
        }
    }
    let fresh = query_feature_flags(remote).ok()?;
    if let Ok(mut cache) = feature_cache().lock() {
        cache.insert(remote.to_string(), (fresh, Instant::now()));
    }
    Some(fresh)
}

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
    /// Backend sao chép tại chỗ được (cờ `Copy` gốc — hồi sinh từ nhà copy).
    pub support_copy: bool,
    /// Backend hỗ trợ fallback copy + purge (`Copy` + `Purge`).
    pub support_copy_and_delete: bool,
    /// Backend dọn sạch thùng rác được (cờ `CleanUp` gốc).
    pub support_cleanup: bool,
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
            support_copy: copy,
            support_copy_and_delete: support_copy_and_delete.0,
            // UNIVERSAL: hàm này không nhận cờ CleanUp nên luôn false —
            // đường chạy đọc `support_cleanup` từ `cap_with_features`.
            support_cleanup: false,
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
                support_copy: false,
                support_copy_and_delete: true,
                support_cleanup: false,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Local→Cloud là upload; rclone tải lên thẳng backend.
            Self::LocalCloud => Cap {
                server_side: false,
                sudo_fallback: false,
                support_move: true,
                support_copy: false,
                support_copy_and_delete: true,
                support_cleanup: false,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: Cloud→Local là download; rclone tải về thẳng đĩa.
            Self::CloudLocal => Cap {
                server_side: false,
                sudo_fallback: false,
                support_move: true,
                support_copy: false,
                support_copy_and_delete: true,
                support_cleanup: false,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
            // UNIVERSAL: cùng backend có thể rename server-side, không qua local.
            Self::SameCloud => Cap {
                server_side: true,
                sudo_fallback: false,
                support_move: true,
                support_copy: false,
                support_copy_and_delete: true,
                support_cleanup: false,
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
                support_copy: false,
                support_copy_and_delete: false,
                support_cleanup: false,
                // UNIVERSAL: moveto dời nguồn hẳn nên fallback purge nguồn khớp ngữ nghĩa; Trash chỉ dùng khi move-an-toàn dọn nguồn qua trash.
                delete_scope: DeleteScope::NoTrash,
            },
        }
    }
}

/// UNIVERSAL: phủ cờ backend THẬT lên Cap route-cứng (thuần, test được).
/// - Cùng backend (SameCloud): một backend nói hết — move/copy/copy_delete/
///   cleanup lấy đúng cờ native của nó.
/// - Khác backend: giữ nguyên đáp án route (tín hiệu định tuyến copy-vs-move,
///   không suy native chéo); chỉ `support_copy`/`support_cleanup` lấy theo DST
///   (đích nhận copy/dọn được thì báo đúng — đúng ca Copy-mà-thiếu-Purge).
/// - Không hỏi được cờ (Local/lỗi/mạng rớt): giữ nguyên route-cứng 100%.
pub fn cap_with_features(
    route: Route,
    same: SameProvider,
    across_enabled: bool,
    dst_f: Option<&BackendFeatures>,
) -> Cap {
    let mut cap = route.cap_with_provider(same, across_enabled);
    if matches!(route, Route::SameCloud) {
        if let Some(f) = dst_f {
            cap.support_move = f.move_native || f.dir_move;
            cap.support_copy = f.copy;
            cap.support_copy_and_delete = f.copy && f.purge;
            cap.support_cleanup = f.clean_up;
        }
    } else if let Some(f) = dst_f {
        cap.support_copy = f.copy;
        cap.support_cleanup = f.clean_up;
    }
    cap
}
/// UNIVERSAL: check_cap là não chung duy nhất cho move/copy — UI hỏi
/// (qua `checkfeature::query_transfer_options`) + đường chạy hỏi
/// (`logic::queue`/`logic::jobs`) đều gọi đây; THUẦN quyết định (cache cờ
/// backend, fallback route-cứng khi không hỏi được).
pub fn check_cap(src: &str, dst: &str) -> Cap {
    let across_enabled = crate::settings::engine::load_engine_flags()
        .map(|f| f.switches.server_side_across)
        .unwrap_or(false);
    check_cap_with_flags(src, dst, across_enabled)
}

/// UNIVERSAL: lõi của `check_cap` với cờ xuyên-config bơm vào (để `jobs`
/// tái dùng mà không đọc đĩa lần nữa); vẫn hỏi `config dump` cho cùng-hãng.
/// Cờ native lấy qua cache theo tên remote (Local/lỗi → None → fallback).
pub fn check_cap_with_flags(src: &str, dst: &str, across_enabled: bool) -> Cap {
    let (src_remote, _) = crate::core::path::cut_remote_path(src);
    let (dst_remote, _) = crate::core::path::cut_remote_path(dst);
    let route = Route::classify(&src_remote, &dst_remote);
    let same = SameProvider(crate::actions::types::same_provider(&src_remote, &dst_remote));
    let dst_f = backend_features_cached(&dst_remote);
    cap_with_features(route, same, across_enabled, dst_f.as_ref())
}

/// Năng lực move/copy giữa 2 đường dẫn (mặt tiền trả lời của cửa quyết định).
/// Dịch `Cap` ra 3 cờ JSON cũ (key giữ nguyên cho frontend/IPC); công thức tổ
/// hợp nằm ở `super::combinefeature::can_copy` — không tự ráp `||` tại chỗ.
pub fn query_transfer_options(src: &str, dst: &str) -> serde_json::Value {
    let cap = check_cap(src, dst);
    let can_copy = super::combinefeature::can_copy(
        cap.support_move,
        cap.support_copy_and_delete,
        cap.support_copy,
    );
    serde_json::json!({
        "canMove": cap.support_move,
        "canCopy": can_copy,
        "canCopyDelete": cap.support_copy_and_delete
    })
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

    /// Snapshot THẬT đóng băng từ `rclone backend features /tmp` (đủ 52 cờ).
    fn real_local_features() -> BackendFeatures {
        crate::actions::feature::getfeature::parse_feature_flags(&serde_json::from_str(r#"{"About":true,"BucketBased":false,"BucketBasedRootOK":false,"CanHaveEmptyDirectories":true,"CaseInsensitive":false,"ChangeNotify":false,"ChunkWriterDoesntSeek":false,"CleanUp":false,"Command":true,"Copy":false,"DirCacheFlush":false,"DirModTimeUpdatesOnWrite":true,"DirMove":true,"DirSetModTime":true,"Disconnect":false,"DoubleSlash":false,"DuplicateFiles":false,"FilterAware":true,"GetTier":false,"IsLocal":true,"ListP":false,"ListR":false,"MergeDirs":false,"MkdirMetadata":true,"Move":true,"NoMultiThreading":false,"OpenChunkWriter":false,"OpenWriterAt":true,"Overlay":false,"PartialUploads":true,"PublicLink":false,"Purge":false,"PutStream":true,"PutUnchecked":false,"ReadDirMetadata":true,"ReadMetadata":true,"ReadMimeType":false,"ServerSideAcrossConfigs":false,"SetTier":false,"SetWrapper":false,"Shutdown":false,"SlowHash":true,"SlowModTime":false,"UnWrap":false,"UserDirMetadata":true,"UserInfo":false,"UserMetadata":true,"WrapFs":false,"WriteDirMetadata":true,"WriteDirSetModTime":true,"WriteMetadata":true,"WriteMimeType":false}"#).expect("snapshot that"))
    }

    /// Snapshot THẬT đóng băng từ `rclone backend features "Box Ninja:"`.
    fn real_box_features() -> BackendFeatures {
        crate::actions::feature::getfeature::parse_feature_flags(&serde_json::from_str(r#"{"About":true,"BucketBased":false,"BucketBasedRootOK":false,"CanHaveEmptyDirectories":true,"CaseInsensitive":true,"ChangeNotify":true,"ChunkWriterDoesntSeek":false,"CleanUp":true,"Command":false,"Copy":true,"DirCacheFlush":true,"DirModTimeUpdatesOnWrite":false,"DirMove":true,"DirSetModTime":false,"Disconnect":false,"DoubleSlash":false,"DuplicateFiles":false,"FilterAware":false,"GetTier":false,"IsLocal":false,"ListP":true,"ListR":false,"MergeDirs":false,"MkdirMetadata":false,"Move":true,"NoMultiThreading":false,"OpenChunkWriter":false,"OpenWriterAt":false,"Overlay":false,"PartialUploads":false,"PublicLink":true,"Purge":true,"PutStream":true,"PutUnchecked":true,"ReadDirMetadata":false,"ReadMetadata":false,"ReadMimeType":false,"ServerSideAcrossConfigs":false,"SetTier":false,"SetWrapper":false,"Shutdown":true,"SlowHash":false,"SlowModTime":false,"UnWrap":false,"UserDirMetadata":false,"UserInfo":false,"UserMetadata":false,"WrapFs":false,"WriteDirMetadata":false,"WriteDirSetModTime":false,"WriteMetadata":false,"WriteMimeType":false}"#).expect("snapshot that"))
    }

    #[test]
    fn grounded_same_backend_reads_real_flags() {
        // UNIVERSAL KIỂM THỰC DỤNG (snapshot thật, in giá trị để đối chiếu):
        // cùng backend thì cờ native quyết — box đủ move/copy/purge/cleanup.
        let b = real_box_features();
        let c = cap_with_features(Route::SameCloud, SameProvider(true), true, Some(&b));
        println!("[REAL grounded box×box] move={} copy={} copy_delete={} cleanup={} across={}",
            c.support_move, c.support_copy, c.support_copy_and_delete, c.support_cleanup, c.server_side);
        assert!(c.support_move && c.support_copy && c.support_copy_and_delete && c.support_cleanup);
        assert!(c.server_side);
    }

    #[test]
    fn grounded_local_falls_back_and_copy_stays_false() {
        // UNIVERSAL: local Copy/Purge native không có → copy/copy_delete false
        // (đúng ca rclone báo); move route-cứng true giữ nguyên.
        let l = real_local_features();
        let c = cap_with_features(Route::SameCloud, SameProvider(true), false, Some(&l));
        println!("[REAL grounded local×local] move={} copy={} copy_delete={} cleanup={}",
            c.support_move, c.support_copy, c.support_copy_and_delete, c.support_cleanup);
        assert!(c.support_move);
        assert!(!c.support_copy && !c.support_copy_and_delete && !c.support_cleanup);
    }

    #[test]
    fn unknown_backend_keeps_route_fallback() {
        // UNIVERSAL: không hỏi được cờ (None) → giữ nguyên đáp án route-cứng
        // 100% (Local và remote lạ không đổi hành vi).
        let local = cap_with_features(Route::LocalLocal, SameProvider(false), false, None);
        assert!(local.support_move && local.support_copy_and_delete);
        assert!(!local.support_copy && !local.support_cleanup);
        let diff = cap_with_features(Route::DiffCloud, SameProvider(false), false, None);
        assert!(!diff.support_move && !diff.support_copy_and_delete && !diff.support_copy);
    }

    #[test]
    fn cross_backend_takes_copy_and_cleanup_from_dst() {
        // UNIVERSAL: khác backend giữ đáp án route, chỉ support_copy/cleanup
        // lấy theo đích (đúng ca Copy-mà-thiếu-Purge: dst có Copy → canCopy).
        let b = real_box_features();
        let c = cap_with_features(Route::DiffCloud, SameProvider(false), false, Some(&b));
        println!("[REAL grounded A×box] move={} copy={} copy_delete={} cleanup={}",
            c.support_move, c.support_copy, c.support_copy_and_delete, c.support_cleanup);
        assert!(!c.support_move && !c.support_copy_and_delete);
        assert!(c.support_copy && c.support_cleanup);
    }

    #[test]
    fn capability_local_same_remote_allows_move_copy() {
        // UNIVERSAL: cùng Local luôn move-native + copy, không cần hỏi backend.
        // `query_transfer_options` không lỗi được (fallback rớt về route-cứng).
        let v = query_transfer_options("Local::/a", "Local::/b");
        assert_eq!(v.get("canMove").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("canCopy").and_then(|x| x.as_bool()), Some(true));
    }

    #[test]
    fn capability_cross_remote_denies_all() {
        // UNIVERSAL: DiffCloud khác hãng/tắt cờ thì không move-native lẫn
        // copy-purge (trung chuyển qua local).
        let v = query_transfer_options("A::/a", "B::/b");
        assert_eq!(v.get("canMove").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopy").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopyDelete").and_then(|x| x.as_bool()), Some(false));
    }
}
