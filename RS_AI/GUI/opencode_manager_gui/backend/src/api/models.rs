/*
[INTEGRITY NOTES]
- Mục đích: Ma trận so sánh model + chọn model CHÍNH (field `model` của
  opencode.json, dạng "provider_id/model_id").
- Trách nhiệm:
  + `list_model_matrix`: gộp mọi model trong config (đã merge auth) thành bảng
    so sánh — khả năng (tool/reasoning/vision), giới hạn (context/output), giá
    (USD/1M token). Làm giàu dữ liệu thiếu từ cache models.dev của opencode
    (`~/.cache/opencode/models.json`) — KHÔNG gọi mạng.
  + `set_primary_model`: ghi/xoá field `model` của opencode.json.
- Tương tác: `core::store` (gộp/tách config), frontend `pages/ModelsPage.tsx`
  (tab So sánh model) + `components/ModelsModal.tsx` (chọn model chính ngay
  trong form model của provider).

Độ ưu tiên dữ liệu: field người dùng TỰ SET trong opencode.json thắng; models.dev
chỉ điền chỗ trống — không ghi đè lựa chọn của người dùng.
*/

use crate::core::store::{load_merged, save_split};
use crate::ipc::{from_string, respond, Empty, IpcResult, Req};
use opencode_manager::app::App;
use opencode_manager::config::ModelEntry;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================
// CACHE models.dev (đọc, không ghi, không gọi mạng)
// ============================================================

/// Metadata một model từ cache models.dev của opencode.
///
/// Cache do opencode tự tải (`~/.cache/opencode/models.json`); manager đọc lại
/// để "biết" khả năng/giá của model phổ biến mà config không khai. Thiếu file
/// hoặc field lạ → mặc định trống (bảng so sánh vẫn dựng được từ config).
#[derive(Debug, Clone, Default)]
struct DevModelInfo {
    name: String,
    tool_call: Option<bool>,
    reasoning: Option<bool>,
    vision: Option<bool>,
    context: Option<u64>,
    output: Option<u64>,
    price_input: Option<f64>,
    price_output: Option<f64>,
    price_cache_read: Option<f64>,
}

use std::sync::Mutex;
use std::time::SystemTime;

struct DevModelsCache {
    mtime: SystemTime,
    len: u64,
    data: HashMap<String, HashMap<String, DevModelInfo>>,
}

static DEV_MODELS_CACHE: Mutex<Option<DevModelsCache>> = Mutex::new(None);

fn models_dev_cache_path() -> Option<PathBuf> {
    opencode_manager::config::get_home_dir().map(|home| home.join(".cache").join("opencode").join("models.json"))
}

/// Đọc cache models.dev → map (provider_id → model_key → info).
///
/// Model key trong cache là id ĐẦY ĐỦ ("qwen/qwen3-max" với aggregator) hoặc
/// id trần ("claude-sonnet-4-6" với provider gốc) — tra cứu thử cả hai dạng.
/// Có in-memory cache kiểm tra mtime + len để tránh đọc/parse lặp lại file 4.7MB.
fn load_dev_models() -> HashMap<String, HashMap<String, DevModelInfo>> {
    let mut out: HashMap<String, HashMap<String, DevModelInfo>> = HashMap::new();
    let Some(path) = models_dev_cache_path() else {
        return out;
    };
    let Ok(meta) = std::fs::metadata(&path) else {
        return out;
    };
    let Ok(mtime) = meta.modified() else {
        return out;
    };
    let len = meta.len();

    if let Ok(guard) = DEV_MODELS_CACHE.lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.mtime == mtime && entry.len == len {
                return entry.data.clone();
            }
        }
    }

    let Ok(text) = std::fs::read_to_string(&path) else {
        return out;
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) else {
        return out;
    };
    let Some(providers) = root.as_object() else { return out };

    for (pid, pv) in providers {
        let Some(models) = pv.get("models").and_then(|m| m.as_object()) else {
            continue;
        };
        let mut map = HashMap::new();
        for (mid, mv) in models {
            let get_bool = |k: &str| mv.get(k).and_then(|v| v.as_bool());
            let vision = mv
                .get("modalities")
                .and_then(|m| m.get("input"))
                .and_then(|i| i.as_array())
                .map(|items| items.iter().any(|x| x.as_str() == Some("image")));
            let limit = mv.get("limit");
            let cost = mv.get("cost");
            map.insert(
                mid.clone(),
                DevModelInfo {
                    name: mv.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    tool_call: get_bool("tool_call"),
                    reasoning: get_bool("reasoning"),
                    vision,
                    context: limit.and_then(|l| l.get("context")).and_then(|v| v.as_u64()),
                    output: limit.and_then(|l| l.get("output")).and_then(|v| v.as_u64()),
                    price_input: cost.and_then(|c| c.get("input")).and_then(|v| v.as_f64()),
                    price_output: cost.and_then(|c| c.get("output")).and_then(|v| v.as_f64()),
                    price_cache_read: cost.and_then(|c| c.get("cache_read")).and_then(|v| v.as_f64()),
                },
            );
        }
        out.insert(pid.clone(), map);
    }

    if let Ok(mut guard) = DEV_MODELS_CACHE.lock() {
        *guard = Some(DevModelsCache {
            mtime,
            len,
            data: out.clone(),
        });
    }

    out
}

