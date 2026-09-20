//! BLOCK anti-api — mọi thứ liên quan provider ngoài nằm trong file này.
//!
//! Ranh giới với khối Core (`core/server.rs`):
//! - `server.rs` chỉ gọi 3 điểm biên: `is_external_model` (chọn đường đi),
//!   `forward_external_stream` (forward chat), `merge_external_models`
//!   (gộp `/v1/models`). Không có logic anti-api nào nằm ngoài file này.
//!
//! Triển khai native trong Rust (lấy ý tưởng Flow/Account routing +
//! multi-provider từ anti-api silasxbt/anti-api, MIT):
//! - Giữ của khối Core: Rust native, pool weighted, session-sticky, SQLite vault,
//!   per-account proxy/UA, scheduler. Không thêm Bun/Node runtime.
//! - Lấy của anti-api: khái niệm External provider (Antigravity/Codex/Copilot/Zed/
//!   Kiro/Grok), per-account model discovery, tách control/public listener,
//!   Flow (`route:*`) + Account (model-native) routing.
//! - Kết quả: Universal API `:7863` làm unified gateway. Model nội bộ (`glm/kimi/
//!   minimax/hy3/deepseek`) đi pool nội bộ; model external đi HTTP forward tới
//!   anti-api `base_url` do user tự chạy (docker/binary) ở port tùy chỉnh.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use axum::response::{IntoResponse, Response, Sse, sse::Event};
use log::info;
use serde_json::{json, Value};
use tokio_stream::wrappers::ReceiverStream;

use crate::core::protocol::{
    anthropic_content_block_delta, anthropic_content_block_start, anthropic_content_block_stop,
    anthropic_message_delta, anthropic_message_start, anthropic_message_stop,
};
use crate::core::server::SharedState;

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalConfig {
    /// Bật/tắt toàn bộ external routing.
    #[serde(default)]
    pub enabled: bool,
    /// Base URL của anti-api service, ví dụ `http://127.0.0.1:8964`.
    #[serde(default = "default_base_url")]
    pub base_url: String,
    /// API key gửi kèm khi forward (anti-api chấp nhận placeholder local).
    #[serde(default)]
    pub api_key: String,
    /// Timeout cho health/models (giây).
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Timeout cho chat forward (giây, stream dài).
    #[serde(default = "default_chat_timeout")]
    pub chat_timeout_seconds: u64,
    /// Port control (dashboard) của anti-api — dùng để gợi ý base_url, check trùng.
    #[serde(default = "default_control_port")]
    pub control_port: u16,
    /// Port public inference gateway của anti-api (mặc định 8966).
    #[serde(default = "default_public_port")]
    pub public_port: u16,
    /// Nếu external lỗi thì fallback về pool nội bộ (khi model cũng tồn tại nội bộ).
    #[serde(default = "default_true")]
    pub auto_fallback: bool,
}

fn default_base_url() -> String {
    "http://127.0.0.1:8964".to_string()
}
fn default_timeout() -> u64 {
    10
}
fn default_chat_timeout() -> u64 {
    600
}
fn default_control_port() -> u16 {
    8964
}
fn default_public_port() -> u16 {
    8966
}
fn default_true() -> bool {
    true
}

impl Default for ExternalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_base_url(),
            api_key: String::new(),
            timeout_seconds: default_timeout(),
            chat_timeout_seconds: default_chat_timeout(),
            control_port: default_control_port(),
            public_port: default_public_port(),
            auto_fallback: true,
        }
    }
}

impl ExternalConfig {
    /// Chuẩn hoá base_url: trim, bỏ `/` cuối. Trả về lỗi nếu scheme/host thiếu.
    pub fn normalized_base_url(&self) -> Result<String, String> {
        normalize_base_url(&self.base_url)
    }

