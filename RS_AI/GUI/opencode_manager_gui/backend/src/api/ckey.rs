/*
[INTEGRITY NOTES]
- Mục đích: Tích hợp CKey (ckey.vn) — quản lý TÀI KHOẢN (profile) + cache TTL,
  xem thống kê/lịch sử/nạp tiền theo profile, và import model vào provider.
 - Trách nhiệm: CRUD profile trong `~/.config/opencode-manager/ckey.json`, gọi API CKey
  QUA CACHE (tránh gọi liên tục bị cấm/rate-limit), ghi model vào provider.
- Tương tác: crate `opencode_manager::ckey` (client + kiểu dữ liệu),
  `opencode_manager::config::CkeyConfig` (profiles/bindings), `core::store`,
  frontend `pages/CkeyPage.tsx`.

Mô hình dữ liệu (đổi từ bản cũ "mỗi provider một account key"):
  - PROFILE = một tài khoản ckey.vn (account key từ trang Profile). Người dùng
    lưu được nhiều profile và chuyển đổi (active) — dashboard/usage/deposit xem
    đúng tài khoản active, KHÔNG gắn với provider nào.
  - BINDING = provider_id → profile_id: import model dùng AI key của profile
    được gắn. Account key không còn nằm trong provider select.
  - AI KEY (`ck-...`) vẫn là khoá gọi LLM (https://api.xah.io/v1), ghi vào
    provider khi import.

Cache TTL (tránh ban): mọi request quản lý đi qua cache trong RAM của backend;
quá TTL mới gọi mạng. `force = true` (nút Refresh) bỏ cache. Frontend tự
refresh định kỳ bằng force = false → cache còn hạn thì không đụng mạng.
*/

use crate::core::store::{load_merged, mask_key, save_split};
use opencode_manager::app::DynamicPreset;
use opencode_manager::ckey::{
    CkeyAiKey, CkeyClient, CkeyModel, CkeyProfile, CkeyUsagePage, CkeyUsageStats, CKEY_LLM_BASE_URL,
    CKEY_MANAGE_API_BASE,
};
use opencode_manager::config::{CkeyConfig, ModelEntry, ModelLimit, ModelModalities, Provider, ProviderOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

// ============================================================
// CACHE TTL
// ============================================================

/// Cache phần "hồ sơ" (profile info) — đổi hiếm khi.
const TTL_PROFILE: Duration = Duration::from_secs(600);
/// Cache thống kê dùng AI.
const TTL_STATS: Duration = Duration::from_secs(120);
/// Cache danh sách AI key của tài khoản.
const TTL_KEYS: Duration = Duration::from_secs(120);
/// Cache danh sách model + giá.
const TTL_MODELS: Duration = Duration::from_secs(300);
/// Cache lịch sử dùng / nạp tiền.
const TTL_LIST: Duration = Duration::from_secs(120);

struct CacheEntry {
    at: Instant,
    value: serde_json::Value,
}

/// Cache dùng DefaultHasher cho key: mục đích là TRÁNH gọi mạng lặp lại, không
/// phải bảo mật (key account không nằm nguyên văn trong cache key).
fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Giới hạn số entry: trang usage/amount khác nhau có thể tạo key mới mãi;
/// quá giới hạn thì xoá sạch (đơn giản, an toàn — chỉ mất cache).
const CACHE_MAX_ENTRIES: usize = 128;

fn cache_key(account_key: &str, endpoint: &str, extra: &str) -> String {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    account_key.hash(&mut h);
    format!("{:x}|{}|{}", h.finish(), endpoint, extra)
}

/// Cache key TOÀN CỦA (không phụ thuộc tài khoản): catalogue model + giá của
/// CKey GIỐNG NHAU với mọi tài khoản — chỉ cần MỘT tài khoản bất kỳ để tải.
/// Nhờ vậy đổi tài khoản không phải tải lại bảng giá.
fn catalog_key() -> String {
    "catalog|models".to_string()
}

fn cache_get<T: serde::de::DeserializeOwned>(key: &str, ttl: Duration) -> Option<T> {
    let map = cache().lock().ok()?;
    let entry = map.get(key)?;
    if entry.at.elapsed() > ttl {
        return None;
    }
    serde_json::from_value(entry.value.clone()).ok()
}

fn cache_put<T: Serialize>(key: &str, value: &T) {
    let Ok(json) = serde_json::to_value(value) else { return };
    if let Ok(mut map) = cache().lock() {
        if map.len() >= CACHE_MAX_ENTRIES {
            map.clear();
        }
        map.insert(
            key.to_string(),
            CacheEntry {
                at: Instant::now(),
                value: json,
            },
        );
    }
}

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("không tạo được tokio runtime")
        .block_on(fut)
}

fn presets() -> Vec<DynamicPreset> {
    opencode_manager::app::App::load_dynamic_presets()
}

