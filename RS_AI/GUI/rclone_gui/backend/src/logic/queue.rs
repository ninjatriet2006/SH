//! UNIVERSAL S2: tầng con của job queue — vé con + manifest + chạy từng vé.
//! Tách từ `core/jobs.rs` cũ: tầng cha (`Job`, `JobStore`, điều phối, persist,
//! IPC) nằm ở `super::jobs`; file này giữ `QueueItem`, manifest
//! (`manifest`/`manifest_args`/`parse_manifest_json`/`sort_manifest`) và worker
//! chạy tuần tự từng vé con qua runner dùng chung bên dưới.

use super::jobs::{Job, JobKind, JobStatus, JobStore};
use crate::actions::perm::Policy;
use crate::actions::rclone_stream::join_child;
use crate::logic::tracker::{Tracker, TransferMode, TransferTicket};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// UNIVERSAL worker-check: vé rel có nằm trong vùng skip không (khớp đúng
/// hoặc tiền tố `skip/` — skip dir cha thì cả cây con nghỉ).
pub(super) fn child_skipped(skip_paths: &[String], rel: &str) -> bool {
    skip_paths.iter().any(|s| {
        let s = s.trim().trim_matches('/');
        !s.is_empty() && (rel == s || rel.starts_with(&format!("{s}/")))
    })
}

/// UNIVERSAL worker-check: loại trùng của 1 vé (None = mới toanh hoặc
/// dir/dir merge → chạy; còn lại áp policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChildConflict {
    FileFile,
    FileDir,
    DirFile,
}

/// UNIVERSAL worker-check: tra 1 vé vào map đích (thuần, test được).
/// `dest_is_dir=None` = đích chưa có → không trùng.
pub(super) fn child_conflict(child_is_dir: bool, dest_is_dir: Option<bool>) -> Option<ChildConflict> {
    match (child_is_dir, dest_is_dir) {
        (_, None) => None,
        (false, Some(false)) => Some(ChildConflict::FileFile),
        (false, Some(true)) => Some(ChildConflict::FileDir),
        (true, Some(false)) => Some(ChildConflict::DirFile),
        (true, Some(true)) => None,
    }
}

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
/// (UNIVERSAL: `EngineSettings` giờ đã `Eq` nên `QueueItem` cũng phái sinh được.)
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
    pub engine_flags: crate::settings::engine::EngineSettings,
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
        .map(|f| f.switches.fast_list)
        .unwrap_or(false);
    let args = manifest_args(&target, fast);
    let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let out = crate::core::rclone_caller::run_cmd(&refs)?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    parse_manifest_json(&String::from_utf8_lossy(&out.stdout))
}

