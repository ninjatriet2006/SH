use serde::Serialize;
use std::sync::Mutex;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use crate::core::runtime::Metrics;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Serialize)]
pub struct GatewayInfo {
    pub status: GatewayStatus,
    pub listen: Option<String>,
    pub total_accounts: usize,
    pub healthy_accounts: usize,
    pub error: Option<String>,
    pub requests_total: u64,
    pub requests_failed: u64,
    pub tokens_total: u64,
}

impl Default for GatewayInfo {
    fn default() -> Self {
        Self {
            status: GatewayStatus::Stopped,
            listen: None,
            total_accounts: 0,
            healthy_accounts: 0,
            error: None,
            requests_total: 0,
            requests_failed: 0,
            tokens_total: 0,
        }
    }
}

#[derive(Debug)]
pub struct GatewayState {
    pub info: Mutex<GatewayInfo>,
    pub shutdown_tx: watch::Sender<bool>,
    pub server_task: Mutex<Option<JoinHandle<()>>>,
    pub metrics: std::sync::Arc<Metrics>,
}

impl Default for GatewayState {
    fn default() -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self { info: Mutex::new(GatewayInfo::default()), shutdown_tx, server_task: Mutex::new(None), metrics: std::sync::Arc::new(Metrics::default()) }
    }
}
