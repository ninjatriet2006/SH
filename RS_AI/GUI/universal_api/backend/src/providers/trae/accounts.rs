//! Trae, Trae Solo & Trae CN account data structures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraeAccountData {
    pub id: String,
    pub email: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub plan_type: Option<String>,
    pub created_at: i64,
    pub tags: Vec<String>,
}
