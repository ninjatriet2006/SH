//! Persistence mirror used by sticky sessions (interface only — the Redis/
//! Upstash implementation was removed; durable state now lives in SQLite
//! via [`crate::core::storage`]).

use std::time::Duration;

pub trait Store: Send + Sync {
    fn set_bind(&self, key: &str, uid: &str, ttl: Duration);
    fn del_bind(&self, key: &str);
    fn load_binds(&self) -> std::collections::HashMap<String, String>;
    fn save_state(&self, data: &[u8]);
    fn load_state(&self) -> Option<Vec<u8>>;
}

#[derive(Debug, Default)]
pub struct NoopStore;

impl Store for NoopStore {
    fn set_bind(&self, _: &str, _: &str, _: Duration) {}
    fn del_bind(&self, _: &str) {}
    fn load_binds(&self) -> std::collections::HashMap<String, String> { Default::default() }
    fn save_state(&self, _: &[u8]) {}
    fn load_state(&self) -> Option<Vec<u8>> { None }
}

impl NoopStore {
    pub fn save_snapshot(&self, _: &[u8]) -> Result<(), String> { Ok(()) }
    pub fn load_snapshot(&self) -> Result<Option<Vec<u8>>, String> { Ok(None) }
}
