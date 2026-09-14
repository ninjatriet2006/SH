//! Realm routing — CN (default) / Intl / WorkBuddy.ai per credential.
//!
//! Ported from Go: internal/upstream/realms.go

use crate::core::auth::Auth;

pub const REALM_CN: &str = "cn";
pub const REALM_INTL: &str = "intl";

// Default base URLs per realm.
const CHAT_BASE_CN: &str = "https://copilot.tencent.com";
const BILLING_BASE_CN: &str = "https://www.codebuddy.cn";
const CHAT_BASE_INTL: &str = "https://www.codebuddy.ai";
const BILLING_BASE_INTL: &str = "https://www.codebuddy.ai";
const WORKBUDDY_BASE: &str = "https://www.workbuddy.ai";
pub const ORIGIN_WORKBUDDY: &str = "https://www.workbuddy.ai";
pub const ORIGIN_REFERER_CN: &str = "https://www.codebuddy.cn";

/// Normalise a realm string from config/env: "" → cn.
pub fn normalize_realm(s: &str) -> (&'static str, bool) {
    match s.to_lowercase().trim() {
        "" | "cn" | "zh" | "china" => (REALM_CN, true),
        "intl" | "international" | "global" => (REALM_INTL, true),
        _ => (REALM_CN, false), // unknown → fallback cn, not recognised
    }
}

fn has_domain_suffix(domain: &str, suffix: &str) -> bool {
    let d = domain.to_lowercase();
    let d = d.trim();
    d == suffix || d.ends_with(&format!(".{}", suffix))
}

pub fn is_workbuddy_credential(auth: &Auth) -> bool {
    has_domain_suffix(&auth.domain, "workbuddy.ai")
}

/// Origin/Referer to send for a credential.
pub fn origin_referer_for(auth: &Auth) -> &'static str {
    if is_workbuddy_credential(auth) {
        ORIGIN_WORKBUDDY
    } else {
        ORIGIN_REFERER_CN
    }
}

/// Resolve the chat base URL for a credential + realm.
pub fn route_chat_base(auth: &Auth, default_realm: &str) -> &'static str {
    route_base(auth, default_realm, CHAT_BASE_CN, CHAT_BASE_INTL)
}

/// Resolve the billing base URL for a credential + realm.
pub fn route_billing_base(auth: &Auth, default_realm: &str) -> &'static str {
    route_base(auth, default_realm, BILLING_BASE_CN, BILLING_BASE_INTL)
}

fn route_base(auth: &Auth, default_realm: &str, cn: &'static str, intl: &'static str) -> &'static str {
    let d = &auth.domain;
    if is_workbuddy_credential(auth) {
        return WORKBUDDY_BASE;
    }
    if has_domain_suffix(d, "codebuddy.ai") {
        return intl;
    }
    if has_domain_suffix(d, "codebuddy.cn") || has_domain_suffix(d, "tencent.com") {
        return cn;
    }
    if default_realm == REALM_INTL {
        intl
    } else {
        cn
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::auth::Auth;

    fn dummy_auth(domain: &str) -> Auth {
        Auth {
            access_token: "tok".into(), refresh_token: String::new(),
            expires_at: 0, domain: domain.into(), uid: String::new(),
            enterprise_id: String::new(), nickname: String::new(),
            file_path: String::new(),
        }
    }

    #[test]
    fn test_route_cn() {
        let a = dummy_auth("xxx.codebuddy.cn");
        assert_eq!(route_chat_base(&a, REALM_CN), CHAT_BASE_CN);
    }

    #[test]
    fn test_route_workbuddy() {
        let a = dummy_auth("acme.workbuddy.ai");
        assert_eq!(route_chat_base(&a, REALM_CN), WORKBUDDY_BASE);
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize_realm("intl"), (REALM_INTL, true));
        assert_eq!(normalize_realm(""), (REALM_CN, true));
    }
}
