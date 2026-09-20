//! P2 job queue: Job{id,kind,src,dst,status,progress}, store RAM + JSON,
//! worker tuần tự gọi actions thật theo kind, event `job_update`, cancel qua cờ.
//! Copy/Move đi qua phân tuyến + args dùng chung (`copy_op`/`move_op`/`transfer`),
//! Delete/List/Manifest qua `delete_op`/`list`/`lsjson -R` hiện có.

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

/// Một đơn vị việc trong hàng chờ.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub kind: JobKind,
    pub src: Option<String>,
    pub dst: Option<String>,
    pub status: JobStatus,
    pub progress: u8,
    pub error: Option<String>,
}

/// UNIVERSAL: một vé điểm danh — một món trong thư mục đã bóc qua `lsjson -R`.
/// `path` là `Path` tương đối rclone trả; `is_dir` để tạo vỏ trước, chép file sau.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestItem {
    pub path: String,
    pub is_dir: bool,
}

/// UNIVERSAL: dựng args `lsjson -R` cho `manifest()`; bật `fast_list` thì thêm
/// `--fast-list` (khớp quy ước `file_ops::check_conflicts`), tắt thì giữ args cũ.
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
    let (remote, real) = crate::logic::file_ops::parse_remote_path(src);
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

/// Store thread-safe: RAM (`Mutex`) + persist JSON best-effort.
pub struct JobStore {
    inner: Mutex<HashMap<String, Job>>,
    queue: Mutex<VecDeque<String>>,
    cancelled: Mutex<HashSet<String>>,
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
                    self.persist();
                    return Ok(done);
                }
            }
        }
        self.persist();
        self.get(id).ok_or_else(|| "job not found".to_string())
    }

    fn is_cancel_requested(&self, id: &str) -> bool {
        self.cancelled.lock().map(|f| f.contains(id)).unwrap_or(false)
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut Job)) -> Option<Job> {
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

    /// Chạy tuần tự toàn bộ hàng chờ trên thread hiện tại (P2: gọi actions thật).
    /// `emit` được gọi sau mỗi đổi trạng thái để IPC phát event `job_update`.
    pub fn run_queue_sync(&self, mut emit: Option<impl FnMut(Job)>) {
        while let Some(id) = self.pop_next() {
            if self.is_cancel_requested(&id) {
                if let Some(job) = self.update(&id, |j| {
                    j.status = JobStatus::Cancelled;
                }) {
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
            }) {
                if let Some(e) = emit.as_mut() {
                    e(job);
                }
            } else {
                continue;
            }
            // UNIVERSAL: chạy thật theo kind; hủy giữa chừng → `Cancelled`,
            // lỗi rclone → `Error` kèm stderr, xong → `Done` 100%.
            let outcome = self.execute_job(&snapshot, &mut emit);
            let done = match outcome {
                Ok(()) => self.update(&id, |j| {
                    if self.is_cancel_requested(&id) {
                        j.status = JobStatus::Cancelled;
                    } else {
                        j.status = JobStatus::Done;
                        j.progress = 100;
                    }
                }),
                Err(e) if e == "job cancelled" || self.is_cancel_requested(&id) => {
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

    /// UNIVERSAL: copy/move thật — phân tuyến `copy_op`/`move_op` + cùng-hãng +
    /// cờ engine → args `transfer::build_transfer_args`, chạy rclone pipe log JSON
    /// rồi map `stats` thành `job.progress`; Local↔Local rớt qua sudo fallback.
    fn run_transfer_job<E: FnMut(Job)>(
        &self,
        job: &Job,
        cmd: &str,
        is_copy: bool,
        emit: &mut Option<E>,
    ) -> Result<(), String> {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};

        let src = job.src.clone().ok_or_else(|| format!("{} job missing src", cmd))?;
        let dst = job.dst.clone().ok_or_else(|| format!("{} job missing dst", cmd))?;
        let (src_remote, src_real) = crate::logic::file_ops::parse_remote_path(&src);
        let (dst_remote, dst_real) = crate::logic::file_ops::parse_remote_path(&dst);
        // UNIVERSAL: cùng phân tuyến Copy/Move như `execute_copy`/`execute_move`.
        let local_local = src_remote == "Local" && dst_remote == "Local";
        let server_side_across = if src_remote == "Local" || dst_remote == "Local" || src_remote == dst_remote {
            false
        } else {
            let same = crate::actions::types::same_provider(&src_remote, &dst_remote);
            let enabled = crate::settings::engine::load_engine_flags()
                .map(|f| f.server_side_across)
                .unwrap_or(false);
            same && enabled
        };
        // UNIVERSAL: chạm Tier phân tuyến để giữ một nguồn sự thật về route/cap.
        if is_copy {
            let route = crate::actions::copy_op::Route::classify(&src_remote, &dst_remote);
            let _ = route.cap_with_provider(crate::actions::types::SameProvider(server_side_across), server_side_across);
        } else {
            let route = crate::actions::move_op::Route::classify(&src_remote, &dst_remote);
            let _ = route.cap_with_provider(crate::actions::types::SameProvider(server_side_across), server_side_across);
        }
        let src_target = crate::core::rclone_caller::build_target(&src_remote, &src_real);
        let dst_target = crate::core::rclone_caller::build_target(&dst_remote, &dst_real);
        let flags = crate::settings::engine::load_engine_flags().unwrap_or_default();
        let args =
            crate::logic::transfer::build_transfer_args(cmd, &src_target, &dst_target, &flags, server_side_across);
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        let mut child = Command::new("rclone")
            .args(&args)
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Lỗi khi khởi chạy tiến trình rclone: {}", e))?;
        // UNIVERSAL: map event transfer (`stats` trong log JSON) vào `job.progress`.
        let mut error_msgs: Vec<String> = Vec::new();
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                if self.is_cancel_requested(&job.id) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("job cancelled".to_string());
                }
                if let Some(p) = parse_progress_line(&line) {
                    if let Some(updated) = self.update(&job.id, |j| {
                        j.progress = p;
                    }) {
                        if let Some(e) = emit.as_mut() {
                            e(updated);
                        }
                    }
                }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                    let is_err = v.get("level").and_then(|l| l.as_str()) == Some("error");
                    if is_err {
                        if let Some(msg) = v.get("msg").and_then(|m| m.as_str()) {
                            error_msgs.push(msg.to_string());
                        }
                    }
                }
            }
        }
        let status = child
            .wait()
            .map_err(|e| format!("Lỗi khi đợi tiến trình rclone kết thúc: {}", e))?;
        if self.is_cancel_requested(&job.id) {
            return Err("job cancelled".to_string());
        }
        if status.success() {
            return Ok(());
        }
        let err_str = if error_msgs.is_empty() {
            format!("Lệnh {} thất bại với mã lỗi: {}", cmd, status)
        } else {
            error_msgs.join("\n")
        };
        // UNIVERSAL: Local↔Local rớt qua `pkexec cp/mv` như `execute_copy`/`execute_move`.
        if local_local {
            let action = if is_copy { "cp" } else { "mv" };
            return crate::logic::file_ops::run_with_sudo_fallback(
                "Local",
                action,
                &[src_real, dst_real],
                || Err(err_str),
            );
        }
        Err(err_str)
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

    /// Ghi snapshot jobs ra JSON (best-effort, không bao giờ crash app).
    pub fn persist(&self) {
        let jobs: Vec<Job> = self.list();
        let text = match serde_json::to_string(&jobs) {
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
        let mut jobs: Vec<Job> = match serde_json::from_str(&text) {
            Ok(jobs) => jobs,
            Err(_) => return,
        };
        if let (Ok(mut inner), Ok(mut q)) = (self.inner.lock(), self.queue.lock()) {
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
    fn manifest_scans_real_dir_dirs_first() {
        // UNIVERSAL: quét thật — vỏ rỗng `Ok(vec![])`, có đồ thì dir trước file sau.
        if !rclone_present() {
            return;
        }
        let base = std::env::temp_dir().join("rclone_gui_jobs_manifest");
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
}
