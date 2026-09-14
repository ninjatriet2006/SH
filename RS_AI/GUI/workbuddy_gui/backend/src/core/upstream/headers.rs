//! HTTP header builders for upstream requests — Common / Chat / Billing / Refresh.
//!
//! Ported from Go: internal/upstream/headers.go

use crate::core::auth::Auth;
use super::realms;

const CLIENT_UA: &str = "CLI/2.63.2 CodeBuddy/2.63.2";

/// A collected set of (name, value) pairs to apply to a request builder.
pub struct HeaderSet {
    pairs: Vec<(String, String)>,
}

impl HeaderSet {
    fn new() -> Self {
        Self { pairs: Vec::with_capacity(12) }
    }

    fn set(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.pairs.push((k.into(), v.into()));
    }

    /// Mutable access to the pairs (for adding extra headers after construction).
    pub fn pairs_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.pairs
    }

    /// Iterator over all header pairs.
    pub fn iter(&self) -> impl Iterator<Item = &(String, String)> {
        self.pairs.iter()
    }
}

/// Common headers for all API requests.
pub fn common_headers(auth: &Auth, user_agent: &str) -> HeaderSet {
    let ua = if user_agent.is_empty() { CLIENT_UA } else { user_agent };
    let origin = realms::origin_referer_for(auth);
    let mut h = HeaderSet::new();
    h.set("Content-Type", "application/json");
    h.set("Accept", "application/json, text/plain, */*");
    h.set("X-Requested-With", "XMLHttpRequest");
    h.set("Origin", origin);
    h.set("Referer", format!("{}/", origin));
    h.set("User-Agent", ua);
    h
}

/// Chat headers = common + auth/uid/enterprise/domain.
pub fn chat_headers(auth: &Auth, user_agent: &str) -> HeaderSet {
    let mut h = common_headers(auth, user_agent);
    if !auth.access_token.is_empty() {
        h.set("Authorization", format!("Bearer {}", auth.access_token));
    } else {
        h.set("X-No-Authorization", "1");
    }
    if !auth.uid.is_empty() {
        h.set("X-User-Id", &auth.uid);
    } else {
        h.set("X-No-User-Id", "1");
    }
    if !auth.enterprise_id.is_empty() {
        h.set("X-Enterprise-Id", &auth.enterprise_id);
    } else {
        h.set("X-No-Enterprise-Id", "1");
    }
    if !auth.domain.is_empty() {
        h.set("X-Domain", &auth.domain);
    } else {
        h.set("X-No-Department-Info", "1");
    }
    h.set("X-Product", "SaaS");
    h
}

/// Billing headers (balance, checkin, report).
pub fn billing_headers(auth: &Auth, user_agent: &str) -> HeaderSet {
    let mut h = HeaderSet::new();
    h.set("Authorization", format!("Bearer {}", auth.access_token));
    h.set("Accept", "application/json");
    h.set("Content-Type", "application/json");
    if !user_agent.is_empty() {
        h.set("User-Agent", user_agent);
    }
    if !auth.uid.is_empty() {
        h.set("X-User-Id", &auth.uid);
    }
    if !auth.enterprise_id.is_empty() {
        h.set("X-Enterprise-Id", &auth.enterprise_id);
        h.set("X-Tenant-Id", &auth.enterprise_id);
    }
    if !auth.domain.is_empty() {
        h.set("X-Domain", &auth.domain);
    }
    h
}

/// Refresh-token headers.
pub fn refresh_headers(auth: &Auth, user_agent: &str) -> HeaderSet {
    let mut h = common_headers(auth, user_agent);
    h.set("X-Refresh-Token", &auth.refresh_token);
    if !auth.enterprise_id.is_empty() {
        h.set("X-Enterprise-Id", &auth.enterprise_id);
    }
    h.set("X-Auth-Refresh-Source", "workbuddy");
    h
}
