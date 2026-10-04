//! GitHub Copilot account data structures and ID formatting.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCopilotAccountData {
    pub id: String,
    pub github_login: String,
    pub github_email: Option<String>,
    pub github_access_token: String,
    pub copilot_token: Option<String>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
    pub tags: Vec<String>,
}

/// Generate canonical GitHub Copilot storage ID matching Cockpit Tools (`github_copilot_account.rs:278`).
pub fn build_github_copilot_storage_id(login: &str, github_user_id: &str) -> String {
    format!("ghcp_{:x}", md5::compute(format!("{}:{}", login, github_user_id)))
}
