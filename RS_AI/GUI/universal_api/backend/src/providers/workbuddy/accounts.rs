//! Account data structures.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAccountData {
    pub id: String,
    pub email: String,
    pub access_token: String,
    pub created_at: i64,
    pub tags: Vec<String>,
}