/// Provider có phải dùng endpoint LLM của CKey không (mới hỗ trợ các API này).
/// Provider có dùng endpoint LLM của CKey không — chỉ còn giá trị test sau
/// khi bỏ luồng "chọn provider đích" (đích giờ suy từ binding).
#[cfg(test)]
fn is_ckey_provider(base_url: &str) -> bool {
    opencode_manager::config::normalize_base_url(base_url)
        == opencode_manager::config::normalize_base_url(CKEY_LLM_BASE_URL)
}

// ============================================================
// PROFILE (tài khoản CKey) — CRUD
// ============================================================

/// Một tài khoản CKey đã lưu (key đã che).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyProfileView {
    pub id: String,
    pub name: String,
    pub key_masked: String,
    /// Profile đang dùng (dashboard/usage/deposit xem tài khoản này).
    pub is_active: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_ckey_profiles() -> Result<Vec<CkeyProfileView>, String> {
    let cfg = CkeyConfig::load()?;
    let active = cfg.active_profile().map(|p| p.id.clone());
    let mut out: Vec<CkeyProfileView> = cfg
        .profiles
        .iter()
        .map(|p| CkeyProfileView {
            id: p.id.clone(),
            name: if p.name.trim().is_empty() {
                p.id.clone()
            } else {
                p.name.clone()
            },
            key_masked: mask_key(&p.key),
            is_active: active.as_deref() == Some(p.id.as_str()),
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// Thêm hoặc sửa một profile. `profile_id` = None → tạo mới (trả id mới).
/// Đổi key của profile là đổi key cho MỌI provider đang gắn nó (binding tham
/// chiếu, không copy) — chính là ưu điểm của mô hình profile.
#[tauri::command(rename_all = "snake_case")]
pub fn save_ckey_profile(profile_id: Option<String>, name: String, key: String) -> Result<String, String> {
    let name = name.trim().to_string();
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err("Account key không được để trống.".to_string());
    }

    let mut cfg = CkeyConfig::load()?;
    match profile_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => {
            let p = cfg
                .profiles
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| format!("Không tìm thấy profile: {id}"))?;
            p.key = key;
            if !name.is_empty() {
                p.name = name;
            }
            cfg.save()?;
            Ok(id.to_string())
        }
        None => {
            let id = cfg.next_profile_id();
            cfg.profiles.push(opencode_manager::config::CkeyProfileEntry {
                id: id.clone(),
                name: if name.is_empty() {
                    format!("Tài khoản {}", cfg.profiles.len() + 1)
                } else {
                    name
                },
                key,
            });
            // Profile đầu tiên được thêm → dùng luôn làm active.
            if cfg.active.is_none() {
                cfg.active = Some(id.clone());
            }
            cfg.save()?;
            Ok(id)
        }
    }
}

/// Xoá profile + dọn binding trỏ tới nó. Trả về id profile active mới (nếu còn).
#[tauri::command(rename_all = "snake_case")]
pub fn delete_ckey_profile(profile_id: String) -> Result<Option<String>, String> {
    let mut cfg = CkeyConfig::load()?;
    if !cfg.remove_profile(&profile_id) {
        return Err(format!("Không tìm thấy profile: {profile_id}"));
    }
    cfg.save()?;
    Ok(cfg.active_profile().map(|p| p.id.clone()))
}

/// Chuyển tài khoản đang xem (dashboard/usage/deposit) sang profile khác.
#[tauri::command(rename_all = "snake_case")]
pub fn set_active_ckey_profile(profile_id: String) -> Result<(), String> {
    let mut cfg = CkeyConfig::load()?;
    if cfg.profile(&profile_id).is_none() {
        return Err(format!("Không tìm thấy profile: {profile_id}"));
    }
    cfg.active = Some(profile_id);
    cfg.save()?;
    Ok(())
}

/// Account key của profile (báo lỗi thân thiện nếu thiếu).
fn account_key_of(profile_id: &str) -> Result<String, String> {
    CkeyConfig::load()?.key_of(profile_id).ok_or_else(|| {
        format!("Profile '{profile_id}' không tồn tại hoặc chưa có account key. Thêm key từ trang Profile ckey.vn.")
    })
}

/// Account key của profile BẤT KỲ (ưu tiên active) — dùng cho các phần catalogue
/// toàn cục (model + giá giống nhau với mọi tài khoản).
fn any_account_key() -> Result<String, String> {
    let cfg = CkeyConfig::load()?;
    cfg.active_profile()
        .and_then(|p| {
            if p.key.trim().is_empty() {
                None
            } else {
                Some(p.key.clone())
            }
        })
        .or_else(|| {
            cfg.profiles
                .iter()
                .find(|p| !p.key.trim().is_empty())
                .map(|p| p.key.clone())
        })
        .ok_or_else(|| {
            "Chưa có tài khoản CKey nào — thêm một account key trước (danh sách model cần một tài khoản để tải)."
                .to_string()
        })
}