    /// Validate toàn bộ config. `main_listen` là `config.listen` (vd `:7863`)
    /// để phát hiện trùng port — lỗi ngầm hay gặp khi user đổi port bừa.
    pub fn validate(&self, main_listen: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        let base = self.normalized_base_url()?;
        if self.timeout_seconds == 0 || self.timeout_seconds > 120 {
            return Err("external.timeout_seconds must be 1..120".into());
        }
        if self.chat_timeout_seconds == 0 || self.chat_timeout_seconds > 3600 {
            return Err("external.chat_timeout_seconds must be 1..3600".into());
        }
        if self.control_port == 0 || self.public_port == 0 {
            return Err("external ports must be 1..65535".into());
        }
        if self.control_port == self.public_port {
            return Err(format!(
                "external.control_port ({}) must differ from external.public_port ({})",
                self.control_port, self.public_port
            ));
        }
        // Tránh chiếm port của gateway chính.
        if let Some(main_port) = parse_port_from_listen(main_listen) {
            let base_port = port_from_url(&base);
            if base_port == Some(main_port) {
                return Err(format!(
                    "external.base_url port ({main_port}) conflicts with gateway listen ({main_listen}). Change control_port/base_url."
                ));
            }
            if self.control_port == main_port || self.public_port == main_port {
                return Err(format!(
                    "external ports ({}/{}) conflict with gateway listen port ({main_port})",
                    self.control_port, self.public_port
                ));
            }
        }
        Ok(())
    }
}

pub fn normalize_base_url(raw: &str) -> Result<String, String> {
    let t = raw.trim().trim_end_matches('/');
    if t.is_empty() {
        return Err("external.base_url is empty".into());
    }
    if !(t.starts_with("http://") || t.starts_with("https://")) {
        return Err("external.base_url must start with http:// or https://".into());
    }
    // Phải có host sau scheme (tránh `http://` trần — lỗi ngầm).
    let host_part = t
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or("");
    if host_part.is_empty() {
        return Err("external.base_url missing host".into());
    }
    Ok(t.to_string())
}

/// Tách port từ listen dạng `:7863`, `0.0.0.0:7863`, `127.0.0.1:8080`, `7863`.
pub fn parse_port_from_listen(listen: &str) -> Option<u16> {
    let t = listen.trim();
    if t.is_empty() {
        return None;
    }
    // Lấy phần sau dấu `:` cuối cùng; nếu không có `:` thì toàn chuỗi là port.
    let port_str = t.rsplit(':').next().unwrap_or(t).trim();
    // Trường hợp `[::]:7863` → rsplit vẫn cho `7863`. Trường hợp `abc` → parse fail → None.
    port_str.parse::<u16>().ok().filter(|p| *p != 0)
}

fn port_from_url(url: &str) -> Option<u16> {
    // Bỏ scheme, lấy authority (host[:port]), bỏ path.
    let auth = url.split_once("://")?.1.split('/').next()?;
    // Bỏ userinfo nếu có.
    let hostport = auth.rsplit('@').next()?;
    // IPv6 `[::1]:8964`.
    if let Some(rest) = hostport.strip_prefix('[') {
        let end = rest.find(']')?;
        let after = &rest[end + 1..];
        return after.strip_prefix(':')?.parse::<u16>().ok();
    }
    // hostname/ipv4: lấy sau `:` cuối, nhưng phải chắc phần đó là số và host không rỗng.
    let mut parts = hostport.rsplit(':');
    let maybe_port = parts.next()?;
    let host = parts.next()?;
    if host.is_empty() {
        return None;
    }
    // Nếu còn `:` nữa (host chứa `:` mà không phải IPv6 ngoặc) → ambiguous → None.
    if host.contains(':') {
        return None;
    }
    maybe_port.parse::<u16>().ok()
}

/// Kiểm tra port có bind được trên 127.0.0.1 không (true = trống).
pub fn port_available(port: u16) -> bool {
    std::net::TcpListener::bind(format!("127.0.0.1:{port}")).is_ok()
}

/// Tìm port trống từ gợi ý, thử tối đa `tries` port liên tiếp.
pub fn find_free_port(preferred: u16, tries: u16) -> Option<u16> {
    let mut p = preferred.max(1);
    for _ in 0..tries.max(1) {
        if port_available(p) {
            return Some(p);
        }
        p = p.wrapping_add(1);
        if p == 0 {
            p = 1;
        }
    }
    None
}

// ─── Multi-provider registry (chuẩn bị tách anti-api thành nhiều provider) ────

