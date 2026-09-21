//! UNIVERSAL S2: tầng cha của job queue — `Job`, `JobStore`, worker điều phối,
//! persist, IPC `job_enqueue`/`job_list`/`job_cancel`, event `job_update`.
//! Tầng con (vé con, manifest, chạy từng vé) nằm ở `super::queue`.
//! Tách từ `core/jobs.rs` cũ; tên/trường IPC + event giữ NGUYÊN.

use super::queue::{ChildMode, ManifestItem, QueueItem, manifest, sort_manifest};
use crate::actions::perm::Policy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Loại việc worker P2 chạy thật (copy/move/delete/list/manifest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Copy,
    Move,
    Delete,
    List,
    Manifest,
}

impl JobKind {
    /// Parse từ chuỗi frontend gửi; sai chuỗi trả lỗi để IPC map 400.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "copy" => Some(JobKind::Copy),
            "move" => Some(JobKind::Move),
            "delete" => Some(JobKind::Delete),
            "list" => Some(JobKind::List),
            // UNIVERSAL: vé điểm danh — bóc thư mục thành từng món qua `manifest()`.
            "manifest" => Some(JobKind::Manifest),
            // UNIVERSAL: chuỗi lạ rớt về None như cũ (IPC map 400).
            other => {
                crate::core::debug::warn(None, "jobs/JobKind::parse", format!("kind lạ '{other}', rớt về None"));
                None
            }
        }
    }
}

/// Trạng thái vòng đời của job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Error,
    Cancelled,
}

/// Một đơn vị việc trong hàng chờ (cha S2: giữ nguyên tên/trường IPC cũ,
/// chỉ thêm `child_done`/`child_total` cho tiến độ tổng con).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub kind: JobKind,
    pub src: Option<String>,
    pub dst: Option<String>,
    pub status: JobStatus,
    pub progress: u8,
    pub error: Option<String>,
    /// UNIVERSAL S2: số con đã xong (giữ trường cũ, chỉ thêm mới, default 0).
    #[serde(default)]
    pub child_done: usize,
    /// UNIVERSAL S2: tổng số con sau khi `manifest()` nở (default 0 = chưa nở).
    #[serde(default)]
    pub child_total: usize,
    /// UNIVERSAL: chính sách leo thang quyền đóng dấu lúc dispatch
    /// (`enqueue_with_policy` đọc từ `AppState.policy`); file cũ thiếu trường
    /// vẫn nạp nhờ `default` = `AskOnce`.
    #[serde(default)]
    pub policy: Policy,
}

/// UNIVERSAL: parser log transfer sống ở `super::tracker` (thư viện đọc log
/// thuần); re-export tại đây để giữ đường dùng cũ (`jobs::stats_percent`).
pub use super::tracker::{parse_progress_line, stats_percent};

fn config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        let dir = dir.trim();
        if !dir.is_empty() {
            return PathBuf::from(dir).join("rclone_gui");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("rclone_gui")
}

/// Store thread-safe: RAM (`Mutex`) + persist JSON best-effort (2 tầng cha/con).
pub struct JobStore {
    inner: Mutex<HashMap<String, Job>>,
    // UNIVERSAL S2: tầng con cho `queue.rs` cùng nhà `logic` chạm trực tiếp.
    pub(super) children: Mutex<HashMap<String, Vec<QueueItem>>>,
    queue: Mutex<VecDeque<String>>,
    // UNIVERSAL S2: cờ hủy cho `queue.rs` đọc khi chạy từng vé con.
    pub(super) cancelled: Mutex<HashSet<String>>,
    running: AtomicBool,
    path: PathBuf,
    id_counter: AtomicU64,
}

impl JobStore {
    /// Store mặc định, file persist `<config_dir>/jobs.json`.
    pub fn new() -> Self {
        Self::with_path(config_dir().join("jobs.json"))
    }

    /// Store với đường dẫn custom (dùng cho test).
    pub fn with_path(path: PathBuf) -> Self {
        let store = Self {
            inner: Mutex::new(HashMap::new()),
            children: Mutex::new(HashMap::new()),
            queue: Mutex::new(VecDeque::new()),
            cancelled: Mutex::new(HashSet::new()),
            running: AtomicBool::new(false),
            path,
            id_counter: AtomicU64::new(0),
        };
        store.load_from_disk();
        store
    }

    fn next_id(&self) -> String {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let n = self.id_counter.fetch_add(1, Ordering::SeqCst);
        format!("job-{millis}-{n}")
    }

    /// Thêm job mới ở trạng thái `queued`, persist, trả bản clone.
    /// UNIVERSAL: đường cũ — policy mặc định (`AskOnce`); luồng có consent
    /// dùng `enqueue_with_policy` để đóng dấu từ `AppState.policy`.
    pub fn enqueue(&self, kind: JobKind, src: Option<String>, dst: Option<String>) -> Job {
        self.enqueue_with_policy(kind, src, dst, Policy::default())
    }

    /// UNIVERSAL: thêm job + đóng dấu policy lúc dispatch (IPC `job_enqueue`
    /// đọc `AppState.policy` rồi gọi hàm này; worker chỉ đọc `job.policy`,
    /// không đọc settings/state giữa chừng).
    pub fn enqueue_with_policy(
        &self,
        kind: JobKind,
        src: Option<String>,
        dst: Option<String>,
        policy: Policy,
    ) -> Job {
        let job = Job {
            id: self.next_id(),
            kind,
            src,
            dst,
            status: JobStatus::Queued,
            progress: 0,
            error: None,
            child_done: 0,
            child_total: 0,
            policy,
        };
        if let Ok(mut inner) = self.inner.lock() {
            inner.insert(job.id.clone(), job.clone());
        }
        if let Ok(mut q) = self.queue.lock() {
            q.push_back(job.id.clone());
        }
        self.persist();
        job
    }