/// Catalogue model + giá (TOÀN CỤC): lấy từ cache, hết hạn mới gọi API bằng
/// account key bất kỳ. `force` = bỏ cache.
fn models_catalog(force: bool) -> Result<Vec<CkeyModel>, String> {
    if !force {
        if let Some(list) = cache_get::<Vec<CkeyModel>>(&catalog_key(), TTL_MODELS) {
            return Ok(list);
        }
    }
    let account_key = any_account_key()?;
    let list = block_on(async { CkeyClient::new(CKEY_MANAGE_API_BASE).fetch_models(&account_key).await })?;
    cache_put(&catalog_key(), &list);
    Ok(list)
}

// ============================================================
// PROVIDER CKEY (đích import)
// ============================================================
// DASHBOARD (theo profile, qua cache)
// ============================================================

/// Toàn bộ thông tin tài khoản trong MỘT lần gọi.
///
/// Gộp 4 phần (profile/stats/keys/models) vào một command; từng phần đi qua
/// cache TTL riêng nên refresh định kỳ gần như không đụng mạng.
///
/// `since_days` != None → thống kê chỉ tính từ mốc đó (docs: param `since`).
/// `force` = true → bỏ cache, gọi thẳng (nút Refresh).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyDashboard {
    pub profile: Option<CkeyAccountInfoView>,
    pub stats: Option<CkeyStatsView>,
    pub keys: Vec<CkeyKeyView>,
    pub models: Vec<CkeyModelView>,
    /// Lỗi của từng phần — một phần lỗi không được làm mất dữ liệu phần khác.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyAccountInfoView {
    pub username: String,
    pub name: String,
    pub email: String,
    pub balance: String,
    pub balance_raw: f64,
    pub created_at: String,
}