/// Một provider ngoài (hiện mỗi cái map tới một anti-api instance hoặc tương tự).
/// Tương lai: mỗi provider có thể là native module riêng (copilot.rs, zed.rs...)
/// mà không phá vỡ config — `kind` quyết định transport, các field khác giữ nguyên.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEntry {
    /// Tên hiển thị + ID duy nhất (vd "antigravity", "copilot").
    pub name: String,
    /// Loại transport: "http-bridge" (anti-api) — mở rộng "native" sau.
    #[serde(default = "default_provider_kind")]
    pub kind: String,
    /// Model pattern: tiền tố phân tách `|`, prefix-match lowercase.
    /// Trống = không match model nào (an toàn mặc định).
    #[serde(default)]
    pub model_prefixes: String,
    #[serde(flatten)]
    pub config: ExternalConfig,
}

fn default_provider_kind() -> String {
    "http-bridge".to_string()
}

/// Prefix mặc định của entry anti-api migrate từ config cũ — giữ nguyên tập
/// model đã route trước đây (claude/gpt/gemini/grok/copilot/codex/zed/kiro...).
pub fn default_antiapi_prefixes() -> String {
    [
        "route:", "route/",
        "claude-", "claude/", "gpt-", "gpt/", "o1", "o3", "o4",
        "gemini-", "gemini/", "grok-", "grok/", "xbuild",
        "copilot-", "copilot/", "codex", "zed", "kiro",
        "opus", "sonnet", "haiku",
    ]
    .join("|")
}

/// Danh sách provider mặc định khi user chưa cấu hình gì: 1 anti-api bridge
/// (tương thích ngược với config `external` cũ).
impl Default for ProviderEntry {
    fn default() -> Self {
        Self {
            name: "anti-api".to_string(),
            kind: default_provider_kind(),
            model_prefixes: default_antiapi_prefixes(),
            config: ExternalConfig::default(),
        }
    }
}

/// Registry toàn cục: map name → ProviderEntry. Nguồn chân lý cho routing:
/// request model khớp pattern của provider nào → forward sang đó.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderRegistry {
    #[serde(default)]
    pub providers: Vec<ProviderEntry>,
}

impl ProviderRegistry {
    /// Migrate từ config cũ: nếu config.json có `external` (single) mà
    /// `providers` rỗng → tự tạo 1 entry tên "anti-api" kèm full prefixes cũ.
    /// Không mất dữ liệu, routing giữ nguyên.
    pub fn from_legacy(external: &ExternalConfig) -> Self {
        let mut reg = Self::default();
        if external.enabled || !external.base_url.is_empty() {
            reg.providers.push(ProviderEntry {
                name: "anti-api".to_string(),
                kind: default_provider_kind(),
                model_prefixes: default_antiapi_prefixes(),
                config: external.clone(),
            });
        }
        reg
    }

    /// Provider đầu tiên (theo thứ tự) enabled và khớp model-pattern.
    /// Mỗi entry mang `model_prefixes` riêng → model nào rơi vào provider đó.
    pub fn route_model(&self, model: &str) -> Option<&ProviderEntry> {
        self.providers.iter().find(|p| p.matches_model(model))
    }

    pub fn enabled_any(&self) -> bool {
        self.providers.iter().any(|p| p.config.enabled)
    }
}

/// Providers hiệu dụng cho routing/validate: `config.providers` nếu có,
/// ngược lại migrate từ legacy `external` (config cũ / default).
/// Trả về Vec clone để caller không giữ lock config qua I/O.
pub fn effective_providers(config: &crate::core::config::Config) -> Vec<ProviderEntry> {
    if config.providers.is_empty() {
        ProviderRegistry::from_legacy(&config.external).providers
    } else {
        config.providers.clone()
    }
}

/// Resolve model → ProviderEntry clone (http-bridge). Native blocks (Zed...)
/// tách hoàn toàn: tự resolve cache riêng, KHÔNG đi qua registry này.

// ─── Model routing ────────────────────────────────────────────────────────────

/// Prefix/ID đặc trưng của anti-api providers (Antigravity/Codex/Copilot/Zed/
/// Kiro/Grok). Model nội bộ (CodeBuddy): glm/kimi/minimax/hy3/deepseek.
pub fn is_external_model(model: &str) -> bool {
    let m = model.trim().to_lowercase();
    if m.is_empty() {
        return false;
    }
    // Flow routing của anti-api: `route:xxx` hoặc flow name.
    if m.starts_with("route:") || m.starts_with("route/") {
        return true;
    }
    const PREFIXES: &[&str] = &[
        "claude-", "claude/", "gpt-", "gpt/", "o1", "o3", "o4",
        "gemini-", "gemini/", "grok-", "grok/", "xbuild",
        "copilot-", "copilot/", "codex", "zed", "kiro",
        "opus", "sonnet", "haiku",
    ];
    PREFIXES.iter().any(|p| m.starts_with(p))
}



