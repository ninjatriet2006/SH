use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

static TIME_OFFSET: OnceLock<i64> = OnceLock::new();
/// Bộ đếm đơn điệu để `generate_id` không trùng khi gọi nhiều lần trong cùng mili-giây.
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Lấy mili-giây từ epoch, không panic khi clock lệch (trả 0 thay vì crash thread).
fn system_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn init_time_sync() {
    let _ = std::thread::spawn(|| {
        let local_now = system_millis();
        
        // Dùng HTTPS để tránh lộ nội dung/lỗi mạng giữa đường.
        if let Ok(resp) = ureq::get("https://worldtimeapi.org/api/timezone/Etc/UTC").call() {
            // Read body as string
            if let Ok(body) = resp.into_string() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(unixtime) = json.get("unixtime").and_then(|v| v.as_i64()) {
                        let online_now = unixtime * 1000;
                        let offset = online_now - local_now;
                        // Kẹp offset bất thường (>24h chắc chắn là clock/API lỗi):
                        // áp nguyên sẽ làm mọi timestamp trong session nhảy vọt.
                        const DAY_MS: i64 = 24 * 60 * 60 * 1000;
                        if offset.abs() > DAY_MS {
                            eprintln!(
                                "[time] offset bất thường ({}ms), bỏ qua, dùng giờ local",
                                offset
                            );
                            let _ = TIME_OFFSET.set(0);
                        } else {
                            let _ = TIME_OFFSET.set(offset);
                            println!("Synced time with WorldTimeAPI. Offset: {}ms", offset);
                        }
                        return;
                    }
                }
            }
        }
        
        let _ = TIME_OFFSET.set(0);
        println!("Failed to sync time, using local time (offset = 0)");
    });
}

pub fn current_timestamp() -> i64 {
    let local = system_millis();
    let offset = TIME_OFFSET.get().copied().unwrap_or(0);
    local + offset
}

/// Tạo ID duy nhất: prefix + timestamp_ms + counter. Counter đảm bảo hai lệnh
/// trong cùng mili-giây vẫn khác ID (trước đây chỉ dùng timestamp nên dễ trùng).
pub fn generate_id(prefix: &str) -> String {
    let timestamp = current_timestamp();
    let n = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}_{}_{}", prefix, timestamp, n)
}
