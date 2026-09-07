/*
[INTEGRITY NOTES]
- Mục đích: API quản lý AI Provider — liệt kê, thêm, sửa, xoá, kiểm tra kết nối,
  quét model (kèm phát hiện model "chết"), và dọn provider lỗi.
- Trách nhiệm: Mỗi command tự nạp cấu hình từ đĩa, sửa, rồi ghi lại. Không giữ
  state giữa các lần gọi — tránh trạng thái trong RAM lệch với file khi người
  dùng sửa file bằng tay hoặc chạy song song TUI.
- Tương tác: `core::store` (gộp/tách config), crate `opencode_manager` (định
  dạng file + client API), frontend `store/useProviderStore.ts`.
*/

use crate::core::store::{build_views, detect_duplicate, is_builtin, load_merged, save_split, unique_id, ProviderView};
use opencode_manager::api::{ApiClient, ApiStatus};
use opencode_manager::app::{App, DynamicPreset};
use opencode_manager::config::{normalize_base_url, Interleaved, ModelEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Chạy một future tới khi xong trên runtime đa luồng.
///
/// Command của Tauri ở đây là đồng bộ (dễ suy luận, không phải quản lý task),
/// còn `ApiClient` là async → cần runtime cục bộ. Đa luồng để
/// `test_all_providers` chạy song song được.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("không tạo được tokio runtime")
        .block_on(fut)
}

fn presets() -> Vec<DynamicPreset> {
    App::load_dynamic_presets()
}

/// Trạng thái kết nối, dạng phẳng để frontend không phải xử lý enum lồng.
#[derive(Debug, Clone, Serialize)]
pub struct StatusView {
    pub provider_id: String,
    /// "alive" | "no_credits" | "invalid_key" | "offline"
    pub kind: String,
    pub message: String,
}

