//! HTTP header builders for upstream requests — Common / Chat / Billing / Refresh.
//!
//! Ported from Go: internal/upstream/headers.go

use crate::core::auth::Auth;
use super::realms;

/// Phiên bản CLI giả mạo — đối chiếu TS tham chiếu (orangeboyChen/codebuddy2api
/// `CODEBUDDY_CLI_VERSION`). Bản Go/Rust cũ kẹt ở 2.63.2; upstream siết version
/// client nên UA cũ là nghi phạm hàng đầu khi chat tạch hàng loạt.
pub const CODEBUDDY_CLI_VERSION: &str = "2.137.1";

fn default_ua() -> String {
    format!("CLI/{CODEBUDDY_CLI_VERSION} CodeBuddy/{CODEBUDDY_CLI_VERSION}")
}

/// UUID v4 (không thêm dep): `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`.
pub fn new_uuid_v4() -> String {
    let mut b = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}",
        &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

/// ID 32 hex (request-id / trace) — đối chiếu TS `randomUUID().replaceAll('-','')`.
pub fn new_hex_id() -> String {
    let mut b = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

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
    let ua = if user_agent.is_empty() { default_ua() } else { user_agent.to_string() };
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

/// Chat headers = common + auth/uid/enterprise/domain + IDE/conversation set.
/// Đối chiếu TS `buildUpstreamHeaders`: thiếu các header X-IDE-*/X-Conversation-*,
/// upstream (WAF/fingerprint) có thể từ chối request từ "client lạ".
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
        h.set("X-Tenant-Id", &auth.enterprise_id);
    } else {
        h.set("X-No-Enterprise-Id", "1");
    }
    if !auth.domain.is_empty() {
        h.set("X-Domain", &auth.domain);
    } else {
        h.set("X-No-Department-Info", "1");
    }
    h.set("X-Product", "SaaS");
    h.set("X-Product-Version", CODEBUDDY_CLI_VERSION);
    // Danh tính IDE — TS gửi cố định CLI.
    h.set("X-IDE-Name", "CLI");
    h.set("X-IDE-Type", "CLI");
    h.set("X-IDE-Version", CODEBUDDY_CLI_VERSION);
    h.set("X-Client-Platform", "web");
    h.set("X-Agent-Intent", "craft");
    // ID hội thoại/request mỗi request một bộ mới (TS randomUUID mỗi lần).
    h.set("X-Conversation-ID", new_uuid_v4());
    h.set("X-Conversation-Message-ID", new_hex_id());
    h.set("X-Conversation-Request-ID", new_hex_id());
    h.set("X-Request-ID", new_hex_id());
    h
}

/// Billing headers (balance, checkin, report) — đối chiếu TS `fetchJson`
/// (account-status.ts): đủ Origin/Referer + IDE set, thiếu là APISIX 401
/// ngay ở cổng (đã gặp thật: chat OK nhưng billing 401).
pub fn billing_headers(auth: &Auth, user_agent: &str) -> HeaderSet {
    let ua = if user_agent.is_empty() { default_ua() } else { user_agent.to_string() };
    let origin = realms::origin_referer_for(auth);
    let mut h = HeaderSet::new();
    h.set("Authorization", format!("Bearer {}", auth.access_token));
    h.set("Accept", "application/json, text/plain, */*");
    h.set("Content-Type", "application/json");
    h.set("Origin", origin);
    h.set("Referer", format!("{}/", origin));
    h.set("User-Agent", ua);
    h.set("X-Requested-With", "XMLHttpRequest");
    h.set("X-IDE-Name", "CLI");
    h.set("X-IDE-Type", "CLI");
    h.set("X-IDE-Version", CODEBUDDY_CLI_VERSION);
    h.set("X-Product", "SaaS");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_auth() -> Auth {
        Auth {
            access_token: "tok".into(),
            refresh_token: String::new(),
            expires_at: 0,
            domain: "codebuddy.ai".into(),
            uid: "u1".into(),
            enterprise_id: "e1".into(),
            nickname: String::new(),
            file_path: String::new(),
        }
    }

    fn get(h: &HeaderSet, name: &str) -> Option<String> {
        h.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.clone())
    }

    #[test]
    fn chat_headers_match_ts_reference() {
        // Đối chiếu orangeboyChen/codebuddy2api buildUpstreamHeaders.
        let h = chat_headers(&dummy_auth(), "");
        assert!(get(&h, "User-Agent").unwrap().contains(CODEBUDDY_CLI_VERSION));
        assert_ne!(CODEBUDDY_CLI_VERSION, "2.63.2");
        for required in [
            "X-IDE-Name", "X-IDE-Type", "X-IDE-Version", "X-Client-Platform",
            "X-Agent-Intent", "X-Product-Version", "X-Conversation-ID",
            "X-Conversation-Message-ID", "X-Conversation-Request-ID",
            "X-Request-ID", "X-Tenant-Id", "X-User-Id", "X-Domain",
        ] {
            assert!(get(&h, required).is_some(), "missing {required}");
        }
        assert_eq!(get(&h, "X-IDE-Name").as_deref(), Some("CLI"));
        assert_eq!(get(&h, "X-Tenant-Id").as_deref(), Some("e1"));
        // IDs duy nhất mỗi lần build.
        let a = chat_headers(&dummy_auth(), "");
        assert_ne!(get(&h, "X-Request-ID"), get(&a, "X-Request-ID"));
    }

    #[test]
    fn uuid_shapes() {
        let u = new_uuid_v4();
        assert_eq!(u.len(), 36);
        assert_eq!(u.chars().filter(|c| *c == '-').count(), 4);
        assert_eq!(u.chars().nth(14), Some('4'));
        assert_eq!(new_hex_id().len(), 32);
    }

    #[test]
    fn billing_headers_match_ts_fetchjson() {
        // billing từng thiếu Origin/Referer/IDE → APISIX 401 trong khi chat OK.
        let h = billing_headers(&dummy_auth(), "");
        for required in [
            "Origin", "Referer", "X-Requested-With", "X-IDE-Name",
            "X-IDE-Type", "X-IDE-Version", "X-Product", "X-User-Id",
            "X-Enterprise-Id", "X-Tenant-Id", "X-Domain",
        ] {
            assert!(get(&h, required).is_some(), "missing {required}");
        }
        assert!(get(&h, "User-Agent").unwrap().contains(CODEBUDDY_CLI_VERSION));
    }
}
