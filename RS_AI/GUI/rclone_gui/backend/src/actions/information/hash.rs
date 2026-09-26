/*
[INTEGRITY NOTES]
- Mục đích: Tầng Actions — Kiểm tra mã băm (hashsum), danh sách hash hỗ trợ,
  và so sánh tính toàn vẹn (integrity check) giữa nguồn và đích.
- Trách nhiệm:
  + `execute_hashsum`: Gọi `rclone hashsum <hash_type> <target>` để lấy mã băm của tệp.
  + `get_supported_hashes`: Truy vấn `rclone backend features <remote:>` bóc trường `Hashes`.
  + `execute_check_integrity`: Gọi `rclone check <src> <dst>` để kiểm tra tính toàn vẹn.
- Tương tác: Được gọi bởi `api::remote_manager` hoặc `api::files_view` qua fastlane.
*/

use crate::core::rclone_caller;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// TTL Cache cho danh sách hash hỗ trợ của remote (5 phút).
const HASH_CACHE_TTL: Duration = Duration::from_secs(300);

type HashCache = Mutex<HashMap<String, (Vec<String>, Instant)>>;

fn hash_cache() -> &'static HashCache {
    static CACHE: std::sync::OnceLock<HashCache> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Kết quả kiểm tra toàn vẹn giữa nguồn và đích (rclone check).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrityCheckResult {
    pub success: bool,
    pub matches: usize,
    pub differences: usize,
    pub errors: usize,
    pub message: String,
}

/// Tính mã băm (hash) của tệp tin qua `rclone hashsum <hash_type> <target>`.
/// Các loại hash thông dụng: "md5", "sha1", "sha256", "crc32", "quickxor", "dropbox".
pub fn execute_hashsum(path: &str, hash_type: &str) -> Result<String, String> {
    let info = super::resolve_target(path)?;
    let target = info.target;
    let hash_type = hash_type.trim().to_lowercase();
    if hash_type.is_empty() {
        return Err("Thiếu loại thuật toán băm (hash_type).".to_string());
    }

    crate::core::debug::info(
        None,
        "actions/information/hash",
        format!("BẮT ĐẦU Hashsum ({}) | target='{}'", hash_type, target),
    );
    let start = std::time::Instant::now();

    let output = rclone_caller::run_cmd(&["hashsum", &hash_type, &target])?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        crate::core::debug::error(
            None,
            "actions/information/hash",
            format!("LỖI Hashsum ({}) | target='{}' | err={}", hash_type, target, err),
        );
        return Err(format!("Lỗi tính mã băm {}: {}", hash_type, err));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Định dạng của rclone hashsum: "<hash_value>  <filename>"
    let hash_value = stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .unwrap_or("")
        .to_string();

    crate::core::debug::info(
        None,
        "actions/information/hash",
        format!("XONG Hashsum ({}) | hash='{}' ({:.2?})", hash_type, hash_value, start.elapsed()),
    );

    Ok(hash_value)
}

/// Lấy danh sách các loại mã băm mà remote hỗ trợ (truy vấn `rclone backend features <remote:>`).
/// Với Local, truy vấn trực tiếp `rclone backend features /`.
/// Kết quả được lưu bộ nhớ đệm TTL 5 phút để tránh spawn tiến trình lặp lại.
pub fn get_supported_hashes(remote: &str) -> Vec<String> {
    let clean_remote = remote.trim().trim_end_matches(':');
    let cache_key = if clean_remote.is_empty() || clean_remote == "Local" {
        "Local".to_string()
    } else {
        clean_remote.to_string()
    };

    if let Some((hashes, at)) = hash_cache().lock().ok().and_then(|c| c.get(&cache_key).cloned()) {
        if at.elapsed() < HASH_CACHE_TTL {
            return hashes;
        }
    }

    let target = if cache_key == "Local" {
        "/".to_string()
    } else {
        format!("{}:", cache_key)
    };

    let output = match rclone_caller::run_cmd(&["backend", "features", &target]) {
        Ok(out) if out.status.success() => out,
        _ => return Vec::new(),
    };

    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return Vec::new();
    };

    let hashes: Vec<String> = json.get("Hashes")
        .and_then(|h| h.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if !hashes.is_empty() {
        if let Ok(mut cache) = hash_cache().lock() {
            cache.insert(cache_key, (hashes.clone(), Instant::now()));
        }
    }

    hashes
}

/// So sánh tính toàn vẹn (Integrity Check) giữa nguồn và đích qua `rclone check <src> <dst>`.
pub fn execute_check_integrity(src: &str, dst: &str) -> Result<IntegrityCheckResult, String> {
    let src_info = super::resolve_target(src)?;
    let dst_info = super::resolve_target(dst)?;
    let src_target = src_info.target;
    let dst_target = dst_info.target;

    crate::core::debug::info(
        None,
        "actions/information/hash",
        format!("BẮT ĐẦU CheckIntegrity | src='{}' dst='{}'", src_target, dst_target),
    );
    let start = std::time::Instant::now();

    let output = rclone_caller::run_cmd(&["check", &src_target, &dst_target, "--combined", "-"])?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut matches = 0;
    let mut differences = 0;
    let mut errors = 0;

    for line in stdout.lines() {
        if let Some(prefix) = line.chars().next() {
            match prefix {
                '=' => matches += 1,
                '*' | '+' | '-' => differences += 1,
                '!' => errors += 1,
                _ => {}
            }
        }
    }

    let success = output.status.success() && differences == 0 && errors == 0;
    let message = if success {
        format!("{matches} tệp hoàn toàn trùng khớp.")
    } else {
        let err_summary = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
        format!("{matches} khớp, {differences} khác biệt, {errors} lỗi. {err_summary}")
    };

    crate::core::debug::info(
        None,
        "actions/information/hash",
        format!("XONG CheckIntegrity | success={} matches={} diffs={} ({:.2?})", success, matches, differences, start.elapsed()),
    );

    Ok(IntegrityCheckResult {
        success,
        matches,
        differences,
        errors,
        message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::information::resolve_target;

    #[test]
    fn test_resolve_target() {
        let info = resolve_target("Local::/tmp/test.txt").expect("resolve");
        assert_eq!(info.remote, "Local");
        assert_eq!(info.real_path, "/tmp/test.txt");
        assert_eq!(info.target, "/tmp/test.txt");

        let info2 = resolve_target("GDrive::/folder/a.zip").expect("resolve");
        assert_eq!(info2.remote, "GDrive");
        assert_eq!(info2.target, "GDrive:/folder/a.zip");
    }

    #[test]
    fn test_get_supported_hashes_local() {
        let hashes = get_supported_hashes("Local");
        assert!(hashes.contains(&"md5".to_string()));
        assert!(hashes.contains(&"sha1".to_string()));
    }

    #[test]
    fn test_execute_hashsum_local() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_hash_sample.txt");
        let _ = std::fs::write(&file_path, b"hello world");

        let res = execute_hashsum(&file_path.to_string_lossy(), "md5");
        assert!(res.is_ok());
        // md5 of "hello world" is 5eb63bbbe01eeed093cb22bb8f5acdc3
        assert_eq!(res.unwrap(), "5eb63bbbe01eeed093cb22bb8f5acdc3");

        let _ = std::fs::remove_file(&file_path);
    }
}
