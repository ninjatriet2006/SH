/*
[INTEGRITY NOTES]
- Mục đích: Tầng Actions — TỔ HỢP cờ native (`getfeature::BackendFeatures`)
  thành đáp án quyết định (dời/copy/dọn được không). Chỉ suy luận thuần, KHÔNG
  spawn rclone, KHÔNG đọc settings.
- Trách nhiệm: mọi công thức "cờ rclone → việc làm được" nằm ở đây, 1 nơi duy
  nhất (trước đây rải ở `check_cap` route-cứng + `from_backend_features` bỏ không).
- Tương tác: `check_cap`/`query_transfer_options` (giữ tương thích cũ) sẽ gọi
  sang khi đợt nối dây hoàn thành; hiện test trực tiếp từng tổ hợp.
*/

use super::getfeature::BackendFeatures;

/// UNIVERSAL: backend dời tại chỗ được theo LOẠI nguồn (không gộp OR —
/// `Move` là dời file, `DirMove` là dời thư mục, backend có thể chỉ có một
/// trong hai). Dùng chung cho move-native VÀ rename-native (rename chính là
/// `moveto` cùng remote) — không đẻ hàm riêng trùng bảng chân lý.
pub fn move_splitter(f: &BackendFeatures, is_dir: bool) -> bool {
    if is_dir {
        f.dir_move
    } else {
        f.move_native
    }
}

/// UNIVERSAL: move bằng copy+delete được khi vừa copy vừa purge được.
/// Đây mới là TỔ HỢP thật (AND hai cờ) — khác với đọc thẳng một cờ có sẵn.
pub fn move_by_copy_delete(f: &BackendFeatures) -> bool {
    f.copy && f.purge
}

/// UNIVERSAL: copy được khi một trong ba đường thông — move-native, fallback
/// copy+purge, HOẶC bare-Copy (backend có Copy mà thiếu Purge vẫn copyto được).
/// Công thức 3 nguồn tập trung 1 mối (checkfeature chỉ gọi, không tự ráp `||`).
pub fn can_copy(move_support: bool, copy_delete_support: bool, copy_support: bool) -> bool {
    move_support || copy_delete_support || copy_support
}
























#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::getfeature::parse_feature_flags;
    use serde_json::json;

    #[test]
    fn support_move_splits_file_vs_dir() {
        // UNIVERSAL: backend chỉ có Move (thiếu DirMove) thì file dời được,
        // thư mục không — gộp OR sẽ nói dối trường hợp thư mục.
        let f = parse_feature_flags(&json!({"Move": true, "DirMove": false}));
        assert!(move_splitter(&f, false));
        assert!(!move_splitter(&f, true));
    }

    #[test]
    fn can_copy_covers_all_three_routes() {
        // UNIVERSAL: ba đường thông độc lập — move-native, copy+purge, bare-Copy;
        // tắt cả ba mới không copy được (đúng ca Copy-mà-thiếu-Purge).
        assert!(can_copy(true, false, false));
        assert!(can_copy(false, true, false));
        assert!(can_copy(false, false, true));
        assert!(!can_copy(false, false, false));
        println!("[REAL combine-can_copy] TFF=T FTF=T FFT=T FFF=F");
    }

    #[test]
    fn support_copy_and_purge_combine_to_move_via_copy_delete() {
        // UNIVERSAL: đọc thẳng cờ có sẵn (f.copy/f.purge), chỉ tổ hợp mới cần
        // hàm. Đủ Copy+Purge mới move bằng copy+delete được.
        let full = parse_feature_flags(&json!({"Copy": true, "Purge": true}));
        assert!(full.copy && full.purge);
        assert!(move_by_copy_delete(&full));
        let local_like = parse_feature_flags(&json!({"Copy": false, "Purge": false}));
        assert!(!local_like.copy);
        assert!(!move_by_copy_delete(&local_like));
        // Có Copy mà thiếu Purge: copy được, nhưng move-bằng-copy+delete không.
        let copy_only = parse_feature_flags(&json!({"Copy": true, "Purge": false}));
        assert!(copy_only.copy);
        assert!(!move_by_copy_delete(&copy_only));
        // CleanUp đọc thẳng, không qua hàm bọc.
        let clean = parse_feature_flags(&json!({"CleanUp": true}));
        assert!(clean.clean_up);
    }
}