/// UNIVERSAL tối ưu: dựng vé transfer THUẦN từ vé con đã đóng dấu (không
/// spawn, không đọc settings — test được trực tiếp). `across`/`engine_flags`
/// lấy đúng tem JOB đóng lúc dispatch (trước đây tính lại mỗi vé = N lần
/// spawn `config dump` + đọc flags, lại còn vứt tem đi).
/// UNIVERSAL: `join_child` dùng chung từ `actions::rclone_stream` (1 mối).
pub(super) fn build_child_ticket(
    src: &str,
    dst: &str,
    child: &QueueItem,
    policy: Policy,
) -> TransferTicket {
    TransferTicket {
        src: src.to_string(),
        dst: dst.to_string(),
        rel: child.path.clone(),
        mode: match child.mode {
            ChildMode::Whole => TransferMode::Whole,
            ChildMode::Item => TransferMode::Item,
        },
        across: child.across,
        engine_flags: child.engine_flags.clone(),
        policy,
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
    // UNIVERSAL: vé đã đóng dấu đủ (across/engine_flags do JOB tính 1 lần lúc
    // dispatch) — dựng vé thuần rồi chạy, không hỏi lại não mỗi vé con.
    let ticket = build_child_ticket(src, dst, child, policy);
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
/// UNIVERSAL tối ưu: `is_dir` lấy từ vé (manifest đã biết) để khỏi probe lại.
pub(super) fn run_child_delete(src: &str, rel: &str, is_dir: bool, policy: Policy) -> Result<(), String> {
    let (remote, real) = crate::core::path::cut_remote_path(src);
    let full = join_child(&real, rel);
    // UNIVERSAL: dựng lại chuỗi gốc cho actions parse (`Remote::/path`, Local trần).
    let path = if remote == "Local" {
        full
    } else {
        format!("{remote}::{full}")
    };
    crate::actions::delete_op::execute_delete_sync(&path, policy, Some(is_dir))
}

impl JobStore {
    /// UNIVERSAL worker-check: đích Whole có tồn tại không (probe `is_dir`
    /// ĐÚNG 1 lần/job, gọi lười khi gặp vé Whole đầu tiên; dst thiếu → false).
    fn dst_exists(dst: Option<&str>) -> bool {
        let Some(d) = dst else {
            return false;
        };
        let (remote, real) = crate::core::path::cut_remote_path(d);
        let target = crate::core::rclone_caller::build_target(&remote, &real);
        crate::actions::types::is_dir(&target).unwrap_or(false)
    }

    /// UNIVERSAL worker-check: map đích (rel → is_dir) quét ĐÚNG 1 lần/job
    /// lúc chạy (manifest dst; dst chưa có/lỗi đọc → map rỗng = khỏi check).
    /// Resume mất map (không persist) thì quét lại 1 lần ở đây — vẫn chặn trên
    /// 1/job, không bao giờ N lần.
    fn dest_map_for(&self, job_id: &str, dst: Option<&str>) -> HashMap<String, bool> {
        if let Some(hit) = self
            .dest_maps
            .lock()
            .ok()
            .and_then(|m| m.get(job_id).cloned())
        {
            return hit;
        }
        let mut map = HashMap::new();
        if let Some(d) = dst {
            // UNIVERSAL: dst chưa có là bình thường (copy mới) → map rỗng, im lặng.
            if let Ok(items) = manifest(d) {
                map = items.into_iter().map(|m| (m.path, m.is_dir)).collect();
            }
        }
        if let Ok(mut maps) = self.dest_maps.lock() {
            maps.insert(job_id.to_string(), map.clone());
        }
        map
    }

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
    /// UNIVERSAL tối ưu: persist THƯA — mỗi 10 vé + vé lỗi/hủy + vé cuối cùng
    /// (kết cục job trong `run_queue_sync` luôn persist qua `update()` nên
    /// không mất an toàn khi crash giữa chừng: vé dở `reconcile_orphans` dọn).
    pub(super) fn update_child_progress(
        &self,
        job_id: &str,
        idx: usize,
        status: JobStatus,
        err: Option<String>,
    ) -> Option<Job> {
        const PERSIST_EVERY: usize = 10;
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
        let updated = self.apply_update(job_id, |j| {
            j.child_done = done;
            j.child_total = total;
            j.progress = if total == 0 {
                j.progress
            } else {
                ((done as u64 * 100 / total as u64).min(100)) as u8
            };
        });
        if updated.is_some()
            && (done % PERSIST_EVERY == 0
                || done == total
                || !matches!(status, JobStatus::Done))
        {
            self.persist();
        }
        updated
    }

    /// UNIVERSAL S2: chạy tuần tự từng vé con; dừng ở con lỗi/hủy đầu tiên,
    /// lỗi con thành lỗi cha để UI thấy thay vì `Done` giả.
    /// UNIVERSAL worker-check: chỉ Copy/Move mới check trùng đích (Delete/List
    /// không đáp sang đâu). Dest map quét 1 lần/job; Whole probe đích 1 lần/job.
    pub(super) fn execute_children(&self, job: &Job) -> Result<(), String> {
        let total = self
            .children
            .lock()
            .map(|m| m.get(&job.id).map(|l| l.len()).unwrap_or(0))
            .unwrap_or(0);
        let checks = matches!(job.kind, JobKind::Copy | JobKind::Move);
        let dest_map = if checks {
            self.dest_map_for(&job.id, job.dst.as_deref())
        } else {
            HashMap::new()
        };
        // UNIVERSAL worker-check: Whole không tra map được (rel rỗng) — probe
        // đích 1 lần/job khi gặp vé Whole đầu tiên (None = chưa probe).
        let mut whole_dst_exists: Option<bool> = None;
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
            let child = match self
                .children
                .lock()
                .ok()
                .and_then(|m| m.get(&job.id)?.get(idx).cloned())
            {
                // UNIVERSAL tối ưu: clone đúng 1 vé (trước đây clone cả vec mỗi
                // vòng → O(n²) với n vé con).
                Some(c) => c,
                None => continue,
            };
            // UNIVERSAL worker-check: trùng đích thì áp policy đóng dấu — skip
            // (kể cả tiền tố cây con) thì bỏ vé + đếm, còn lại (mặc định Replace)
            // ghi log rồi chạy đè tại đó tuỳ ý. Lệnh lặp không modal vẫn chảy.
            if checks {
                let conflicted = match child.mode {
                    ChildMode::Item => child_conflict(child.is_dir, dest_map.get(&child.path).copied()).is_some(),
                    ChildMode::Whole => {
                        if whole_dst_exists.is_none() {
                            whole_dst_exists = Some(Self::dst_exists(job.dst.as_deref()));
                        }
                        whole_dst_exists.unwrap_or(false)
                    }
                };
                if conflicted {
                    if child_skipped(&job.skip_paths, &child.path) {
                        self.update_child_progress(
                            &job.id,
                            idx,
                            JobStatus::Cancelled,
                            Some("bỏ qua theo policy conflict".to_string()),
                        );
                        self.apply_update(&job.id, |j| j.skipped += 1);
                        crate::core::debug::info(
                            &job.id,
                            format!("  con {}/{} bỏ qua (policy) | {}", idx + 1, total, child.path),
                        );
                        continue;
                    }
                    crate::core::debug::info(
                        &job.id,
                        format!("  con {}/{} trùng, ghi đè | {}", idx + 1, total, child.path),
                    );
                }
            }
            // UNIVERSAL: Mức B — nhật ký từng vé con, cùng tag `job-...` với cha
            // để lọc 1 job ra thấy trọn các bước con (con thứ mấy/tổng + path).
            let pos = format!("con {}/{}", idx + 1, total);
            match self.execute_child(job, &child) {
                Ok(()) => {
                    self.update_child_progress(&job.id, idx, JobStatus::Done, None);
                    crate::core::debug::info(&job.id, format!("  {pos} xong | {}", child.path));
                }
                Err(e) if e == "job cancelled" || self.is_cancel_requested(&job.id) => {
                    self.update_child_progress(&job.id, idx, JobStatus::Cancelled, None);
                    crate::core::debug::info(&job.id, format!("  {pos} hủy | {}", child.path));
                    return Err("job cancelled".to_string());
                }
                Err(e) => {
                    self.update_child_progress(&job.id, idx, JobStatus::Error, Some(e.clone()));
                    crate::core::debug::error(&job.id, format!("  {pos} LỖI | {} | {e}", child.path));
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
                // UNIVERSAL: liệt kê đủ enum nội bộ (cấm wildcard để compiler bắt thiếu nhánh).
                JobKind::Delete | JobKind::List | JobKind::Manifest | JobKind::Rename | JobKind::Mkdir | JobKind::Touch => Ok(()),
            },
            ChildMode::Item => {
                if child.is_dir {
                    return Ok(());
                }
                match job.kind {
                    JobKind::List | JobKind::Manifest | JobKind::Rename | JobKind::Mkdir | JobKind::Touch => Ok(()),
                    JobKind::Delete => {
                        let src = job.src.clone().unwrap_or_default();
                        run_child_delete(&src, &child.path, child.is_dir, job.policy)
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
    use crate::settings::engine::{EngineSettings, EngineSwitches, EngineTuning};
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
            false,
            crate::settings::engine::EngineSettings::default(),
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

    #[test]
    fn children_stamp_dispatch_flags_not_default() {
        // UNIVERSAL tối ưu (mục 1): vé Item mang đúng cờ dispatch (dry_run/
        // backup_dir/across), không còn gán default rồi lờ cài đặt người dùng.
        let (store, path) = temp_store("stamp_flags");
        let job = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        let flags = EngineSettings {
            switches: EngineSwitches { dry_run: true, ..Default::default() },
            tuning: EngineTuning {
                backup_dir: Some("/tmp/bk".to_string()),
                ..Default::default()
            },
        };
        let kids = store.set_children_from_items(
            &job.id,
            vec![ManifestItem { path: "f.txt".into(), is_dir: false }],
            true,
            flags,
        );
        assert!(kids[0].across, "across dispatch phai duoc dong dau");
        assert!(kids[0].engine_flags.switches.dry_run);
        assert_eq!(
            kids[0].engine_flags.tuning.backup_dir.as_deref(),
            Some("/tmp/bk")
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn build_child_ticket_uses_stamped_across() {
        // UNIVERSAL tối ưu (mục 2): vé transfer lấy across ĐÚNG tem vé con
        // (không hỏi lại não mỗi vé → hết N spawn `config dump`).
        let child = QueueItem {
            job_id: "j".into(),
            path: "sub/f.txt".into(),
            is_dir: false,
            status: JobStatus::Queued,
            error: None,
            mode: ChildMode::Item,
            across: true,
            engine_flags: EngineSettings::default(),
        };
        let t = build_child_ticket("A::/a", "B::/b", &child, Policy::AskOnce);
        assert_eq!((t.src.as_str(), t.dst.as_str()), ("A::/a", "B::/b"));
        assert_eq!(t.rel, "sub/f.txt");
        assert!(matches!(t.mode, TransferMode::Item));
        assert!(t.across, "across phai lay tu tem, khong tinh lai");
        assert_eq!(t.policy, Policy::AskOnce);
        // Whole map sang Whole.
        let mut whole = child.clone();
        whole.mode = ChildMode::Whole;
        let t2 = build_child_ticket("A::/a", "B::/b", &whole, Policy::Deny);
        assert!(matches!(t2.mode, TransferMode::Whole));
    }

    #[test]
    fn progress_counts_twelve_children_pure() {
        // UNIVERSAL tối ưu (mục 5): đếm tiến độ đúng qua nhiều vé (persist thưa
        // không làm sai số đếm); thuần store, không cần rclone.
        let (store, path) = temp_store("twelve");
        let job = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        let items: Vec<ManifestItem> = (0..12)
            .map(|i| ManifestItem { path: format!("f{i}.txt"), is_dir: false })
            .collect();
        store.set_children_from_items(
            &job.id,
            items,
            false,
            EngineSettings::default(),
        );
        for idx in 0..12 {
            store.update_child_progress(&job.id, idx, JobStatus::Done, None);
        }
        let done = store.get(&job.id).expect("done");
        assert_eq!((done.child_done, done.child_total, done.progress), (12, 12, 100));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn child_skipped_matches_exact_and_prefix() {
        // UNIVERSAL worker-check: khớp đúng rel hoặc tiền tố cây con; rỗng và
        // khoảng trắng không skip bậy.
        let skips = vec!["d".to_string(), "f.txt".to_string()];
        assert!(child_skipped(&skips, "d"));
        assert!(child_skipped(&skips, "d/sub/f.txt"));
        assert!(child_skipped(&skips, "f.txt"));
        assert!(!child_skipped(&skips, "other.txt"));
        assert!(!child_skipped(&skips, "dd/f.txt"));
        assert!(!child_skipped(&[], "d"));
        assert!(!child_skipped(&["  ".to_string()], "d"));
    }

    #[test]
    fn child_conflict_covers_five_arms() {
        // UNIVERSAL worker-check: đủ 5 tay — mới/merge thì None (chạy),
        // còn lại ra đúng loại để log + áp policy.
        use ChildConflict::*;
        assert_eq!(child_conflict(false, None), None);
        assert_eq!(child_conflict(true, None), None);
        assert_eq!(child_conflict(true, Some(true)), None);
        assert_eq!(child_conflict(false, Some(false)), Some(FileFile));
        assert_eq!(child_conflict(false, Some(true)), Some(FileDir));
        assert_eq!(child_conflict(true, Some(false)), Some(DirFile));
    }

    #[test]
    fn worker_skips_stamped_paths_without_spawning() {
        // UNIVERSAL worker-check: vé trùng + nằm trong skip_paths thì bỏ qua
        // (Cancelled + đếm skipped), KHÔNG spawn rclone — chạy offline được.
        // dest map cắm tay để khỏi quét đĩa.
        let (store, path) = temp_store("skip_offline");
        let job = store.enqueue_with_policy(
            JobKind::Copy,
            Some("/a".into()),
            Some("/b".into()),
            Policy::AskOnce,
            vec!["d".to_string()],
        );
        store.set_children_from_items(
            &job.id,
            vec![
                ManifestItem { path: "d/f.txt".into(), is_dir: false },
                ManifestItem { path: "ok.txt".into(), is_dir: false },
            ],
            false,
            EngineSettings::default(),
        );
        // Dest có d/f.txt (trùng) nhưng không có ok.txt (mới).
        if let Ok(mut maps) = store.dest_maps.lock() {
            maps.insert(
                job.id.clone(),
                [("d/f.txt".to_string(), false)].into_iter().collect(),
            );
        }
        // ok.txt không trùng → chạy thật → thiếu rclone thì bỏ qua test.
        if !rclone_present() {
            let _ = std::fs::remove_file(&path);
            return;
        }
        let _ = store.execute_children(&store.get(&job.id).expect("job"));
        let kids = store.children_of(&job.id);
        let skipped = kids.iter().find(|k| k.path == "d/f.txt").expect("ve d");
        assert_eq!(skipped.status, JobStatus::Cancelled);
        let mid = store.get(&job.id).expect("mid");
        assert_eq!(mid.skipped, 1, "dem dung 1 ve skip");
        println!("[REAL worker-skip] d/f.txt=Cancelled skipped=1");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn old_job_json_without_skip_fields_still_loads() {
        // UNIVERSAL: file persist cũ thiếu skip_paths/skipped vẫn nạp (default).
        let path = std::env::temp_dir().join(format!(
            "rclone_gui_queue_test_skipcompat_{}.json",
            std::process::id()
        ));
        let raw = r#"{"jobs": [{"id": "j1", "kind": "copy", "src": "/a", "dst": "/b", "status": "queued", "progress": 0, "error": null, "child_done": 0, "child_total": 0, "policy": "ask_once"}], "children": {}}"#;
        std::fs::write(&path, raw).expect("seed");
        let loaded = JobStore::with_path(path.clone());
        let j = loaded.get("j1").expect("job cu");
        assert!(j.skip_paths.is_empty() && j.skipped == 0);
        let _ = std::fs::remove_file(&path);
    }
}
