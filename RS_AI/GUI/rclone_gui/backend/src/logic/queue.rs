//! UNIVERSAL S2: tầng con của job queue — vé con + manifest + chạy từng vé.
//! Tách từ `core/jobs.rs` cũ: tầng cha (`Job`, `JobStore`, điều phối, persist,
//! IPC) nằm ở `super::jobs`; file này giữ `QueueItem`, manifest
//! (`manifest`/`manifest_args`/`parse_manifest_json`/`sort_manifest`) và worker
//! chạy tuần tự từng vé con qua runner dùng chung bên dưới.

use super::jobs::{Job, JobKind, JobStatus, JobStore};
use crate::actions::perm::Policy;
use crate::logic::tracker::{Tracker, TransferMode, TransferTicket};
use serde::{Deserialize, Serialize};

/// UNIVERSAL S2: chế độ chạy của vé con — tường minh trên vé, QUEUE chỉ làm theo.
/// - `Whole`: nguyên khối, toàn bộ src→dst một lệnh (`across` + snapshot cờ
///   do JOB đóng dấu lúc dispatch).
/// - `Item` (mặc định để vé cũ thiếu trường vẫn đọc được): từng món, một lệnh
///   đơn cho `path` tương đối của vé.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildMode {
    #[default]
    Item,
    Whole,
}

/// UNIVERSAL S2: vé con trong hàng chờ — một món sau khi `manifest()` nở.
/// `status` riêng từng con để worker chạy tuần tự và báo tiến độ cha.
/// QUEUE là thằng làm, không quyết: `mode`/`across`/`engine_flags` do JOB
/// đóng dấu 1 lần lúc dispatch, tầng chạy chỉ đọc vé, không đọc engine settings.
/// (UNIVERSAL: chỉ `PartialEq` vì snapshot `GlobalFlags` của engine không `Eq`.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueItem {
    pub job_id: String,
    pub path: String,
    pub is_dir: bool,
    pub status: JobStatus,
    #[serde(default)]
    pub error: Option<String>,
    /// UNIVERSAL S2: chế độ chạy tường minh (thay mẹo ngầm "`path` rỗng = bulk").
    #[serde(default)]
    pub mode: ChildMode,
    /// UNIVERSAL S2: quyết định across do JOB tính sẵn (chỉ có nghĩa khi `Whole`).
    #[serde(default)]
    pub across: bool,
    /// UNIVERSAL S2: snapshot cờ engine do JOB đóng dấu (vé cũ thiếu → default).
    #[serde(default)]
    pub engine_flags: crate::settings::engine::GlobalFlags,
}

/// UNIVERSAL: một vé điểm danh — một món trong thư mục đã bóc qua `lsjson -R`.
/// `path` là `Path` tương đối rclone trả; `is_dir` để tạo vỏ trước, chép file sau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestItem {
    pub path: String,
    pub is_dir: bool,
}

/// UNIVERSAL: dựng args `lsjson -R` cho `manifest()`; bật `fast_list` thì thêm
/// `--fast-list` (khớp quy ước `actions::conflicts::check_conflicts`), tắt thì giữ args cũ.
pub fn manifest_args(target: &str, fast_list: bool) -> Vec<String> {
    if fast_list {
        vec![
            "lsjson".to_string(),
            "-R".to_string(),
            "--fast-list".to_string(),
            target.to_string(),
        ]
    } else {
        vec!["lsjson".to_string(), "-R".to_string(), target.to_string()]
    }
}

/// UNIVERSAL: xếp vé điểm danh — thư mục trước, file sau, mỗi nhóm theo tên
/// không phân biệt hoa/thường (khớp thứ tự `execute_list`).
pub fn sort_manifest(items: &mut [ManifestItem]) {
    items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.path.to_lowercase().cmp(&b.path.to_lowercase()),
    });
}

