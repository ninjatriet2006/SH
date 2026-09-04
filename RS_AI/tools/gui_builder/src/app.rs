/*
[INTEGRITY NOTES]
- Mục đích: Trạng thái ứng dụng GUI BUILDER và logic điều phối build.
- Trách nhiệm: Giữ danh sách project, lựa chọn của người dùng, hàng đợi build,
  log và tiến trình. Nhận `BuildEvent` từ thread nền rồi cập nhật trạng thái.
- Tương tác: `main.rs` (event loop), `ui.rs` (render), `builder.rs` (thread build).
*/

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::builder::{BuildEvent, build_project};
use crate::discovery::Project;

/// Màn hình hiện tại.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Chọn project bằng phím.
    Select,
    /// Đang build / đã build xong, hiển thị log + tiến trình.
    Building,
}

/// Kết quả build của một project (hiện ở phần tổng kết).
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub name: String,
    pub ok: bool,
    pub message: String,
}

pub struct App {
    pub root: PathBuf,
    pub projects: Vec<Project>,
    /// Chỉ số con trỏ trong danh sách.
    pub cursor: usize,
    /// Các project đã tick chọn (theo chỉ số).
    pub selected: Vec<bool>,
    pub screen: Screen,

    /// Hàng đợi các chỉ số project còn phải build.
    queue: Vec<usize>,
    /// Project đang build (chỉ số trong `projects`).
    pub current: Option<usize>,
    /// Số project đã hoàn tất trong lượt này.
    pub done_count: usize,
    /// Tổng số project của lượt build này.
    pub total_count: usize,

    pub stage: String,
    pub logs: Vec<String>,
    pub progress: (usize, usize),
    pub current_unit: String,
    pub results: Vec<BuildResult>,
    pub warnings: usize,

    tx: Sender<BuildEvent>,
    rx: Receiver<BuildEvent>,
    pub should_quit: bool,
}

/// Số dòng log giữ lại — đủ để xem lỗi mà không phình bộ nhớ.
const LOG_LIMIT: usize = 500;

impl App {
    pub fn new(root: PathBuf, projects: Vec<Project>) -> Self {
        let (tx, rx) = channel();
        let n = projects.len();
        Self {
            root,
            projects,
            cursor: 0,
            selected: vec![false; n],
            screen: Screen::Select,
            queue: Vec::new(),
            current: None,
            done_count: 0,
            total_count: 0,
            stage: String::new(),
            logs: Vec::new(),
            progress: (0, 0),
            current_unit: String::new(),
            results: Vec::new(),
            warnings: 0,
            tx,
            rx,
            should_quit: false,
        }
    }

    // ── Điều hướng danh sách ─────────────────────────────────────────────────

    pub fn move_cursor(&mut self, delta: isize) {
        if self.projects.is_empty() {
            return;
        }
        let len = self.projects.len() as isize;
        let mut next = self.cursor as isize + delta;
        // Cuộn vòng để không bị kẹt ở hai đầu danh sách.
        if next < 0 {
            next = len - 1;
        } else if next >= len {
            next = 0;
        }
        self.cursor = next as usize;
    }

    pub fn toggle_current(&mut self) {
        if let Some(flag) = self.selected.get_mut(self.cursor) {
            *flag = !*flag;
        }
    }

    pub fn select_all(&mut self) {
        let all = self.selected.iter().all(|s| *s);
        // Đang chọn hết → bỏ hết, ngược lại chọn hết.
        self.selected.iter_mut().for_each(|s| *s = !all);
    }

    pub fn selected_count(&self) -> usize {
        self.selected.iter().filter(|s| **s).count()
    }

    // ── Bắt đầu build ────────────────────────────────────────────────────────

    /// Xếp hàng đợi build. Nếu chưa tick gì thì build project đang trỏ tới.
    pub fn start_build(&mut self) {
        let mut queue: Vec<usize> = self
            .selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect();

        if queue.is_empty() {
            if self.projects.is_empty() {
                return;
            }
            queue.push(self.cursor);
        }

        self.queue = queue;
        self.total_count = self.queue.len();
        self.done_count = 0;
        self.results.clear();
        self.logs.clear();
        self.warnings = 0;
        self.screen = Screen::Building;
        self.next_in_queue();
    }

    /// Lấy project tiếp theo trong hàng đợi và spawn thread build.
    fn next_in_queue(&mut self) {
        let Some(idx) = self.queue.first().copied() else {
            self.current = None;
            self.stage = "Hoàn tất".to_string();
            return;
        };
        self.queue.remove(0);
        self.current = Some(idx);
        self.progress = (0, 0);
        self.current_unit.clear();

        let project = self.projects[idx].clone();
        let root = self.root.clone();
        let tx = self.tx.clone();
        self.push_log(format!("── Bắt đầu: {} ──", project.bin_name));
        std::thread::spawn(move || build_project(root, project, tx));
    }

    // ── Nhận sự kiện từ thread build ────────────────────────────────────────

