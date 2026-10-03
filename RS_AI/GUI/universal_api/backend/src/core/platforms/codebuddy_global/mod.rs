//! Module CodeBuddy Global (Quốc tế) hoàn toàn độc lập.

pub mod auth;
pub mod account;
pub mod inject;

use super::{PlatformHandler, PlatformLoginStart, PlatformPollOutcome, PlatformAccountInfo};

pub struct CodeBuddyGlobalHandler;

impl PlatformHandler for CodeBuddyGlobalHandler {
    fn platform_id(&self) -> &'static str {
        "codebuddy_global"
    }

    fn display_name(&self) -> &'static str {
        "CodeBuddy (Global - codebuddy.ai)"
    }

    fn start_login(&self, proxy_url: Option<&str>) -> Result<PlatformLoginStart, String> {
        auth::start_login_global(proxy_url)
    }

    fn poll_login(&self, state: &str, proxy_url: Option<&str>) -> Result<PlatformPollOutcome, String> {
        auth::poll_login_global(state, proxy_url)
    }

    fn fetch_account(&self, state: &str, access_token: &str, proxy_url: Option<&str>) -> PlatformAccountInfo {
        account::fetch_account_global(state, access_token, proxy_url)
    }
}