fn status_view(provider_id: &str, status: &ApiStatus) -> StatusView {
    let (kind, message) = match status {
        ApiStatus::Alive => ("alive", String::new()),
        ApiStatus::InsufficientCredits(m) => ("no_credits", m.clone()),
        ApiStatus::InvalidKey(m) => ("invalid_key", m.clone()),
        ApiStatus::Offline(m) => ("offline", m.clone()),
    };
    StatusView {
        provider_id: provider_id.to_string(),
        kind: kind.to_string(),
        message,
    }
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_providers() -> Result<Vec<ProviderView>, String> {
    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    Ok(build_views(&config, &presets))
}

/// Danh sách preset để chọn khi thêm provider.
#[derive(Debug, Clone, Serialize)]
pub struct PresetView {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub id_prefix: String,
    pub npm: Option<String>,
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_presets() -> Result<Vec<PresetView>, String> {
    Ok(presets()
        .into_iter()
        .map(|p| PresetView {
            id: p.id,
            name: p.name,
            base_url: p.base_url,
            id_prefix: p.id_prefix,
            npm: p.npm,
        })
        .collect())
}

/// Lấy API key THẬT của một provider — chỉ dùng khi mở form sửa.
///
/// Tách riêng khỏi `list_providers` (chỉ trả key đã che) để key không nằm sẵn
/// trong state của frontend suốt phiên làm việc.
#[tauri::command(rename_all = "snake_case")]
pub fn get_provider_secret(provider_id: String) -> Result<String, String> {
    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    config
        .provider
        .get(&provider_id)
        .map(|p| p.options.api_key.clone())
        .ok_or_else(|| format!("Không tìm thấy provider: {provider_id}"))
}

/// Kết quả lưu provider. `duplicate_of` != None nghĩa là CHƯA lưu — frontend
/// phải hỏi người dùng có gộp vào provider trùng hay không.
#[derive(Debug, Clone, Serialize)]
pub struct SaveResult {
    pub saved_id: Option<String>,
    pub duplicate_of: Option<DuplicateInfo>,
    /// Base URL sau khi tự sửa (nếu khác bản người dùng nhập).
    pub normalized_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateInfo {
    pub id: String,
    pub name: String,
}

/// Ký tự an toàn cho id provider do người dùng tự đặt: chữ, số, `_`, `-`.
///
/// `/` phá tham chiếu `provider/model` trong opencode (`model` field và UI
/// picker tách theo `/`); khoảng trắng/ký tự lạ phá CLI và file config. Id
/// sinh tự động (preset prefix, `custom_2`, tên random bulk) đều đã thuộc bộ
/// này nên chỉ cần chặn id người dùng gõ tay.
pub fn validate_custom_id(id: &str) -> Result<(), String> {
    let n = id.chars().count();
    if !(1..=64).contains(&n) {
        return Err("ID provider phải dài 1–64 ký tự.".to_string());
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("ID provider chỉ được chứa chữ, số, '_' và '-' (cấm khoảng trắng, '/', '.', ...)".to_string());
    }
    Ok(())
}

/// Thêm hoặc sửa provider.
///
/// `provider_id` rỗng = thêm mới. `force_overwrite_id` = id của provider trùng
/// mà người dùng đã đồng ý gộp vào. `npm` = package AI SDK do người dùng chọn
/// (docs troubleshooting: `/v1/chat/completions` dùng `@ai-sdk/openai-compatible`,
/// `/v1/responses` dùng `@ai-sdk/openai`); để trống = theo preset, cuối cùng
/// mặc định openai-compatible.
///
/// `custom_id` = id do người dùng TỰ ĐẶT (thay vì tự sinh `custom`, `custom_2`…):
/// - Thêm mới: dùng làm id luôn (báo lỗi nếu trùng id đang có — không tự tăng
///   hậu tố vì id là lựa chọn có chủ đích).
/// - Sửa: khác id hiện tại = ĐỔI TÊN provider (giữ nguyên models), id cũ bị
///   thay thế; trùng id hiện tại = giữ nguyên.
/// - Gộp vào provider trùng: id của provider trùng thắng — không đổi id cùng
///   lúc gộp (báo lỗi nếu người dùng cố).
/// Id phải duy nhất và qua `validate_custom_id`. Đặt trùng id của preset +
/// đúng URL preset thì provider thành built-in (key lưu ở auth.json) — cùng
/// cơ chế tự động như id sinh từ preset.
#[tauri::command(rename_all = "snake_case")]
pub fn save_provider(
    provider_id: String,
    preset_id: String,
    name: String,
    base_url: String,
    api_key: String,
    force_overwrite_id: Option<String>,
    npm: Option<String>,
    custom_id: Option<String>,
) -> Result<SaveResult, String> {
    let name = name.trim().to_string();
    let raw_url = base_url.trim().to_string();
    let api_key = api_key.trim().to_string();
    // Tự sửa base_url nhập thừa path (vd `.../v1/chat/completions` → `.../v1`).
    let base_url = normalize_base_url(&raw_url);

    if name.is_empty() || base_url.is_empty() || api_key.is_empty() {
        return Err("Vui lòng nhập đầy đủ Tên, Base URL và API Key".to_string());
    }

    let presets = presets();
    let (mut config, mut auth) = load_merged(&presets)?;

    let editing_id = provider_id.trim().to_string();
    let normalized = (base_url != raw_url).then(|| base_url.clone());
    let custom_id = custom_id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    // Chưa xác nhận gộp → phát hiện trùng và trả về cho frontend hỏi.
    if force_overwrite_id.is_none() {
        let skip = (!editing_id.is_empty()).then_some(editing_id.as_str());
        if let Some((dup_id, dup_name)) = detect_duplicate(&config, &base_url, &api_key, skip) {
            return Ok(SaveResult {
                saved_id: None,
                duplicate_of: Some(DuplicateInfo {
                    id: dup_id,
                    name: dup_name,
                }),
                normalized_base_url: normalized,
            });
        }
    }

    // Id do người dùng đặt khác id đang sửa: kiểm tra ngay trước khi dùng.
    // (Bằng id đang sửa thì coi như "không đổi" — id cũ có thể chứa ký tự ngoài
    // bộ an toàn do app khác tạo, không được chặn người dùng chỉ vì sửa tên.)
    if let Some(cid) = custom_id.as_deref().filter(|c| *c != editing_id) {
        validate_custom_id(cid)?;
        if let Some(existing) = config.provider.get(cid) {
            return Err(format!(
                "ID '{cid}' đã được dùng bởi provider '{}' — chọn id khác.",
                existing.name
            ));
        }
    }

    // Xác định id đích.
    let merging = force_overwrite_id.as_ref().is_some_and(|s| !s.is_empty());
    let target_id = if merging {
        let dup_id = force_overwrite_id.clone().unwrap_or_default();
        // Gộp = "đây chính là provider trùng" → id của nó thắng.
        if let Some(cid) = custom_id.as_deref().filter(|c| *c != dup_id) {
            return Err(format!(
                "Không thể đặt ID '{cid}' khi gộp vào provider trùng '{dup_id}' — bỏ trống ô ID để gộp."
            ));
        }
        dup_id
    } else if !editing_id.is_empty() {
        // Sửa: đổi id nếu người dùng đặt id mới, không thì giữ nguyên.
        custom_id.clone().unwrap_or_else(|| editing_id.clone())
    } else {
        // Thêm mới: id tự đặt, hoặc sinh theo preset (khớp id+URL preset →
        // key vào `auth.json` như native provider).
        match &custom_id {
            Some(cid) => cid.clone(),
            None => {
                let preset = presets.iter().find(|p| p.id == preset_id);
                match preset {
                    Some(p)
                        if p.id != "custom"
                            && normalize_base_url(&p.base_url) == base_url
                            && !config.provider.contains_key(&p.id) =>
                    {
                        p.id.clone()
                    }
                    Some(p) => unique_id(&config, &p.id_prefix),
                    None => unique_id(&config, "custom"),
                }
            }
        }
    };

    let npm = npm
        .filter(|s| !s.trim().is_empty())
        .or_else(|| presets.iter().find(|p| p.id == preset_id).and_then(|p| p.npm.clone()))
        .or_else(|| Some("@ai-sdk/openai-compatible".to_string()));

    // Lấy entry hiện có để GIỮ models, whitelist/blacklist và headers qua lần
    // lưu này (case `hy3`: sửa URL/khoá/id không được làm mất cấu hình model).
    // Gỡ entry đang sửa trước để đổi id không để lại "bóng ma" id cũ; entry ở
    // id đích (gộp/giữ nguyên) lấy sau cùng. Gộp thì entry ĐÍCH thắng (models
    // của provider trùng được giữ), sửa/đổi id thì entry đang sửa thắng.
    let editing_entry = (!editing_id.is_empty())
        .then(|| config.provider.remove(&editing_id))
        .flatten();
    let target_entry = config.provider.remove(&target_id);
    let old = if merging {
        target_entry.or(editing_entry)
    } else {
        editing_entry.or(target_entry)
    };
    if merging && !editing_id.is_empty() && editing_id != target_id {
        // Provider đang sửa bị gộp vào provider trùng → xoá key auth cũ để nó
        // không "sống lại" ở lần nạp sau.
        auth.remove(&editing_id);
    }

    let mut provider = old.unwrap_or_default();
    provider.npm = npm;
    provider.name = name;
    provider.options.base_url = base_url;
    provider.options.api_key = api_key;

    config.provider.insert(target_id.clone(), provider);

    save_split(&config, &mut auth, &presets)?;

    Ok(SaveResult {
        saved_id: Some(target_id),
        duplicate_of: None,
        normalized_base_url: normalized,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_provider(provider_id: String) -> Result<(), String> {
    let presets = presets();
    let (mut config, mut auth) = load_merged(&presets)?;

    if config.provider.remove(&provider_id).is_none() {
        return Err(format!("Không tìm thấy provider: {provider_id}"));
    }
    // Xoá luôn ở auth.json, nếu không `merge_auth_into_providers` sẽ dựng lại
    // provider này ở lần nạp sau — người dùng tưởng xoá không có tác dụng.
    auth.remove(&provider_id);

    save_split(&config, &mut auth, &presets)?;
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub fn test_provider(provider_id: String) -> Result<StatusView, String> {
    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    let p = config
        .provider
        .get(&provider_id)
        .ok_or_else(|| format!("Không tìm thấy provider: {provider_id}"))?;

    let client = ApiClient::new();
    let status = block_on(client.test_api(&p.options.base_url, &p.options.api_key));
    Ok(status_view(&provider_id, &status))
}

/// Kiểm tra kết nối một cặp URL/key CHƯA lưu (nút "Kiểm tra" trong form).
#[tauri::command(rename_all = "snake_case")]
pub fn test_connection(base_url: String, api_key: String) -> Result<StatusView, String> {
    if base_url.trim().is_empty() || api_key.trim().is_empty() {
        return Err("Cần cả Base URL và API Key để kiểm tra".to_string());
    }
    let client = ApiClient::new();
    let status = block_on(client.test_api(&normalize_base_url(&base_url), &api_key));
    Ok(status_view("", &status))
}

#[tauri::command(rename_all = "snake_case")]
pub fn test_all_providers() -> Result<Vec<StatusView>, String> {
    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;

    let targets: Vec<(String, String, String)> = config
        .provider
        .iter()
        .map(|(id, p)| (id.clone(), p.options.base_url.clone(), p.options.api_key.clone()))
        .collect();

    // Chạy song song: kiểm tra tuần tự 10 provider với timeout 6s mỗi cái là
    // tối đa 60s treo UI. Mỗi task dựng client riêng để không phải chia sẻ
    // tham chiếu qua biên task.
    let mut out = block_on(async move {
        let mut set = tokio::task::JoinSet::new();
        for (id, url, key) in targets {
            set.spawn(async move {
                let client = ApiClient::new();
                let status = client.test_api(&url, &key).await;
                status_view(&id, &status)
            });
        }
        let mut results = Vec::new();
        while let Some(res) = set.join_next().await {
            match res {
                Ok(v) => results.push(v),
                // Task panic: báo thành offline thay vì mất provider khỏi kết quả
                // (thiếu dòng sẽ làm UI tưởng provider đó chưa kiểm tra).
                Err(e) => eprintln!("[provider] task kiểm tra thất bại: {e}"),
            }
        }
        results
    });

    out.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
    Ok(out)
}

/// Một model sau khi quét.
#[derive(Debug, Clone, Serialize)]
pub struct ScannedModel {
    pub id: String,
    /// Đã có trong config → mặc định tick sẵn.
    pub in_config: bool,
    /// Có trong config nhưng provider KHÔNG còn trả về nữa (model "chết").
    /// Đây là thứ bản TUI từng bỏ sót khiến config tích luỹ model không tồn tại.
    pub stale: bool,
    /// Khả năng model đang lưu trong config (nếu có) — để UI sửa lại thay vì
    /// mất khi lưu (case `hy3`: thiếu `tool_call`/`reasoning`/`interleaved` làm
    /// OpenCode gửi request sai shape cho model reasoning).
    pub caps: Option<ScannedModelCaps>,
}

/// Dạng phẳng của phần capability trong `ModelEntry`, cho frontend.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ScannedModelCaps {
    pub tool_call: Option<bool>,
    pub reasoning: Option<bool>,
    /// Tên field reasoning (vd "reasoning_content").
    pub interleaved: Option<String>,
}

fn caps_of_entry(entry: &ModelEntry) -> Option<ScannedModelCaps> {
    let interleaved = match &entry.interleaved {
        // Chỉ 2 dạng có "field" mới dịch sang chuỗi để sửa; `true` thuần giữ
        // nguyên bên dưới (không hiện nhưng cũng không bị ghi đè).
        Some(Interleaved::Object { field }) => Some(field.clone()),
        Some(Interleaved::Field(f)) => Some(f.clone()),
        _ => None,
    };
    (entry.tool_call.is_some() || entry.reasoning.is_some() || interleaved.is_some()).then(|| ScannedModelCaps {
        tool_call: entry.tool_call,
        reasoning: entry.reasoning,
        interleaved,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn scan_provider_models(provider_id: String) -> Result<Vec<ScannedModel>, String> {
    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    let p = config
        .provider
        .get(&provider_id)
        .ok_or_else(|| format!("Không tìm thấy provider: {provider_id}"))?;

    // Ưu tiên quét API thật (cặp URL/key của provider). Lỗi mạng/key → dùng
    // catalogue models.dev của provider này (opencode cũng quét từ đó cho
    // provider built-in) thay vì trơ ra: người dùng vẫn chọn được model,
    // đặc biệt cho provider BUILT-IN key-only vốn không có model trong
    // opencode.json của manager.
    let client = ApiClient::new();
    let fetched: Vec<String> = match block_on(client.fetch_models(&p.options.base_url, &p.options.api_key)) {
        Ok(list) => list,
        Err(api_err) => {
            // Danh sách ĐẦY ĐỦ của provider (không lọc declared): lọc sẽ biến
            // model đã chọn thành "stale" và đồng bộ sau đó XOÁ nó khỏi config.
            let catalog = opencode_manager::model_knowledge::catalog_models_by_provider()
                .get(provider_id.as_str())
                .cloned()
                .unwrap_or_default();
            if catalog.is_empty() {
                // Catalogue không có provider này → lỗi API là lỗi thật.
                return Err(api_err);
            }
            catalog.into_iter().map(|(mid, _)| mid).collect()
        }
    };

    let mut out: Vec<ScannedModel> = fetched
        .iter()
        .map(|id| {
            let entry = p.models.get(id);
            ScannedModel {
                id: id.clone(),
                in_config: entry.is_some(),
                stale: false,
                caps: entry.and_then(caps_of_entry),
            }
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));

    // Model có trong config nhưng provider/catalogue không còn → đưa xuống
    // cuối, đánh dấu stale để UI cảnh báo và người dùng quyết định giữ hay xoá.
    let mut stale: Vec<ScannedModel> = p
        .models
        .iter()
        .filter(|(id, _)| !fetched.contains(id))
        .map(|(id, entry)| ScannedModel {
            id: id.clone(),
            in_config: true,
            stale: true,
            caps: caps_of_entry(entry),
        })
        .collect();
    stale.sort_by(|a, b| a.id.cmp(&b.id));
    out.extend(stale);

    Ok(out)
}

/// Khả năng model do người dùng đặt trên UI (dạng phẳng của capability trong
/// `ModelEntry`). Field `None` = "không đổi", `Some` = ghi đè.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModelCaps {
    pub tool_call: Option<bool>,
    pub reasoning: Option<bool>,
    /// Tên field reasoning (vd "reasoning_content"); rỗng = bỏ qua.
    pub interleaved: Option<String>,
}

fn apply_caps(entry: &mut ModelEntry, caps: Option<&ModelCaps>) {
    let Some(c) = caps else { return };
    if let Some(v) = c.tool_call {
        entry.tool_call = Some(v);
    }
    if let Some(v) = c.reasoning {
        entry.reasoning = Some(v);
    }
    if let Some(field) = c.interleaved.as_deref() {
        let field = field.trim();
        if !field.is_empty() {
            entry.interleaved = Some(Interleaved::Object {
                field: field.to_string(),
            });
        }
    }
}

/// Ghi danh sách model được chọn vào config, kèm capability mỗi model.
///
/// `selected` là danh sách CUỐI CÙNG: model trong config mà không có trong đây
/// sẽ bị xoá. Nhờ vậy bỏ tick một model stale là nó biến mất khỏi config.
#[tauri::command(rename_all = "snake_case")]
pub fn set_provider_models(
    provider_id: String,
    selected: Vec<String>,
    caps: Option<HashMap<String, ModelCaps>>,
) -> Result<ProviderView, String> {
    let presets = presets();
    let (mut config, mut auth) = load_merged(&presets)?;

    {
        let p = config
            .provider
            .get_mut(&provider_id)
            .ok_or_else(|| format!("Không tìm thấy provider: {provider_id}"))?;

        let mut models: HashMap<String, ModelEntry> = HashMap::new();
        for id in &selected {
            // Giữ lại metadata (limit/modalities/caps) nếu model đã có trong config.
            let mut entry = p.models.get(id).cloned().unwrap_or(ModelEntry {
                name: id.clone(),
                limit: None,
                modalities: None,
                ..Default::default()
            });
            apply_caps(&mut entry, caps.as_ref().and_then(|m| m.get(id)));
            models.insert(id.clone(), entry);
        }
        p.models = models;
    }

    save_split(&config, &mut auth, &presets)?;

    let views = build_views(&config, &presets);
    views
        .into_iter()
        .find(|v| v.id == provider_id)
        .ok_or_else(|| "Không dựng được thông tin provider sau khi lưu".to_string())
}

/// Provider bị lỗi kết nối, dùng cho chức năng dọn nhanh.
#[derive(Debug, Clone, Serialize)]
pub struct BadProvider {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub message: String,
    pub is_builtin: bool,
}

/// Tìm provider KHÔNG hoạt động (sai key / hết tiền / offline) để đề xuất xoá.
#[tauri::command(rename_all = "snake_case")]
pub fn find_bad_providers() -> Result<Vec<BadProvider>, String> {
    let presets = presets();
    let (config, _auth) = load_merged(&presets)?;
    let statuses = test_all_providers()?;

    let mut out: Vec<BadProvider> = statuses
        .into_iter()
        .filter(|s| s.kind != "alive")
        .filter_map(|s| {
            config.provider.get(&s.provider_id).map(|p| BadProvider {
                id: s.provider_id.clone(),
                name: p.name.clone(),
                kind: s.kind,
                message: s.message,
                is_builtin: is_builtin(&s.provider_id, p, &presets),
            })
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Xoá nhiều provider một lượt (sau khi người dùng chọn trong danh sách dọn).
#[tauri::command(rename_all = "snake_case")]
pub fn delete_providers(provider_ids: Vec<String>) -> Result<usize, String> {
    if provider_ids.is_empty() {
        return Err("Chưa chọn provider nào để xoá".to_string());
    }
    let presets = presets();
    let (mut config, mut auth) = load_merged(&presets)?;

    let mut removed = 0usize;
    for id in &provider_ids {
        if config.provider.remove(id).is_some() {
            removed += 1;
        }
        auth.remove(id);
    }

    if removed == 0 {
        return Err("Không tìm thấy provider nào trong danh sách đã chọn".to_string());
    }

    save_split(&config, &mut auth, &presets)?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::validate_custom_id;

    #[test]
    fn custom_id_hop_le() {
        // Đủ dạng id thật trong thực tế: preset, custom_x, tên tự đặt.
        for id in ["custom_4", "agentrouter", "ckey", "A-b_9", "x"] {
            assert!(validate_custom_id(id).is_ok(), "'{id}' phải hợp lệ");
        }
        // Đúng biên 64 ký tự.
        assert!(validate_custom_id(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn custom_id_tu_choi_ky_tu_danger() {
        // `/` phá tham chiếu `provider/model`; khoảng trắng/dấu chấm/ký tự
        // có dấu phá CLI + JSON — đều phải chặn với thông báo rõ ràng.
        for id in ["", "a b", "a/b", "a.b", "tên-của-tôi", "a\tb"] {
            let err = validate_custom_id(id).unwrap_err();
            assert!(
                err.contains("chỉ được chứa") || err.contains("1–64"),
                "thông báo lạ cho '{id:?}': {err}"
            );
        }
        // Vượt biên độ dài.
        assert!(validate_custom_id(&"a".repeat(65)).is_err());
    }
}

/// Integration test cho cơ chế ID tự đặt — gọi thẳng `save_provider` với home
/// test cô lập (qua OPENCODE_TEST_HOME), không cần chạy app Tauri.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test_support::{isolate_home, TEST_ENV_LOCK};
    use opencode_manager::config::{AuthEntry, ModelEntry, ModelModalities, OpencodeConfig, Provider, ProviderOptions};
    use std::collections::HashMap;

    fn save(editing: &str, custom_id: Option<&str>, name: &str, url: &str, key: &str) -> Result<SaveResult, String> {
        save_provider(
            editing.to_string(),
            "custom".to_string(),
            name.to_string(),
            url.to_string(),
            key.to_string(),
            None,
            None,
            custom_id.map(str::to_string),
        )
    }

    fn seed_custom(id: &str) {
        let mut cfg = OpencodeConfig::load().unwrap();
        let mut models = HashMap::new();
        models.insert(
            "hy3".to_string(),
            ModelEntry {
                name: "hy3".to_string(),
                tool_call: Some(true),
                reasoning: Some(true),
                modalities: Some(ModelModalities {
                    input: vec!["text".to_string()],
                    output: vec!["text".to_string()],
                }),
                ..Default::default()
            },
        );
        cfg.provider.insert(
            id.to_string(),
            Provider {
                npm: None,
                name: format!("Provider {id}"),
                options: ProviderOptions {
                    base_url: "https://router.example.com/v1".to_string(),
                    api_key: "sk-old".to_string(),
                    headers: Some(HashMap::from([("X-Tag".to_string(), "keep".to_string())])),
                },
                models,
                whitelist: Some(vec!["allowed".to_string()]),
                blacklist: Some(vec!["hidden".to_string()]),
                ..Default::default()
            },
        );
        cfg.save().unwrap();
    }

    /// Seed preset động vào ~/.cache/opencode/models.json — cách app thật nạp
    /// preset, không phụ thuộc danh sách fallback hardcode.
    fn seed_preset(id: &str, name: &str, url: &str) {
        let home = opencode_manager::config::get_home_dir().unwrap();
        let cache = home.join(".cache").join("opencode");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(
            cache.join("models.json"),
            format!(
                r#"{{"p": {{"id": "{id}", "name": "{name}", "api": "{url}", "npm": "@ai-sdk/openai-compatible"}}}}"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn them_moi_voi_id_tu_dat() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        isolate_home("custom_id_add");

        let r = save(
            "",
            Some("agentrouter"),
            "Agent Router",
            "https://router.example.com/v1",
            "sk-1",
        )
        .unwrap();
        assert_eq!(
            r.saved_id.as_deref(),
            Some("agentrouter"),
            "phải dùng id người dùng đặt"
        );

        let cfg = OpencodeConfig::load().unwrap();
        assert_eq!(cfg.provider["agentrouter"].name, "Agent Router");
        assert_eq!(cfg.provider["agentrouter"].options.api_key, "sk-1");
    }

    #[test]
    fn id_tu_dat_trung_bao_loi_khong_tu_tang_hau_to() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        isolate_home("custom_id_dup");

        save("", Some("agentrouter"), "A", "https://router.example.com/v1", "sk-1").unwrap();

        // Id trùng là lựa chọn có chủ đích → báo lỗi rõ ràng, KHÔNG tự sinh
        // "agentrouter_2" rồi im lặng.
        let err = save("", Some("agentrouter"), "B", "https://other.example.com/v1", "sk-2").unwrap_err();
        assert!(err.contains("đã được dùng"), "lỗi: {err}");

        // Không ghi gì thêm — config vẫn y nguyên.
        let cfg = OpencodeConfig::load().unwrap();
        assert_eq!(cfg.provider.len(), 1);
    }

    #[test]
    fn id_tu_dat_ky_tu_danger_bi_chan() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        isolate_home("custom_id_charset");

        // `/` phá tham chiếu provider/model; khoảng trắng phá file/CLI.
        for bad in ["agent/router", "agent router", "tên-tôi", "a.b"] {
            let err = save("", Some(bad), "A", "https://router.example.com/v1", "sk-1").unwrap_err();
            assert!(
                err.contains("chỉ được chứa") || err.contains("1–64"),
                "id '{bad}' phải bị chặn, được: {err}"
            );
        }
        // Không có provider nào được tạo từ các lần lỗi.
        let cfg = OpencodeConfig::load().unwrap();
        assert!(cfg.provider.is_empty());
    }

    #[test]
    fn doi_id_giu_nguyen_models_headers_va_list() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        isolate_home("custom_id_rename");

        seed_custom("custom_4");

        // Đổi id custom_4 → agentrouter, đổi luôn khoá.
        let r = save(
            "custom_4",
            Some("agentrouter"),
            "TEMP",
            "https://router.example.com/v1",
            "sk-new",
        )
        .unwrap();
        assert_eq!(r.saved_id.as_deref(), Some("agentrouter"));

        let cfg = OpencodeConfig::load().unwrap();
        // Id cũ biến mất hoàn toàn — không còn "bóng ma" trùng lặp.
        assert!(!cfg.provider.contains_key("custom_4"), "id cũ phải bị thay thế");
        let p = &cfg.provider["agentrouter"];
        // Toàn bộ cấu hình model phải sống sót qua lần đổi id (case hy3).
        let hy3 = &p.models["hy3"];
        assert_eq!(hy3.tool_call, Some(true));
        assert_eq!(hy3.reasoning, Some(true));
        assert_eq!(p.options.api_key, "sk-new");
        assert_eq!(
            p.options.headers.as_ref().and_then(|h| h.get("X-Tag").cloned()),
            Some("keep".to_string()),
            "headers bị mất khi sửa"
        );
        assert_eq!(p.whitelist, Some(vec!["allowed".to_string()]));
        assert_eq!(p.blacklist, Some(vec!["hidden".to_string()]));
    }

    #[test]
    fn doi_id_built_in_di_chuyen_khoi_auth_json() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        isolate_home("custom_id_rename_builtin");

        seed_preset("openai", "OpenAI", "https://api.openai.com/v1");
        // Provider built-in sống trong auth.json (chưa có gì trong opencode.json).
        let mut auth = HashMap::new();
        auth.insert(
            "openai".to_string(),
            AuthEntry {
                auth_type: "api".to_string(),
                key: "sk-live".to_string(),
            },
        );
        AuthEntry::save_config(&auth).unwrap();

        // Đổi tên provider built-in → thành custom: key phải chuyển sang
        // opencode.json, entry auth cũ phải bị dọn (không sống lại).
        let r = save(
            "openai",
            Some("myopenai"),
            "My OpenAI",
            "https://api.openai.com/v1",
            "sk-live",
        )
        .unwrap();
        assert_eq!(r.saved_id.as_deref(), Some("myopenai"));

        let cfg = OpencodeConfig::load().unwrap();
        assert!(cfg.provider.contains_key("myopenai"));
        assert_eq!(
            cfg.provider["myopenai"].options.api_key, "sk-live",
            "key phải theo provider"
        );
        assert!(!cfg.provider.contains_key("openai"));

        let auth_after = AuthEntry::load_config().unwrap();
        assert!(
            !auth_after.contains_key("openai"),
            "entry auth cũ phải bị dọn, còn: {:?}",
            auth_after.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn sua_khong_doi_id_khong_can_hop_le_ky_tu() {
        let _guard = TEST_ENV_LOCK.lock().unwrap();
        isolate_home("custom_id_keep");

        // Provider id lạ do app khác tạo (chấm, ngoài bộ an toàn).
        let mut cfg = OpencodeConfig::load().unwrap();
        cfg.provider.insert(
            "weird.id".to_string(),
            Provider {
                npm: None,
                name: "Weird".to_string(),
                options: ProviderOptions {
                    base_url: "https://w.example.com/v1".to_string(),
                    api_key: "k".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        cfg.save().unwrap();

        // Sửa bình thường (ID giữ nguyên) phải được — không chặn người dùng
        // chỉ vì id cũ nằm ngoài bộ ký tự mới.
        let r = save(
            "weird.id",
            Some("weird.id"),
            "Weird 2",
            "https://w.example.com/v1",
            "k2",
        )
        .unwrap();
        assert_eq!(r.saved_id.as_deref(), Some("weird.id"));
    }
}