/// Tra metadata models.dev cho (provider_id, model_id) — thử id trần rồi id
/// đầy đủ "pid/mid" (aggregator lưu key kèm prefix nhà cung cấp).
fn dev_lookup(
    dev: &HashMap<String, HashMap<String, DevModelInfo>>,
    provider_id: &str,
    model_id: &str,
) -> Option<DevModelInfo> {
    let models = dev.get(provider_id)?;
    models
        .get(model_id)
        .or_else(|| models.get(&format!("{provider_id}/{model_id}")))
        .cloned()
}

// ============================================================
// MA TRẬN SO SÁNH MODEL
// ============================================================

/// Một dòng bảng so sánh. `Option<bool>`: None = "chưa khai" (OpenCode sẽ dùng
/// mặc định của model); Some(v) = đã khai rõ.
#[derive(Debug, Clone, Serialize)]
pub struct ModelMatrixRow {
    pub provider_id: String,
    pub provider_name: String,
    pub model_id: String,
    pub display_name: String,
    /// Model chính (field `model` của opencode.json = "provider_id/model_id").
    pub is_primary: bool,
    pub tool_call: Option<bool>,
    pub reasoning: Option<bool>,
    /// Nhận ảnh (input modalities có "image").
    pub vision: Option<bool>,
    pub context: Option<u64>,
    pub output: Option<u64>,
    /// Giá USD / 1M token (từ cache models.dev).
    pub price_input: Option<f64>,
    pub price_output: Option<f64>,
    pub price_cache_read: Option<f64>,
    /// `true` nếu có ít nhất một field lấy từ models.dev.
    pub enriched: bool,
    /// Giới hạn theo models.dev (để đối chiếu/tooltip khi lệch config).
    pub dev_context: Option<u64>,
    pub dev_output: Option<u64>,
    /// Ước lượng theo TÊN model (heuristic — nguồn yếu nhất, chỉ điền chỗ
    /// trống; xem `model_knowledge`).
    pub heur_context: Option<u64>,
    pub heur_output: Option<u64>,
    /// Nguồn giá trị đang hiển thị: "config" | "models.dev" | "name" | "".
    pub context_source: &'static str,
    pub output_source: &'static str,
    /// CÙNG field có giá trị ở config VÀ models.dev mà LỆCH nhau — dấu hiệu
    /// limit mặc định cũ (thời hardcode 1048576/131072) còn sót trong
    /// opencode.json và đang CHE giá trị thật (config thắng models.dev).
    pub limit_conflict: bool,
}

