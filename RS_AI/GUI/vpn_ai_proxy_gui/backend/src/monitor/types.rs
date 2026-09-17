use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::fingerprint::LeakFinding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTrafficLog {
    pub id: String,
    pub timestamp: String,
    pub route_id: String,
    pub method: String,
    pub port: u16,
    pub path: String,
    pub target_url: String,
    pub tunnel_id: String,
    pub key_used_preview: Option<String>,

    // Status & Timing
    pub status_code: u16,
    pub duration_ms: u64,
    pub is_streaming: bool,

    // 100% Raw Wire Data
    pub raw_request_headers: Vec<(String, String)>,
    pub raw_forwarded_headers: Vec<(String, String)>,
    pub raw_request_body: String,
    pub raw_response_headers: Vec<(String, String)>,
    pub raw_response_body: String,

    // Forensic findings for privacy awareness
    pub leaked_findings: Vec<LeakFinding>,
}

pub struct RingBufferLog {
    max_entries: AtomicUsize,
    entries: RwLock<VecDeque<RawTrafficLog>>,
}

impl RingBufferLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: AtomicUsize::new(max_entries),
            entries: RwLock::new(VecDeque::with_capacity(max_entries)),
        }
    }

    pub fn push(&self, log: RawTrafficLog) {
        let mut queue = self.entries.write();
        if queue.len() >= self.max_entries.load(Ordering::Relaxed) {
            queue.pop_front(); // Tự động dọn dẹp ghi đè bản ghi cũ nhất
        }
        queue.push_back(log);
    }

    pub fn get_all(&self) -> Vec<RawTrafficLog> {
        let queue = self.entries.read();
        queue.iter().cloned().collect()
    }

    pub fn clear(&self) {
        let mut queue = self.entries.write();
        queue.clear();
    }

    /// Đổi giới hạn entries lúc runtime (khi user đổi max_log_entries trong Settings).
    /// Trước đây setting này là no-op cho tới khi restart app vì capacity chỉ set ở new().
    pub fn set_max_entries(&self, max_entries: usize) {
        self.max_entries.store(max_entries, Ordering::Relaxed);
        let mut queue = self.entries.write();
        while queue.len() > max_entries {
            queue.pop_front();
        }
    }

    pub fn count(&self) -> usize {
        self.entries.read().len()
    }

    pub fn update_response_body(&self, id: &str, body: String) {
        let mut queue = self.entries.write();
        if let Some(entry) = queue.iter_mut().find(|e| e.id == id) {
            entry.raw_response_body = body;
        }
    }
}