impl ProviderEntry {
    /// Check model có thuộc provider này không (prefix match, lowercase).
    /// Tắt hoặc prefixes trống → không match (an toàn mặc định).
    pub fn matches_model(&self, model: &str) -> bool {
        if !self.config.enabled || self.model_prefixes.trim().is_empty() {
            return false;
        }
        let m = model.trim().to_lowercase();
        if m.is_empty() {
            return false;
        }
        self.model_prefixes
            .split('|')
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .any(|p| m.starts_with(&p))
    }
}

// ─── Registry routing (http-bridge + mọi kind khác) ────────────────────────────

/// Resolve model → ProviderEntry clone (mọi kind).
pub fn resolve_route_entry(
    config: &crate::core::config::Config,
    model: &str,
) -> Option<ProviderEntry> {
    let reg = ProviderRegistry { providers: effective_providers(config) };
    reg.route_model(model).cloned()
}

// ─── HTTP bridge (ureq blocking — gọi trong spawn_blocking) ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalStatus {
    pub reachable: bool,
    pub http_status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalModelEntry {
    pub id: String,
    pub owned_by: String,
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(timeout).build()
}

/// GET `{base}/health` — anti-api trả `{status:"ok"}`.
pub fn check_health(cfg: &ExternalConfig) -> ExternalStatus {
    let base = match cfg.normalized_base_url() {
        Ok(b) => b,
        Err(e) => {
            return ExternalStatus {
                reachable: false,
                http_status: None,
                latency_ms: None,
                error: Some(e),
            }
        }
    };
    let start = std::time::Instant::now();
    let url = format!("{base}/health");
    match agent(Duration::from_secs(cfg.timeout_seconds.max(1))).get(&url).call() {
        Ok(resp) => ExternalStatus {
            reachable: true,
            http_status: Some(resp.status()),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Err(ureq::Error::Status(code, _)) => ExternalStatus {
            // Host trả lời nhưng mã lỗi — vẫn coi là reachable để UI báo rõ.
            reachable: true,
            http_status: Some(code),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(format!("health returned HTTP {code}")),
        },
        Err(e) => ExternalStatus {
            reachable: false,
            http_status: None,
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: Some(format!("health check failed: {e}")),
        },
    }
}

/// GET `{base}/v1/models` → list `{id, owned_by}`. Giới hạn 500 entry, mỗi id ≤256.
pub fn fetch_external_models(cfg: &ExternalConfig) -> Result<Vec<ExternalModelEntry>, String> {
    let base = cfg.normalized_base_url()?;
    let url = format!("{base}/v1/models");
    let mut req = agent(Duration::from_secs(cfg.timeout_seconds.max(1))).get(&url);
    if !cfg.api_key.trim().is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", cfg.api_key.trim()));
    }
    let resp = req.call().map_err(|e| format!("GET /v1/models failed: {e}"))?;
    let v: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("Invalid /v1/models JSON: {e}"))?;
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or("Invalid /v1/models: missing data[]")?;
    let mut out = Vec::new();
    for item in data.iter().take(500) {
        if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
            let id = id.trim();
            if id.is_empty() || id.len() > 256 {
                continue;
            }
            let owned = item
                .get("owned_by")
                .and_then(|o| o.as_str())
                .unwrap_or("anti-api")
                .to_string();
            out.push(ExternalModelEntry {
                id: id.to_string(),
                owned_by: owned,
            });
        }
    }
    Ok(out)
}

