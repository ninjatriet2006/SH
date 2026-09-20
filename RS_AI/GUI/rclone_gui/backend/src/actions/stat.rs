/*
[INTEGRITY NOTES]
- Mục đích: Trial S1 bóc đặc tả `fs_stat_advanced` thành plan thuần (`size --json`).
- Trách nhiệm: parse → build_target → chọn lệnh `size`/`lsjson`; khớp `match` trên `RemoteKind`.
- Tương tác: Chỉ gọi hàm thuần `logic::file_ops::parse_remote_path`,
  `core::rclone::build_target`. Không chạy lệnh, không wire `fs_*` cũ / IPC.
*/

use crate::actions::explorer::Cap;
use crate::actions::types::RemoteKind;
use crate::core::rclone::build_target;
use crate::logic::file_ops::parse_remote_path;

/// Đặc tả thuần cho `stat`: lệnh `size --json` + đếm thư mục `lsjson`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatPlan {
    pub target: String,
    pub size_args: Vec<String>,
    pub dirs_args: Vec<String>,
}

/// Dựng plan `stat` (`size --json` + `lsjson -R --dirs-only`).
pub fn plan_stat(path: &str) -> Result<StatPlan, String> {
    let (remote, real) = parse_remote_path(path);
    let kind = RemoteKind::classify(&remote);
    let _cap = Cap::of(kind);
    let target = build_target(&remote, &real);
    let size_args = vec![target.clone(), "--json".to_string()];
    let dirs_args = match kind {
        // UNIVERSAL: cả Local và remote đều đếm thư mục con qua `lsjson -R --dirs-only`.
        RemoteKind::Local | RemoteKind::Remote => {
            vec![target.clone()]
        }
    };
    Ok(StatPlan {
        target,
        size_args,
        dirs_args,
    })
}
