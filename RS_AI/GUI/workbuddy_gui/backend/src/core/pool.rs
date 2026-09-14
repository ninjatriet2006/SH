//! Thread-safe account pool with weighted selection, cooldowns and leases.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::core::auth::Auth;

const DEFAULT_BREAKER_THRESHOLD: i32 = 3;
const DEFAULT_BREAKER_COOLDOWN: Duration = Duration::from_secs(30 * 60);
const DEFAULT_BREAKER_MAX: Duration = Duration::from_secs(6 * 60 * 60);
const DEFAULT_SOFT_MAX: Duration = Duration::from_secs(2 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CoolKind {
    #[default]
    HardCredit,
    SoftRate,
}

impl CoolKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::HardCredit => "hard_credit",
            Self::SoftRate => "soft_rate",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountStatus {
    pub uid: String,
    pub email: String,
    pub nickname: String,
    pub domain: String,
    pub healthy: bool,
    pub disabled: bool,
    pub cooling: bool,
    pub cool_kind: Option<String>,
    pub cool_remaining_sec: Option<i64>,
    pub until: Option<i64>,
    pub reason: Option<String>,
    pub disabled_reason: Option<String>,
    pub success_count: i64,
    pub err_total: i64,
    pub in_flight: i64,
    pub credits: i64,
    pub breaker_fails: i32,
    pub breaker_until: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PickedAccount {
    pub uid: String,
    pub email: String,
    pub token: String,
    pub auth: Auth,
}

#[derive(Debug)]
struct Entry {
    auth: Auth,
    credits: i64,
    success_count: i64,
    err_total: i64,
    last_success: i64,
    last_error: i64,
    cool_kind: CoolKind,
    until: i64,
    disabled: bool,
    reason: String,
    last_used: i64,
    breaker_until: i64,
    breaker_fails: i32,
    retry_count: i32,
    soft_streak: i32,
    session_dead_fails: i32,
    in_flight: AtomicI64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PersistedState {
    #[serde(default)]
    accounts: HashMap<String, PersistedAccount>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PersistedAccount {
    #[serde(default)]
    credits: i64,
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    until: i64,
    #[serde(default)]
    cool_kind: CoolKind,
    #[serde(default)]
    success_count: i64,
    #[serde(default)]
    err_total: i64,
    #[serde(default)]
    err_count: i64,
    #[serde(default)]
    last_success: i64,
    #[serde(default)]
    last_err: i64,
    #[serde(default)]
    soft_streak: i32,
}

#[derive(Clone)]
pub struct Pool {
    inner: Arc<RwLock<PoolInner>>,
}

struct PoolInner {
    entries: HashMap<String, Entry>,
    state_path: String,
    max_in_flight: i64,
    breaker_threshold: i32,
    breaker_cooldown: Duration,
    breaker_cooldown_max: Duration,
    soft_rate_max: Duration,
    idle_weight_per_hour: f64,
    idle_weight_max: f64,
}

impl Pool {
    pub fn new(state_path: &str) -> Self {
        let mut inner = PoolInner {
            entries: HashMap::new(),
            state_path: state_path.to_string(),
            max_in_flight: 0,
            breaker_threshold: DEFAULT_BREAKER_THRESHOLD,
            breaker_cooldown: DEFAULT_BREAKER_COOLDOWN,
            breaker_cooldown_max: DEFAULT_BREAKER_MAX,
            soft_rate_max: DEFAULT_SOFT_MAX,
            idle_weight_per_hour: 0.5,
            idle_weight_max: 5.0,
        };
        if let Ok(raw) = state_path.is_empty().then(|| None).unwrap_or_else(|| fs::read(state_path).ok()).ok_or(()) {
            if let Ok(state) = serde_json::from_slice::<PersistedState>(&raw) {
                for (uid, value) in state.accounts {
                    inner.entries.insert(uid.clone(), Entry {
                        auth: placeholder_auth(uid),
                        credits: value.credits,
                        success_count: value.success_count,
                        err_total: value.err_total.max(value.err_count),
                        last_success: value.last_success,
                        last_error: value.last_err,
                        cool_kind: value.cool_kind,
                        until: value.until,
                        disabled: value.disabled,
                        reason: value.reason,
                        last_used: 0,
                        breaker_until: 0,
                        breaker_fails: 0,
                        retry_count: 0,
                        soft_streak: value.soft_streak,
                        session_dead_fails: 0,
                        in_flight: AtomicI64::new(0),
                    });
                }
            }
        }
        Self { inner: Arc::new(RwLock::new(inner)) }
    }

    pub fn configure(&self, max_in_flight: i64, breaker_threshold: i32, breaker_cooldown: Duration,
                     breaker_cooldown_max: Duration, soft_rate_max: Duration,
                     idle_weight_per_hour: f64, idle_weight_max: f64) {
        let mut p = self.inner.write();
        p.max_in_flight = max_in_flight.max(0);
        if breaker_threshold > 0 { p.breaker_threshold = breaker_threshold; }
        if !breaker_cooldown.is_zero() { p.breaker_cooldown = breaker_cooldown; }
        if !breaker_cooldown_max.is_zero() { p.breaker_cooldown_max = breaker_cooldown_max; }
        if !soft_rate_max.is_zero() { p.soft_rate_max = soft_rate_max; }
        if idle_weight_per_hour > 0.0 { p.idle_weight_per_hour = idle_weight_per_hour; }
        if idle_weight_max > 0.0 { p.idle_weight_max = idle_weight_max; }
    }

    pub fn add(&self, auth: Auth) {
        let mut p = self.inner.write();
        let uid = auth.uid.clone();
        if let Some(entry) = p.entries.get_mut(&uid) { entry.auth = auth; } else { p.entries.insert(uid, new_entry(auth)); }
        persist_locked(&p);
    }

    pub fn update_auth(&self, auth: Auth) -> bool {
        let mut p = self.inner.write();
        let Some(entry) = p.entries.get_mut(&auth.uid) else { return false; };
        entry.auth = auth;
        persist_locked(&p);
        true
    }

    pub fn remove(&self, uid: &str) {
        let mut p = self.inner.write();
        p.entries.remove(uid);
        persist_locked(&p);
    }

    pub fn pick(&self, preferred_uid: Option<&str>) -> Option<PickedAccount> {
        let mut p = self.inner.write();
        let now = now_secs();
        let max_in_flight = p.max_in_flight;
        if let Some(uid) = preferred_uid {
            if let Some(entry) = p.entries.get_mut(uid) {
                if healthy(entry, now) && !full(entry, max_in_flight) {
                    entry.last_used = now;
                    return Some(picked(entry));
                }
            }
        }
        let mut candidates: Vec<(String, f64)> = p.entries.iter()
            .filter(|(_, e)| healthy(e, now) && !full(e, max_in_flight))
            .map(|(uid, e)| (uid.clone(), weight(e, now, p.idle_weight_per_hour, p.idle_weight_max)))
            .collect();
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.cmp(&b.0)));
        candidates.truncate(5);
        let Some((uid, _)) = weighted_uid(&candidates) else { return fallback(&mut p, now); };
        let entry = p.entries.get_mut(&uid)?;
        entry.last_used = now;
        Some(picked(entry))
    }

    pub fn acquire(&self, uid: &str) -> bool {
        let p = self.inner.read();
        let Some(entry) = p.entries.get(uid) else { return false; };
        if p.max_in_flight <= 0 { entry.in_flight.fetch_add(1, Ordering::AcqRel); return true; }
        loop {
            let current = entry.in_flight.load(Ordering::Acquire);
            if current >= p.max_in_flight { return false; }
            if entry.in_flight.compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire).is_ok() { return true; }
        }
    }

    pub fn release(&self, uid: &str) {
        let p = self.inner.read();
        if let Some(entry) = p.entries.get(uid) {
            let _ = entry.in_flight.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| (v > 0).then_some(v - 1));
        }
    }

    pub fn note_success(&self, uid: &str) {
        let mut p = self.inner.write();
        if let Some(e) = p.entries.get_mut(uid) { e.success_count += 1; e.last_success = now_secs(); e.breaker_fails = 0; e.retry_count = 0; e.breaker_until = 0; e.soft_streak = 0; e.session_dead_fails = 0; persist_locked(&p); }
    }

    pub fn note_error(&self, uid: &str) {
        let mut p = self.inner.write();
        let threshold = p.breaker_threshold;
        let base = p.breaker_cooldown;
        let max = p.breaker_cooldown_max;
        if let Some(e) = p.entries.get_mut(uid) { e.err_total += 1; e.last_error = now_secs(); breaker_failure(e, threshold, base, max); persist_locked(&p); }
    }

    pub fn cooldown(&self, uid: &str, duration: Duration) {
        self.cooldown_kind(uid, CoolKind::SoftRate, duration, "upstream rate limit");
    }

    pub fn cooldown_kind(&self, uid: &str, kind: CoolKind, duration: Duration, reason: &str) {
        let mut p = self.inner.write();
        let threshold = p.breaker_threshold; let base = p.breaker_cooldown; let max = p.breaker_cooldown_max; let soft_max = p.soft_rate_max;
        if let Some(e) = p.entries.get_mut(uid) {
            let duration = if kind == CoolKind::SoftRate { e.soft_streak += 1; duration.saturating_mul(1u32 << e.soft_streak.saturating_sub(1).min(16)).min(soft_max) } else { duration };
            e.until = now_secs().saturating_add(duration.as_secs() as i64); e.cool_kind = kind; e.reason = reason.to_string(); breaker_failure(e, threshold, base, max); persist_locked(&p);
        }
    }

    pub fn healthy_count(&self) -> usize { let p = self.inner.read(); let now = now_secs(); p.entries.values().filter(|e| healthy(e, now)).count() }
    pub fn total_count(&self) -> usize { self.inner.read().entries.len() }

    pub fn status_json(&self) -> Vec<AccountStatus> {
        let p = self.inner.read(); let now = now_secs();
        let mut out: Vec<_> = p.entries.iter().map(|(uid, e)| status(uid, e, now)).collect(); out.sort_by(|a, b| a.uid.cmp(&b.uid)); out
    }

    pub fn all_accounts(&self) -> Vec<PickedAccount> { self.inner.read().entries.values().map(picked).collect() }

    pub fn reenable_if_credits(&self, uid: &str, remain: i64) {
        let mut p = self.inner.write(); if let Some(e) = p.entries.get_mut(uid) { e.credits = remain; if remain > 0 && !e.disabled { e.until = 0; e.soft_streak = 0; e.reason.clear(); } persist_locked(&p); }
    }
    pub fn note_session_dead(&self, uid: &str) -> bool { let mut p = self.inner.write(); if let Some(e) = p.entries.get_mut(uid) { e.session_dead_fails += 1; if e.session_dead_fails >= 3 { e.disabled = true; e.reason = "12153 session dead".into(); e.session_dead_fails = 0; persist_locked(&p); return true; } } false }
    pub fn clear_session_dead(&self, uid: &str) { if let Some(e) = self.inner.write().entries.get_mut(uid) { e.session_dead_fails = 0; } }
    pub fn disable(&self, uid: &str, reason: &str) { let mut p = self.inner.write(); if let Some(e) = p.entries.get_mut(uid) { e.disabled = true; e.reason = reason.into(); persist_locked(&p); } }
    pub fn enable(&self, uid: &str) { let mut p = self.inner.write(); if let Some(e) = p.entries.get_mut(uid) { e.disabled = false; e.reason.clear(); e.session_dead_fails = 0; persist_locked(&p); } }
    pub fn set_credits(&self, uid: &str, credits: i64) { let mut p = self.inner.write(); if let Some(e) = p.entries.get_mut(uid) { e.credits = credits; persist_locked(&p); } }
}