/// Forward chat completions tới `{base}/v1/chat/completions`.
/// Trả về (status, body_bytes, response_headers). Caller chịu trách nhiệm stream/SSE về client.
pub fn forward_chat(
    cfg: &ExternalConfig,
    body: &[u8],
    extra_headers: &[(String, String)],
) -> Result<(u16, Vec<u8>, Vec<(String, String)>), String> {
    let base = cfg.normalized_base_url()?;
    let url = format!("{base}/v1/chat/completions");
    let timeout = Duration::from_secs(cfg.chat_timeout_seconds.clamp(1, 3600));
    let mut req = agent(timeout).post(&url).set("Content-Type", "application/json");
    // Forward auth: ưu tiên api_key cấu hình, nếu không thì lấy Bearer/x-api-key của caller.
    let mut auth_set = false;
    if !cfg.api_key.trim().is_empty() {
        req = req.set("Authorization", &format!("Bearer {}", cfg.api_key.trim()));
        auth_set = true;
    }
    for (k, v) in extra_headers {
        let kl = k.to_ascii_lowercase();
        if kl == "authorization" && auth_set {
            continue;
        }
        if kl == "authorization" || kl == "x-api-key" {
            if v.len() > 4096 {
                continue; // chặn header quá khổ — lỗi ngầm.
            }
            if kl == "authorization" {
                req = req.set("Authorization", v);
                auth_set = true;
            } else {
                req = req.set("x-api-key", v);
            }
        }
    }
    // Chặn body quá khổ ở tầng forward (server đã chặn max_body_mb, đây là belt & braces).
    if body.len() > 16 * 1024 * 1024 {
        return Err("forward body exceeds 16 MB".into());
    }
    let resp = req
        .send_bytes(body)
        .map_err(|e| match e {
            ureq::Error::Status(code, r) => {
                let preview: String = r
                    .into_string()
                    .unwrap_or_default()
                    .chars()
                    .take(2000)
                    .collect();
                format!("external returned HTTP {code}: {preview}")
            }
            other => format!("forward failed: {other}"),
        })?;
    let status = resp.status();
    let resp_headers: Vec<(String, String)> = resp
        .headers_names()
        .into_iter()
        .map(|n| {
            let v = resp.header(&n).unwrap_or_default().to_string();
            (n, v)
        })
        .collect();
    let mut buf = Vec::new();
    resp.into_reader()
        .take(32 * 1024 * 1024)
        .read_to_end(&mut buf)
        .map_err(|e| format!("read external body failed: {e}"))?;
    Ok((status, buf, resp_headers))
}