    /// Rút hết sự kiện đang chờ. Trả `true` nếu có thay đổi cần vẽ lại.
    pub fn drain_events(&mut self) -> bool {
        let mut changed = false;
        while let Ok(ev) = self.rx.try_recv() {
            changed = true;
            match ev {
                BuildEvent::Stage(s) => {
                    self.stage = s.clone();
                    self.push_log(format!("▶ {s}"));
                }
                BuildEvent::Log(l) => self.push_log(l),
                BuildEvent::Warn(w) => {
                    self.warnings += 1;
                    self.push_log(format!("⚠ {w}"));
                }
                BuildEvent::Progress { done, total, name } => {
                    self.progress = (done, total);
                    self.current_unit = name;
                }
                BuildEvent::Finished { ok, message } => {
                    let name = self
                        .current
                        .and_then(|i| self.projects.get(i))
                        .map(|p| p.bin_name.clone())
                        .unwrap_or_else(|| "?".to_string());
                    self.results.push(BuildResult {
                        name,
                        ok,
                        message: message.clone(),
                    });
                    self.push_log(if ok {
                        format!("✔ {message}")
                    } else {
                        format!("✖ {message}")
                    });
                    self.done_count += 1;
                    // Tiếp tục project kế tiếp (nếu còn) để build hàng loạt.
                    self.next_in_queue();
                }
            }
        }
        changed
    }

    fn push_log(&mut self, line: String) {
        self.logs.push(line);
        if self.logs.len() > LOG_LIMIT {
            let excess = self.logs.len() - LOG_LIMIT;
            self.logs.drain(0..excess);
        }
    }

    /// Còn project đang build hay không (để UI biết đã xong chưa).
    pub fn is_running(&self) -> bool {
        self.current.is_some() && self.done_count < self.total_count
    }

    /// Quay lại màn hình chọn (chỉ khi đã build xong).
    pub fn back_to_select(&mut self) {
        if !self.is_running() {
            self.screen = Screen::Select;
            self.current = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::BuildKind;

    fn sample(n: usize) -> Vec<Project> {
        (0..n)
            .map(|i| Project {
                package: format!("p{i}"),
                bin_name: format!("bin{i}"),
                rel_dir: format!("dir/{i}"),
                kind: BuildKind::Cargo,
            })
            .collect()
    }

    fn app(n: usize) -> App {
        App::new(PathBuf::from("/tmp"), sample(n))
    }

    #[test]
    fn cursor_wraps_at_both_ends() {
        let mut a = app(3);
        assert_eq!(a.cursor, 0);
        a.move_cursor(-1);
        assert_eq!(a.cursor, 2, "lên từ đầu danh sách phải nhảy xuống cuối");
        a.move_cursor(1);
        assert_eq!(a.cursor, 0, "xuống từ cuối phải quay về đầu");
    }

    #[test]
    fn cursor_noop_on_empty_list() {
        let mut a = app(0);
        a.move_cursor(1);
        a.move_cursor(-1);
        assert_eq!(a.cursor, 0);
    }

    #[test]
    fn toggle_and_select_all() {
        let mut a = app(3);
        a.toggle_current();
        assert_eq!(a.selected_count(), 1);
        a.select_all();
        assert_eq!(a.selected_count(), 3, "chưa chọn hết → chọn tất cả");
        a.select_all();
        assert_eq!(a.selected_count(), 0, "đang chọn hết → bỏ tất cả");
    }

    #[test]
    fn start_build_uses_cursor_when_nothing_ticked() {
        let mut a = app(3);
        a.cursor = 2;
        a.start_build();
        assert_eq!(a.total_count, 1);
        assert_eq!(a.current, Some(2));
        assert_eq!(a.screen, Screen::Building);
    }

    #[test]
    fn start_build_queues_all_ticked() {
        let mut a = app(4);
        a.selected[1] = true;
        a.selected[3] = true;
        a.start_build();
        assert_eq!(a.total_count, 2);
        assert_eq!(a.current, Some(1), "build theo thứ tự danh sách");
    }

    #[test]
    fn start_build_noop_on_empty_project_list() {
        let mut a = app(0);
        a.start_build();
        assert_eq!(a.screen, Screen::Select, "không có project thì không đổi màn hình");
    }

    #[test]
    fn log_is_capped() {
        let mut a = app(1);
        for i in 0..(LOG_LIMIT + 50) {
            a.push_log(format!("line {i}"));
        }
        assert_eq!(a.logs.len(), LOG_LIMIT);
        // Giữ lại phần cuối (mới nhất), bỏ phần đầu.
        assert!(a.logs.last().unwrap().contains(&format!("{}", LOG_LIMIT + 49)));
    }

    #[test]
    fn back_to_select_blocked_while_running() {
        let mut a = app(2);
        a.selected[0] = true;
        a.selected[1] = true;
        a.total_count = 2;
        a.done_count = 0;
        a.current = Some(0);
        a.screen = Screen::Building;
        a.back_to_select();
        assert_eq!(a.screen, Screen::Building, "đang build thì không cho thoát");

        a.done_count = 2;
        a.back_to_select();
        assert_eq!(a.screen, Screen::Select);
    }
}
