/*
[INTEGRITY NOTES]
- Mục đích: Theo dõi biến động hệ thống file nội bộ (Local) để UI tự làm mới.
- Trách nhiệm:
  + Khởi tạo inotify watcher (qua crate `notify`) khi ứng dụng start.
  + Chuyển đường dẫn đang xem của mỗi pane thành lệnh watch/unwatch.
  + Phát sự kiện `local-dir-changed` lên Frontend khi có thay đổi thật sự.
- Tương tác: Gọi từ `lib.rs` (setup) và `api/files.rs` (mỗi lần list_files).
  Frontend lắng nghe trong `components/DualPaneExplorer.ts`.
*/

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::logic::app_state::AppState;

/// Khoảng thời gian gộp các sự kiện liên tiếp (debounce).
/// Ghi một file thường sinh nhiều event (create + modify + close_write); nếu phát
/// hết thì Frontend sẽ nạp lại thư mục nhiều lần liên tiếp một cách vô ích.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Tên hàm: init
/// Mô tả: Khởi tạo watcher và luồng nền chuyển sự kiện thành `local-dir-changed`.
/// Gọi một lần trong `tauri::Builder::setup`.
pub fn init(app: &AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();
    let app_for_thread = app.clone();

    // Luồng nền: nhận event từ watcher, lọc nhiễu, debounce rồi phát lên UI.
    std::thread::spawn(move || {
        let mut last_emit = Instant::now() - DEBOUNCE;
        for res in rx {
            let event: notify::Event = match res {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[watcher] lỗi theo dõi thư mục: {:?}", e);
                    continue;
                }
            };

            // Bỏ qua sự kiện chỉ đọc/mở file — không làm đổi nội dung thư mục.
            if matches!(event.kind, EventKind::Access(_)) {
                continue;
            }

            if last_emit.elapsed() < DEBOUNCE {
                continue;
            }
            last_emit = Instant::now();
            let _ = app_for_thread.emit("local-dir-changed", ());
        }
    });

    match notify::RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        notify::Config::default(),
    ) {
        Ok(watcher) => {
            let state = app.state::<AppState>();
            store_watcher(&state, watcher);
        }
        Err(e) => {
            // Không phải lỗi chí tử: ứng dụng vẫn dùng được, chỉ mất tự làm mới.
            eprintln!("[watcher] không khởi tạo được inotify watcher: {}", e);
        }
    }
}

/// Lưu watcher vừa tạo vào AppState. Tách thành hàm riêng để `MutexGuard` không
/// còn sống khi `tauri::State` tạm bị drop.
fn store_watcher(state: &AppState, watcher: RecommendedWatcher) {
    if let Ok(mut slot) = state.local_watcher.lock() {
        *slot = Some(watcher);
    }
}

/// Tên hàm: watch_pane
/// Mô tả: Đặt `pane` theo dõi `local_path`. Truyền `None` để ngừng theo dõi
/// (ví dụ khi pane chuyển sang một remote cloud).
///
/// Chỉ unwatch một đường dẫn khi không còn pane nào dùng nó — nếu không, hai pane
/// mở cùng thư mục sẽ khiến pane này tắt watcher của pane kia.
pub fn watch_pane(state: &AppState, pane: &str, local_path: Option<&str>) {
    let Ok(mut watcher_slot) = state.local_watcher.lock() else {
        return;
    };
    let Some(watcher) = watcher_slot.as_mut() else {
        return; // Watcher không khả dụng trên nền tảng này
    };
    let Ok(mut watched) = state.watched_paths.lock() else {
        return;
    };

    let previous = watched.get(pane).cloned();
    if previous.as_deref() == local_path {
        return; // Không đổi gì
    }

    // Cập nhật bảng trước để tính tập đường dẫn còn cần thiết cho chính xác.
    match local_path {
        Some(p) => watched.insert(pane.to_string(), p.to_string()),
        None => watched.remove(pane),
    };

    if let Some(old) = previous {
        let still_needed: HashSet<&String> = watched.values().collect();
        if !still_needed.contains(&old) {
            let _ = watcher.unwatch(Path::new(&old));
        }
    }

    if let Some(new_path) = local_path {
        // NonRecursive: chỉ quan tâm biến động ngay trong thư mục đang hiển thị.
        if let Err(e) = watcher.watch(Path::new(new_path), RecursiveMode::NonRecursive) {
            eprintln!("[watcher] không theo dõi được '{}': {}", new_path, e);
        }
    }
}
