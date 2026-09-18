use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficAuditLog {
    pub id: String,
    pub timestamp: String,
    pub route: String,
    pub model: String,
    pub status_code: u16,
    pub duration_ms: u64,
    pub account_uid: String,
    pub proxy_used: Option<String>,
    pub raw_request_headers: Vec<(String, String)>,
    pub raw_forwarded_headers: Vec<(String, String)>,
    pub raw_request_body: String,
    pub raw_response_preview: String,
    // ── Raw-wire mở rộng (học vpn_ai_proxy_gui RawTrafficLog) ──
    /// Header upstream trả về (100% raw, lấy trước khi đọc body).
    #[serde(default)]
    pub raw_response_headers: Vec<(String, String)>,
    /// Body THỰC gửi upstream SAU prompt-injection. Rỗng = trùng request.
    /// UI chỉ hiện mục này khi khác rỗng (tránh nhầm với raw_request_body).
    #[serde(default)]
    pub raw_forwarded_body: String,
    /// Request có đòi stream không (client gửi stream:true).
    #[serde(default)]
    pub is_streaming: bool,
}

pub struct AuditBuffer {
    max_entries: usize,
    entries: RwLock<VecDeque<TrafficAuditLog>>,
}

impl AuditBuffer {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: RwLock::new(VecDeque::with_capacity(max_entries)),
        }
    }

    pub fn push(&self, log: TrafficAuditLog) {
        let mut queue = self.entries.write();
        if queue.len() >= self.max_entries {
            queue.pop_front();
        }
        queue.push_back(log);
    }

    pub fn get_all(&self) -> Vec<TrafficAuditLog> {
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

impl Default for AuditBuffer {
    fn default() -> Self {
        Self::new(100)
    }
}
