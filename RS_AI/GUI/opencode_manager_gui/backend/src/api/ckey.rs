/*
[INTEGRITY NOTES]
- Mục đích: Tích hợp CKey (ckey.vn) — xem tài khoản, thống kê dùng, danh sách AI
  key/model, và import model vào cấu hình OpenCode.
- Trách nhiệm: Quản lý account key (lưu ở `~/.config/opencode/ckey.json`), gọi
  API CKey, và ghi model đã chọn vào provider CKey.
- Tương tác: crate `opencode_manager::ckey` (client + kiểu dữ liệu),
  `core::store` (gộp/tách config), frontend `pages/CkeyPage.tsx`.

Phân biệt hai loại khoá (rất dễ lẫn, lẫn là không gọi được API):
  - ACCOUNT KEY: lấy ở trang Profile ckey.vn, dùng để gọi API QUẢN LÝ
    (`https://ckey.vn/api/...`). Lưu trong `ckey.json`, KHÔNG vào opencode.json.
  - AI KEY (`ck-...`): dùng để gọi LLM qua `https://api.xah.io/v1`. Đây mới là
    khoá ghi vào provider trong cấu hình OpenCode.
*/

use crate::core::store::{load_merged, save_split};
use opencode_manager::app::DynamicPreset;
use opencode_manager::ckey::{
    CkeyAiKey, CkeyClient, CkeyModel, CkeyProfile, CkeyUsagePage, CkeyUsageStats, CKEY_LLM_BASE_URL,
    CKEY_MANAGE_API_BASE, CKEY_PRESET_ID,
};
use opencode_manager::config::{
    normalize_base_url, CkeyConfig, ModelEntry, ModelLimit, ModelModalities, Provider, ProviderOptions,
};
use serde::Serialize;

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
fn is_ckey_provider(base_url: &str) -> bool {
    normalize_base_url(base_url) == normalize_base_url(CKEY_LLM_BASE_URL)
}