/// UNIVERSAL: bóc JSON `lsjson -R` thành vé đã xếp; stdout rỗng/tàn dư khoảng
/// trắng là vỏ rỗng tường minh (`Ok(vec![])`), không phải lỗi.
pub fn parse_manifest_json(json_str: &str) -> Result<Vec<ManifestItem>, String> {
    if json_str.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries: Vec<serde_json::Value> = serde_json::from_str(json_str)
        .map_err(|e| format!("Lỗi phân tích JSON manifest: {}", e))?;
    let mut items: Vec<ManifestItem> = entries
        .into_iter()
        .filter_map(|v| {
            let path = v
                .get("Path")
                .and_then(|p| p.as_str())
                .or_else(|| v.get("Name").and_then(|p| p.as_str()))?
                .to_string();
            let is_dir = v.get("IsDir").and_then(|b| b.as_bool()).unwrap_or(false);
            Some(ManifestItem { path, is_dir })
        })
        .collect();
    sort_manifest(&mut items);
    Ok(items)
}

/// UNIVERSAL: quét `lsjson -R` rồi bóc thư mục thành vé từng món qua
/// [`parse_manifest_json`]; tôn trọng cờ `fast_list` của engine.
pub fn manifest(src: &str) -> Result<Vec<ManifestItem>, String> {
    let (remote, real) = crate::core::path::cut_remote_path(src);
    let safe = if remote == "Local" && real.is_empty() {
        "/".to_string()
    } else {
        real
    };
    let target = crate::core::rclone_caller::build_target(&remote, &safe);
    // UNIVERSAL: đọc cờ engine; lỗi đọc thì rớt về tắt để giữ hành vi cũ.
    let fast = crate::settings::engine::load_engine_flags()
        .map(|f| f.fast_list)
        .unwrap_or(false);
    let args = manifest_args(&target, fast);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = crate::core::rclone_caller::run_cmd(&refs)?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    parse_manifest_json(&String::from_utf8_lossy(&out.stdout))
}

/// UNIVERSAL S2: nối đường dẫn base với path tương đối của vé con
/// (giữ `/` phân cách, không phụ thuộc OS vì remote dùng `/`).
fn join_child(base: &str, rel: &str) -> String {
    let b = base.trim_end_matches('/');
    let r = rel.trim_start_matches('/');
    if b.is_empty() {
        format!("/{r}")
    } else {
        format!("{b}/{r}")
    }
}

/// UNIVERSAL S2: chạy một vé con copy/move bằng actions `execute_*` (QUEUE chỉ
/// điều phối, không tự spawn rclone, không tự parse — actions phun dòng thô,
/// queue hỏi `Tracker` (nơi duy nhất tính %) rồi ghi vào Job).
/// - Vé (`mode`/`across`/`engine_flags`) do JOB đóng dấu lúc dispatch — đây chỉ
///   map `ChildMode` → `TransferMode` rồi truyền vé nguyên dạng.
/// - `should_cancel` ngó `JobStore::is_cancel_requested` (do `execute_child` cấp);
///   `on_log_line` nhận dòng thô từ actions (do `execute_child` cấp, tự hỏi tracker).
/// - `policy` lấy từ `job.policy` (JOB đóng dấu lúc dispatch) — actions bọc
///   `perm::escalate` nên chưa consent trả `PERMISSION_CONSENT` thay vì pkexec.
pub(super) fn run_child_transfer(
    is_copy: bool,
    src: &str,
    dst: &str,
    child: &QueueItem,
    policy: Policy,
    should_cancel: impl Fn() -> bool,
    on_log_line: impl FnMut(&str),
) -> Result<(), String> {
    // UNIVERSAL: check_cap là não chung (UI hỏi + đường chạy hỏi) — across
    // lấy từ Cap.server_side, giữ bulk/item/policy/progress/cancel như cũ.
    let cap = crate::actions::checkcap::check_cap(src, dst);
    let ticket = TransferTicket {
        src: src.to_string(),
        dst: dst.to_string(),
        rel: child.path.clone(),
        mode: match child.mode {
            ChildMode::Whole => TransferMode::Whole,
            ChildMode::Item => TransferMode::Item,
        },
        across: cap.server_side,
        engine_flags: child.engine_flags.clone(),
        policy,
    };
    if is_copy {
        crate::actions::copy_op::execute_copy(ticket, should_cancel, on_log_line)
    } else {
        crate::actions::move_op::execute_move(ticket, should_cancel, on_log_line)
    }
}

