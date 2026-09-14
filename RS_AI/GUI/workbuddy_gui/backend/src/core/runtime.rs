use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;

use crate::core::config::Config;
use crate::core::pool::Pool;

pub struct RuntimeState {
    pub config: Mutex<Config>,
    pub pool: Pool,
}

#[derive(Debug)]
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub requests_failed: AtomicU64,
    pub tokens_total: AtomicU64,
    traces: Mutex<Vec<String>>,
    state_path: Mutex<Option<PathBuf>>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct MetricsSnapshot {
    requests_total: u64,
    requests_failed: u64,
    tokens_total: u64,
    #[serde(default)]
    traces: Vec<String>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            requests_failed: AtomicU64::new(0),
            tokens_total: AtomicU64::new(0),
            traces: Mutex::new(Vec::new()),
            state_path: Mutex::new(None),
        }
    }
}

impl Metrics {
    pub fn configure_path(&self, state_file: &str) {
        let path = if state_file.trim().is_empty() {
            None
        } else {
            Some(PathBuf::from(state_file).with_file_name("metrics.json"))
        };
        if let Ok(mut configured) = self.state_path.lock() { *configured = path.clone(); }
        let Some(path) = path else { return; };
        let Ok(raw) = std::fs::read(&path) else { return; };
        let Ok(snapshot) = serde_json::from_slice::<MetricsSnapshot>(&raw) else { return; };
        self.requests_total.store(snapshot.requests_total, Ordering::Relaxed);
        self.requests_failed.store(snapshot.requests_failed, Ordering::Relaxed);
        self.tokens_total.store(snapshot.tokens_total, Ordering::Relaxed);
        if let Ok(mut traces) = self.traces.lock() {
            *traces = snapshot.traces.into_iter().rev().take(200).collect::<Vec<_>>();
            traces.reverse();
        }
    }

    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.requests_total.load(Ordering::Relaxed),
            self.requests_failed.load(Ordering::Relaxed),
            self.tokens_total.load(Ordering::Relaxed),
        )
    }

    pub fn trace(&self, route: &str, model: &str, status: u16, tokens: u64, elapsed_ms: u64) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        if status >= 400 { self.requests_failed.fetch_add(1, Ordering::Relaxed); }
        self.tokens_total.fetch_add(tokens, Ordering::Relaxed);
        let record = serde_json::json!({
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            "route": route,
            "model": model,
            "status": status,
            "tokens": tokens,
            "elapsed_ms": elapsed_ms,
        }).to_string();
        if let Ok(mut traces) = self.traces.lock() {
            traces.push(record);
            if traces.len() > 200 { let excess = traces.len() - 200; traces.drain(..excess); }
        }
        self.persist();
    }

    pub fn recent_traces(&self) -> Vec<String> {
        self.traces.lock().map(|traces| traces.clone()).unwrap_or_default()
    }

    fn persist(&self) {
        let path = self.state_path.lock().ok().and_then(|path| path.clone());
        let Some(path) = path else { return; };
        let traces = self.recent_traces();
        let snapshot = MetricsSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            requests_failed: self.requests_failed.load(Ordering::Relaxed),
            tokens_total: self.tokens_total.load(Ordering::Relaxed),
            traces,
        };
        let Ok(raw) = serde_json::to_vec_pretty(&snapshot) else { return; };
        if let Some(parent) = path.parent() { let _ = std::fs::create_dir_all(parent); }
        let temp = path.with_extension("json.tmp");
        if std::fs::write(&temp, raw).is_ok() { let _ = std::fs::rename(temp, path); }
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn metrics_round_trip() {
        let root = std::env::temp_dir().join(format!("workbuddy-metrics-{}", std::process::id()));
        let state = root.join("state.json");
        let metrics = Metrics::default();
        metrics.configure_path(state.to_str().unwrap());
        metrics.trace("/test", "model", 200, 7, 12);

        let restored = Metrics::default();
        restored.configure_path(state.to_str().unwrap());
        assert_eq!(restored.snapshot(), (1, 0, 7));
        assert_eq!(restored.recent_traces().len(), 1);

        let _ = std::fs::remove_dir_all(root);
    }
}

impl RuntimeState {
    pub fn accounts_dir(&self) -> Result<String, String> {
        Ok(self.config.lock().map_err(|_| "Config lock poisoned".to_string())?.auth_dir.clone())
    }
}

impl Default for RuntimeState {
    fn default() -> Self {
        let config = Config::default();
        let pool = Pool::new(&config.state_file);
        Self { config: Mutex::new(config), pool }
    }
}