/// Dựng ma trận (hàm thuần cho `list_model_matrix` và arbiter cùng dùng).
pub fn matrix_rows() -> Result<Vec<ModelMatrixRow>, String> {
    let presets = App::load_dynamic_presets();
    let (config, _auth) = load_merged(&presets)?;
    let dev = load_dev_models();

    // Field `model` = "provider_id/model_id" (tách ở "/" ĐẦU — model id có thể
    // chứa "/" tiếp theo, vd model của CKey dạng "vendor/tên model").
    let primary: Option<(&str, &str)> = config.model.as_deref().and_then(|s| s.split_once('/'));

    let mut rows: Vec<ModelMatrixRow> = Vec::new();
    let mut provider_ids: Vec<&String> = config.provider.keys().collect();
    provider_ids.sort();

    // Catalogue models.dev theo provider — cho provider BUILT-IN key-only
    // (model không nằm trong opencode.json; opencode tự quét catalogue khi
    // chạy). Manager liệt kê chúng vào ma trận để: đếm đúng, chọn làm AI
    // judge được, và tick chọn → ghi danh sách RÕ RÀNG vào opencode.json.
    let catalog = opencode_manager::model_knowledge::catalog_models_by_provider();

    for pid in provider_ids {
        let p = &config.provider[pid];
        let provider_name = if p.name.trim().is_empty() {
            pid.clone()
        } else {
            p.name.clone()
        };

        // Model hiển thị = đã khai trong config + (builtin key-only) catalogue
        // của provider này. Catalogue bỏ model đã khai (tránh đúp).
        let mut model_ids: Vec<String> = p.models.keys().cloned().collect();
        if p.models.is_empty() {
            if let Some(list) = catalog.get(pid.as_str()) {
                for (mid, _) in list {
                    if !model_ids.contains(mid) {
                        model_ids.push(mid.clone());
                    }
                }
            }
        }
        model_ids.sort();

        for mid in model_ids {
            // Model catalogue (chưa chọn) không có entry — coi như rỗng.
            let empty_entry = ModelEntry::default();
            let entry = p.models.get(&mid).unwrap_or(&empty_entry);
            let info = dev_lookup(&dev, pid, &mid);

            // Config người dùng set THẮNG metadata models.dev (chỉ điền chỗ trống).
            let tool_call = entry.tool_call.or(info.as_ref().and_then(|i| i.tool_call));
            let reasoning = entry.reasoning.or(info.as_ref().and_then(|i| i.reasoning));
            let vision = entry
                .modalities
                .as_ref()
                .map(|m| m.input.iter().any(|x| x == "image"))
                .or_else(|| info.as_ref().and_then(|i| i.vision));
            // Chuỗi ưu tiên HIỂN THỊ: config (người dùng) > models.dev >
            // suy đoán theo tên (heuristic — yếu nhất, chỉ điền chỗ trống).
            let cfg_ctx = entry.limit.as_ref().and_then(|l| l.context);
            let cfg_out = entry.limit.as_ref().and_then(|l| l.output);
            let dev_ctx = info.as_ref().and_then(|i| i.context);
            let dev_out = info.as_ref().and_then(|i| i.output);
            // Thử id TRƯỚC rồi tên hiển thị: id router thường mờ ("hy3") trong
            // khi display name lộ họ thật ("GPT-4o") — chỉ dùng cái đầu khớp.
            let entry_name = entry.name.trim().to_string();
            let dev_name = info.as_ref().map(|i| i.name.trim().to_string()).unwrap_or_default();
            let heur = opencode_manager::model_knowledge::infer_from_name(&mid)
                .or_else(|| {
                    (!entry_name.is_empty())
                        .then(|| opencode_manager::model_knowledge::infer_from_name(&entry_name))
                        .flatten()
                })
                .or_else(|| {
                    (!dev_name.is_empty())
                        .then(|| opencode_manager::model_knowledge::infer_from_name(&dev_name))
                        .flatten()
                });
            let heur_ctx = heur.as_ref().and_then(|h| h.context);
            let heur_out = heur.as_ref().and_then(|h| h.output);
            let (context, context_source) = if cfg_ctx.is_some() {
                (cfg_ctx, "config")
            } else if dev_ctx.is_some() {
                (dev_ctx, "models.dev")
            } else {
                (heur_ctx, if heur_ctx.is_some() { "name" } else { "" })
            };
            let (output, output_source) = if cfg_out.is_some() {
                (cfg_out, "config")
            } else if dev_out.is_some() {
                (dev_out, "models.dev")
            } else {
                (heur_out, if heur_out.is_some() { "name" } else { "" })
            };
            let price_input = info.as_ref().and_then(|i| i.price_input);
            let price_output = info.as_ref().and_then(|i| i.price_output);
            let price_cache_read = info.as_ref().and_then(|i| i.price_cache_read);
            let dev_context = dev_ctx;
            let dev_output = dev_out;
            // Chỉ là xung đột khi CÙNG field khai ở cả hai nơi mà lệch giá trị
            // (config khai một phía, models.dev điền phía còn lại là hợp lệ).
            let limit_conflict = (context.is_some_and(|c| dev_context.is_some_and(|d| c != d)))
                || (output.is_some_and(|o| dev_output.is_some_and(|d| o != d)));

            let enriched = info.is_some()
                && (tool_call != entry.tool_call
                    || reasoning != entry.reasoning
                    || vision != entry.modalities.as_ref().map(|m| m.input.iter().any(|x| x == "image"))
                    || context != entry.limit.as_ref().and_then(|l| l.context)
                    || output != entry.limit.as_ref().and_then(|l| l.output)
                    || price_input.is_some());

            let display_name = if entry.name.trim().is_empty() {
                info.as_ref()
                    .map(|i| i.name.trim().to_string())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| mid.clone())
            } else {
                entry.name.clone()
            };

            rows.push(ModelMatrixRow {
                provider_id: pid.clone(),
                provider_name: provider_name.clone(),
                model_id: mid.clone(),
                display_name,
                is_primary: primary
                    .map(|(pp, pm)| pp == pid.as_str() && pm == mid.as_str())
                    .unwrap_or(false),
                tool_call,
                reasoning,
                vision,
                context,
                output,
                price_input,
                price_output,
                price_cache_read,
                enriched,
                dev_context,
                dev_output,
                heur_context: heur_ctx,
                heur_output: heur_out,
                context_source,
                output_source,
                limit_conflict,
            });
        }
    }

    Ok(rows)
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_model_matrix(request: Req<Empty>) -> IpcResult<Vec<ModelMatrixRow>> {
    let (request_id, _) = request.validate()?;
    matrix_rows().map(|data| respond(request_id, data)).map_err(from_string)
}

