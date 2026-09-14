use std::collections::VecDeque;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::fingerprint::LeakFinding;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: String,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub target_url: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub leaked_findings: Vec<LeakFinding>,
    pub client_headers: Vec<(String, String)>,
    pub forwarded_headers: Vec<(String, String)>,
    pub prompt_preview: Option<String>,
    pub response_preview: Option<String>,
    pub is_streaming: bool,
    pub bytes_sent: usize,
    pub bytes_received: usize,
}

pub struct RingBufferLog {
    max_entries: usize,
    entries: RwLock<VecDeque<RequestLog>>,
}

impl RingBufferLog {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: RwLock::new(VecDeque::with_capacity(max_entries)),
        }
    }

    pub fn push(&self, log: RequestLog) {
        let mut queue = self.entries.write();
        if queue.len() >= self.max_entries {
            queue.pop_front(); // Tự động dọn dẹp overwrite bản ghi cũ nhất
        }
        queue.push_back(log);
    }

    pub fn get_all(&self) -> Vec<RequestLog> {
        let queue = self.entries.read();
        queue.iter().cloned().collect()
    }

    pub fn clear(&self) {
        let mut queue = self.entries.write();
        queue.clear();
    }

    pub fn count(&self) -> usize {
        self.entries.read().len()
    }
}
