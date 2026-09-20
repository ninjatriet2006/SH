//! UNIVERSAL S2: tầng cha của job queue — `Job`, `JobStore`, worker điều phối,
//! persist, IPC `job_enqueue`/`job_list`/`job_cancel`, event `job_update`.
//! Tầng con (vé con, manifest, chạy từng vé) nằm ở `super::queue`.
//! Tách từ `core/jobs.rs` cũ; tên/trường IPC + event giữ NGUYÊN.

use super::queue::{ChildMode, ManifestItem, QueueItem, manifest, sort_manifest};
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
            _ => None,
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
}

/// UNIVERSAL: trích % hoàn thành từ object `stats` của rclone (`--use-json-log`).
/// Ưu tiên `percentage` (float 0-100), rớt về `bytes/totalBytes` khi thiếu.
pub fn stats_percent(stats: &serde_json::Value) -> Option<u8> {
    if let Some(p) = stats.get("percentage").and_then(|v| v.as_f64()) {
        if p.is_finite() {
            return Some(p.clamp(0.0, 100.0).round() as u8);
        }
    }
    let bytes = stats.get("bytes").and_then(|v| v.as_f64())?;
    let total = stats.get("totalBytes").and_then(|v| v.as_f64())?;
    if !bytes.is_finite() || !total.is_finite() || total <= 0.0 {
        return None;
    }
    Some((bytes / total * 100.0).clamp(0.0, 100.0).round() as u8)
}

/// UNIVERSAL: bóc một dòng log JSON của transfer thành % để map vào `job.progress`;
/// dòng không phải stats (log thường, lỗi text) trả `None` để worker bỏ qua.
pub fn parse_progress_line(line: &str) -> Option<u8> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    stats_percent(v.get("stats")?)
}

/// UNIVERSAL S2: JOB tính quyết định across 1 lần lúc dispatch (chuyển từ
/// `queue::run_child_transfer` lên đây để QUEUE khỏi đọc engine settings):
/// dính ổ máy / cùng tên remote → false (trung chuyển qua local như cũ);
/// khác tên → cùng hãng (`same_provider`) + bật cờ mới true.
fn decide_across(src: &str, dst: &str, flags: &crate::settings::engine::GlobalFlags) -> bool {
    let (src_remote, _) = crate::logic::file_ops::parse_remote_path(src);
    let (dst_remote, _) = crate::logic::file_ops::parse_remote_path(dst);
    if src_remote == "Local" || dst_remote == "Local" || src_remote == dst_remote {
        return false;
    }
    crate::actions::types::same_provider(&src_remote, &dst_remote) && flags.server_side_across
}

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
    pub fn enqueue(&self, kind: JobKind, src: Option<String>, dst: Option<String>) -> Job {
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
            _ => {}
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
                _ => {
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
                Ok(()) => self.update(&id, |j| {
                    if self.is_cancel_requested(&id) {
                        j.status = JobStatus::Cancelled;
                    } else {
                        j.status = JobStatus::Done;
                        j.progress = 100;
                        j.child_done = j.child_total;
                    }
                }),
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
            _ => {}
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
        if flags.bulk_transfer {
            let across = decide_across(&src, &dst, &flags);
            self.set_children_bulk(&job.id, across, flags);
        } else {
            match manifest(&src) {
                Ok(items) if !items.is_empty() => {
                    self.set_children_from_items(&job.id, items);
                }
                _ => {
                    let across = decide_across(&src, &dst, &flags);
                    self.set_children_bulk(&job.id, across, flags);
                }
            }
        }
        self.execute_children(job)
    }

    /// UNIVERSAL: delete thật (`NoTrash`) — phán đoán kiểu trước qua `is_dir`
    /// rồi `purge`/`deletefile` + thử lệnh còn lại, như `execute_delete`.
    fn run_delete_job(&self, job: &Job) -> Result<(), String> {
        let src = job.src.clone().ok_or_else(|| "delete job missing src".to_string())?;
        let (remote, real_path) = crate::logic::file_ops::parse_remote_path(&src);
        let target = crate::core::rclone_caller::build_target(&remote, &real_path);
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        // UNIVERSAL: chạm Tier route để giữ một nguồn sự thật về tuyến xóa.
        let _ = crate::actions::delete_op::Route::classify(&remote);
        let is_dir = crate::actions::types::is_dir(&target).unwrap_or(true);
        if is_dir {
            crate::logic::file_ops::run_with_sudo_fallback(&remote, "rm", std::slice::from_ref(&real_path), || {
                let output = crate::core::rclone_caller::run_cmd(&["purge", &target])?;
                if output.status.success() {
                    return Ok(());
                }
                let retry = crate::core::rclone_caller::run_cmd(&["deletefile", &target])?;
                if retry.status.success() {
                    return Ok(());
                }
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            })
        } else {
            crate::logic::file_ops::run_with_sudo_fallback(&remote, "rm", std::slice::from_ref(&real_path), || {
                let output = crate::core::rclone_caller::run_cmd(&["deletefile", &target])?;
                if output.status.success() {
                    return Ok(());
                }
                let retry = crate::core::rclone_caller::run_cmd(&["purge", &target])?;
                if retry.status.success() {
                    return Ok(());
                }
                Err(String::from_utf8_lossy(&output.stderr).into_owned())
            })
        }
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
                // Job dở dang từ lần chạy trước → đưa về queued để chạy lại ở P2.
                let mut job = job;
                if matches!(job.status, JobStatus::Running) {
                    job.status = JobStatus::Queued;
                    job.progress = 0;
                }
                if job.status == JobStatus::Queued {
                    q.push_back(job.id.clone());
                }
                inner.insert(job.id.clone(), job);
            }
            for (id, mut list) in kids {
                for k in list.iter_mut() {
                    if matches!(k.status, JobStatus::Running) {
                        k.status = JobStatus::Queued;
                    }
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
        // remote luôn false mà không cần gọi rclone.
        let flags = crate::settings::engine::GlobalFlags {
            server_side_across: true,
            ..crate::settings::engine::GlobalFlags::default()
        };
        assert!(!decide_across("Local:/a", "Local:/b", &flags));
        assert!(!decide_across("Local:/a", "R:/b", &flags));
        assert!(!decide_across("R:/a", "R:/b", &flags));
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
        // IPC giữ trường cũ + thêm mới.
        let v = serde_json::to_value(store.get(&job.id).expect("job")).expect("json");
        for f in ["id", "kind", "src", "dst", "status", "progress", "error", "child_done", "child_total"] {
            assert!(v.get(f).is_some(), "missing field {f}");
        }
        let _ = std::fs::remove_file(&path);
    }
}