/// Danh sách provider CKey kèm việc đã có account key hay chưa.
///
/// Khi chưa có provider nào dùng endpoint CKey, trả về một entry "ảo" theo
/// preset `ckey` để người dùng nhập account key rồi import model — nếu không,
/// trang CKey là ngõ cụt: không có gì để chọn, không tạo được provider.
#[derive(Debug, Clone, Serialize)]
pub struct CkeyProviderView {
    pub provider_id: String,
    pub name: String,
    pub has_account_key: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_ckey_providers() -> Result<Vec<CkeyProviderView>, String> {
    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    let ckey_cfg = CkeyConfig::load()?;

    let mut out: Vec<CkeyProviderView> = config
        .provider
        .iter()
        .filter(|(_, p)| is_ckey_provider(&p.options.base_url))
        .map(|(id, p)| CkeyProviderView {
            provider_id: id.clone(),
            name: p.name.clone(),
            has_account_key: ckey_cfg.accounts.contains_key(id),
        })
        .collect();

    // Ngõ cụt: chưa có provider CKey nào → đề xuất entry preset để bắt đầu
    // (import_ckey_models sẽ tự tạo provider cho id chuẩn này).
    if out.is_empty() {
        if let Some(p) = presets.iter().find(|p| p.id == CKEY_PRESET_ID) {
            out.push(CkeyProviderView {
                provider_id: p.id.clone(),
                name: p.name.clone(),
                has_account_key: ckey_cfg.accounts.contains_key(&p.id),
            });
        }
    }

    out.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    Ok(out)
}

/// Các account key đã lưu (đã che) — để người dùng gán lại cho provider khác mà
/// không phải nhập lại từ trang Profile.
#[derive(Debug, Clone, Serialize)]
pub struct CkeyAccountOption {
    pub provider_id: String,
    pub key_masked: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_ckey_accounts() -> Result<Vec<CkeyAccountOption>, String> {
    let cfg = CkeyConfig::load()?;
    let mut out: Vec<CkeyAccountOption> = cfg
        .accounts
        .iter()
        .map(|(pid, key)| CkeyAccountOption {
            provider_id: pid.clone(),
            key_masked: crate::core::store::mask_key(key),
        })
        .collect();
    out.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    Ok(out)
}

/// Gán account key cho một provider CKey.
/// `copy_from_provider_id` != None nghĩa là dùng lại key đã lưu của provider đó.
#[tauri::command(rename_all = "snake_case")]
pub fn set_ckey_account_key(
    provider_id: String,
    account_key: Option<String>,
    copy_from_provider_id: Option<String>,
) -> Result<(), String> {
    let mut cfg = CkeyConfig::load()?;

    let key = match (account_key, copy_from_provider_id) {
        (Some(k), _) if !k.trim().is_empty() => k.trim().to_string(),
        (_, Some(src)) => cfg
            .accounts
            .get(&src)
            .cloned()
            .ok_or_else(|| format!("Provider '{src}' chưa có account key để dùng lại"))?,
        _ => return Err("Vui lòng nhập account key hoặc chọn key đã lưu".to_string()),
    };

    cfg.accounts.insert(provider_id, key);
    cfg.save()?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_ckey_account_key(provider_id: String) -> Result<(), String> {
    let mut cfg = CkeyConfig::load()?;
    if cfg.accounts.remove(&provider_id).is_none() {
        return Err(format!("Provider '{provider_id}' chưa có account key"));
    }
    cfg.save()?;
    Ok(())
}

fn account_key_of(provider_id: &str) -> Result<String, String> {
    CkeyConfig::load()?.accounts.get(provider_id).cloned().ok_or_else(|| {
        format!("Provider '{provider_id}' chưa có account key CKey. Hãy nhập key từ trang Profile ckey.vn.")
    })
}

/// Toàn bộ thông tin tài khoản trong MỘT lần gọi.
///
/// Gộp 4 request (profile/stats/keys/models) vào một command và chạy song song:
/// gọi lần lượt từ frontend sẽ mất 4 lượt IPC và ~4× thời gian mạng.
#[derive(Debug, Clone, Serialize)]
pub struct CkeyDashboard {
    pub profile: Option<CkeyProfileView>,
    pub stats: Option<CkeyStatsView>,
    pub keys: Vec<CkeyKeyView>,
    pub models: Vec<CkeyModelView>,
    /// Lỗi của từng phần — một phần lỗi không được làm mất dữ liệu phần khác.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CkeyProfileView {
    pub username: String,
    pub name: String,
    pub email: String,
    pub balance: String,
    pub balance_raw: f64,
    pub created_at: String,
}

impl From<CkeyProfile> for CkeyProfileView {
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct CkeyKeyView {
    pub id: u64,
    pub key_name: String,
    pub key_prefix: String,
    pub is_active: bool,
    pub created_at_text: String,
    /// AI key đã che — không gửi key thật ra danh sách.
    pub key_masked: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CkeyModelView {
    pub public_name: String,
    pub display_name: String,
    pub input_price_per_million_vnd: f64,
    pub output_price_per_million_vnd: f64,
    pub context_tokens_limit: u64,
    pub max_output_tokens_limit: u64,
    pub cache_enabled: bool,
}

impl From<CkeyModel> for CkeyModelView {
    fn from(m: CkeyModel) -> Self {
        Self {
            public_name: m.public_name,
            display_name: m.display_name,
            input_price_per_million_vnd: m.input_price_per_million_vnd,
            output_price_per_million_vnd: m.output_price_per_million_vnd,
            context_tokens_limit: m.context_tokens_limit,
            max_output_tokens_limit: m.max_output_tokens_limit,
            cache_enabled: m.cache_enabled,
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn fetch_ckey_dashboard(provider_id: String) -> Result<CkeyDashboard, String> {
    let account_key = account_key_of(&provider_id)?;

    let (profile, stats, keys, models) = block_on(async move {
        let client = CkeyClient::new(CKEY_MANAGE_API_BASE);
        // 4 request độc lập → chạy đồng thời.
        tokio::join!(
            client.fetch_profile(&account_key),
            client.fetch_usage_stats(&account_key),
            client.fetch_keys(&account_key),
            client.fetch_models(&account_key),
        )
    });

    let mut errors = Vec::new();
    // Mỗi phần lỗi độc lập: hết tiền vẫn xem được danh sách model, và ngược lại.
    let profile = match profile {
        Ok(p) => Some(CkeyProfileView::from(p)),
        Err(e) => {
            errors.push(format!("Thông tin tài khoản: {e}"));
            None
        }
    };
    let stats = match stats {
        Ok(s) => Some(CkeyStatsView::from(s)),
        Err(e) => {
            errors.push(format!("Thống kê sử dụng: {e}"));
            None
        }
    };
    let keys = match keys {
        Ok(list) => list
            .into_iter()
            .map(|k: CkeyAiKey| CkeyKeyView {
                id: k.id,
                key_name: k.key_name,
                key_prefix: k.key_prefix,
                is_active: k.is_active,
                created_at_text: k.created_at_text,
                key_masked: crate::core::store::mask_key(&k.api_key),
            })
            .collect(),
        Err(e) => {
            errors.push(format!("Danh sách AI key: {e}"));
            Vec::new()
        }
    };
    let models = match models {
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

/// Lịch sử request, phân trang.
#[derive(Debug, Clone, Serialize)]
pub struct CkeyUsageView {
    pub items: Vec<CkeyUsageItemView>,
    pub page: u64,
    pub total_pages: u64,
}

#[derive(Debug, Clone, Serialize)]
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
pub fn fetch_ckey_usage(provider_id: String, page: u64, limit: u64) -> Result<CkeyUsageView, String> {
    let account_key = account_key_of(&provider_id)?;
    // Tham số theo docs: `key`, `page`, `limit` (không gửi tham số bịa).
    let page = page.max(1);
    let limit = limit.clamp(1, 100);

    let result: Result<CkeyUsagePage, String> = block_on(async move {
        let client = CkeyClient::new(CKEY_MANAGE_API_BASE);
        client.fetch_usage(&account_key, page, limit).await
    });

    let page_data = result?;
    let (p, total) = page_data
        .pagination
        .map(|pg| (pg.page, pg.total_pages))
        .unwrap_or((page, 1));

    Ok(CkeyUsageView {
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
    })
}

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

/// Model để import, kèm trạng thái so với cấu hình hiện tại.
#[derive(Debug, Clone, Serialize)]
pub struct CkeyImportItem {
    pub id: String,
    pub display_name: String,
    pub input_price: f64,
    pub output_price: f64,
    pub context_limit: u64,
    pub output_limit: u64,
    pub in_config: bool,
    /// Còn trong config nhưng CKey không còn cung cấp.
    pub stale: bool,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_ckey_import_items(provider_id: String) -> Result<Vec<CkeyImportItem>, String> {
    let account_key = account_key_of(&provider_id)?;
    let models = block_on(async move {
        let client = CkeyClient::new(CKEY_MANAGE_API_BASE);
        client.fetch_models(&account_key).await
    })?;

    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    let existing: Vec<String> = config
        .provider
        .get(&provider_id)
        .map(|p| p.models.keys().cloned().collect())
        .unwrap_or_default();

    let mut out: Vec<CkeyImportItem> = models
        .iter()
        .map(|m| CkeyImportItem {
            id: m.public_name.clone(),
            display_name: m.display_name.clone(),
            input_price: m.input_price_per_million_vnd,
            output_price: m.output_price_per_million_vnd,
            context_limit: m.context_tokens_limit,
            output_limit: m.max_output_tokens_limit,
            in_config: existing.contains(&m.public_name),
            stale: false,
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));

    // Model còn trong config nhưng CKey đã bỏ → hiện cuối, đánh dấu stale.
    let available: Vec<&String> = models.iter().map(|m| &m.public_name).collect();
    let mut stale: Vec<CkeyImportItem> = existing
        .iter()
        .filter(|id| !available.contains(id))
        .map(|id| CkeyImportItem {
            id: id.clone(),
            display_name: id.clone(),
            input_price: 0.0,
            output_price: 0.0,
            context_limit: 0,
            output_limit: 0,
            in_config: true,
            stale: true,
        })
        .collect();
    stale.sort_by(|a, b| a.id.cmp(&b.id));
    out.extend(stale);

    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct CkeyImportResult {
    pub added: usize,
    pub removed: usize,
    pub kept: usize,
    /// `true` nếu provider CKey vừa được tạo trong lần import này.
    pub provider_created: bool,
}

/// Ghi danh sách model đã chọn vào provider CKey.
///
/// `selected` là danh sách CUỐI CÙNG — model trong config mà không có trong đây
/// sẽ bị xoá (nhờ vậy bỏ tick model stale là nó biến mất khỏi cấu hình).
#[tauri::command(rename_all = "snake_case")]
pub fn import_ckey_models(provider_id: String, selected: Vec<String>) -> Result<CkeyImportResult, String> {
    let account_key = account_key_of(&provider_id)?;
    let presets = presets();
    let (mut config, mut auth) = load_merged(&presets)?;

    // Lấy metadata (giá, giới hạn) để ghi `limit` chính xác thay vì số cứng.
    let models = block_on(async move {
        let client = CkeyClient::new(CKEY_MANAGE_API_BASE);
        let keys = client.fetch_keys(&account_key).await.unwrap_or_default();
        let models = client.fetch_models(&account_key).await;
        (keys, models)
    });
    let (ai_keys, models) = models;
    let models = models?;

    let mut provider_created = false;
    if !config.provider.contains_key(&provider_id) {
        // Chỉ tự tạo provider cho id chuẩn của CKey. Id lạ mà không có provider
        // là dấu hiệu người dùng gọi sai, tạo bừa sẽ sinh cấu hình rác.
        if provider_id != CKEY_PRESET_ID {
            return Err(format!("Không tìm thấy provider: {provider_id}"));
        }
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

        // Provider không có khoá thì mọi request LLM đều lỗi — điền AI key active.
        if provider.options.api_key.trim().is_empty() {
            if let Some(k) = ai_keys.iter().find(|k| k.is_active) {
                provider.options.api_key = k.api_key.clone();
            }
        }
    }

    save_split(&config, &mut auth, &presets)?;
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
}
