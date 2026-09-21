/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_search` thành plan thuần (`lsjson -R --include`).
- Trách nhiệm: parse → build_target → chọn lệnh `lsjson`; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `core::path::cut_remote_path`,
  `core::rclone_caller::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use super::list::FileItem;
use serde::Serialize;

/// Một kết quả tìm kiếm (DTO gốc ở thợ `search`).
#[derive(Serialize)]
pub struct SearchResultItem {
    pub item: FileItem,
    pub path: String,
}
use crate::core::rclone_caller;
use crate::logic::fastlane::fastlane;
use crate::core::rclone_caller::build_target;
use crate::core::path::cut_remote_path;

/// Đặc tả thuần cho `search`: lệnh `lsjson -R --include`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPlan {
    pub target: String,
    pub filter: String,
    pub rclone_args: Vec<String>,
}

/// Dựng plan `search` (`lsjson -R --include *query* --files-only`); query rỗng → lỗi.
/// UNIVERSAL: `fast_list` bật thì gắn `--fast-list` để liệt kê đệ quy nhanh.
pub fn plan_search(path: &str, query: &str, fast_list: bool) -> Result<SearchPlan, String> {
    if query.is_empty() {
        return Err("Thiếu từ khóa tìm kiếm.".to_string());
    }
    let (remote, real) = cut_remote_path(path);
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    let target = build_target(&remote, &real);
    let filter = format!("*{}*", query);
    // UNIVERSAL: đệ quy + fast-list khi bật cờ engine; tắt thì giữ args cũ.
    let mut tail = vec![
        "-R".to_string(),
        "--include".to_string(),
        filter.clone(),
        "--files-only".to_string(),
    ];
    if fast_list {
        tail.push("--fast-list".to_string());
    }
    let rclone_args = match kind {
        // UNIVERSAL: cả Local và remote đều tìm qua `lsjson` đệ quy, không sudo.
        RemoteKind::Local | RemoteKind::Remote => {
            let mut v = vec!["lsjson".to_string(), target.clone()];
            v.extend(tail);
            v
        }
    };
    Ok(SearchPlan {
        target,
        filter,
        rclone_args,
    })
}

/// S2: thực thi search — `plan_search` (fast-list từ cờ engine) + `fastlane` run + parse cũ.
/// UNIVERSAL: cả Local và remote đều tìm qua `lsjson` đệ quy.
pub async fn execute_search(path: String, query: String) -> Result<Vec<SearchResultItem>, String> {
    let fast_list = crate::settings::engine::load_engine_flags()
        .map(|f| f.switches.fast_list)
        .unwrap_or(false);
    let plan = plan_search(&path, &query, fast_list)?;
    let (remote, real_path) = cut_remote_path(&path);
    fastlane(move || {
        let args: Vec<&str> = plan.rclone_args.iter().map(|s| s.as_str()).collect();
        let output = rclone_caller::run_cmd(&args)?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        let items: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Lỗi phân tích JSON khi tìm kiếm: {}", e))?;
        let mut files = Vec::new();
        for item in items {
            let name = item["Name"].as_str().unwrap_or("").to_string();
            let rel_path = item["Path"].as_str().unwrap_or("").to_string();
            let file_path = if real_path.ends_with('/') {
                format!("{}{}", real_path, rel_path)
            } else {
                format!("{}/{}", real_path, rel_path)
            };
            let file_info = FileItem {
                uuid: file_path.clone(),
                name,
                is_dir: item["IsDir"].as_bool().unwrap_or(false),
                size: item["Size"].as_i64().unwrap_or(0),
                mod_time: item["ModTime"].as_str().unwrap_or("").to_string(),
                file_type: None,
            };
            let ui_path = if remote == "Local" {
                format!("Local::{}", file_path)
            } else {
                format!("{}::{}", remote, file_path)
            };
            files.push(SearchResultItem { item: file_info, path: ui_path });
        }
        Ok(files)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_empty_query_rejected() {
        assert!(plan_search("Local::/tmp", "", false).is_err());
    }

    #[test]
    fn search_fast_list_flag() {
        // UNIVERSAL: bật fast-list thì args có cờ, tắt thì giữ nguyên.
        let on = plan_search("Local::/tmp", "doc", true).expect("plan");
        assert!(on.rclone_args.contains(&"--fast-list".to_string()));
        let off = plan_search("Local::/tmp", "doc", false).expect("plan");
        assert!(!off.rclone_args.contains(&"--fast-list".to_string()));
    }
}
