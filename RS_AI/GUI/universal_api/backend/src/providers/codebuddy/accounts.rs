//! CodeBuddy & CodeBuddy CN account data structures and helpers.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebuddyAccountData {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub created_at: i64,
    pub tags: Vec<String>,
}
