//! Scheduled account maintenance tasks.

use std::time::Duration;

use chrono::{DateTime, Local, NaiveTime, TimeZone, Timelike};
use log::{info, warn};
use tokio::sync::watch;

use crate::core::config::ScheduleConfig;
use crate::core::pool::Pool;
use crate::core::upstream::client::Client;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind { Checkin, Travel, Activity, Keepalive }

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self { Self::Checkin => "checkin", Self::Travel => "travel", Self::Activity => "activity", Self::Keepalive => "keepalive" })
    }
}

pub struct Scheduler {
    config: ScheduleConfig,
    pool: Pool,
    upstream: Client,
    shutdown_rx: watch::Receiver<bool>,
}

impl Scheduler {
    pub fn new(config: ScheduleConfig, pool: Pool, upstream: Client, shutdown_rx: watch::Receiver<bool>) -> Self {
        Self { config, pool, upstream, shutdown_rx }
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> { tokio::spawn(self.run()) }

    async fn run(mut self) {
        loop {
            let now = Local::now();
            let tasks = self.enabled_tasks();
            let next = tasks.iter().filter_map(|(kind, hours)| next_fire(&now, hours).map(|at| (*kind, at))).min_by_key(|(_, at)| *at);
            let Some((kind, at)) = next else {
                tokio::select! { _ = tokio::time::sleep(Duration::from_secs(3600)) => continue, _ = self.shutdown_rx.changed() => return }
            };
            let delay = (at - now).to_std().unwrap_or(Duration::from_secs(1));
            tokio::select! {
                _ = tokio::time::sleep(delay) => self.fire(kind),
                _ = self.shutdown_rx.changed() => return,
            }
        }
    }

    fn enabled_tasks(&self) -> Vec<(TaskKind, Vec<u8>)> {
        let c = &self.config;
        let mut tasks = Vec::new();
        if c.checkin_enabled { tasks.push((TaskKind::Checkin, hours(&c.checkin_hours))); }
        if c.travel_enabled { tasks.push((TaskKind::Travel, hours(&c.travel_hours))); }
        if c.activity_enabled { tasks.push((TaskKind::Activity, hours(&c.activity_hours))); }
        if c.keepalive_enabled { tasks.push((TaskKind::Keepalive, hours(&c.keepalive_hours))); }
        tasks
    }

    fn fire(&self, kind: TaskKind) {
        info!("scheduler firing {}", kind);
        for account in self.pool.all_accounts() {
            let result = match kind {
                TaskKind::Checkin => self.upstream.daily_checkin(&account.auth).map(|_| ()),
                TaskKind::Travel => self.upstream.travel_status(&account.auth).map(|_| ()),
                TaskKind::Activity => self.upstream.report_chat_activity(&account.auth, &format!("wb2api-{}", chrono::Utc::now().timestamp_millis())),
                TaskKind::Keepalive => self.upstream.user_resource(&account.auth).map(|_| ()),
            };
            if let Err(error) = result { warn!("scheduler {} [{}]: {}", kind, account.uid, error); }
            std::thread::sleep(Duration::from_millis(800));
        }
    }
}

fn hours(values: &[i32]) -> Vec<u8> { values.iter().filter_map(|h| u8::try_from(*h).ok()).filter(|h| *h < 24).collect() }

fn next_fire(now: &DateTime<Local>, hours: &[u8]) -> Option<DateTime<Local>> {
    let mut sorted = hours.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.is_empty() { return None; }
    for hour in &sorted {
        if u32::from(*hour) > now.hour() || (u32::from(*hour) == now.hour() && now.minute() == 0 && now.second() == 0) {
            return local_at(now, 0, *hour);
        }
    }
    local_at(now, 1, sorted[0])
}

fn local_at(now: &DateTime<Local>, days: i64, hour: u8) -> Option<DateTime<Local>> {
    let date = now.date_naive() + chrono::Duration::days(days);
    Local.from_local_datetime(&date.and_time(NaiveTime::from_hms_opt(hour as u32, 0, 0)?)).single()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn next_hour_wraps() {
        let now = Local.with_ymd_and_hms(2026, 1, 15, 19, 0, 0).unwrap();
        let next = next_fire(&now, &[8, 12, 18]).unwrap();
        assert_eq!((next.hour(), next.day()), (8, 16));
    }
}