/// UNIVERSAL S2: chạy một vé con delete qua actions `execute_delete_sync`
/// (cùng ngữ nghĩa nhánh `NoTrash` của `execute_delete`: xóa vĩnh viễn như
/// `fs_delete` — `deletefile` → rớt `purge` + sudo fallback).
/// UNIVERSAL: `policy` từ `job.policy` — chưa consent trả `PERMISSION_CONSENT`.
pub(super) fn run_child_delete(src: &str, rel: &str, policy: Policy) -> Result<(), String> {
    let (remote, real) = crate::core::path::cut_remote_path(src);
    let full = join_child(&real, rel);
    // UNIVERSAL: dựng lại chuỗi gốc cho actions parse (`Remote::/path`, Local trần).
    let path = if remote == "Local" {
        full
    } else {
        format!("{remote}::{full}")
    };
    crate::actions::delete_op::execute_delete_sync(&path, policy)
}

impl JobStore {
    /// UNIVERSAL S2: đánh dấu vé con chưa xong thành `Cancelled` (hủy theo cha).
    pub(super) fn mark_children_cancelled(&self, id: &str) {
        if let Ok(mut kids) = self.children.lock() {
            if let Some(list) = kids.get_mut(id) {
                for k in list.iter_mut() {
                    if matches!(k.status, JobStatus::Queued | JobStatus::Running) {
                        k.status = JobStatus::Cancelled;
                    }
                }
            }
        }
        self.persist();
    }

    /// UNIVERSAL S2: cập nhật một vé con + tiến độ cha (`child_done`,
    /// `progress` = con xong / tổng con), persist cả 2 tầng.
    pub(super) fn update_child_progress(
        &self,
        job_id: &str,
        idx: usize,
        status: JobStatus,
        err: Option<String>,
    ) -> Option<Job> {
        let (done, total) = {
            let mut kids = self.children.lock().ok()?;
            let list = kids.get_mut(job_id)?;
            if let Some(k) = list.get_mut(idx) {
                k.status = status;
                k.error = err;
            }
            let total = list.len();
            let done = list.iter().filter(|k| matches!(k.status, JobStatus::Done)).count();
            (done, total)
        };
        self.update(job_id, |j| {
            j.child_done = done;
            j.child_total = total;
            j.progress = if total == 0 {
                j.progress
            } else {
                ((done as u64 * 100 / total as u64).min(100)) as u8
            };
        })
    }

