//! Module Zed Platform hoàn toàn độc lập.

pub mod auth;
pub mod account;
pub mod inject;

use super::{PlatformHandler, PlatformLoginStart, PlatformPollOutcome, PlatformAccountInfo};

pub struct ZedPlatformHandler;

impl PlatformHandler for ZedPlatformHandler {
    fn platform_id(&self) -> &'static str {
        "zed"
    }

    fn display_name(&self) -> &'static str {
        "Zed Editor (cloud.zed.dev)"
    }

    fn start_login(&self, _proxy_url: Option<&str>) -> Result<PlatformLoginStart, String> {
        Err("Zed supports credential file import or manual token. OAuth browser login not implemented yet.".to_string())
    }

    fn poll_login(&self, _state: &str, _proxy_url: Option<&str>) -> Result<PlatformPollOutcome, String> {
        Err("Zed polling not supported".to_string())
    }

    fn fetch_account(&self, _state: &str, access_token: &str, _proxy_url: Option<&str>) -> PlatformAccountInfo {
        if let Ok(cred) = auth::parse_credential_json(access_token) {
            if let Ok(profile) = account::fetch_user(&cred.id, &cred.access_token) {
                return account::to_platform_account(&profile);
            }
        }
        PlatformAccountInfo {
            uid: "unknown_zed".to_string(),
            nickname: "Zed Account".to_string(),
            domain: "zed.dev".to_string(),
            ..Default::default()
        }
    }
}