fn new_entry(auth: Auth) -> Entry { Entry { auth, credits: 0, success_count: 0, err_total: 0, last_success: 0, last_error: 0, cool_kind: CoolKind::HardCredit, until: 0, disabled: false, reason: String::new(), last_used: 0, breaker_until: 0, breaker_fails: 0, retry_count: 0, soft_streak: 0, session_dead_fails: 0, in_flight: AtomicI64::new(0) } }
fn placeholder_auth(uid: String) -> Auth { Auth { access_token: String::new(), refresh_token: String::new(), expires_at: 0, domain: String::new(), uid, enterprise_id: String::new(), nickname: String::new(), file_path: String::new() } }
fn now_secs() -> i64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64 }
fn healthy(e: &Entry, now: i64) -> bool { !e.disabled && (e.until == 0 || e.until <= now) && (e.breaker_until == 0 || e.breaker_until <= now) }
fn full(e: &Entry, limit: i64) -> bool { limit > 0 && e.in_flight.load(Ordering::Acquire) >= limit }
fn picked(e: &Entry) -> PickedAccount { PickedAccount { uid: e.auth.uid.clone(), email: e.auth.nickname.clone(), token: e.auth.access_token.clone(), auth: e.auth.clone() } }
fn weight(e: &Entry, now: i64, idle_per_hour: f64, idle_max: f64) -> f64 { let credit = e.credits.max(0) as f64; let idle = if e.last_used == 0 { idle_max } else { ((now - e.last_used) as f64 / 3600.0 * idle_per_hour).clamp(0.0, idle_max) }; let success = if e.success_count + e.err_total == 0 { 1.5 } else { e.success_count as f64 / (e.success_count + e.err_total) as f64 * 3.0 }; 1.0 + credit + idle + success }
fn weighted_uid(items: &[(String, f64)]) -> Option<(String, f64)> { let total: f64 = items.iter().map(|(_, w)| *w).sum(); if total <= 0.0 { return items.first().cloned(); } let target = rand::thread_rng().gen_range(0.0..total); let mut sum = 0.0; for item in items { sum += item.1; if target < sum { return Some(item.clone()); } } items.last().cloned() }
fn fallback(p: &mut PoolInner, now: i64) -> Option<PickedAccount> { let uid = p.entries.iter().filter(|(_, e)| !e.disabled && e.cool_kind != CoolKind::HardCredit && !full(e, p.max_in_flight) && e.until.max(e.breaker_until) > now).min_by_key(|(_, e)| e.until.max(e.breaker_until)).map(|(uid, _)| uid.clone())?; let e = p.entries.get_mut(&uid)?; e.last_used = now; Some(picked(e)) }
fn breaker_failure(e: &mut Entry, threshold: i32, base: Duration, max: Duration) { e.breaker_fails += 1; if e.breaker_fails >= threshold { e.breaker_fails = 0; e.retry_count += 1; let d = base.checked_mul(1u32 << e.retry_count.saturating_sub(1).min(8)).unwrap_or(max).min(max); e.breaker_until = now_secs().saturating_add(d.as_secs() as i64); } }
fn status(uid: &str, e: &Entry, now: i64) -> AccountStatus { let until = e.until.max(e.breaker_until); AccountStatus { uid: uid.into(), email: e.auth.nickname.clone(), nickname: e.auth.nickname.clone(), domain: e.auth.domain.clone(), healthy: healthy(e, now), disabled: e.disabled, cooling: until > now, cool_kind: (until > now).then(|| if e.breaker_until > e.until { "breaker".into() } else { e.cool_kind.as_str().into() }), cool_remaining_sec: (until > now).then_some(until - now), until: (until > now).then_some(until), reason: (!e.reason.is_empty()).then(|| e.reason.clone()), disabled_reason: e.disabled.then(|| e.reason.clone()), success_count: e.success_count, err_total: e.err_total, in_flight: e.in_flight.load(Ordering::Acquire), credits: e.credits, breaker_fails: e.breaker_fails, breaker_until: (e.breaker_until > now).then_some(e.breaker_until) } }
fn persist_locked(p: &PoolInner) { if p.state_path.is_empty() { return; } let state = PersistedState { accounts: p.entries.iter().map(|(uid, e)| (uid.clone(), PersistedAccount { credits: e.credits, disabled: e.disabled, reason: e.reason.clone(), until: e.until, cool_kind: e.cool_kind, success_count: e.success_count, err_total: e.err_total, err_count: 0, last_success: e.last_success, last_err: e.last_error, soft_streak: e.soft_streak })).collect() }; if let Ok(raw) = serde_json::to_vec_pretty(&state) { let path = Path::new(&p.state_path); if let Some(parent) = path.parent() { let _ = fs::create_dir_all(parent); } let tmp = path.with_extension("tmp"); if fs::write(&tmp, raw).is_ok() { let _ = fs::rename(tmp, path); } } }