    /// UNIVERSAL S2: chạy tuần tự từng vé con; dừng ở con lỗi/hủy đầu tiên,
    /// lỗi con thành lỗi cha để UI thấy thay vì `Done` giả.
    pub(super) fn execute_children(&self, job: &Job) -> Result<(), String> {
        let total = self.children_of(&job.id).len();
        for idx in 0..total {
            if self.is_cancel_requested(&job.id) {
                return Err("job cancelled".to_string());
            }
            // Đánh dấu con đang chạy (không emit giữa chừng để giữ event gọn).
            if let Ok(mut kids) = self.children.lock() {
                if let Some(k) = kids.get_mut(&job.id).and_then(|l| l.get_mut(idx)) {
                    k.status = JobStatus::Running;
                }
            }
            let child = self.children_of(&job.id).get(idx).cloned();
            let child = match child {
                Some(c) => c,
                None => continue,
            };
            match self.execute_child(job, &child) {
                Ok(()) => {
                    self.update_child_progress(&job.id, idx, JobStatus::Done, None);
                }
                Err(e) if e == "job cancelled" || self.is_cancel_requested(&job.id) => {
                    self.update_child_progress(&job.id, idx, JobStatus::Cancelled, None);
                    return Err("job cancelled".to_string());
                }
                Err(e) => {
                    self.update_child_progress(&job.id, idx, JobStatus::Error, Some(e.clone()));
                    // Vé còn lại chưa chạy → cancelled theo cha lỗi.
                    if let Ok(mut kids) = self.children.lock() {
                        if let Some(list) = kids.get_mut(&job.id) {
                            for k in list.iter_mut().skip(idx + 1) {
                                if k.status == JobStatus::Queued {
                                    k.status = JobStatus::Cancelled;
                                }
                            }
                        }
                    }
                    self.persist();
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// UNIVERSAL S2: một vé con — rẽ theo `mode` tường minh trên vé (không suy
    /// từ "`path` rỗng = bulk"): `Whole` chạy toàn bộ src→dst qua actions
    /// `execute_*` với vé đã đóng dấu + closure ngó cờ hủy + sink dòng thô hỏi
    /// `Tracker` (nơi duy nhất tính %) rồi ghi % vào tiến độ cha (Whole 1 vé =
    /// tiến độ job, jobs nhận % từ queue như cũ); `Item` là vỏ dir `Ok` ngay,
    /// file thì một lệnh đơn theo kind cha (tiến độ cha giữ theo con xong/tổng
    /// con nên sink no-op); `List`/`Manifest` chỉ điểm danh nên `Ok` ngay.
    fn execute_child(&self, job: &Job, child: &QueueItem) -> Result<(), String> {
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        match child.mode {
            ChildMode::Whole => match job.kind {
                JobKind::Copy | JobKind::Move => {
                    let cmd = if job.kind == JobKind::Copy { "copyto" } else { "moveto" };
                    let src = job.src.clone().unwrap_or_default();
                    let dst = job
                        .dst
                        .clone()
                        .ok_or_else(|| format!("{} job missing dst", cmd))?;
                    // UNIVERSAL: actions phun dòng thô → hỏi Tracker lấy % rồi ghi
                    // vào tiến độ cha (Tracker lọc trùng % sẵn, không cần last riêng).
                    let job_id = job.id.clone();
                    let mut tracker = Tracker::new();
                    run_child_transfer(
                        job.kind == JobKind::Copy,
                        &src,
                        &dst,
                        child,
                        job.policy,
                        || self.is_cancel_requested(&job_id),
                        |line: &str| {
                            if let Some(st) = tracker.next_stats(line) {
                                let pct = st.percent;
                                self.update(&job_id, |j| {
                                    j.progress = pct;
                                });
                            }
                        },
                    )
                }
                _ => Ok(()),
            },
            ChildMode::Item => {
                if child.is_dir {
                    return Ok(());
                }
                match job.kind {
                    JobKind::List | JobKind::Manifest => Ok(()),
                    JobKind::Delete => {
                        let src = job.src.clone().unwrap_or_default();
                        run_child_delete(&src, &child.path, job.policy)
                    }
                    JobKind::Copy | JobKind::Move => {
                        let cmd = if job.kind == JobKind::Copy { "copyto" } else { "moveto" };
                        let src = job.src.clone().unwrap_or_default();
                        let dst = job
                            .dst
                            .clone()
                            .ok_or_else(|| format!("{} job missing dst", cmd))?;
                        // UNIVERSAL: tiến độ cha = con xong/tổng con (vòng
                        // `execute_children` giữ) nên intra-file stream no-op.
                        let job_id = job.id.clone();
                        run_child_transfer(
                            job.kind == JobKind::Copy,
                            &src,
                            &dst,
                            child,
                            job.policy,
                            || self.is_cancel_requested(&job_id),
                            |_| {},
                        )
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::jobs::{JobKind, JobStatus, JobStore};
    use std::path::PathBuf;

    fn temp_store(name: &str) -> (JobStore, PathBuf) {
        let path = std::env::temp_dir().join(format!("rclone_gui_queue_test_{name}.json"));
        let _ = std::fs::remove_file(&path);
        (JobStore::with_path(path.clone()), path)
    }

    fn rclone_present() -> bool {
        crate::core::rclone_caller::run_cmd(&["version"])
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn child_path_join_keeps_slash_sep() {
        // UNIVERSAL S2: nối base + rel vé con, giữ `/` vì remote dùng `/`.
        assert_eq!(join_child("/a/b", "c/d.txt"), "/a/b/c/d.txt");
        assert_eq!(join_child("/a/b/", "/c.txt"), "/a/b/c.txt");
        assert_eq!(join_child("", "c.txt"), "/c.txt");
    }

    #[test]
    fn manifest_args_respects_fast_list() {
        // UNIVERSAL: bật fast-list thì có cờ, tắt thì giữ args cũ.
        assert_eq!(manifest_args("R:/d", false), vec!["lsjson", "-R", "R:/d"]);
        assert_eq!(
            manifest_args("R:/d", true),
            vec!["lsjson", "-R", "--fast-list", "R:/d"]
        );
    }

    #[test]
    fn manifest_sort_dirs_first_and_empty_explicit() {
        // UNIVERSAL: vỏ rỗng tường minh — stdout trắng trả vec rỗng, không lỗi.
        assert!(parse_manifest_json("  \n ").expect("empty manifest").is_empty());
        // UNIVERSAL: thư mục trước file sau, mỗi nhóm theo tên thường.
        let items = parse_manifest_json(
            r#"[{"Path":"b.txt","Name":"b.txt","IsDir":false},{"Path":"A","Name":"A","IsDir":true},{"Path":"a.txt","Name":"a.txt","IsDir":false}]"#,
        )
        .expect("parse manifest");
        assert_eq!(
            items,
            vec![
                ManifestItem { path: "A".into(), is_dir: true },
                ManifestItem { path: "a.txt".into(), is_dir: false },
                ManifestItem { path: "b.txt".into(), is_dir: false },
            ]
        );
    }

    #[test]
    fn manifest_scans_real_dir_dirs_first() {
        // UNIVERSAL: quét thật — vỏ rỗng `Ok(vec![])`, có đồ thì dir trước file sau.
        if !rclone_present() {
            return;
        }
        let base = std::env::temp_dir().join("rclone_gui_queue_manifest");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("sub")).expect("seed sub");
        std::fs::write(base.join("b.txt"), b"hi").expect("seed file");
        let items = manifest(&base.to_string_lossy()).expect("manifest ok");
        assert!(items.iter().any(|i| i.is_dir && i.path.contains("sub")));
        assert!(items.iter().any(|i| !i.is_dir && i.path.contains("b.txt")));
        let first_file = items.iter().position(|i| !i.is_dir).expect("has file");
        assert!(items[..first_file].iter().all(|i| i.is_dir));
        // UNIVERSAL: vỏ rỗng tường minh — thư mục trống không vé nào, không lỗi.
        let empty = base.join("empty");
        std::fs::create_dir_all(&empty).expect("seed empty");
        assert!(manifest(&empty.to_string_lossy()).expect("empty manifest").is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn s2_children_expand_dirs_first_and_parent_progress() {
        // UNIVERSAL S2: manifest() nở thành con dir trước file sau; tiến độ
        // cha = tổng con xong (pure, không cần rclone).
        let (store, path) = temp_store("s2_expand");
        let job = store.enqueue(JobKind::Manifest, Some("/src".into()), None);
        let kids = store.set_children_from_items(
            &job.id,
            vec![
                ManifestItem { path: "b.txt".into(), is_dir: false },
                ManifestItem { path: "A".into(), is_dir: true },
                ManifestItem { path: "a.txt".into(), is_dir: false },
            ],
        );
        assert_eq!(kids.len(), 3);
        assert!(kids[0].is_dir);
        // UNIVERSAL S2: vé nở từ manifest mang mode tường minh `Item`.
        assert!(kids.iter().all(|k| k.mode == ChildMode::Item && !k.across));
        let parent = store.get(&job.id).expect("parent");
        assert_eq!(parent.child_total, 3);
        assert_eq!(parent.child_done, 0);
        // Giả lập worker xong từng con: progress cha theo tổng con.
        store.update_child_progress(&job.id, 0, JobStatus::Done, None);
        store.update_child_progress(&job.id, 1, JobStatus::Done, None);
        let mid = store.get(&job.id).expect("mid");
        assert_eq!(mid.child_done, 2);
        assert_eq!(mid.progress, 66);
        store.update_child_progress(&job.id, 2, JobStatus::Done, None);
        let done = store.get(&job.id).expect("done");
        assert_eq!((done.child_done, done.child_total, done.progress), (3, 3, 100));
        let _ = std::fs::remove_file(&path);
    }
}
