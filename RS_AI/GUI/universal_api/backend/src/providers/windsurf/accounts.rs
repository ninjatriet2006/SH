//! Windsurf account data structures.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindsurfAccountData {
    pub id: String,
    pub email: String,
    pub api_key: String,
    pub plan: Option<String>,
    pub created_at: i64,
    pub tags: Vec<String>,
}