    /// Liệt kê snapshot toàn bộ job.
    pub fn list(&self) -> Vec<Job> {
        self.inner
            .lock()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Lấy một job theo id.
    pub fn get(&self, id: &str) -> Option<Job> {
        self.inner.lock().ok()?.get(id).cloned()
    }

    /// UNIVERSAL S2: snapshot vé con của một job cha (rỗng nếu chưa nở).
    pub fn children_of(&self, id: &str) -> Vec<QueueItem> {
        self.children
            .lock()
            .map(|m| m.get(id).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    /// UNIVERSAL S2: dựng vé con từ `ManifestItem` đã xếp (dir trước file sau);
    /// cập nhật `child_total` của cha + persist. Pure, dùng cho test.
    /// Mỗi vé mang `mode=Item` tường minh (tầng con chạy lệnh đơn cho `path`).
    pub fn set_children_from_items(&self, id: &str, mut items: Vec<ManifestItem>) -> Vec<QueueItem> {
        sort_manifest(&mut items);
        let kids: Vec<QueueItem> = items
            .into_iter()
            .map(|m| QueueItem {
                job_id: id.to_string(),
                path: m.path,
                is_dir: m.is_dir,
                status: JobStatus::Queued,
                error: None,
                mode: ChildMode::Item,
                across: false,
                engine_flags: crate::settings::engine::GlobalFlags::default(),
            })
            .collect();
        let total = kids.len();
        if let Ok(mut m) = self.children.lock() {
            m.insert(id.to_string(), kids.clone());
        }
        let _ = self.update(id, |j| {
            j.child_total = total;
            j.child_done = 0;
        });
        kids
    }

    /// UNIVERSAL S2: dựng 1 vé con nguyên khối (`mode=Whole`, toàn bộ src→dst)
    /// cho chế độ `bulk_transfer=true` (và rớt về khi manifest vỏ rỗng/lỗi đọc);
    /// `across` + snapshot cờ do JOB đóng dấu 1 lần lúc dispatch (tầng con chỉ
    /// đọc vé, không đọc engine settings). Cập nhật `child_total=1` + persist.
    /// Pure, dùng cho test.
    pub fn set_children_bulk(
        &self,
        id: &str,
        across: bool,
        flags: crate::settings::engine::GlobalFlags,
    ) -> Vec<QueueItem> {
        let kids = vec![QueueItem {
            job_id: id.to_string(),
            path: String::new(),
            is_dir: false,
            status: JobStatus::Queued,
            error: None,
            mode: ChildMode::Whole,
            across,
            engine_flags: flags,
        }];
        if let Ok(mut m) = self.children.lock() {
            m.insert(id.to_string(), kids.clone());
        }
        let _ = self.update(id, |j| {
            j.child_total = 1;
            j.child_done = 0;
        });
        kids
    }

    /// Yêu cầu hủy: queued → `cancelled` ngay + gỡ khỏi hàng chờ;
    /// running → đặt cờ để worker dừng ở bước tiếp theo.
    pub fn request_cancel(&self, id: &str) -> Result<Job, String> {
        let status = self.get(id).map(|j| j.status).ok_or_else(|| "job not found".to_string())?;
        match status {
            JobStatus::Done | JobStatus::Error | JobStatus::Cancelled => {
                return Err("job already finished".to_string());
            }
            // UNIVERSAL: liệt kê đủ enum nội bộ (cấm wildcard để compiler bắt thiếu nhánh).
            JobStatus::Queued | JobStatus::Running => {}
        }
        if let Ok(mut flags) = self.cancelled.lock() {
            flags.insert(id.to_string());
        }
        // Job còn trong hàng chờ: gỡ ra và đánh dấu luôn để `list` phản ánh ngay.
        let mut was_queued = false;
        if let Ok(mut q) = self.queue.lock() {
            let before = q.len();
            q.retain(|x| x != id);
            was_queued = q.len() != before;
        }
        if was_queued {
            if let Ok(mut inner) = self.inner.lock() {
                if let Some(job) = inner.get_mut(id) {
                    job.status = JobStatus::Cancelled;
                    let done = job.clone();
                    drop(inner);
                    // UNIVERSAL S2: hủy theo cha — vé con chưa chạy cũng cancelled.
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
                    return Ok(done);
                }
            }
        }
        self.persist();
        self.get(id).ok_or_else(|| "job not found".to_string())
    }

    // UNIVERSAL S2: tầng con (`queue.rs`) đọc cờ hủy khi chạy từng vé.
    pub(super) fn is_cancel_requested(&self, id: &str) -> bool {
        self.cancelled.lock().map(|f| f.contains(id)).unwrap_or(false)
    }

    // UNIVERSAL S2: tầng con (`queue.rs`) cập nhật tiến độ cha qua đây.
    pub(super) fn update(&self, id: &str, f: impl FnOnce(&mut Job)) -> Option<Job> {
        let updated = if let Ok(mut inner) = self.inner.lock() {
            if let Some(job) = inner.get_mut(id) {
                f(job);
                Some(job.clone())
            } else {
                None
            }
        } else {
            None
        };
        if updated.is_some() {
            self.persist();
        }
        updated
    }

    /// UNIVERSAL: smart compact — cha `Done` thì XÓA vé con `Done`, GIỮ vé con
    /// lỗi/hủy để xem lại; giữ `child_done/total` trên cha (đếm không đổi);
    /// `Error`/`Cancelled` giữ nguyên hết để retry/review.
    fn compact_done_children(&self, id: &str) {
        if let Ok(mut kids) = self.children.lock() {
            if let Some(list) = kids.get_mut(id) {
                list.retain(|k| !matches!(k.status, JobStatus::Done));
            }
        }
        self.persist();
    }

    fn pop_next(&self) -> Option<String> {
        self.queue.lock().ok()?.pop_front()
    }

    fn queue_empty(&self) -> bool {
        self.queue.lock().map(|q| q.is_empty()).unwrap_or(true)
    }

    /// Chạy tuần tự toàn bộ hàng chờ trên thread hiện tại (S2 hai tầng).
    /// `emit` phát event `job_update` (giữ trường cũ, thêm `child_done/total`).
    pub fn run_queue_sync(&self, mut emit: Option<impl FnMut(Job)>) {
        while let Some(id) = self.pop_next() {
            if self.is_cancel_requested(&id) {
                if let Some(job) = self.update(&id, |j| {
                    j.status = JobStatus::Cancelled;
                }) {
                    self.mark_children_cancelled(&id);
                    if let Some(e) = emit.as_mut() {
                        e(job);
                    }
                }
                continue;
            }
            let snapshot = match self.get(&id) {
                Some(j) => j,
                None => continue,
            };
            if let Some(job) = self.update(&id, |j| {
                j.status = JobStatus::Running;
                j.progress = 0;
                j.error = None;
                j.child_done = 0;
            }) {
                if let Some(e) = emit.as_mut() {
                    e(job);
                }
            } else {
                continue;
            }
            // UNIVERSAL S2: Copy/Move do `run_transfer_job` tự điều phối vé
            // (bulk 1 vé / off bóc manifest) rồi chạy qua `execute_children`;
            // kind khác giữ đường cũ (nở best-effort, vỏ rỗng → chạy đơn).
            let outcome = match snapshot.kind {
                JobKind::Copy | JobKind::Move => self.execute_job(&snapshot, &mut emit),
                // UNIVERSAL: liệt kê đủ enum nội bộ (cấm wildcard).
                JobKind::Delete | JobKind::List | JobKind::Manifest => {
                    self.populate_children_best_effort(&snapshot);
                    if self.children_of(&id).is_empty() {
                        // UNIVERSAL: chạy thật theo kind; hủy giữa chừng → `Cancelled`,
                        // lỗi rclone → `Error` kèm stderr, xong → `Done` 100%.
                        self.execute_job(&snapshot, &mut emit)
                    } else {
                        // UNIVERSAL S2: worker chạy tuần tự từng con ở `queue.rs`,
                        // tiến độ cha = tổng con xong; cancel/error theo cha.
                        self.execute_children(&snapshot)
                    }
                }
            };
            // UNIVERSAL S2: sau vòng con vẫn đẩy tiến độ cha qua emit từng vé
            // (execute_children đã update+emit nội bộ qua update_child_progress).
            let done = match outcome {
                Ok(()) => {
                    let updated = self.update(&id, |j| {
                        if self.is_cancel_requested(&id) {
                            j.status = JobStatus::Cancelled;
                        } else {
                            j.status = JobStatus::Done;
                            j.progress = 100;
                            j.child_done = j.child_total;
                        }
                    });
                    // UNIVERSAL: smart compact — cha Done thì gọn vé con ngay,
                    // event terminal bên dưới vẫn phát 1 lần đầy đủ đếm.
                    if updated.as_ref().is_some_and(|j| j.status == JobStatus::Done) {
                        self.compact_done_children(&id);
                    }
                    updated
                }
                Err(e) if e == "job cancelled" || self.is_cancel_requested(&id) => {
                    self.mark_children_cancelled(&id);
                    self.update(&id, |j| {
                        j.status = JobStatus::Cancelled;
                    })
                }
                Err(e) => self.update(&id, |j| {
                    j.status = JobStatus::Error;
                    j.error = Some(e);
                }),
            };
            if let (Some(job), Some(e)) = (done, emit.as_mut()) {
                e(job);
            } else if emit.is_some() {
                // UNIVERSAL S2: dù update cuối None, vẫn phát lại snapshot để
                // UI không treo ở Running khi store lỗi lock thoáng qua.
                if let (Some(job), Some(e)) = (self.get(&id), emit.as_mut()) {
                    e(job);
                }
            }
        }
    }

    /// UNIVERSAL S2: nở vé con best-effort qua [`manifest`]; chỉ nở khi cha
    /// có `src` và chưa có con; lỗi đọc → giữ đường đơn cũ.
    fn populate_children_best_effort(&self, job: &Job) {
        if !self.children_of(&job.id).is_empty() {
            return;
        }
        // Chỉ các kind thao tác theo cây mới nở; kind khác giữ đường đơn.
        if !matches!(job.kind, JobKind::Copy | JobKind::Move | JobKind::Delete | JobKind::List | JobKind::Manifest) {
            return;
        }
        let src = match job.src.as_deref() {
            Some(s) => s,
            None => return,
        };
        match manifest(src) {
            Ok(items) if !items.is_empty() => {
                self.set_children_from_items(&job.id, items);
            }
            // UNIVERSAL: vỏ rỗng giữ đường đơn như cũ (không warn: bình thường).
            Ok(_) => {}
            // UNIVERSAL: lỗi đọc → warn rồi giữ đường đơn như cũ.
            Err(e) => {
                crate::core::debug::warn(None, "jobs/populate_children", format!("manifest lỗi: {e}"));
            }
        }
    }

    /// Điều phối một job tới actions thật theo kind (đồng bộ, chặn thread worker).
    fn execute_job<E: FnMut(Job)>(
        &self,
        job: &Job,
        emit: &mut Option<E>,
    ) -> Result<(), String> {
        match job.kind {
            // UNIVERSAL: copy/move qua phân tuyến + args transfer dùng chung.
            JobKind::Copy => self.run_transfer_job(job, "copyto", true, emit),
            JobKind::Move => self.run_transfer_job(job, "moveto", false, emit),
            // UNIVERSAL: delete vĩnh viễn (`NoTrash`) như `fs_delete`.
            JobKind::Delete => self.run_delete_job(job),
            // UNIVERSAL: list kiểm tra đọc được qua plan dùng chung.
            JobKind::List => self.run_list_job(job),
            // UNIVERSAL: vé điểm danh — quét `lsjson -R` thành từng món.
            JobKind::Manifest => self.run_manifest_job(job),
        }
    }

    /// UNIVERSAL S2: cha CHỈ điều phối vé Copy/Move, KHÔNG tự spawn rclone
    /// transfer nữa (mô hình đúng: cha tách→đẩy queue, con chạy action).
    /// Cờ engine (`bulk_transfer` + `server_side_across`) chỉ đọc 1 lần ở đây
    /// rồi đóng dấu vào vé: bulk→1 vé `Whole` (kèm across đã tính + snapshot cờ),
    /// off→`manifest()` bóc thành N vé `Item`; file đơn / vỏ rỗng / lỗi đọc
    /// best-effort → rớt về 1 vé `Whole` để tầng con vẫn chạy thật thay vì
    /// `Done` giả. Cả hai nhánh đều đi qua `queue::execute_children`.
    /// UNIVERSAL: hủy là hợp tác giữa các vé (`is_cancel_requested` mỗi vé);
    /// vé đang chạy là `run_cmd` blocking nên không có handle để `kill` thẳng —
    /// mọi kết thúc cưỡng bức đều hủy êm qua `super::process` (đường transfer
    /// cũ), tuyệt đối không `child.kill()` trực tiếp trong logic job.
    fn run_transfer_job<E: FnMut(Job)>(
        &self,
        job: &Job,
        cmd: &str,
        _is_copy: bool,
        _emit: &mut Option<E>,
    ) -> Result<(), String> {
        let src = job.src.clone().ok_or_else(|| format!("{} job missing src", cmd))?;
        // UNIVERSAL: validate dst sớm (báo lỗi thiếu dst trước khi nở vé).
        let dst = job.dst.clone().ok_or_else(|| format!("{} job missing dst", cmd))?;
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        // UNIVERSAL: đọc cờ engine 1 lần duy nhất lúc dispatch; lỗi đọc →
        // default (giữ hành vi cũ: bóc con, không across).
        let flags = crate::settings::engine::load_engine_flags().unwrap_or_default();
        // UNIVERSAL: não chung check_cap (UI hỏi + đường chạy hỏi) — across
        // lấy từ Cap.server_side; bulk/item/policy/progress/cancel giữ nguyên.
        if flags.bulk_transfer {
            let across = crate::actions::checkcap::check_cap(&src, &dst).server_side;
            self.set_children_bulk(&job.id, across, flags);
        } else {
            match manifest(&src) {
                Ok(items) if !items.is_empty() => {
                    self.set_children_from_items(&job.id, items);
                }
                // UNIVERSAL: vỏ rỗng/lỗi đọc → 1 vé Whole như cũ; lỗi đọc warn.
                Ok(_) => {
                    let across = crate::actions::checkcap::check_cap(&src, &dst).server_side;
                    self.set_children_bulk(&job.id, across, flags);
                }
                Err(e) => {
                    crate::core::debug::warn(None, "jobs/run_transfer_job", format!("manifest lỗi: {e}"));
                    let across = crate::actions::checkcap::check_cap(&src, &dst).server_side;
                    self.set_children_bulk(&job.id, across, flags);
                }
            }
        }
        self.execute_children(job)
    }

    /// UNIVERSAL: delete thật (`NoTrash`) qua actions `execute_delete_sync`
    /// (JOB chỉ điều phối, actions mới là nơi gọi rclone).
    /// UNIVERSAL: tôn trọng `job.policy` qua `perm::escalate` — chưa consent
    /// thì trả `PERMISSION_CONSENT` thay vì tự `pkexec`.
    fn run_delete_job(&self, job: &Job) -> Result<(), String> {
        let src = job.src.clone().ok_or_else(|| "delete job missing src".to_string())?;
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        crate::actions::delete_op::execute_delete_sync(&src, job.policy)
    }

    /// UNIVERSAL: list thật — dựng plan dùng chung rồi chạy `lsjson`, JSON hỏng
    /// hay rclone lỗi đều thành `Error` để UI thấy thay vì `Done` giả.
    fn run_list_job(&self, job: &Job) -> Result<(), String> {
        let src = job.src.clone().ok_or_else(|| "list job missing src".to_string())?;
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        let plan = crate::actions::list::plan_list(&src)?;
        let refs: Vec<&str> = plan.rclone_args.iter().map(|s| s.as_str()).collect();
        let out = crate::core::rclone_caller::run_cmd(&refs)?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
        }
        let text = String::from_utf8_lossy(&out.stdout);
        if text.trim().is_empty() {
            return Ok(());
        }
        let _: Vec<serde_json::Value> = serde_json::from_str(&text)
            .map_err(|e| format!("Lỗi phân tích JSON rclone_files: {}", e))?;
        Ok(())
    }

    /// UNIVERSAL: vé điểm danh — quét `lsjson -R` qua [`manifest`]; vỏ rỗng
    /// vẫn `Done` (không có vé nào để phát), lỗi đọc thành `Error`.
    fn run_manifest_job(&self, job: &Job) -> Result<(), String> {
        let src = job.src.clone().ok_or_else(|| "manifest job missing src".to_string())?;
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        let _ = manifest(&src)?;
        Ok(())
    }

    /// Spawn worker tuần tự nếu chưa chạy; gọi từ IPC sau `enqueue`.
    pub fn spawn_worker(self: &Arc<Self>, app: tauri::AppHandle) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let store = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let emit_job = |job: Job| {
                use tauri::Emitter;
                let _ = app.emit("job_update", job);
            };
            store.run_queue_sync(Some(emit_job));
            store.running.store(false, Ordering::SeqCst);
            if !store.queue_empty() {
                store.spawn_worker(app);
            }
        });
    }

    /// Ghi snapshot 2 tầng (cha + con) ra JSON (best-effort, không crash app).
    pub fn persist(&self) {
        #[derive(serde::Serialize)]
        struct Snapshot<'a> {
            jobs: Vec<Job>,
            children: &'a HashMap<String, Vec<QueueItem>>,
        }
        let jobs: Vec<Job> = self.list();
        let text = match self.children.lock() {
            Ok(kids) => serde_json::to_string(&Snapshot { jobs, children: &kids }),
            Err(_) => return,
        };
        let text = match text {
            Ok(text) => text,
            Err(_) => return,
        };
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let tmp = self.path.with_extension("json.tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &self.path);
        }
    }

    fn load_from_disk(&self) {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(_) => return,
        };
        // UNIVERSAL S2: đọc format mới `{jobs, children}`; rớt về mảng cũ
        // `Vec<Job>` để giữ tương thích file persist trước đây.
        #[derive(serde::Deserialize)]
        struct Snapshot {
            #[serde(default)]
            jobs: Vec<Job>,
            #[serde(default)]
            children: HashMap<String, Vec<QueueItem>>,
        }
        let (mut jobs, kids): (Vec<Job>, HashMap<String, Vec<QueueItem>>) =
            if let Ok(snap) = serde_json::from_str::<Snapshot>(&text) {
                // Phân biệt mảng cũ (parse Snapshot thất bại thường) — nhưng mảng
                // `[]` cũng parse được thành Snapshot rỗng nên thử Vec trước khi
                // tin Snapshot có dữ liệu? Ưu tiên: nếu jobs rỗng + text là mảng
                // thì vẫn đúng. Chỉ cần fallback khi Snapshot lỗi.
                (snap.jobs, snap.children)
            } else if let Ok(legacy) = serde_json::from_str::<Vec<Job>>(&text) {
                (legacy, HashMap::new())
            } else {
                return;
            };
        if let (Ok(mut inner), Ok(mut q), Ok(mut ck)) = (
            self.inner.lock(),
            self.queue.lock(),
            self.children.lock(),
        ) {
            for job in jobs.drain(..) {
                // UNIVERSAL: giữ nguyên status đĩa (kể cả `Running` mồ côi) để
                // `reconcile_orphans()` chạy 1 lần lúc khởi động quyết định
                // (đánh `Error` + dọn `.partial`); chỉ `Queued` mới vào hàng chờ.
                if job.status == JobStatus::Queued {
                    q.push_back(job.id.clone());
                }
                inner.insert(job.id.clone(), job);
            }
            for (id, mut list) in kids {
                for k in list.iter_mut() {
                    // UNIVERSAL: giữ `Running` mồ côi cho `reconcile_orphans()`;
                    // chỉ suy lại `Whole` cho vé bulk đời cũ (`Item` + path rỗng).
                    // UNIVERSAL S2: vé cũ (trước `mode` tường minh) thiếu trường
                    // → serde default `Item`; vé `path` rỗng thời đó là bulk
                    // nguyên khối nên suy lại `Whole` 1 lần ở biên load (tầng
                    // chạy chỉ đọc `mode`, không bao giờ suy từ `path`).
                    if k.mode == ChildMode::Item && k.path.is_empty() {
                        k.mode = ChildMode::Whole;
                    }
                }
                ck.insert(id, list);
            }
        }
    }

    /// UNIVERSAL: dọn job mồ côi sau crash — chạy 1 lần lúc khởi động (gọi từ
    /// `lib.rs setup` sau khi tạo `JobStore`).
    /// - Job (hoặc con) còn `Running` khi nạp lại → cha thành `Error`
    ///   "gián đoạn phiên trước", con `Running` → `Error`, con `Queued` của cha
    ///   chết → `Cancelled`; persist một lần cuối.
    /// - Job transfer (`Copy`/`Move`) bị gián đoạn mà đích là Local → xóa file
    ///   rác `*.partial` trong đúng thư mục đích của job (không đệ quy, không
    ///   quét cả ổ); đích Remote thì bỏ qua. Mọi lỗi đọc/xóa chỉ `warn`.
    pub fn reconcile_orphans(&self) -> usize {
        const MSG: &str = "gián đoạn phiên trước: tiến trình bị giết ngang";
        // Thu snapshot dst của job transfer mồ côi trước để dọn sau khi nhả lock.
        let mut dirty: Vec<(String, Option<String>, JobKind)> = Vec::new();
        if let Ok(inner) = self.inner.lock() {
            let kids = self.children.lock().ok();
            for job in inner.values() {
                let mut child_running = false;
                if let Some(m) = kids.as_ref() {
                    if let Some(list) = m.get(&job.id) {
                        child_running = list.iter().any(|k| k.status == JobStatus::Running);
                    }
                }
                if job.status == JobStatus::Running || child_running {
                    dirty.push((job.id.clone(), job.dst.clone(), job.kind));
                }
            }
        }
        if dirty.is_empty() {
            return 0;
        }
        let n = dirty.len();
        if let Ok(mut inner) = self.inner.lock() {
            for (id, _, _) in &dirty {
                if let Some(job) = inner.get_mut(id) {
                    if job.status == JobStatus::Running {
                        job.status = JobStatus::Error;
                        job.error = Some(MSG.to_string());
                    }
                }
            }
        }
        if let Ok(mut ck) = self.children.lock() {
            for (id, _, _) in &dirty {
                if let Some(list) = ck.get_mut(id) {
                    for k in list.iter_mut() {
                        if k.status == JobStatus::Running {
                            k.status = JobStatus::Error;
                            k.error = Some(MSG.to_string());
                        } else if k.status == JobStatus::Queued {
                            k.status = JobStatus::Cancelled;
                        }
                    }
                }
            }
        }
        // Dọn `.partial` cho job transfer mồ côi (đích Local only).
        for (_, dst, kind) in &dirty {
            if !matches!(kind, JobKind::Copy | JobKind::Move) {
                continue;
            }
            if let Some(d) = dst.as_deref() {
                Self::cleanup_partial_dir(d);
            }
        }
        self.persist();
        n
    }

    /// UNIVERSAL: xóa file `*.partial` trong đúng 1 thư mục đích Local (không
    /// đệ quy); dst là file thì dọn thư mục cha; Remote/rỗng/lỗi chỉ `warn`.
    fn cleanup_partial_dir(dst: &str) {
        let (remote, real) = crate::core::path::cut_remote_path(dst);
        if remote != "Local" {
            return;
        }
        let real = real.trim();
        if real.is_empty() {
            return;
        }
        let p = PathBuf::from(real);
        let dir = if p.is_dir() {
            p
        } else if let Some(parent) = p.parent() {
            if parent.as_os_str().is_empty() {
                return;
            }
            parent.to_path_buf()
        } else {
            return;
        };
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                crate::core::debug::warn(None, "jobs", format!("bỏ qua dọn .partial '{dir:?}': {e}"));
                return;
            }
        };
        for ent in entries {
            let ent = match ent {
                Ok(e) => e,
                Err(e) => {
                    crate::core::debug::warn(None, "jobs", format!("bỏ qua entry .partial: {e}"));
                    continue;
                }
            };
            let path = ent.path();
            let is_file = ent.file_type().map(|t| t.is_file()).unwrap_or(false) || path.is_file();
            if !is_file {
                continue;
            }
            let name = ent.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".partial") {
                continue;
            }
            if let Err(e) = std::fs::remove_file(&path) {
                crate::core::debug::warn(None, "jobs", format!("không xóa được '{path:?}': {e}"));
            }
        }
    }
}

