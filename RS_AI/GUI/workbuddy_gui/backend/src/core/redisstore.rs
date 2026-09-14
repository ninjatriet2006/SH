//! Persistence mirror used by sticky sessions and pool state.

use std::time::Duration;

pub trait Store: Send + Sync {
    fn set_bind(&self, key: &str, uid: &str, ttl: Duration);
    fn del_bind(&self, key: &str);
    fn load_binds(&self) -> std::collections::HashMap<String, String>;
    fn save_state(&self, data: &[u8]);
    fn load_state(&self) -> Option<Vec<u8>>;
}

#[derive(Debug, Clone)]
pub struct StoreConfig { pub url: String, pub token: String }

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

pub struct RedisStore { config: StoreConfig }

impl RedisStore {
    pub fn new(config: StoreConfig) -> Option<Self> {
        (!config.url.trim().is_empty()).then_some(Self { config })
    }
}

impl Store for RedisStore {
    fn set_bind(&self, key: &str, uid: &str, ttl: Duration) {
        let _ = ureq::post(&self.config.url)
            .set("Authorization", &format!("Bearer {}", self.config.token))
            .send_json(serde_json::json!({"commands":[["SET", format!("wb2api:bind:{}", key), uid, "EX", ttl.as_secs().max(1)]]}));
    }
    fn del_bind(&self, key: &str) {
        let _ = ureq::post(&self.config.url)
            .set("Authorization", &format!("Bearer {}", self.config.token))
            .send_json(serde_json::json!({"commands":[["DEL", format!("wb2api:bind:{}", key)]]}));
    }
    fn load_binds(&self) -> std::collections::HashMap<String, String> { Default::default() }
    fn save_state(&self, data: &[u8]) {
        let value = String::from_utf8_lossy(data);
        let _ = ureq::post(&self.config.url)
            .set("Authorization", &format!("Bearer {}", self.config.token))
            .send_json(serde_json::json!({"commands":[["SET", "wb2api:state", value]]}));
    }
    fn load_state(&self) -> Option<Vec<u8>> { None }
}
