/*
[INTEGRITY NOTES]
- Mục đích: Thợ đo dung lượng/kích thước remote (S2) — logic gọi rclone about/size.
- Trách nhiệm: Hàm đồng bộ thuần (chạy trong `fastlane` do tầng `api::remote_manager` bọc).
- Tương tác: Chỉ `api::remote_manager` gọi sang; không đụng IPC/frontend.
*/

use crate::core::rclone_caller;
use serde_json::Value;

/// UNIVERSAL: lõi chung about/size — chạy `rclone <cmd> <target> --json`,
/// lỗi spawn/exit thành `Err` kèm stderr, stdout parse JSON.
/// `target` do NGƯỜI GỌI chuẩn bị sẵn (`"Name:"` cho remote, path trần cho
/// Local) — KHÔNG tự gắn ":" vì đã xác minh gắn bừa gãy cả hai chiều
/// (`"Box Ninja"` thiếu colon thành thư mục local; `"/tmp:"` thừa colon thì
/// directory-not-found). Ai gọi thì người đó chịu định dạng target.
fn run_json(cmd: &str, target: &str) -> Result<Value, String> {
    let output = rclone_caller::run_cmd(&[cmd, target, "--json"])?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Lỗi rclone {cmd}: {}", err_msg));
    }
    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

/// UNIVERSAL check-first: backend hỏi được cờ mà `About=false` thì từ chối
/// rõ ngay, khỏi tốn spawn `about`. Không hỏi được cờ (Local/path trần/lỗi)
/// thì cho qua để attempt quyết — KHÔNG từ chối trên cái chưa biết.
fn about_allowed(flags: Option<&crate::actions::feature::getfeature::BackendFeatures>) -> bool {
    flags.map(|f| f.about).unwrap_or(true)
}

/// Dung lượng remote `rclone about --json` (đồng bộ; tầng api bọc `fastlane`).
pub fn about(remote: &str) -> Result<Value, String> {
    // UNIVERSAL: chuẩn hoá tên (frontend gửi kèm ":" — cắt đi vì query tự gắn;
    // path trần giữ nguyên để attempt quyết).
    let key = remote.trim_end_matches(':');
    let cached = crate::actions::feature::checkcap::backend_features_cached(key);
    if !about_allowed(cached.as_ref()) {
        return Err(format!("{remote} không hỗ trợ xem dung lượng (About)."));
    }
    run_json("about", remote)
}

/// Kích thước remote `rclone size --json` (đồng bộ; tầng api bọc `fastlane`).
pub fn size(remote: &str) -> Result<Value, String> {
    run_json("size", remote)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rclone_present() -> bool {
        crate::core::rclone_caller::run_cmd(&["version"])
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Thư mục sạch tự gieo (2 file) — `/tmp` máy thật bẩn (socket + thư mục
    /// cấm đọc của systemd làm `size` exit lỗi dù JSON vẫn ra).
    fn seed_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rclone_gui_size_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("gieo thu muc");
        std::fs::write(dir.join("a.txt"), b"12345").expect("gieo file");
        std::fs::write(dir.join("b.txt"), b"1234567890").expect("gieo file");
        dir
    }

    #[test]
    fn about_allowed_decides_from_flag_or_unknown() {
        // UNIVERSAL check-first: cờ About=false → từ chối rõ; không hỏi được
        // cờ (None) → cho qua để attempt quyết, KHÔNG từ chối trên cái chưa biết.
        use crate::actions::feature::getfeature::parse_feature_flags;
        use serde_json::json;
        assert!(about_allowed(None));
        let no_about = parse_feature_flags(&json!({"About": false}));
        assert!(!about_allowed(Some(&no_about)));
        let yes_about = parse_feature_flags(&json!({"About": true}));
        assert!(about_allowed(Some(&yes_about)));
        println!("[REAL check-first] None=qua False=chan True=qua");
    }

    #[test]
    fn live_about_local_reports_disk_numbers() {
        // UNIVERSAL KIỂM THỰC DỤNG: `about` offline vẫn chạy, IN số thật
        // để đối chiếu (total/used/free byte đĩa). Không có rclone thì bỏ qua.
        if !rclone_present() {
            println!("SKIP: máy này không có rclone");
            return;
        }
        let dir = seed_dir("about");
        let v = about(&dir.to_string_lossy()).expect("about thu muc gieo");
        let total = v.get("total").and_then(|x| x.as_u64()).unwrap_or(0);
        let used = v.get("used").and_then(|x| x.as_u64()).unwrap_or(0);
        println!("[REAL about] {}: total={total} used={used}", dir.display());
        assert!(total > 0, "total phải dương");
        assert!(used <= total, "used không vượt total");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_size_local_counts_entries() {
        // UNIVERSAL KIỂM THỰC DỤNG: `size` thư mục gieo 2 file → count đúng 2,
        // bytes đúng 15. IN số thật để đối chiếu.
        if !rclone_present() {
            println!("SKIP: máy này không có rclone");
            return;
        }
        let dir = seed_dir("size");
        let v = size(&dir.to_string_lossy()).expect("size thu muc gieo");
        let count = v.get("count").and_then(|x| x.as_u64()).unwrap_or(0);
        let bytes = v.get("bytes").and_then(|x| x.as_u64()).unwrap_or(0);
        println!("[REAL size] {}: count={count} bytes={bytes}", dir.display());
        assert_eq!(count, 2, "gieo đúng 2 file");
        assert_eq!(bytes, 15, "5 + 10 byte");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
