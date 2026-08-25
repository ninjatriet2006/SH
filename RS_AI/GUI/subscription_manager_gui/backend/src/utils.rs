use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::OnceLock;

static TIME_OFFSET: OnceLock<i64> = OnceLock::new();

pub fn init_time_sync() {
    let _ = std::thread::spawn(|| {
        let local_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        
        // Use a basic ureq get. In v3, ureq::get exists.
        if let Ok(resp) = ureq::get("http://worldtimeapi.org/api/timezone/Etc/UTC").call() {
            // Read body as string
            if let Ok(body) = resp.into_string() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(unixtime) = json.get("unixtime").and_then(|v| v.as_i64()) {
                        let online_now = unixtime * 1000;
                        let offset = online_now - local_now;
                        let _ = TIME_OFFSET.set(offset);
                        println!("Synced time with WorldTimeAPI. Offset: {}ms", offset);
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
    let local = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
    let offset = TIME_OFFSET.get().copied().unwrap_or(0);
    local + offset
}

/// Tạo ID ngẫu nhiên có độ dài thống nhất với prefix và được đồng bộ hóa thời gian
pub fn generate_id(prefix: &str) -> String {
    let timestamp = current_timestamp();
    // Tạo thêm 4 số ngẫu nhiên nếu cần hoặc dùng thẳng timestamp, ở đây ta dùng timestamp để đơn giản và nhất quán với cũ
    format!("{}_{}", prefix, timestamp)
}
