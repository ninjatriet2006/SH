/*
[INTEGRITY NOTES]
- Mục đích: Thợ kiểm tra năng lực backend (S2) — logic gọi rclone backend/features + năng lực move/copy.
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do tầng `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::actions::checkcap::check_cap;
use crate::core::rclone_caller;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// UNIVERSAL: toàn bộ 52 cờ `Features` của `rclone backend features` — model
/// 1-1 với rclone gốc (không lược). Thiếu key/lệch kiểu → `false` (rclone thêm
/// cờ mới không làm vỡ parse cũ). Hai ngoại lệ đặt tên: `Move` (từ khóa Rust →
/// `move_native`) và `BucketBasedRootOK` (rclone viết hoa K).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BackendFeatures {
    #[serde(default)] pub about: bool,
    #[serde(default)] pub bucket_based: bool,
    #[serde(rename = "BucketBasedRootOK", default)] pub bucket_based_root_ok: bool,
    #[serde(default)] pub can_have_empty_directories: bool,
    #[serde(default)] pub case_insensitive: bool,
    #[serde(default)] pub change_notify: bool,
    #[serde(default)] pub chunk_writer_doesnt_seek: bool,
    #[serde(default)] pub clean_up: bool,
    #[serde(default)] pub command: bool,
    #[serde(default)] pub copy: bool,
    #[serde(default)] pub dir_cache_flush: bool,
    #[serde(default)] pub dir_mod_time_updates_on_write: bool,
    #[serde(default)] pub dir_move: bool,
    #[serde(default)] pub dir_set_mod_time: bool,
    #[serde(default)] pub disconnect: bool,
    #[serde(default)] pub double_slash: bool,
    #[serde(default)] pub duplicate_files: bool,
    #[serde(default)] pub filter_aware: bool,
    #[serde(default)] pub get_tier: bool,
    #[serde(default)] pub is_local: bool,
    #[serde(default)] pub list_p: bool,
    #[serde(default)] pub list_r: bool,
    #[serde(default)] pub merge_dirs: bool,
    #[serde(default)] pub mkdir_metadata: bool,
    #[serde(rename = "Move", default)] pub move_native: bool,
    #[serde(default)] pub no_multi_threading: bool,
    #[serde(default)] pub open_chunk_writer: bool,
    #[serde(default)] pub open_writer_at: bool,
    #[serde(default)] pub overlay: bool,
    #[serde(default)] pub partial_uploads: bool,
    #[serde(default)] pub public_link: bool,
    #[serde(default)] pub purge: bool,
    #[serde(default)] pub put_stream: bool,
    #[serde(default)] pub put_unchecked: bool,
    #[serde(default)] pub read_dir_metadata: bool,
    #[serde(default)] pub read_metadata: bool,
    #[serde(default)] pub read_mime_type: bool,
    #[serde(default)] pub server_side_across_configs: bool,
    #[serde(default)] pub set_tier: bool,
    #[serde(default)] pub set_wrapper: bool,
    #[serde(default)] pub shutdown: bool,
    #[serde(default)] pub slow_hash: bool,
    #[serde(default)] pub slow_mod_time: bool,
    #[serde(default)] pub un_wrap: bool,
    #[serde(default)] pub user_dir_metadata: bool,
    #[serde(default)] pub user_info: bool,
    #[serde(default)] pub user_metadata: bool,
    #[serde(default)] pub wrap_fs: bool,
    #[serde(default)] pub write_dir_metadata: bool,
    #[serde(default)] pub write_dir_set_mod_time: bool,
    #[serde(default)] pub write_metadata: bool,
    #[serde(default)] pub write_mime_type: bool,
}

impl BackendFeatures {
    /// UNIVERSAL: backend đổi tên tại chỗ được (Move hoặc DirMove).
    pub fn support_move(&self) -> bool {
        self.move_native || self.dir_move
    }
    /// UNIVERSAL: backend sao chép tại chỗ được (cờ `Copy` gốc).
    pub fn support_copy(&self) -> bool {
        self.copy
    }
    /// UNIVERSAL: backend dọn được (Purge) — fallback copy+purge cho move.
    pub fn support_purge(&self) -> bool {
        self.purge
    }
    /// UNIVERSAL: backend dọn sạch thùng rác được (CleanUp).
    pub fn support_cleanup(&self) -> bool {
        self.clean_up
    }
}

/// UNIVERSAL: bóc object `Features` thành struct đủ 52 cờ (thuần, test được).
/// Rác/thiếu key → `false` từng cờ, không lỗi cả cụm.
pub fn parse_feature_flags(features: &Value) -> BackendFeatures {
    serde_json::from_value(features.clone()).unwrap_or_default()
}

/// Toàn bộ 52 cờ `Features` của một remote (đồng bộ; tầng api bọc `fastlane`).
/// Lỗi rclone/JSON thiếu `Features` → `Err` rõ (không đoán).
pub fn query_feature_flags(remote: &str) -> Result<BackendFeatures, String> {
    // UNIVERSAL: đuôi ":" báo cho rclone biết đây là một remote.
    let remote_with_colon = format!("{}:", remote);
    let output = rclone_caller::run_cmd(&["backend", "features", &remote_with_colon])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    let root: Value =
        serde_json::from_str(&json_str).map_err(|e| format!("Lỗi phân tích JSON: {}", e))?;
    root.get("Features")
        .map(parse_feature_flags)
        .ok_or_else(|| format!("Thiếu object Features cho remote '{remote}'"))
}

/// Features backend của một remote (đồng bộ; tầng api bọc `fastlane`).
pub fn query_backend_features(remote: &str) -> Result<Value, String> {
    // UNIVERSAL: đuôi ":" báo cho rclone biết đây là một remote.
    let remote_with_colon = format!("{}:", remote);
    let output = rclone_caller::run_cmd(&["backend", "features", &remote_with_colon])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone: {}", err_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| format!("Lỗi phân tích JSON: {}", e))
}

/// Năng lực move/copy giữa 2 đường dẫn (đồng bộ; tầng api bọc `fastlane`).
// UNIVERSAL: check_cap là não chung (UI hỏi + đường chạy hỏi); đây chỉ dịch
// Cap ra 3 cờ JSON cũ (key giữ nguyên cho frontend/IPC).
pub fn query_transfer_options(src: &str, dst: &str) -> Result<Value, String> {
    let cap = check_cap(src, dst);
    Ok(json!({
        "canMove": cap.support_move,
        "canCopy": cap.support_move || cap.support_copy_and_delete,
        "canCopyDelete": cap.support_copy_and_delete
    }))
}

/// Tên cũ giữ lại cho tương thích (IPC/frontend không đổi).
pub fn transfer_capability(src: &str, dst: &str) -> Result<Value, String> {
    query_transfer_options(src, dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_local_same_remote_allows_move_copy() {
        // UNIVERSAL: cùng Local luôn move-native + copy, không cần hỏi backend.
        let v = transfer_capability("Local::/a", "Local::/b").expect("capability");
        assert_eq!(v.get("canMove").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("canCopy").and_then(|x| x.as_bool()), Some(true));
    }

    #[test]
    fn capability_cross_remote_denies_all() {
        // UNIVERSAL: não chung check_cap — DiffCloud khác hãng/tắt cờ thì
        // không move-native lẫn copy-purge (trung chuyển qua local).
        let v = transfer_capability("A::/a", "B::/b").expect("capability");
        assert_eq!(v.get("canMove").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopy").and_then(|x| x.as_bool()), Some(false));
        assert_eq!(v.get("canCopyDelete").and_then(|x| x.as_bool()), Some(false));
    }

    /// Snapshot THẬT (đóng băng từ `rclone backend features /tmp`): đủ 52 cờ,
    /// local có Move/DirMove nhưng KHÔNG Copy/Purge/CleanUp/ServerSide.
    fn real_local_features_json() -> Value {
        serde_json::from_str(r#"{"About":true,"BucketBased":false,"BucketBasedRootOK":false,"CanHaveEmptyDirectories":true,"CaseInsensitive":false,"ChangeNotify":false,"ChunkWriterDoesntSeek":false,"CleanUp":false,"Command":true,"Copy":false,"DirCacheFlush":false,"DirModTimeUpdatesOnWrite":true,"DirMove":true,"DirSetModTime":true,"Disconnect":false,"DoubleSlash":false,"DuplicateFiles":false,"FilterAware":true,"GetTier":false,"IsLocal":true,"ListP":false,"ListR":false,"MergeDirs":false,"MkdirMetadata":true,"Move":true,"NoMultiThreading":false,"OpenChunkWriter":false,"OpenWriterAt":true,"Overlay":false,"PartialUploads":true,"PublicLink":false,"Purge":false,"PutStream":true,"PutUnchecked":false,"ReadDirMetadata":true,"ReadMetadata":true,"ReadMimeType":false,"ServerSideAcrossConfigs":false,"SetTier":false,"SetWrapper":false,"Shutdown":false,"SlowHash":true,"SlowModTime":false,"UnWrap":false,"UserDirMetadata":true,"UserInfo":false,"UserMetadata":true,"WrapFs":false,"WriteDirMetadata":true,"WriteDirSetModTime":true,"WriteMetadata":true,"WriteMimeType":false}"#).expect("snapshot that hop le")
    }

    #[test]
    fn feature_flags_parse_full_real_snapshot() {
        // UNIVERSAL: snapshot thật phải ra đủ 52 cờ (serialize lại đếm đúng 52
        // key) + giá trị các cờ quyết định đúng như rclone báo.
        let f = parse_feature_flags(&real_local_features_json());
        let back = serde_json::to_value(&f).expect("serialize");
        let n = back.as_object().map(|o| o.len()).unwrap_or(0);
        println!("[REAL feature-flags] local: {n} co");
        println!("  - Move={} DirMove={} Copy={} Purge={} CleanUp={} ServerSide={} IsLocal={}",
            f.move_native, f.dir_move, f.copy, f.purge, f.clean_up,
            f.server_side_across_configs, f.is_local);
        assert_eq!(n, 52, "thieu/gian lan co so voi rclone");
        assert!(f.move_native && f.dir_move);
        assert!(!f.copy && !f.purge && !f.clean_up && !f.server_side_across_configs);
        assert!(f.is_local);
        // Accessor quyết định đọc đúng cờ gốc.
        assert!(f.support_move());
        assert!(!f.support_copy() && !f.support_purge() && !f.support_cleanup());
    }

    #[test]
    fn feature_flags_tolerant_to_missing_and_junk() {
        // UNIVERSAL: thiếu key/rác/khác kiểu → false từng cờ, không lỗi cả cụm;
        // key lạ (rclone tương lai) bị bỏ qua im lặng.
        let empty = parse_feature_flags(&json!({}));
        assert_eq!(empty, BackendFeatures::default());
        let junk = parse_feature_flags(&json!({"Move": "yes", "Copy": 1, "FutureFlag": true}));
        assert!(!junk.move_native && !junk.copy);
        let partial = parse_feature_flags(&json!({"Move": true}));
        assert!(partial.move_native && !partial.copy);
    }

    #[test]
    fn live_local_features_parse_all_flags() {
        // UNIVERSAL KIỂM THỰC DỤNG: hỏi rclone thật (`backend features /tmp`,
        // không cần remote cấu hình), IN các cờ quyết định ra để đối chiếu.
        let output = match crate::core::rclone_caller::run_cmd(&["backend", "features", "/tmp"]) {
            Ok(o) if o.status.success() => o,
            _ => {
                println!("SKIP: máy này không chạy được rclone backend features");
                return;
            }
        };
        let root: Value = serde_json::from_slice(&output.stdout).expect("JSON that tu rclone");
        let features = root.get("Features").expect("co object Features");
        let f = parse_feature_flags(features);
        let n = serde_json::to_value(&f).expect("serialize").as_object().map(|o| o.len()).unwrap_or(0);
        println!("[REAL live-features] /tmp: {n} co");
        println!("  - Move={} DirMove={} Copy={} Purge={} CleanUp={} ServerSide={} IsLocal={}",
            f.move_native, f.dir_move, f.copy, f.purge, f.clean_up,
            f.server_side_across_configs, f.is_local);
        assert_eq!(n, 52, "rclone doi so co -> struct thieu, can bo sung");
        // Bất biến của backend local trên mọi máy/rclone: move được, copy/purge
        // native không, xuyên-config không.
        assert!(f.move_native && f.dir_move && f.is_local);
        assert!(!f.copy && !f.purge && !f.server_side_across_configs);
    }
}
