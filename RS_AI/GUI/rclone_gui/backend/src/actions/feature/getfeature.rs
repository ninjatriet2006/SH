/*
[INTEGRITY NOTES]
- Mục đích: Tầng Actions — ÁNH XẠ cờ native của rclone (`backend features`
  → struct typed đủ 52 cờ). Chỉ lấy + bóc, KHÔNG quyết định gì.
- Trách nhiệm: `BackendFeatures` 1-1 với JSON rclone; `parse_feature_flags`
  thuần; `query_feature_flags` chạy lệnh thật. Mọi tổ hợp/quyết định nằm ở
  `super::combinefeature`.
- Tương tác: `api::remote_manager::get_feature_flags` gọi xuống.
*/

use crate::core::rclone_caller;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// UNIVERSAL: toàn bộ 52 cờ `Features` của rclone — model 1-1, không lược.
/// Thiếu key/lệch kiểu → `false` (rclone thêm cờ mới không vỡ parse cũ).
/// Hai ngoại lệ đặt tên: `Move` (từ khóa Rust) và `BucketBasedRootOK` (hoa K).
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

/// UNIVERSAL: bóc object `Features` thành struct đủ 52 cờ (thuần, test được).
/// Rác/thiếu key → `false` từng cờ, không lỗi cả cụm.
pub fn parse_feature_flags(features: &Value) -> BackendFeatures {
    serde_json::from_value(features.clone()).unwrap_or_default()
}

/// Features backend của một remote (JSON THÔ để hiển thị; đồng bộ, api bọc `fastlane`).
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

/// Toàn bộ 52 cờ `Features` của một remote (struct typed; đồng bộ, api bọc `fastlane`).
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        let back = serde_json::to_value(f).expect("serialize");
        let n = back.as_object().map(|o| o.len()).unwrap_or(0);
        println!("[REAL feature-flags] local: {n} co");
        println!("  - Move={} DirMove={} Copy={} Purge={} CleanUp={} ServerSide={} IsLocal={}",
            f.move_native, f.dir_move, f.copy, f.purge, f.clean_up,
            f.server_side_across_configs, f.is_local);
        assert_eq!(n, 52, "thieu/gian lan co so voi rclone");
        assert!(f.move_native && f.dir_move);
        assert!(!f.copy && !f.purge && !f.clean_up && !f.server_side_across_configs);
        assert!(f.is_local);
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
        let n = serde_json::to_value(f).expect("serialize").as_object().map(|o| o.len()).unwrap_or(0);
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
