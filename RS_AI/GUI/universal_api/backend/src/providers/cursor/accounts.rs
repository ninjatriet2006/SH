//! Cursor account data structures and ID formatting.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorAccountData {
    pub id: String,
    pub email: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub plan_type: Option<String>,
    pub created_at: i64,
    pub tags: Vec<String>,
}

/// Generate canonical Cursor storage ID matching Cockpit Tools (`cursor_account.rs:789`).
pub fn build_cursor_storage_id(identity_seed: &str) -> String {
    format!("cursor_{:x}", md5::compute(identity_seed.as_bytes()))
}