use std::io::Read as _;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ok_and_err() {
        assert_eq!(
            normalize_base_url("http://127.0.0.1:8964/").unwrap(),
            "http://127.0.0.1:8964"
        );
        assert!(normalize_base_url("127.0.0.1:8964").is_err());
        assert!(normalize_base_url("http://").is_err());
        assert!(normalize_base_url("  ").is_err());
    }

    #[test]
    fn parse_listen_ports() {
        assert_eq!(parse_port_from_listen(":7863"), Some(7863));
        assert_eq!(parse_port_from_listen("0.0.0.0:7863"), Some(7863));
        assert_eq!(parse_port_from_listen("7863"), Some(7863));
        assert_eq!(parse_port_from_listen("[::]:8080"), Some(8080));
        assert_eq!(parse_port_from_listen(""), None);
        assert_eq!(parse_port_from_listen("abc"), None);
    }

    #[test]
    fn port_from_url_cases() {
        assert_eq!(port_from_url("http://127.0.0.1:8964"), Some(8964));
        assert_eq!(port_from_url("http://127.0.0.1:8964/"), Some(8964));
        assert_eq!(port_from_url("https://example.com"), None);
        assert_eq!(port_from_url("http://[::1]:8966/x"), Some(8966));
    }

    #[test]
    fn external_model_classification() {
        assert!(is_external_model("claude-sonnet-4-5"));
        assert!(is_external_model("gpt-5.3-codex"));
        assert!(is_external_model("gemini-3-pro-high"));
        assert!(is_external_model("route:fast"));
        assert!(is_external_model("grok-build"));
        assert!(!is_external_model("glm-5.2"));
        assert!(!is_external_model("kimi-k2.7"));
        assert!(!is_external_model("hy3-preview"));
        assert!(!is_external_model(""));
    }

    #[test]
    fn validate_conflict_detection() {
        let cfg = ExternalConfig {
            enabled: true,
            base_url: "http://127.0.0.1:7863".into(),
            ..Default::default()
        };
        assert!(cfg.validate(":7863").is_err());
        let ok = ExternalConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(ok.validate(":7863").is_ok());
        let same_ports = ExternalConfig {
            enabled: true,
            control_port: 9000,
            public_port: 9000,
            ..Default::default()
        };
        assert!(same_ports.validate(":7863").is_err());
    }

    #[test]
    fn disabled_skips_validation() {
        let cfg = ExternalConfig {
            enabled: false,
            base_url: "bad".into(),
            ..Default::default()
        };
        assert!(cfg.validate(":7863").is_ok());
    }

    #[test]
    fn provider_registry_routes_by_prefix() {
        let mk = |name: &str, prefixes: &str, enabled: bool| ProviderEntry {
            name: name.to_string(),
            kind: "http-bridge".to_string(),
            model_prefixes: prefixes.to_string(),
            config: ExternalConfig { enabled, ..Default::default() },
        };
        let reg = ProviderRegistry {
            providers: vec![
                mk("copilot", "copilot-|copilot/", true),
                mk("codex", "gpt-|codex|o1|o3|o4", true),
                mk("off", "claude-", false),
            ],
        };
        assert_eq!(reg.route_model("copilot-claude").unwrap().name, "copilot");
        assert_eq!(reg.route_model("GPT-5").unwrap().name, "codex");
        assert!(reg.route_model("claude-sonnet").is_none()); // tắt → không match
        assert!(reg.route_model("glm-5").is_none()); // model nội bộ
        assert!(reg.route_model("").is_none());
        assert!(reg.enabled_any());
        let empty = ProviderRegistry::default();
        assert!(!empty.enabled_any());
        assert!(empty.route_model("gpt-5").is_none());
    }

    #[test]
    fn legacy_migration_keeps_routing() {
        let mut ext = ExternalConfig::default();
        ext.enabled = true;
        let reg = ProviderRegistry::from_legacy(&ext);
        assert_eq!(reg.providers.len(), 1);
        // Tập model cũ vẫn route về entry migrate.
        assert_eq!(reg.route_model("claude-sonnet-4").unwrap().name, "anti-api");
        assert_eq!(reg.route_model("route:fast").unwrap().name, "anti-api");
        assert!(reg.route_model("glm-5").is_none());
        // Config tắt + base rỗng → không tạo entry (tránh provider ma).
        let off = ExternalConfig { enabled: false, base_url: String::new(), ..Default::default() };
        assert!(ProviderRegistry::from_legacy(&off).providers.is_empty());
    }
}

// ─── Gateway integration (điểm biên server.rs gọi vào) ───────────────────────
// Toàn bộ logic anti-api chạm tới HTTP gateway nằm dưới đây. `server.rs`
// (khối Core) chỉ quyết định đi đường nào qua `is_external_model`.

/// Merge external models vào list (dedupe theo id để tránh trùng).
pub fn merge_external_models(data: &mut Vec<Value>, external: Vec<Value>) {
    let mut seen: std::collections::HashSet<String> = data
        .iter()
        .filter_map(|v| v.get("id")?.as_str().map(|s| s.to_string()))
        .collect();
    for m in external {
        if let Some(id) = m.get("id").and_then(|i| i.as_str()) {
            if seen.insert(id.to_string()) {
                data.push(m);
            }
        }
    }
}

