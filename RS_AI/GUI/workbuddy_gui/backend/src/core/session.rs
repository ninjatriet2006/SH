//! Session sticky routing — ported from Go internal/session.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use chrono::Utc;
use log::info;
use serde_json::Value;

use crate::core::redisstore::Store;

// ─── Config & Router ─────────────────────────────────────────────────────────

/// Config for session router.
pub struct Config {
    pub ttl: Duration,
    pub gc_interval: Duration,
    pub store: Arc<dyn Store + Send + Sync>,
    pub available: Arc<dyn Fn() -> Vec<String> + Send + Sync>,
}

/// Sticky session router.
pub struct SessionRouter {
    inner: Arc<RwLock<Inner>>,
}

struct Inner {
    entries: HashMap<String, Entry>,
    cfg: Config,
    #[allow(dead_code)]
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Debug, Clone)]
struct Entry {
    uid: String,
    last_active: chrono::DateTime<Utc>,
}

impl SessionRouter {
    pub fn new(cfg: Config) -> Self {
        let inner = Inner {
            entries: HashMap::new(),
            cfg,
            stop_tx: None,
        };
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    /// Resolve a session key to a uid.
    pub fn resolve(&self, key: &str) -> Option<String> {
        let now = Utc::now();
        let available = self.available_set();

        // Fast path: RLock read
        {
            let guard = self.inner.read().ok()?;
            if let Some(e) = guard.entries.get(key) {
                let uid = e.uid.clone();
                if !expired(e, now, guard.cfg.ttl) && available.contains_key(&uid) {
                    drop(guard);
                    self.touch(key, &uid, now);
                    return Some(uid);
                }
            }
        }

        // Slow path: WLock recheck and allocate
        let mut guard = self.inner.write().ok()?;
        // Re-check
        if let Some(e2) = guard.entries.get(key) {
            let old_uid = e2.uid.clone();
            if !expired(e2, now, guard.cfg.ttl) && available.contains_key(&old_uid) {
                guard.entries.insert(key.to_string(), Entry {
                    uid: old_uid.clone(),
                    last_active: now,
                });
                return Some(old_uid);
            }
            guard.entries.remove(key);
        }

        let uids = (guard.cfg.available)();
        if uids.is_empty() {
            return None;
        }

        // Two-stage strategy: prefer idle accounts first
        let mut bound: HashMap<String, bool> = HashMap::new();
        for v in guard.entries.values() {
            bound.insert(v.uid.clone(), true);
        }
        let idle: Vec<String> = uids.iter()
            .filter(|u| !bound.contains_key(*u))
            .cloned()
            .collect();
        let pool2 = if idle.is_empty() { uids } else { idle };
        let uid = pool2[hash_index(key, pool2.len())].clone();

        guard.entries.insert(key.to_string(), Entry {
            uid: uid.clone(),
            last_active: now,
        });
        guard.cfg.store.set_bind(key, &uid, guard.cfg.ttl);

        Some(uid)
    }

    pub fn get(&self, key: &str) -> Option<String> { self.resolve(key) }
    pub fn set(&self, key: &str, uid: &str) { self.bind(key, uid); }

    /// Touch (roll) the session key's TTL.
    fn touch(&self, key: &str, uid: &str, now: chrono::DateTime<Utc>) {
        if let Ok(mut guard) = self.inner.write() {
            guard.entries.insert(key.to_string(), Entry {
                uid: uid.to_string(),
                last_active: now,
            });
            guard.cfg.store.set_bind(key, uid, guard.cfg.ttl);
        }
    }

    /// Bind explicitly a key to a uid.
    pub fn bind(&self, key: &str, uid: &str) {
        if key.is_empty() || uid.is_empty() {
            return;
        }
        let now = Utc::now();
        if let Ok(mut guard) = self.inner.write() {
            guard.entries.insert(key.to_string(), Entry {
                uid: uid.to_string(),
                last_active: now,
            });
            guard.cfg.store.set_bind(key, uid, guard.cfg.ttl);
        }
    }

    /// Unbind a session key.
    pub fn unbind(&self, key: &str) {
        if let Ok(mut guard) = self.inner.write() {
            guard.entries.remove(key);
            guard.cfg.store.del_bind(key);
        }
    }

    /// Count current binds.
    pub fn count(&self) -> usize {
        self.inner.read().map(|g| g.entries.len()).unwrap_or(0)
    }

    fn available_set(&self) -> HashMap<String, bool> {
        let uids = self.available_slice();
        uids.into_iter().map(|u| (u, true)).collect()
    }

    fn available_slice(&self) -> Vec<String> {
        self.inner.read().map(|g| (g.cfg.available)()).unwrap_or_default()
    }

    /// Load binds from store at startup.
    pub fn load_from_store(&self) {
        let binds = self.inner.read().map(|g| g.cfg.store.load_binds()).unwrap_or_default();
        if binds.is_empty() {
            return;
        }
        let now = Utc::now();
        if let Ok(mut guard) = self.inner.write() {
            let mut loaded = 0;
            for (key, uid) in binds {
                if guard.entries.contains_key(&key) {
                    continue;
                }
                guard.entries.insert(key, Entry { uid, last_active: now });
                loaded += 1;
            }
            if loaded > 0 {
                info!("[session] restored {} binds", loaded);
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn expired(e: &Entry, now: chrono::DateTime<Utc>, ttl: Duration) -> bool {
    now.signed_duration_since(e.last_active) > chrono::Duration::from_std(ttl).unwrap()
}

fn hash_index(key: &str, n: usize) -> usize {
    let mut h: u32 = 2166136261;
    for b in key.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(16777619);
    }
    (h % (n as u32)) as usize
}

/// Extract session key from JSON bytes.
pub fn extract_key(body: &[u8]) -> String {
    if body.is_empty() {
        return String::new();
    }
    let obj: Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    if let Some(meta) = obj.get("metadata").and_then(|m| m.as_object()) {
        if let Some(v) = meta.get("conversation_id").and_then(|s| s.as_str()) {
            return v.to_string();
        }
        if let Some(v) = meta.get("conversationId").and_then(|s| s.as_str()) {
            return v.to_string();
        }
        if let Some(v) = meta.get("user_id").and_then(|s| s.as_str()) {
            return v.to_string();
        }
    }

    if let Some(v) = obj.get("conversation_id").and_then(|s| s.as_str()) {
        return v.to_string();
    }
    obj.get("conversationId").and_then(|s| s.as_str()).unwrap_or("").to_string()
}
