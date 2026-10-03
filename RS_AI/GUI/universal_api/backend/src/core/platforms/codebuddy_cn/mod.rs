//! Module CodeBuddy CN (Tencent Copilot) hoàn toàn độc lập.

pub mod auth;
pub mod account;
pub mod inject;

use super::{PlatformHandler, PlatformLoginStart, PlatformPollOutcome, PlatformAccountInfo};

pub struct CodeBuddyCnHandler;

impl PlatformHandler for CodeBuddyCnHandler {
    fn platform_id(&self) -> &'static str {
        "codebuddy_cn"
    }

    fn display_name(&self) -> &'static str {
        "CodeBuddy (China - 腾讯代码助手)"
    }

    fn start_login(&self, proxy_url: Option<&str>) -> Result<PlatformLoginStart, String> {
        auth::start_login_cn(proxy_url)
    }

    fn poll_login(&self, state: &str, proxy_url: Option<&str>) -> Result<PlatformPollOutcome, String> {
        auth::poll_login_cn(state, proxy_url)
    }

    fn fetch_account(&self, state: &str, access_token: &str, proxy_url: Option<&str>) -> PlatformAccountInfo {
        account::fetch_account_cn(state, access_token, proxy_url)
    }
}