/// Forward tới external provider (anti-api compatible) — SSE relay.
///
/// Giữ nguyên envelope SSE của gateway để client không phải đổi code:
///
/// - OpenAI route: relay `data:` lines + `[DONE]`.
/// - Anthropic route: bọc `message_start/block_start/delta/stop` như dispatch nội bộ.
///
/// Lỗi external → JSON error event + metrics/audit (không panic — chống lỗi ngầm).
#[allow(clippy::too_many_arguments)]
pub fn forward_external_stream(
    state: SharedState,
    ext_cfg: ExternalConfig,
    key_prefix: String,
    route: &'static str,
    model: String,
    upstream_body: Vec<u8>,
    raw_request_headers: Vec<(String, String)>,
    raw_request_body: String,
    stream: bool,
    is_anthropic: bool,
) -> Response {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, std::convert::Infallible>>(64);
    let metrics = state.metrics.clone();
    let audit = state.audit.clone();
    let storage = state.storage.clone();

    tokio::task::spawn_blocking(move || {
        let relay_start = Instant::now();
        let mut total_tokens: u64 = 0;
        let final_status: u16;
        let mut err_msg = String::new();
        let mut resp_preview = String::new();
        const PREVIEW_LIMIT: usize = 4096;
        let mut final_response_headers: Vec<(String, String)> = Vec::new();

        // Chỉ forward auth headers (tránh rò cookie/internal headers — lỗi ngầm bảo mật).
        let fwd_headers: Vec<(String, String)> = raw_request_headers
            .iter()
            .filter(|(k, _)| {
                let kl = k.to_ascii_lowercase();
                kl == "authorization" || kl == "x-api-key"
            })
            .cloned()
            .collect();

        match forward_chat(&ext_cfg, &upstream_body, &fwd_headers) {
            Ok((status, body_bytes, resp_headers)) => {
                final_status = status;
                final_response_headers = resp_headers;
                if !(200..300).contains(&status) {
                    err_msg = String::from_utf8_lossy(&body_bytes).chars().take(500).collect();
                    let _ = tx.blocking_send(Ok(Event::default().data(format!(
                        "{{\"error\":\"external returned {status}\"}}"
                    ))));
                } else {
                    let text = String::from_utf8_lossy(&body_bytes).to_string();
                    if is_anthropic {
                        let msg_id = format!("msg_{}", relay_start.elapsed().as_nanos());
                        let _ = tx.blocking_send(Ok(Event::default().event("message_start").data(anthropic_message_start(&msg_id, &model))));
                        let _ = tx.blocking_send(Ok(Event::default().event("content_block_start").data(anthropic_content_block_start(0))));
                    }
                    if !stream {
                        // Non-stream: anti-api có thể trả JSON hoàn chỉnh hoặc SSE aggregate.
                        // Thử parse SSE `data:` lines trước, fallback trả nguyên body.
                        let mut emitted = false;
                        for line in text.lines() {
                            let line = line.trim();
                            if let Some(payload) = line.strip_prefix("data:") {
                                let payload = payload.trim();
                                if payload.is_empty() {
                                    continue;
                                }
                                if resp_preview.len() < PREVIEW_LIMIT && payload != "[DONE]" {
                                    resp_preview.push_str(&payload.chars().take(PREVIEW_LIMIT - resp_preview.len()).collect::<String>());
                                    resp_preview.push('\n');
                                }
                                if payload == "[DONE]" {
                                    continue;
                                }
                                total_tokens += 1;
                                if is_anthropic {
                                    let chunk_text = serde_json::from_str::<Value>(payload)
                                        .ok()
                                        .and_then(|v| v.get("choices")?.get(0)?.get("delta")?.get("content")?.as_str().map(|s| s.to_string()))
                                        .unwrap_or_default();
                                    if !chunk_text.is_empty() {
                                        let _ = tx.blocking_send(Ok(Event::default().event("content_block_delta").data(anthropic_content_block_delta(0, &chunk_text))));
                                    }
                                } else {
                                    let _ = tx.blocking_send(Ok(Event::default().data(payload)));
                                }
                                emitted = true;
                            }
                        }
                        if !emitted {
                            // JSON thuần (non-SSE): bọc thành 1 data event.
                            if resp_preview.is_empty() {
                                resp_preview = text.chars().take(PREVIEW_LIMIT).collect();
                            }
                            total_tokens = 1;
                            if is_anthropic {
                                // Trích text từ OpenAI JSON hoàn chỉnh nếu có.
                                let chunk_text = serde_json::from_str::<Value>(&text)
                                    .ok()
                                    .and_then(|v| {
                                        v.get("choices")?.get(0)?.get("message")?.get("content")?.as_str().map(|s| s.to_string())
                                            .or_else(|| v.get("content")?.as_str().map(|s| s.to_string()))
                                    })
                                    .unwrap_or_else(|| text.chars().take(4000).collect());
                                if !chunk_text.is_empty() {
                                    let _ = tx.blocking_send(Ok(Event::default().event("content_block_delta").data(anthropic_content_block_delta(0, &chunk_text))));
                                }
                            } else {
                                let _ = tx.blocking_send(Ok(Event::default().data(text)));
                            }
                        }
                        if is_anthropic {
                            let _ = tx.blocking_send(Ok(Event::default().event("content_block_stop").data(anthropic_content_block_stop(0))));
                            let _ = tx.blocking_send(Ok(Event::default().event("message_delta").data(anthropic_message_delta(total_tokens))));
                            let _ = tx.blocking_send(Ok(Event::default().event("message_stop").data(anthropic_message_stop())));
                        } else {
                            let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
                        }
                    } else {
                        for line in text.lines() {
                            let line = line.trim();
                            if line.is_empty() {
                                continue;
                            }
                            if let Some(payload) = line.strip_prefix("data:") {
                                let payload = payload.trim();
                                if payload.is_empty() {
                                    continue;
                                }
                                if resp_preview.len() < PREVIEW_LIMIT && payload != "[DONE]" {
                                    resp_preview.push_str(&payload.chars().take(PREVIEW_LIMIT - resp_preview.len()).collect::<String>());
                                    resp_preview.push('\n');
                                }
                                total_tokens += 1;
                                if payload == "[DONE]" {
                                    if is_anthropic {
                                        let _ = tx.blocking_send(Ok(Event::default().event("content_block_stop").data(anthropic_content_block_stop(0))));
                                        let _ = tx.blocking_send(Ok(Event::default().event("message_delta").data(anthropic_message_delta(total_tokens))));
                                        let _ = tx.blocking_send(Ok(Event::default().event("message_stop").data(anthropic_message_stop())));
                                    } else {
                                        let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
                                    }
                                    break;
                                }
                                if is_anthropic {
                                    let chunk_text = serde_json::from_str::<Value>(payload)
                                        .ok()
                                        .and_then(|v| v.get("choices")?.get(0)?.get("delta")?.get("content")?.as_str().map(|s| s.to_string()))
                                        .unwrap_or_default();
                                    if !chunk_text.is_empty() {
                                        let _ = tx.blocking_send(Ok(Event::default().event("content_block_delta").data(anthropic_content_block_delta(0, &chunk_text))));
                                    }
                                } else if tx.blocking_send(Ok(Event::default().data(payload))).is_err() {
                                    break;
                                }
                            } else if line.starts_with('{') {
                                // JSON thuần lẫn trong stream — forward nguyên dòng.
                                let _ = tx.blocking_send(Ok(Event::default().data(line)));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                final_status = 502;
                err_msg = e.chars().take(500).collect();
                let _ = tx.blocking_send(Ok(Event::default().data(
                    "{\"error\":\"external provider unavailable\"}",
                )));
            }
        }

        let elapsed = relay_start.elapsed();
        info!(
            "[req] route={} key={} acct=external model={} status={} tokens={} elapsed={}ms err={}",
            route, key_prefix, model, final_status, total_tokens,
            elapsed.as_millis(), err_msg
        );
        metrics.trace_detail(
            route,
            &model,
            final_status,
            total_tokens,
            elapsed.as_millis() as u64,
            Some(json!({
                "req_bytes": upstream_body.len(),
                "req_preview": String::from_utf8_lossy(&upstream_body).chars().take(1000).collect::<String>(),
                "resp_preview": resp_preview.chars().take(2000).collect::<String>(),
                "error": if err_msg.is_empty() { Value::Null } else { Value::String(err_msg.clone()) },
                "via": "external",
            })),
        );
        if let Some(s) = &storage {
            let _ = s.log_usage(&key_prefix, route, &model, final_status, total_tokens, elapsed.as_millis() as u64);
        }
        let forwarded_body = {
            let sent = String::from_utf8_lossy(&upstream_body).to_string();
            if sent == raw_request_body { String::new() } else { sent.chars().take(4096).collect() }
        };
        audit.push(crate::core::audit::TrafficAuditLog {
            id: format!("ext_{}", relay_start.elapsed().as_nanos()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            route: route.to_string(),
            model: model.clone(),
            status_code: final_status,
            duration_ms: elapsed.as_millis() as u64,
            account_uid: "external".to_string(),
            proxy_used: None,
            raw_request_headers,
            raw_forwarded_headers: Vec::new(),
            raw_request_body,
            raw_response_preview: resp_preview,
            raw_response_headers: final_response_headers,
            raw_forwarded_body: forwarded_body,
            is_streaming: stream,
        });
    });

    let stream_resp = ReceiverStream::new(rx);
    Sse::new(stream_resp)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text(":"),
        )
        .into_response()
}