#[derive(Deserialize)]
pub struct SetPrimaryModelRequest {
    #[serde(deserialize_with = "crate::ipc::present_nullable")]
    pub provider_id: Option<String>,
    #[serde(deserialize_with = "crate::ipc::present_nullable")]
    pub model_id: Option<String>,
}

// ============================================================
// MODEL CHÍNH (field `model` của opencode.json)
// ============================================================

/// Đặt model chính cho OpenCode — ghi `model: "provider_id/model_id"` vào
/// opencode.json. Cả hai để trống → XOÁ lựa chọn (OpenCode dùng mặc định).
///
/// Model phải đang có trong config: model chưa import mà đặt làm model chính
/// sẽ khiến OpenCode không resolve được khi khởi động.
#[tauri::command(rename_all = "snake_case")]
pub fn set_primary_model(request: Req<SetPrimaryModelRequest>) -> IpcResult<Option<String>> {
    let (request_id, payload) = request.validate()?;
    set_primary_model_inner(payload.provider_id, payload.model_id)
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

fn set_primary_model_inner(provider_id: Option<String>, model_id: Option<String>) -> Result<Option<String>, String> {
    let presets = App::load_dynamic_presets();
    let (mut config, mut auth) = load_merged(&presets)?;

    match (
        provider_id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        model_id.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
    ) {
        (None, None) => {
            config.model = None;
        }
        (Some(pid), Some(mid)) => {
            let p = config
                .provider
                .get(&pid)
                .ok_or_else(|| format!("Không tìm thấy provider '{pid}' trong cấu hình."))?;
            // Model hợp lệ: đã khai trong opencode.json, HOẶC model catalogue
            // của provider builtin key-only (opencode tự resolve; chọn model
            // catalogue làm model chính là cách dùng bình thường).
            let in_catalog = p.models.is_empty()
                && opencode_manager::model_knowledge::catalog_models_by_provider()
                    .get(pid.as_str())
                    .map(|list| list.iter().any(|(m, _)| *m == mid))
                    .unwrap_or(false);
            if !p.models.contains_key(&mid) && !in_catalog {
                return Err(format!(
                    "Model '{mid}' chưa có trong provider '{pid}' — đồng bộ model vào provider trước khi đặt làm model chính."
                ));
            }
            config.model = Some(format!("{pid}/{mid}"));
        }
        _ => {
            return Err("Cần cả provider và model, hoặc để trống cả hai để bỏ chọn model chính.".to_string());
        }
    }

    save_split(&config, &mut auth, &presets)?;
    Ok(config.model)
}

/// Đồng bộ giới hạn (context/output) của mọi model về giá trị models.dev.
///
/// Chỉ đụng model CÓ xung đột: config khai limit LỆCH models.dev (dấu hiệu
/// số mặc định cũ 1048576/131072 còn sót — thời manager hardcode limit cho
/// mọi model). Field `input` và mọi capability khác giữ nguyên.
/// Trả về số model đã sửa (0 = không có gì lệch).
#[tauri::command(rename_all = "snake_case")]
pub fn sync_limits_from_dev(request: Req<Empty>) -> IpcResult<usize> {
    let (request_id, _) = request.validate()?;
    sync_limits_from_dev_inner()
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

fn sync_limits_from_dev_inner() -> Result<usize, String> {
    let presets = App::load_dynamic_presets();
    let (mut config, mut auth) = load_merged(&presets)?;
    let dev = load_dev_models();

    let mut fixed = 0usize;
    for (pid, provider) in config.provider.iter_mut() {
        // Clone danh sách id trước khi mượn mutable để sửa entry.
        let mut model_ids: Vec<String> = provider.models.keys().cloned().collect();
        model_ids.sort();
        for mid in model_ids {
            let Some(info) = dev_lookup(&dev, pid, &mid) else {
                continue;
            };
            let entry = provider.models.get_mut(&mid).expect("key vừa clone");

            let ctx_conflict = entry
                .limit
                .as_ref()
                .and_then(|l| l.context)
                .is_some_and(|c| info.context.is_some_and(|d| c != d));
            let out_conflict = entry
                .limit
                .as_ref()
                .and_then(|l| l.output)
                .is_some_and(|o| info.output.is_some_and(|d| o != d));
            if !ctx_conflict && !out_conflict {
                continue;
            }

            // Đè bằng giá trị models.dev; giữ field `input` người dùng tự đặt.
            let mut limit = entry.limit.clone().unwrap_or_default();
            if info.context.is_some() {
                limit.context = info.context;
            }
            if info.output.is_some() {
                limit.output = info.output;
            }
            entry.limit = Some(limit);
            fixed += 1;
        }
    }

    // Không có gì lệch → không ghi file (tránh backup vô ích).
    if fixed > 0 {
        save_split(&config, &mut auth, &presets)?;
    }
    Ok(fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ghi opencode.json mẫu (2 provider, model kèm capability) + cache
    /// models.dev mẫu để test ma trận + chọn model chính.
    fn seed(test_dir: &std::path::Path) {
        let opencode_dir = test_dir.join(".config").join("opencode");
        std::fs::create_dir_all(&opencode_dir).unwrap();
        std::fs::write(
            opencode_dir.join("opencode.json"),
            r#"{
                "model": "p1/alpha",
                "provider": {
                    "p1": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "Provider Một",
                        "options": { "baseURL": "https://a.example.com/v1", "apiKey": "k1" },
                        "models": {
                            "alpha": {
                                "name": "Alpha",
                                "tool_call": true,
                                "reasoning": true,
                                "limit": { "context": 64000, "output": 8000 }
                            },
                            "beta": {}
                        }
                    },
                    "p2": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "Provider Hai",
                        "options": { "baseURL": "https://b.example.com/v1", "apiKey": "k2" },
                        "models": {
                            "gamma": {
                                "name": "Gamma Vision",
                                "modalities": { "input": ["text", "image"], "output": ["text"] }
                            },
                            "gpt-4o": {},
                            "weird-id": { "name": "GPT-4o Mini" }
                        }
                    },
                    "zenmux": {
                        "npm": "@ai-sdk/openai-compatible",
                        "name": "Zenmux",
                        "options": { "baseURL": "https://zenmux.example.com/v1", "apiKey": "zk" },
                        "models": {}
                    }
                }
            }"#,
        )
        .unwrap();

        // Cache models.dev: p1/beta có metadata (điền chỗ trống), p2 không có.
        let cache_dir = test_dir.join(".cache").join("opencode");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(
            cache_dir.join("models.json"),
            r#"{
                "zenmux": {
                    "id": "zenmux", "name": "Zenmux",
                    "models": {
                        "zenmux/gpt-4o": {
                            "name": "GPT-4o (Zenmux)",
                            "tool_call": true, "reasoning": true,
                            "limit": { "context": 128000, "output": 16384 },
                            "cost": { "input": 2.0, "output": 6.0 }
                        },
                        "zenmux/cheap": {
                            "name": "Cheap Model",
                            "limit": { "context": 8192 }
                        }
                    }
                },
                "p1": {
                    "id": "p1", "name": "Provider Một",
                    "models": {
                        "beta": {
                            "name": "Beta (models.dev)",
                            "tool_call": true,
                            "reasoning": false,
                            "modalities": { "input": ["text"], "output": ["text"] },
                            "limit": { "context": 32000, "output": 4096 },
                            "cost": { "input": 1.5, "output": 3.0, "cache_read": 0.2 }
                        },
                        "alpha": {
                            "name": "Alpha",
                            "limit": { "context": 128000, "output": 16000 }
                        }
                    }
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn ma_tran_gom_caps_limit_va_enrich() {
        let _guard = crate::test_support::TEST_ENV_LOCK.lock().unwrap();
        let test_dir = crate::test_support::isolate_home("model_matrix");
        seed(&test_dir);

        let rows = matrix_rows().unwrap();
        // p1: 2, p2: 3, zenmux (builtin key-only): 2 từ catalogue = 7.
        assert_eq!(rows.len(), 7);

        // p1/alpha: config khai sẵn — không bị models.dev đụng, nhưng limit
        // LỆCH models.dev (64000/8000 vs 128000/16000) → conflict=true, hiển
        // thị vẫn theo config (config thắng).
        let alpha = rows.iter().find(|r| r.model_id == "alpha").unwrap();
        assert!(alpha.is_primary, "config.model = p1/alpha");
        assert_eq!(alpha.tool_call, Some(true));
        assert_eq!(alpha.reasoning, Some(true));
        assert_eq!(alpha.context, Some(64000));
        assert_eq!(alpha.output, Some(8000));
        assert_eq!(
            alpha.dev_context,
            Some(128000),
            "phải lộ giá trị models.dev để đối chiếu"
        );
        assert_eq!(alpha.dev_output, Some(16000));
        assert!(alpha.limit_conflict, "limit config lệch models.dev → conflict");
        assert_eq!(alpha.vision, None, "alpha chưa khai modalities và không có trong cache");

        // p1/beta: entry rỗng {} → toàn bộ lấy từ models.dev (enriched).
        let beta = rows.iter().find(|r| r.model_id == "beta").unwrap();
        assert!(!beta.is_primary);
        assert_eq!(beta.display_name, "Beta (models.dev)");
        assert_eq!(beta.tool_call, Some(true));
        assert_eq!(beta.reasoning, Some(false));
        assert_eq!(beta.context, Some(32000));
        assert_eq!(beta.price_input, Some(1.5));
        assert_eq!(beta.price_output, Some(3.0));
        assert!(beta.enriched, "beta phải được làm giàu từ cache");
        assert!(!alpha.enriched, "alpha tự khai đủ → enriched = false");
        assert!(
            !beta.limit_conflict,
            "beta không khai limit → models.dev điền chỗ trống, không phải conflict"
        );

        // p2/gamma: vision từ modalities input chứa "image".
        let gamma = rows.iter().find(|r| r.model_id == "gamma").unwrap();
        assert_eq!(gamma.vision, Some(true));
        assert_eq!(gamma.provider_name, "Provider Hai");
        assert_eq!(gamma.display_name, "Gamma Vision");

        // p2/gpt-4o: entry rỗng + KHÔNG có trong dev cache → limit lấy từ suy
        // đoán TÊN (fallback yếu nhất), source = "name".
        let gpt = rows.iter().find(|r| r.model_id == "gpt-4o").unwrap();
        assert_eq!(gpt.context, Some(128_000), "heuristic gpt-4o điền chỗ trống");
        assert_eq!(gpt.output, Some(16_384));
        assert_eq!(gpt.context_source, "name");
        assert_eq!(gpt.heur_context, Some(128_000));
        assert!(!gpt.limit_conflict, "chỉ có heuristic → không có gì để lệch");
        // alpha/beta hiển thị theo config/dev như cũ.
        assert_eq!(alpha.context_source, "config");
        assert_eq!(beta.context_source, "models.dev");

        // p2/weird-id: id mờ không ra họ, nhưng DISPLAY NAME "GPT-4o Mini" lộ
        // họ → heuristic bắt được qua tên hiển thị.
        let weird = rows.iter().find(|r| r.model_id == "weird-id").unwrap();
        assert_eq!(weird.context, Some(128_000), "heuristic qua display name");
        assert_eq!(weird.context_source, "name");
        assert_eq!(weird.display_name, "GPT-4o Mini");

        // zenmux (BUILTIN KEY-ONLY, models rỗng): 2 model catalogue hiện trong
        // ma trận với đầy đủ metadata — đếm đúng, judge dùng được.
        let zgpt = rows.iter().find(|r| r.model_id == "zenmux/gpt-4o").unwrap();
        assert_eq!(zgpt.provider_id, "zenmux");
        assert_eq!(zgpt.display_name, "GPT-4o (Zenmux)");
        assert_eq!(zgpt.tool_call, Some(true), "metadata từ catalogue");
        assert_eq!(zgpt.context, Some(128_000));
        assert_eq!(zgpt.price_input, Some(2.0));
        assert_eq!(zgpt.context_source, "models.dev");
        let zcheap = rows.iter().find(|r| r.model_id == "zenmux/cheap").unwrap();
        assert_eq!(zcheap.context, Some(8_192));
    }

    #[test]
    fn dat_va_xoa_model_chinh() {
        let _guard = crate::test_support::TEST_ENV_LOCK.lock().unwrap();
        let test_dir = crate::test_support::isolate_home("primary_model");
        seed(&test_dir);

        // Đổi model chính sang p2/gamma → file ghi "p2/gamma".
        let m = set_primary_model_inner(Some("p2".into()), Some("gamma".into())).unwrap();
        assert_eq!(m.as_deref(), Some("p2/gamma"));
        let saved = opencode_manager::config::OpencodeConfig::load().unwrap();
        assert_eq!(saved.model.as_deref(), Some("p2/gamma"));
        let rows = matrix_rows().unwrap();
        assert!(rows.iter().find(|r| r.model_id == "gamma").unwrap().is_primary);
        assert!(!rows.iter().find(|r| r.model_id == "alpha").unwrap().is_primary);

        // Model không có trong provider → từ chối (OpenCode sẽ không resolve được).
        let err = set_primary_model_inner(Some("p1".into()), Some("khong-co".into())).unwrap_err();
        assert!(err.contains("chưa có trong provider"), "lỗi: {err}");

        // Provider lạ → từ chối.
        assert!(set_primary_model_inner(Some("la".into()), Some("x".into())).is_err());

        // Thiếu một bên → hướng dẫn rõ.
        assert!(set_primary_model_inner(Some("p1".into()), None).is_err());

        // Trống cả hai → xoá lựa chọn.
        let m = set_primary_model_inner(None, None).unwrap();
        assert_eq!(m, None);
        assert_eq!(opencode_manager::config::OpencodeConfig::load().unwrap().model, None);
    }

    /// Đồng bộ limit về models.dev: chỉ sửa model conflict, giữ capability.
    #[test]
    fn dong_bo_limit_ve_models_dev() {
        let _guard = crate::test_support::TEST_ENV_LOCK.lock().unwrap();
        let test_dir = crate::test_support::isolate_home("sync_limits");
        seed(&test_dir);

        // Chạy lần 1: chỉ alpha conflict (beta không khai limit).
        let fixed = sync_limits_from_dev_inner().unwrap();
        assert_eq!(fixed, 1, "chỉ alpha bị lệch");

        let rows = matrix_rows().unwrap();
        let alpha = rows.iter().find(|r| r.model_id == "alpha").unwrap();
        assert_eq!(alpha.context, Some(128000), "limit phải về giá trị models.dev");
        assert_eq!(alpha.output, Some(16000));
        assert!(!alpha.limit_conflict, "sau sync không còn lệch");
        // Capability/config khác phải sống sót.
        assert_eq!(alpha.tool_call, Some(true));
        assert_eq!(alpha.reasoning, Some(true));
        assert!(alpha.is_primary);

        // Chạy lần 2: không còn gì lệch → 0 và không ghi file.
        assert_eq!(sync_limits_from_dev_inner().unwrap(), 0);

        // File trên đĩa phản ánh đúng giá trị mới.
        let saved = opencode_manager::config::OpencodeConfig::load().unwrap();
        let a = &saved.provider["p1"].models["alpha"];
        assert_eq!(a.limit.as_ref().and_then(|l| l.context), Some(128000));
        assert_eq!(a.limit.as_ref().and_then(|l| l.output), Some(16000));
        assert_eq!(a.tool_call, Some(true), "capability không bị đụng");
    }
}