impl From<CkeyProfile> for CkeyAccountInfoView {
    fn from(p: CkeyProfile) -> Self {
        Self {
            username: p.username,
            name: p.name,
            email: p.email,
            balance: p.balance,
            balance_raw: p.balance_raw,
            created_at: p.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyStatsView {
    pub requests: u64,
    pub success_requests: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub charged_vnd_text: String,
}

impl From<CkeyUsageStats> for CkeyStatsView {
    fn from(s: CkeyUsageStats) -> Self {
        Self {
            requests: s.requests,
            success_requests: s.success_requests,
            prompt_tokens: s.prompt_tokens,
            completion_tokens: s.completion_tokens,
            total_tokens: s.total_tokens,
            charged_vnd_text: s.charged_vnd_text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyKeyView {
    pub id: u64,
    pub key_name: String,
    pub key_prefix: String,
    pub is_active: bool,
    pub created_at_text: String,
    /// AI key đã che — không gửi key thật ra danh sách.
    pub key_masked: String,
}

/// Model + đầy đủ giá theo docs `/api/llm/models`.
///
/// `public_name` của CKey là dạng "provider/model" (vd "provider/GPT Demo") —
/// danh sách là kết hợp NHÀ CUNG CẤP + MODEL, nên tách sẵn `provider`/`model`
/// cho UI hiển thị hai cột riêng. Catalogue này GIỐNG NHAU với mọi tài khoản.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyModelView {
    /// Id đầy đủ (khóa cấu hình khi import) — nguyên văn public_name.
    pub public_name: String,
    /// Phần "provider" của public_name; rỗng nếu tên không có dạng a/b.
    pub provider: String,
    /// Phần "model" của public_name.
    pub model: String,
    pub display_name: String,
    pub input_price_per_million_vnd: f64,
    pub output_price_per_million_vnd: f64,
    pub cache_read_price_per_million_vnd: f64,
    pub cache_write_price_per_million_vnd: f64,
    pub price_per_request_vnd: f64,
    pub min_charge_per_request_vnd: f64,
    pub cache_enabled: bool,
    pub context_tokens_limit: u64,
    pub max_output_tokens_limit: u64,
}

impl From<CkeyModel> for CkeyModelView {
    fn from(m: CkeyModel) -> Self {
        let (provider, model) = opencode_manager::ckey::split_public_name(&m.public_name);
        Self {
            public_name: m.public_name,
            provider,
            model,
            display_name: m.display_name,
            input_price_per_million_vnd: m.input_price_per_million_vnd,
            output_price_per_million_vnd: m.output_price_per_million_vnd,
            cache_read_price_per_million_vnd: m.cache_read_price_per_million_vnd,
            cache_write_price_per_million_vnd: m.cache_write_price_per_million_vnd,
            price_per_request_vnd: m.price_per_request_vnd,
            min_charge_per_request_vnd: m.min_charge_per_request_vnd,
            cache_enabled: m.cache_enabled,
            context_tokens_limit: m.context_tokens_limit,
            max_output_tokens_limit: m.max_output_tokens_limit,
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn fetch_ckey_dashboard(
    profile_id: String,
    since_days: Option<u64>,
    force: Option<bool>,
) -> Result<CkeyDashboard, String> {
    let account_key = account_key_of(&profile_id)?;
    let force = force.unwrap_or(false);
    // Mốc `since` (Unix seconds) theo docs; None/0 = toàn bộ lịch sử.
    let since = since_days.filter(|d| *d > 0).map(|d| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|now| now.as_secs().saturating_sub(d * 86_400))
            .unwrap_or(0)
    });

    let mut errors = Vec::new();

    // Mỗi phần cache riêng: hết hạn phần nào mới gọi mạng phần đó.
    let profile: Option<CkeyAccountInfoView> = if force {
        None
    } else {
        cache_get(&cache_key(&account_key, "profile", ""), TTL_PROFILE)
    };
    let profile = match profile {
        Some(p) => Some(p),
        None => {
            let r = block_on(async { CkeyClient::new(CKEY_MANAGE_API_BASE).fetch_profile(&account_key).await });
            match r {
                Ok(p) => {
                    let v = CkeyAccountInfoView::from(p);
                    cache_put(&cache_key(&account_key, "profile", ""), &v);
                    Some(v)
                }
                Err(e) => {
                    errors.push(format!("Thông tin tài khoản: {e}"));
                    None
                }
            }
        }
    };

    let stats: Option<CkeyStatsView> = if force {
        None
    } else {
        cache_get(&cache_key(&account_key, "stats", &format!("{:?}", since)), TTL_STATS)
    };
    let stats = match stats {
        Some(s) => Some(s),
        None => {
            let r = block_on(async {
                CkeyClient::new(CKEY_MANAGE_API_BASE)
                    .fetch_usage_stats(&account_key, since)
                    .await
            });
            match r {
                Ok(s) => {
                    let v = CkeyStatsView::from(s);
                    cache_put(&cache_key(&account_key, "stats", &format!("{:?}", since)), &v);
                    Some(v)
                }
                Err(e) => {
                    errors.push(format!("Thống kê sử dụng: {e}"));
                    None
                }
            }
        }
    };

    let keys: Option<Vec<CkeyKeyView>> = if force {
        None
    } else {
        cache_get(&cache_key(&account_key, "keys", ""), TTL_KEYS)
    };
    let keys = match keys {
        Some(k) => k,
        None => {
            let r = block_on(async { CkeyClient::new(CKEY_MANAGE_API_BASE).fetch_keys(&account_key).await });
            match r {
                Ok(list) => {
                    let v: Vec<CkeyKeyView> = list
                        .into_iter()
                        .map(|k: CkeyAiKey| CkeyKeyView {
                            id: k.id,
                            key_name: k.key_name,
                            key_prefix: k.key_prefix,
                            is_active: k.is_active,
                            created_at_text: k.created_at_text,
                            key_masked: mask_key(&k.api_key),
                        })
                        .collect();
                    cache_put(&cache_key(&account_key, "keys", ""), &v);
                    v
                }
                Err(e) => {
                    errors.push(format!("Danh sách AI key: {e}"));
                    Vec::new()
                }
            }
        }
    };

    // Catalogue model + giá là TOÀN CỤC (giống nhau mọi tài khoản) → cache
    // dùng chung, đổi tài khoản không phải tải lại bảng giá. LUÔN dùng cache
    // (TTL 5 phút tự làm mới): `force` của dashboard dành cho dữ liệu TÀI
    // KHOẢN (số dư/thống kê/keys) — ép tải lại bảng giá mỗi lần refresh là
    // gọi mạng vô ích và dễ bị rate-limit.
    let models: Vec<CkeyModelView> = match models_catalog(false) {
        Ok(list) => list.into_iter().map(CkeyModelView::from).collect(),
        Err(e) => {
            errors.push(format!("Danh sách model: {e}"));
            Vec::new()
        }
    };

    Ok(CkeyDashboard {
        profile,
        stats,
        keys,
        models,
        errors,
    })
}

// ============================================================
// USAGE (lịch sử dùng AI, theo profile)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyUsageView {
    pub items: Vec<CkeyUsageItemView>,
    pub page: u64,
    pub total_pages: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyUsageItemView {
    pub request_id: String,
    pub model_name: String,
    pub http_status: u64,
    pub total_tokens: u64,
    pub charged_vnd: f64,
    pub status: String,
    pub latency_ms: u64,
    pub created_at_text: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn fetch_ckey_usage(
    profile_id: String,
    page: u64,
    limit: u64,
    model: Option<String>,
    force: Option<bool>,
) -> Result<CkeyUsageView, String> {
    let account_key = account_key_of(&profile_id)?;
    let page = page.max(1);
    let limit = limit.clamp(1, 100);
    // Filter theo tên model (docs: param `model` của /api/llm/usage).
    let model = model.map(|m| m.trim().to_string()).filter(|m| !m.is_empty());
    let force = force.unwrap_or(false);

    let cache_k = cache_key(&account_key, "usage", &format!("{}|{}|{:?}", page, limit, model));
    let cached: Option<CkeyUsageView> = if force { None } else { cache_get(&cache_k, TTL_LIST) };
    let result = match cached {
        Some(v) => Ok(v),
        None => {
            let r: Result<CkeyUsagePage, String> = block_on(async {
                CkeyClient::new(CKEY_MANAGE_API_BASE)
                    .fetch_usage(&account_key, model.as_deref(), page, limit)
                    .await
            });
            match r {
                Ok(page_data) => {
                    let (p, total) = page_data
                        .pagination
                        .map(|pg| (pg.page, pg.total_pages))
                        .unwrap_or((page, 1));
                    let v = CkeyUsageView {
                        items: page_data
                            .items
                            .into_iter()
                            .map(|i| CkeyUsageItemView {
                                request_id: i.request_id,
                                model_name: i.model_name,
                                http_status: i.http_status,
                                total_tokens: i.total_tokens,
                                charged_vnd: i.charged_vnd,
                                status: i.status,
                                latency_ms: i.latency_ms,
                                created_at_text: i.created_at_text,
                            })
                            .collect(),
                        page: p,
                        total_pages: total.max(1),
                    };
                    cache_put(&cache_k, &v);
                    Ok(v)
                }
                Err(e) => Err(e),
            }
        }
    };
    result
}

// ============================================================
// DEPOSIT (nạp tiền, theo profile)
// ============================================================

/// Thông tin + lịch sử nạp tiền trong MỘT lần gọi (docs: /api/deposit-info,
/// /api/deposit-history). `amount` là số tiền muốn nạp — API yêu cầu tham số
/// này để sinh nội dung chuyển khoản/QR riêng cho từng giao dịch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyDepositView {
    pub info: Option<CkeyDepositInfoView>,
    pub history: Vec<CkeyDepositHistoryItemView>,
    pub history_page: u64,
    pub history_total_pages: u64,
    /// Lỗi từng phần (info lỗi không được làm mất history và ngược lại).
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyDepositInfoView {
    pub transfer_content: String,
    pub amount_vnd: u64,
    pub expires_at: String,
    pub qr_url: String,
    pub banks: Vec<CkeyDepositBankView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyDepositBankView {
    pub bank_name: String,
    pub account_owner: String,
    pub account_number: String,
    pub transfer_content: String,
    pub qr_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CkeyDepositHistoryItemView {
    pub id: u64,
    pub amount_text: String,
    pub time_text: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn fetch_ckey_deposit(
    profile_id: String,
    amount: u64,
    page: u64,
    limit: u64,
    force: Option<bool>,
) -> Result<CkeyDepositView, String> {
    let account_key = account_key_of(&profile_id)?;
    let amount = amount.max(1_000);
    let page = page.max(1);
    let limit = limit.clamp(1, 100);
    let force = force.unwrap_or(false);

    let mut errors = Vec::new();

    // --- Phần info (QR/nội dung CK theo số tiền) ---
    let info_k = cache_key(&account_key, "deposit-info", &amount.to_string());
    let info: Option<CkeyDepositInfoView> = if force { None } else { cache_get(&info_k, TTL_LIST) };
    let info = match info {
        Some(i) => Some(i),
        None => {
            let r = block_on(async {
                CkeyClient::new(CKEY_MANAGE_API_BASE)
                    .fetch_deposit_info(&account_key, amount)
                    .await
            });
            match r {
                Ok(i) => {
                    let v = CkeyDepositInfoView {
                        transfer_content: i.transfer_content,
                        amount_vnd: i.amount_vnd,
                        expires_at: i.expires_at,
                        qr_url: i.qr_url,
                        banks: i
                            .banks
                            .into_iter()
                            .map(|b| CkeyDepositBankView {
                                bank_name: b.bank_name,
                                account_owner: b.account_owner,
                                account_number: b.account_number,
                                transfer_content: b.transfer_content,
                                qr_url: b.qr_url,
                            })
                            .collect(),
                    };
                    cache_put(&info_k, &v);
                    Some(v)
                }
                Err(e) => {
                    errors.push(format!("Thông tin nạp tiền: {e}"));
                    None
                }
            }
        }
    };

    // --- Phần history (phân trang) ---
    let hist_k = cache_key(&account_key, "deposit-history", &format!("{page}|{limit}"));
    let (history, p, total) =
        match cache_get::<CkeyDepositHistoryCached>(&hist_k, if force { Duration::ZERO } else { TTL_LIST }) {
            Some(c) => (c.items, c.page, c.total_pages),
            None => {
                let r = block_on(async {
                    CkeyClient::new(CKEY_MANAGE_API_BASE)
                        .fetch_deposit_history(&account_key, page, limit)
                        .await
                });
                match r {
                    Ok(h) => {
                        let (p, total) = h.pagination.map(|pg| (pg.page, pg.total_pages)).unwrap_or((page, 1));
                        let c = CkeyDepositHistoryCached {
                            items: h
                                .items
                                .into_iter()
                                .map(|i| CkeyDepositHistoryItemView {
                                    id: i.id,
                                    amount_text: i.amount_text,
                                    time_text: i.time_text,
                                })
                                .collect(),
                            page: p,
                            total_pages: total.max(1),
                        };
                        cache_put(&hist_k, &c);
                        (c.items, c.page, c.total_pages)
                    }
                    Err(e) => {
                        errors.push(format!("Lịch sử nạp tiền: {e}"));
                        (Vec::new(), page, 1)
                    }
                }
            }
        };

    Ok(CkeyDepositView {
        info,
        history,
        history_page: p,
        history_total_pages: total,
        errors,
    })
}

/// Dạng cache riêng của phần history (view không tách page/total ra field riêng).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CkeyDepositHistoryCached {
    items: Vec<CkeyDepositHistoryItemView>,
    page: u64,
    total_pages: u64,
}

// ============================================================
// IMPORT (model CKey → provider)
// ============================================================

/// Suy ra modalities đầu vào từ tên model.
///
/// CKey không trả về thông tin modalities, nên phải suy đoán. Chỉ nhận diện các
/// từ khoá rõ ràng; đoán bừa "có ảnh" cho model chỉ-text sẽ làm OpenCode gửi
/// payload sai và request lỗi.
fn infer_input_modalities(model_id: &str) -> Vec<String> {
    let id = model_id.to_lowercase();
    let vision = ["vision", "omni", "image", "-vl", "vl-", "multimodal"]
        .iter()
        .any(|k| id.contains(k));
    if vision {
        vec!["text".to_string(), "image".to_string()]
    } else {
        vec!["text".to_string()]
    }
}

/// Model để import, kèm trạng thái so với cấu hình hiện tại + ĐẦY ĐỦ bảng giá
/// (in/out/cache theo 1M token + giá mỗi request) để hiển thị và SẮP XẾP.
///
/// Catalogue này TOÀN CỤC — giống nhau với mọi tài khoản CKey.
#[derive(Debug, Clone, Serialize)]
pub struct CkeyImportItem {
    /// Id đầy đủ (khóa cấu hình) — nguyên văn public_name.
    pub id: String,
    /// Phần "provider" của public_name; rỗng nếu tên không có dạng a/b.
    pub provider: String,
    /// Phần "model" của public_name.
    pub model: String,
    pub display_name: String,
    pub input_price: f64,
    pub output_price: f64,
    pub cache_read_price: f64,
    pub cache_write_price: f64,
    pub price_per_request: f64,
    pub min_charge_per_request: f64,
    pub cache_enabled: bool,
    pub context_limit: u64,
    pub output_limit: u64,
    pub in_config: bool,
    /// Còn trong config nhưng CKey không còn cung cấp.
    pub stale: bool,
}

impl CkeyImportItem {
    /// Dựng từ model catalogue; `in_config` theo provider đích hiện tại.
    fn from_model(m: &CkeyModel, in_config: bool) -> Self {
        let (provider, model) = opencode_manager::ckey::split_public_name(&m.public_name);
        Self {
            id: m.public_name.clone(),
            provider,
            model,
            display_name: m.display_name.clone(),
            input_price: m.input_price_per_million_vnd,
            output_price: m.output_price_per_million_vnd,
            cache_read_price: m.cache_read_price_per_million_vnd,
            cache_write_price: m.cache_write_price_per_million_vnd,
            price_per_request: m.price_per_request_vnd,
            min_charge_per_request: m.min_charge_per_request_vnd,
            cache_enabled: m.cache_enabled,
            context_limit: m.context_tokens_limit,
            output_limit: m.max_output_tokens_limit,
            in_config,
            stale: false,
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_ckey_import_items() -> Result<CkeyImportList, String> {
    // Catalogue toàn cục — KHÔNG phụ thuộc tài khoản (chỉ cần một tài khoản
    // bất kỳ để tải lần đầu). Đích import suy từ binding của tài khoản ĐANG
    // XEM (mặc định id chuẩn "ckey") — không cần người dùng chọn.
    let ckey_cfg = CkeyConfig::load()?;
    let active = ckey_cfg
        .active_profile()
        .map(|p| p.id.clone())
        .ok_or_else(|| "Chưa có tài khoản CKey nào — thêm một account key trước.".to_string())?;
    let provider_id = opencode_manager::ckey::resolve_import_target(&ckey_cfg, &active);

    let models = models_catalog(false)?;

    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    let existing: Vec<String> = config
        .provider
        .get(&provider_id)
        .map(|p| p.models.keys().cloned().collect())
        .unwrap_or_default();

    let mut items: Vec<CkeyImportItem> = models
        .iter()
        .map(|m| CkeyImportItem::from_model(m, existing.contains(&m.public_name)))
        .collect();
    items.sort_by(|a, b| a.id.cmp(&b.id));

    // Model còn trong config nhưng CKey đã bỏ → hiện cuối, đánh dấu stale.
    let available: Vec<&String> = models.iter().map(|m| &m.public_name).collect();
    let mut stale: Vec<CkeyImportItem> = existing
        .iter()
        .filter(|id| !available.contains(id))
        .map(|id| {
            let (provider, model) = opencode_manager::ckey::split_public_name(id);
            CkeyImportItem {
                id: id.clone(),
                provider,
                model,
                display_name: id.clone(),
                input_price: 0.0,
                output_price: 0.0,
                cache_read_price: 0.0,
                cache_write_price: 0.0,
                price_per_request: 0.0,
                min_charge_per_request: 0.0,
                cache_enabled: false,
                context_limit: 0,
                output_limit: 0,
                in_config: true,
                stale: true,
            }
        })
        .collect();
    stale.sort_by(|a, b| a.id.cmp(&b.id));
    items.extend(stale);

    Ok(CkeyImportList {
        target_provider: provider_id,
        items,
    })
}

/// Kết quả `list_ckey_import_items`: đích import đã suy (để hiển thị) + items.
#[derive(Debug, Clone, Serialize)]
pub struct CkeyImportList {
    /// Provider sẽ nhận model khi bấm import (suy từ binding tài khoản active).
    pub target_provider: String,
    pub items: Vec<CkeyImportItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CkeyImportResult {
    pub added: usize,
    pub removed: usize,
    pub kept: usize,
    /// `true` nếu provider CKey vừa được tạo trong lần import này.
    pub provider_created: bool,
}

/// Ghi danh sách model đã chọn vào provider CKey, dùng AI key của `profile_id`.
///
/// Ghi danh sách model đã chọn vào provider ĐÍCH — đích suy từ binding của
/// tài khoản `profile_id` (mặc định id chuẩn "ckey"), AI key lấy từ tài khoản đó.
///
/// `selected` là danh sách CUỐI CÙNG — model trong config mà không có trong đây
/// sẽ bị xoá (nhờ vậy bỏ tick model stale là nó biến mất khỏi cấu hình).
/// Import là lúc DUY NHẤT binding đổi: provider "thuộc" tài khoản nào thì trả
/// tiền tài khoản đó, nên khoá luôn được đồng bộ theo AI key active của profile.
#[tauri::command(rename_all = "snake_case")]
pub fn import_ckey_models(profile_id: String, selected: Vec<String>) -> Result<CkeyImportResult, String> {
    let account_key = account_key_of(&profile_id)?;
    // Đích import: provider đang gắn tài khoản này (id chuẩn nếu chưa gắn gì).
    let ckey_cfg = CkeyConfig::load()?;
    let provider_id = opencode_manager::ckey::resolve_import_target(&ckey_cfg, &profile_id);
    let presets = presets();
    let (mut config, mut auth) = load_merged(&presets)?;

    // Metadata (giá, giới hạn) từ catalogue TOÀN CỤC — không phụ thuộc tài
    // khoản; cache dùng chung nên import lại không đập API.
    let models = models_catalog(false)?;
    // AI key active của profile để điền cho provider (keys là dữ liệu TÀI
    // KHOẢN — lấy theo đúng profile import).
    let ai_keys: Vec<CkeyAiKey> = match cache_get::<Vec<CkeyAiKey>>(&cache_key(&account_key, "keys", ""), TTL_KEYS) {
        Some(k) => k,
        None => {
            let list = block_on(async { CkeyClient::new(CKEY_MANAGE_API_BASE).fetch_keys(&account_key).await })?;
            cache_put(&cache_key(&account_key, "keys", ""), &list);
            list
        }
    };

    let mut provider_created = false;
    if !config.provider.contains_key(&provider_id) {
        let api_key = ai_keys
            .iter()
            .find(|k| k.is_active)
            .map(|k| k.api_key.clone())
            .unwrap_or_default();
        config.provider.insert(
            provider_id.clone(),
            Provider {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: "CKey (ckey.vn)".to_string(),
                options: ProviderOptions {
                    base_url: CKEY_LLM_BASE_URL.to_string(),
                    api_key,
                    ..Default::default()
                },
                models: Default::default(),
                ..Default::default()
            },
        );
        provider_created = true;
    }

    let mut added = 0usize;
    let mut removed = 0usize;
    let mut kept = 0usize;

    {
        let provider = config
            .provider
            .get_mut(&provider_id)
            .expect("provider tồn tại hoặc vừa được tạo");

        // Xoá model không còn được chọn.
        let to_remove: Vec<String> = provider
            .models
            .keys()
            .filter(|id| !selected.contains(id))
            .cloned()
            .collect();
        for id in to_remove {
            provider.models.remove(&id);
            removed += 1;
        }

        for id in &selected {
            if provider.models.contains_key(id) {
                kept += 1;
                continue;
            }
            // Giới hạn lấy từ API; 0 = CKey không khai báo → bỏ trống thay vì
            // ghi số bịa (OpenCode sẽ tự dùng mặc định của model).
            let meta = models.iter().find(|m| &m.public_name == id);
            let limit = meta.and_then(|m| {
                let context = (m.context_tokens_limit > 0).then_some(m.context_tokens_limit);
                let output = (m.max_output_tokens_limit > 0).then_some(m.max_output_tokens_limit);
                (context.is_some() || output.is_some()).then_some(ModelLimit {
                    context,
                    input: None,
                    output,
                })
            });

            provider.models.insert(
                id.clone(),
                ModelEntry {
                    name: meta
                        .map(|m| m.display_name.clone())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| id.clone()),
                    limit,
                    modalities: Some(ModelModalities {
                        input: infer_input_modalities(id),
                        output: vec!["text".to_string()],
                    }),
                    ..Default::default()
                },
            );
            added += 1;
        }

        // Provider không có khoá thì mọi request LLM đều lỗi — và nếu provider
        // vừa được chuyển sang tài khoản khác (rebinding) thì khoá cũ là của
        // tài khoản cũ: luôn đồng bộ theo AI key active của profile import.
        if let Some(k) = ai_keys.iter().find(|k| k.is_active) {
            provider.options.api_key = k.api_key.clone();
        }
    }

    save_split(&config, &mut auth, &presets)?;

    // Gắn binding provider → profile (ghi SAU save_split để file ckey.json
    // không bị đụng bởi save opencode.json — hai file độc lập).
    let mut ckey_cfg = CkeyConfig::load()?;
    ckey_cfg.bindings.insert(provider_id.clone(), profile_id.clone());
    ckey_cfg.active = Some(profile_id);
    ckey_cfg.save()?;

    Ok(CkeyImportResult {
        added,
        removed,
        kept,
        provider_created,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nhan_dien_provider_ckey_theo_endpoint() {
        assert!(is_ckey_provider(CKEY_LLM_BASE_URL));
        // Path thừa vẫn nhận ra sau khi chuẩn hoá.
        assert!(is_ckey_provider("https://api.xah.io/v1/chat/completions"));
        assert!(!is_ckey_provider("https://api.openai.com/v1"));
        assert!(!is_ckey_provider(""));
    }

    #[test]
    fn suy_dien_modalities_chi_khi_ro_rang() {
        // Có từ khoá rõ ràng → nhận ảnh.
        for id in ["gpt-4-vision", "qwen2-vl-7b", "gemini-omni", "image-gen-1"] {
            assert!(
                infer_input_modalities(id).contains(&"image".to_string()),
                "'{id}' phải nhận ảnh"
            );
        }
        // Không có dấu hiệu → chỉ text. Đoán bừa sẽ làm request lỗi.
        for id in ["gpt-4", "claude-3-haiku", "deepseek-chat"] {
            assert_eq!(
                infer_input_modalities(id),
                vec!["text".to_string()],
                "'{id}' phải chỉ có text"
            );
        }
    }

    #[test]
    fn suy_dien_modalities_khong_phan_biet_hoa_thuong() {
        assert!(infer_input_modalities("GPT-4-VISION").contains(&"image".to_string()));
        assert!(infer_input_modalities("Qwen2-VL-7B").contains(&"image".to_string()));
    }

    /// Cache: put → get trong TTL trả đúng dữ liệu; key khác không dính nhau.
    #[test]
    fn cache_put_get_va_isolation_theo_key() {
        let k1 = cache_key("account-a", "models", "");
        let k2 = cache_key("account-b", "models", "");
        assert_ne!(k1, k2, "hai account khác nhau phải ra key khác");

        cache_put(&k1, &vec!["m1", "m2"]);
        let got: Option<Vec<String>> = cache_get(&k1, TTL_MODELS);
        assert_eq!(got, Some(vec!["m1".to_string(), "m2".to_string()]));

        // Key khác (account khác) → miss.
        let miss: Option<Vec<String>> = cache_get(&k2, TTL_MODELS);
        assert!(miss.is_none());

        // TTL 0 → luôn hết hạn.
        let expired: Option<Vec<String>> = cache_get(&k1, Duration::ZERO);
        assert!(expired.is_none());
    }

    /// Cache bị xoá khi vượt số entry cho phép (không phình vô hạn).
    #[test]
    fn cache_khon_gioi_han_entry() {
        let base = cache_key("acct", "usage", "");
        for i in 0..(CACHE_MAX_ENTRIES + 5) {
            cache_put(&format!("{base}-{i}"), &i);
        }
        // Đã vượt giới hạn → cache bị clear, entry đầu không còn.
        let gone: Option<i32> = cache_get(&format!("{base}-0"), TTL_LIST);
        assert!(gone.is_none(), "cache phải bị dọn khi vượt giới hạn");
    }
}