impl Default for JobStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::queue::ManifestItem;
    use super::*;

    fn temp_store(name: &str) -> (JobStore, PathBuf) {
        let path = std::env::temp_dir().join(format!("rclone_gui_jobs_test_{name}.json"));
        let _ = std::fs::remove_file(&path);
        (JobStore::with_path(path.clone()), path)
    }

    fn rclone_present() -> bool {
        crate::core::rclone_caller::run_cmd(&["version"])
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn enqueue_and_list_roundtrip() {
        let (store, path) = temp_store("enqueue");
        let job = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.progress, 0);
        assert!(store.get(&job.id).is_some());
        assert_eq!(store.list().len(), 1);
        // Persist file tồn tại và parse được.
        let text = std::fs::read_to_string(&path).expect("jobs.json persisted");
        assert!(text.contains(&job.id));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn worker_runs_real_list_and_reports_terminal_state() {
        // UNIVERSAL: worker thật — list vỏ Local rỗng phải `Done` 100% + emit.
        if !rclone_present() {
            return;
        }
        let dir = std::env::temp_dir().join("rclone_gui_jobs_list_ok");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("seed dir");
        let (store, path) = temp_store("worker_real");
        let job = store.enqueue(JobKind::List, Some(dir.to_string_lossy().into_owned()), None);
        let mut seen = Vec::new();
        {
            let cb = |j: Job| seen.push((j.status, j.progress));
            store.run_queue_sync(Some(cb));
        }
        let done = store.get(&job.id).expect("job exists");
        assert!(matches!(done.status, JobStatus::Done));
        assert_eq!(done.progress, 100);
        assert!(seen.iter().any(|(s, p)| *s == JobStatus::Done && *p == 100));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn worker_marks_unreadable_source_as_error() {
        // UNIVERSAL: list đường dẫn mất tích phải `Error` kèm message, không `Done` giả.
        if !rclone_present() {
            return;
        }
        let (store, path) = temp_store("worker_err");
        let missing = std::env::temp_dir().join("rclone_gui_jobs_missing_xyz");
        let _ = std::fs::remove_dir_all(&missing);
        let job = store.enqueue(JobKind::List, Some(missing.to_string_lossy().into_owned()), None);
        store.run_queue_sync(None::<fn(Job)>);
        let failed = store.get(&job.id).expect("job exists");
        assert_eq!(failed.status, JobStatus::Error);
        assert!(failed.error.as_deref().map(|e| !e.is_empty()).unwrap_or(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cancel_queued_marks_cancelled_and_skips_worker() {
        let (store, path) = temp_store("cancel");
        let job = store.enqueue(JobKind::Move, Some("/a".into()), Some("/b".into()));
        let cancelled = store.request_cancel(&job.id).expect("cancel ok");
        assert_eq!(cancelled.status, JobStatus::Cancelled);
        store.run_queue_sync(None::<fn(Job)>);
        assert_eq!(store.get(&job.id).map(|j| j.status), Some(JobStatus::Cancelled));
        // Cancel job đã kết thúc phải lỗi.
        let again = store.enqueue(JobKind::Delete, Some("/x".into()), None);
        store.run_queue_sync(None::<fn(Job)>);
        assert!(store.request_cancel(&again.id).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cancel_unknown_or_finished_errors() {
        let (store, path) = temp_store("cancel_err");
        assert!(store.request_cancel("no-such-id").is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn kind_parsing_rejects_unknown() {
        assert_eq!(JobKind::parse("copy"), Some(JobKind::Copy));
        assert_eq!(JobKind::parse("LIST"), Some(JobKind::List));
        // UNIVERSAL: vé điểm danh là kind hợp lệ của P2.
        assert_eq!(JobKind::parse("manifest"), Some(JobKind::Manifest));
        assert_eq!(JobKind::parse("bogus"), None);
    }

    #[test]
    fn transfer_stats_map_to_job_progress() {
        // UNIVERSAL: % từ event transfer (`stats`) map vào `job.progress`.
        let stats = serde_json::json!({"bytes": 50.0, "totalBytes": 200.0});
        assert_eq!(stats_percent(&stats), Some(25));
        let pct = serde_json::json!({"percentage": 33.6});
        assert_eq!(stats_percent(&pct), Some(34));
        let line = r#"{"stats":{"bytes":1.0,"totalBytes":2.0}}"#;
        assert_eq!(parse_progress_line(line), Some(50));
        // UNIVERSAL: dòng log thường / thiếu stats thì bỏ qua, không phá progress.
        assert_eq!(parse_progress_line("not json"), None);
        assert_eq!(parse_progress_line(r#"{"level":"error","msg":"x"}"#), None);
    }

    #[test]
    fn s2_bulk_on_single_ticket_off_expands_n() {
        // UNIVERSAL: bulk on → 1 vé Whole (kèm across + snapshot cờ đã đóng dấu);
        // off (manifest bóc) → N vé Item.
        let (store, path) = temp_store("s2_bulk");
        let job = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        let flags = crate::settings::engine::GlobalFlags {
            bulk_transfer: true,
            server_side_across: true,
            ..crate::settings::engine::GlobalFlags::default()
        };
        let one = store.set_children_bulk(&job.id, true, flags.clone());
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].mode, ChildMode::Whole);
        assert!(one[0].across);
        assert_eq!(one[0].engine_flags, flags);
        assert_eq!(store.get(&job.id).map(|j| j.child_total), Some(1));
        // UNIVERSAL: off bóc manifest N món → N vé Item (pure, qua set_children_*).
        let n = store.set_children_from_items(
            &job.id,
            vec![
                ManifestItem { path: "d".into(), is_dir: true },
                ManifestItem { path: "f1".into(), is_dir: false },
                ManifestItem { path: "f2".into(), is_dir: false },
            ],
        );
        assert_eq!(n.len(), 3);
        assert!(n.iter().all(|k| k.mode == ChildMode::Item && !k.across));
        assert_eq!(store.get(&job.id).map(|j| j.child_total), Some(3));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn s2_ticket_mode_explicit_and_legacy_defaults_item() {
        // UNIVERSAL S2: vé cũ thiếu mode/across/flags vẫn đọc được → Item +
        // across false + cờ default; vé Whole roundtrip giữ nguyên dấu.
        let old = serde_json::json!({
            "job_id": "j", "path": "f.txt", "is_dir": false,
            "status": "queued", "error": null
        });
        let item: QueueItem = serde_json::from_value(old).expect("legacy ticket loads");
        assert_eq!(item.mode, ChildMode::Item);
        assert!(!item.across);
        assert_eq!(item.engine_flags, crate::settings::engine::GlobalFlags::default());
        let whole = QueueItem {
            job_id: "j".into(),
            path: String::new(),
            is_dir: false,
            status: JobStatus::Queued,
            error: None,
            mode: ChildMode::Whole,
            across: true,
            engine_flags: crate::settings::engine::GlobalFlags {
                server_side_across: true,
                ..crate::settings::engine::GlobalFlags::default()
            },
        };
        let back: QueueItem =
            serde_json::from_value(serde_json::to_value(&whole).expect("serialize"))
                .expect("roundtrip");
        assert_eq!(back, whole);
        // UNIVERSAL S2: across đóng dấu 1 lần ở JOB — dính ổ máy / cùng tên
        // remote luôn false mà không cần gọi rclone (não chung check_cap).
        use crate::actions::checkcap::check_cap_with_flags;
        assert!(!check_cap_with_flags("Local:/a", "Local:/b", true).server_side);
        assert!(!check_cap_with_flags("Local:/a", "R:/b", true).server_side);
        assert!(!check_cap_with_flags("R:/a", "R:/b", true).server_side);
    }

    #[test]
    fn s2_persist_two_tiers_and_legacy_compat() {
        // UNIVERSAL S2: persist cả 2 tầng; file cũ (mảng cha) vẫn đọc được.
        let (store, path) = temp_store("s2_persist");
        let job = store.enqueue(JobKind::List, Some("/s".into()), None);
        store.set_children_from_items(
            &job.id,
            vec![ManifestItem { path: "f.txt".into(), is_dir: false }],
        );
        store.persist();
        let text = std::fs::read_to_string(&path).expect("persisted");
        assert!(text.contains(&job.id) && text.contains("f.txt"));
        // Reload format mới giữ cả 2 tầng.
        let reloaded = JobStore::with_path(path.clone());
        assert_eq!(reloaded.children_of(&job.id).len(), 1);
        assert_eq!(reloaded.get(&job.id).map(|j| j.child_total), Some(1));
        // Ghi đè mảng legacy (không child_*): vẫn parse nhờ serde default.
        let legacy = format!(
            r#"[{{"id":"{}","kind":"list","src":"/s","dst":null,"status":"queued","progress":0,"error":null}}]"#,
            job.id
        );
        std::fs::write(&path, legacy).expect("legacy write");
        let legacy_store = JobStore::with_path(path.clone());
        let lj = legacy_store.get(&job.id).expect("legacy job");
        assert_eq!((lj.child_done, lj.child_total), (0, 0));
        assert!(legacy_store.children_of(&job.id).is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn s2_cancel_propagates_to_children_and_event_keeps_old_fields() {
        // UNIVERSAL S2: cancel theo cha — con queued cũng cancelled; event
        // `job_update` giữ tên/trường cũ, chỉ thêm child_done/total.
        let (store, path) = temp_store("s2_cancel");
        let job = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        store.set_children_from_items(
            &job.id,
            vec![
                ManifestItem { path: "d".into(), is_dir: true },
                ManifestItem { path: "f".into(), is_dir: false },
            ],
        );
        store.request_cancel(&job.id).expect("cancel ok");
        assert!(store.children_of(&job.id).iter().all(|k| k.status == JobStatus::Cancelled));
        // IPC giữ trường cũ + thêm mới (policy đóng dấu lúc dispatch).
        let v = serde_json::to_value(store.get(&job.id).expect("job")).expect("json");
        for f in ["id", "kind", "src", "dst", "status", "progress", "error", "child_done", "child_total", "policy"] {
            assert!(v.get(f).is_some(), "missing field {f}");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reconcile_marks_running_error_and_cleans_partial() {
        // UNIVERSAL: job Running khi nạp → Error "gián đoạn phiên trước";
        // file `.partial` giả trong temp bị dọn, file thường giữ nguyên.
        let dst = std::env::temp_dir().join("rclone_gui_reconcile_dst");
        let _ = std::fs::remove_dir_all(&dst);
        std::fs::create_dir_all(&dst).expect("seed dst");
        let junk = dst.join("a.abc123.partial");
        let keep = dst.join("keep.txt");
        std::fs::write(&junk, b"x").expect("seed partial");
        std::fs::write(&keep, b"ok").expect("seed keep");
        let path = std::env::temp_dir().join("rclone_gui_jobs_test_reconcile.json");
        let _ = std::fs::remove_file(&path);
        let dst_s = dst.to_string_lossy().into_owned();
        let snap = serde_json::json!({
            "jobs": [{"id": "job-orphan", "kind": "copy", "src": "/a",
                "dst": dst_s, "status": "running", "progress": 42,
                "error": null, "child_done": 0, "child_total": 1}],
            "children": {"job-orphan": [
                {"job_id": "job-orphan", "path": "a", "is_dir": false,
                 "status": "running", "error": null}]},
        });
        std::fs::write(&path, serde_json::to_string(&snap).expect("snap"))
            .expect("write snap");
        let store = JobStore::with_path(path.clone());
        assert_eq!(store.get("job-orphan").map(|j| j.status), Some(JobStatus::Running));
        assert_eq!(store.reconcile_orphans(), 1);
        let fixed = store.get("job-orphan").expect("job");
        assert_eq!(fixed.status, JobStatus::Error);
        assert!(fixed.error.as_deref().unwrap_or("").contains("gián đoạn phiên trước"));
        let kids = store.children_of("job-orphan");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].status, JobStatus::Error);
        assert!(!junk.exists(), ".partial phải bị dọn");
        assert!(keep.exists(), "file thường phải giữ");
        assert_eq!(store.reconcile_orphans(), 0);
        let _ = std::fs::remove_dir_all(&dst);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn policy_stamped_at_enqueue_and_legacy_defaults_ask_once() {
        // UNIVERSAL: enqueue thường default AskOnce; enqueue_with_policy đóng
        // dấu Deny/AllowSystem; file cũ thiếu `policy` vẫn nạp = AskOnce.
        let (store, path) = temp_store("policy_stamp");
        let plain = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        assert_eq!(plain.policy, Policy::AskOnce);
        let denied =
            store.enqueue_with_policy(JobKind::Move, Some("/a".into()), Some("/b".into()), Policy::Deny);
        assert_eq!(denied.policy, Policy::Deny);
        assert_eq!(store.get(&denied.id).map(|j| j.policy), Some(Policy::Deny));
        // UNIVERSAL: legacy JSON không có `policy` → default AskOnce, IPC đọc được.
        let legacy = serde_json::json!({
            "id": "legacy-1", "kind": "list", "src": "/s", "dst": null,
            "status": "queued", "progress": 0, "error": null,
            "child_done": 0, "child_total": 0
        });
        let job: Job = serde_json::from_value(legacy).expect("legacy job loads");
        assert_eq!(job.policy, Policy::AskOnce);
        let v = serde_json::to_value(&denied).expect("serialize");
        assert_eq!(v.get("policy").and_then(|p| p.as_str()), Some("deny"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn smart_compact_done_keeps_errors_drops_done() {
        // UNIVERSAL: Done gọn con — lỗi-giữ/xong-xóa, giữ đếm, Error giữ hết.
        let (store, path) = temp_store("smart_compact");
        let job = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        store.set_children_from_items(
            &job.id,
            vec![
                ManifestItem { path: "ok1".into(), is_dir: false },
                ManifestItem { path: "ok2".into(), is_dir: false },
                ManifestItem { path: "bad".into(), is_dir: false },
            ],
        );
        store.update_child_progress(&job.id, 0, JobStatus::Done, None);
        store.update_child_progress(
            &job.id,
            2,
            JobStatus::Error,
            Some("boom".to_string()),
        );
        store.update(&job.id, |j| {
            j.status = JobStatus::Done;
            j.progress = 100;
            j.child_done = j.child_total;
        });
        store.compact_done_children(&job.id);
        let kids = store.children_of(&job.id);
        // UNIVERSAL: vé Done bị xóa, vé lỗi giữ lại để xem lại.
        assert!(kids.iter().all(|k| k.status != JobStatus::Done));
        assert!(kids.iter().any(|k| k.status == JobStatus::Error));
        let parent = store.get(&job.id).expect("parent");
        assert_eq!((parent.child_done, parent.child_total), (3, 3));
        // UNIVERSAL: Error giữ nguyên hết (không gọn) để retry/review.
        let ejob = store.enqueue(JobKind::Copy, Some("/a".into()), Some("/b".into()));
        store.set_children_from_items(
            &ejob.id,
            vec![ManifestItem { path: "bad".into(), is_dir: false }],
        );
        store.update_child_progress(&ejob.id, 0, JobStatus::Error, Some("boom".to_string()));
        store.update(&ejob.id, |j| j.status = JobStatus::Error);
        let ekids = store.children_of(&ejob.id);
        assert_eq!(ekids.len(), 1);
        let _ = std::fs::remove_file(&path);
    }
}
