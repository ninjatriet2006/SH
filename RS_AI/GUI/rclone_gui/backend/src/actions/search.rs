/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_search` thành plan thuần (`lsjson -R --include`).
- Trách nhiệm: parse → build_target → chọn lệnh `lsjson`; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `logic::file_ops::parse_remote_path`,
  `core::rclone::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use crate::core::rclone::build_target;
use crate::logic::file_ops::parse_remote_path;

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
    let (remote, real) = parse_remote_path(path);
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
