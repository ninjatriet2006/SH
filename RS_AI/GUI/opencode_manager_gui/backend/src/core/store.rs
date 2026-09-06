/*
[INTEGRITY NOTES]
- Mục đích: Nghiệp vụ hợp nhất cấu hình OpenCode — gộp `auth.json` với
  `opencode.json` khi đọc, và tách trở lại khi ghi.
- Trách nhiệm: Các hàm THUẦN trên dữ liệu (không giữ UI state) để test được.
- Tương tác: `api::provider`, `api::ckey`, crate `opencode_manager` (định dạng
  file, client API).

VÌ SAO CÓ FILE NÀY (thay vì dùng `opencode_manager::app::App`):
  `App` của TUI nhúng UI state của ratatui (`ListState`) nên không dùng được ở
  GUI. Phần nghiệp vụ thật thì lại nằm lẫn trong đó. File này tách riêng đúng
  phần nghiệp vụ dưới dạng hàm thuần, dùng chung `config.rs` của crate TUI nên
  ĐỊNH DẠNG FILE vẫn là một nguồn sự thật duy nhất — chỉ luồng gộp/tách được
  viết lại, và có test chốt hành vi.

QUY TẮC GỘP/TÁCH (phải giữ đúng, nếu không sẽ làm hỏng config của người dùng):
  - Provider mà `id` khớp một preset VÀ `base_url` khớp preset đó → coi là
    "built-in": key lưu ở `auth.json`, KHÔNG ghi vào `opencode.json`.
  - Các provider còn lại → "custom": ghi trọn vào `opencode.json`.
  - Khi đọc: nạp `opencode.json` rồi bù thêm provider dựng từ `auth.json`.
*/

use opencode_manager::app::DynamicPreset;
use opencode_manager::config::{normalize_base_url, AuthConfig, AuthEntry, OpencodeConfig, Provider, ProviderOptions};
use serde::Serialize;
use std::collections::HashMap;

/// Toàn bộ cấu hình đã hợp nhất, dạng phẳng để frontend dùng trực tiếp.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub base_url: String,
    /// Key ĐÃ CHE. Không bao giờ gửi key thật ra frontend cho danh sách —
    /// tránh key nằm trong DOM/devtools. Muốn sửa thì gọi `get_provider_secret`.
    pub api_key_masked: String,
    pub has_api_key: bool,
    pub npm: Option<String>,
    pub model_count: usize,
    /// Danh sách id model đã lưu trong config, đã sắp xếp.
    pub models: Vec<String>,
    /// `true` nếu provider này là built-in (key nằm ở `auth.json`).
    pub is_builtin: bool,
}

/// Che API key: giữ 4 ký tự đầu và 4 cuối để người dùng đối chiếu được mà không
/// lộ toàn bộ. Key ngắn thì che hết.
pub fn mask_key(key: &str) -> String {
    let k = key.trim();
    if k.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = k.chars().collect();
    if chars.len() <= 10 {
        return "•".repeat(chars.len().min(8));
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

/// Provider có phải built-in không (id khớp preset và base_url khớp preset).
pub fn is_builtin(id: &str, provider: &Provider, presets: &[DynamicPreset]) -> bool {
    presets.iter().any(|preset| {
        preset.id == id && normalize_base_url(&provider.options.base_url) == normalize_base_url(&preset.base_url)
    })
}

/// Bù provider dựng từ `auth.json` vào config đã nạp từ `opencode.json`.
///
/// Trả về số provider được thêm mới. Provider đã có thì chỉ điền key nếu đang
/// trống — KHÔNG ghi đè key trong `opencode.json` bằng key của `auth.json`.
pub fn merge_auth_into_providers(
    config: &mut OpencodeConfig,
    auth: &AuthConfig,
    presets: &[DynamicPreset],
) -> Vec<String> {
    let mut added = Vec::new();
    for (auth_id, entry) in auth {
        if entry.auth_type != "api" || entry.key.is_empty() {
            continue;
        }
        let Some(preset) = presets.iter().find(|p| p.id == *auth_id) else {
            continue;
        };

        match config.provider.get_mut(auth_id) {
            Some(prov) => {
                if prov.options.api_key.is_empty() {
                    prov.options.api_key = entry.key.clone();
                }
            }
            None => {
                config.provider.insert(
                    auth_id.clone(),
                    Provider {
                        npm: preset
                            .npm
                            .clone()
                            .or_else(|| Some("@ai-sdk/openai-compatible".to_string())),
                        name: preset.name.clone(),
                        options: ProviderOptions {
                            base_url: preset.base_url.clone(),
                            api_key: entry.key.clone(),
                            ..Default::default()
                        },
                        models: HashMap::new(),
                        ..Default::default()
                    },
                );
                added.push(preset.name.clone());
            }
        }
    }
    added
}

/// Nạp cấu hình hợp nhất từ đĩa.
pub fn load_merged(presets: &[DynamicPreset]) -> Result<(OpencodeConfig, AuthConfig), String> {
    let mut config = OpencodeConfig::load()?;
    let auth = AuthEntry::load_config()?;
    merge_auth_into_providers(&mut config, &auth, presets);
    Ok((config, auth))
}

/// Tách cấu hình hợp nhất thành hai file rồi ghi xuống đĩa.
///
/// `auth` được cập nhật tại chỗ: thêm key của built-in, xoá entry không còn
/// provider tương ứng (nếu giữ lại, provider đã xoá sẽ "sống lại" ở lần nạp sau
/// nhờ `merge_auth_into_providers`).
pub fn save_split(config: &OpencodeConfig, auth: &mut AuthConfig, presets: &[DynamicPreset]) -> Result<(), String> {
    // 1. Built-in → auth.json.
    for (id, provider) in &config.provider {
        if is_builtin(id, provider, presets) {
            auth.insert(
                id.clone(),
                AuthEntry {
                    auth_type: "api".to_string(),
                    key: provider.options.api_key.clone(),
                },
            );
        }
    }
    // 2. Dọn entry không còn đúng vai trò.
    //
    // Hai dạng phải xoá, đều là entry mà `merge_auth_into_providers` HỢP LỆ
    // dùng (khớp preset + type "api" + key không rỗng):
    //   - Mồ côi: provider tương ứng đã bị xoá — giữ lại thì provider đó
    //     "sống lại" ở lần nạp sau.
    //   - Treo: provider còn nhưng KHÔNG còn là built-in (người dùng trỏ URL
    //     đi chỗ khác hoặc đổi id) — entry auth giữ khoá cũ, để lại là dữ liệu
    //     lệch; khoá thật đã nằm trong opencode.json của provider custom.
    // Entry OAuth hoặc id ngoài preset (app khác ghi) KHÔNG thuộc phạm vi quản
    // lý của merge → phải giữ nguyên: xoá bừa là mất dữ liệu người dùng.
    let orphans: Vec<String> = auth
        .iter()
        .filter(|(k, e)| {
            let would_merge = e.auth_type == "api" && !e.key.trim().is_empty() && presets.iter().any(|p| p.id == **k);
            if !would_merge {
                return false;
            }
            match config.provider.get(*k) {
                None => true,
                Some(p) => !is_builtin(k, p, presets),
            }
        })
        .map(|(k, _)| k.clone())
        .collect();
    for k in orphans {
        auth.remove(&k);
    }
    // 3. opencode.json chỉ chứa provider custom.
    let mut file_config = config.clone();
    file_config
        .provider
        .retain(|id, provider| !is_builtin(id, provider, presets));

    file_config.save()?;
    AuthEntry::save_config(auth)?;
    Ok(())
}

/// Dựng danh sách provider để hiển thị, đã sắp theo id.
pub fn build_views(config: &OpencodeConfig, presets: &[DynamicPreset]) -> Vec<ProviderView> {
    let mut views: Vec<ProviderView> = config
        .provider
        .iter()
        .map(|(id, p)| {
            let mut models: Vec<String> = p.models.keys().cloned().collect();
            models.sort();
            ProviderView {
                id: id.clone(),
                name: p.name.clone(),
                base_url: p.options.base_url.clone(),
                api_key_masked: mask_key(&p.options.api_key),
                has_api_key: !p.options.api_key.trim().is_empty(),
                npm: p.npm.clone(),
                model_count: models.len(),
                models,
                is_builtin: is_builtin(id, p, presets),
            }
        })
        .collect();
    views.sort_by(|a, b| a.id.cmp(&b.id));
    views
}

/// Tìm provider trùng cả `base_url` (đã chuẩn hoá) và `api_key`.
/// `skip_id` để chế độ sửa không tự so với chính nó.
pub fn detect_duplicate(
    config: &OpencodeConfig,
    base_url: &str,
    api_key: &str,
    skip_id: Option<&str>,
) -> Option<(String, String)> {
    let base_url = base_url.trim();
    let api_key = api_key.trim();
    if base_url.is_empty() || api_key.is_empty() {
        return None;
    }
    let clean_new = normalize_base_url(base_url);

    config.provider.iter().find_map(|(id, p)| {
        if Some(id.as_str()) == skip_id {
            return None;
        }
        if normalize_base_url(&p.options.base_url) == clean_new && p.options.api_key.trim() == api_key {
            Some((id.clone(), p.name.clone()))
        } else {
            None
        }
    })
}

/// Sinh id chưa dùng từ tiền tố: `prefix`, `prefix_2`, `prefix_3`…
pub fn unique_id(config: &OpencodeConfig, prefix: &str) -> String {
    if !config.provider.contains_key(prefix) {
        return prefix.to_string();
    }
    let mut suffix = 2u32;
    loop {
        let candidate = format!("{prefix}_{suffix}");
        if !config.provider.contains_key(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str, url: &str) -> DynamicPreset {
        DynamicPreset {
            id: id.to_string(),
            name: format!("Preset {id}"),
            base_url: url.to_string(),
            id_prefix: id.to_string(),
            npm: Some("@ai-sdk/openai-compatible".to_string()),
        }
    }

    fn provider(name: &str, url: &str, key: &str) -> Provider {
        Provider {
            npm: None,
            name: name.to_string(),
            options: ProviderOptions {
                base_url: url.to_string(),
                api_key: key.to_string(),
                ..Default::default()
            },
            models: HashMap::new(),
            ..Default::default()
        }
    }

    fn empty_config() -> OpencodeConfig {
        OpencodeConfig {
            schema: None,
            model: None,
            provider: HashMap::new(),
        }
    }

    #[test]
    fn mask_key_khong_lo_toan_bo() {
        assert_eq!(mask_key(""), "");
        // Key ngắn: che hết, không hé ký tự nào.
        let short = mask_key("abc123");
        assert!(!short.contains('a') && !short.contains('1'), "{short}");
        // Key dài: hé 4 đầu + 4 cuối để đối chiếu.
        let long = mask_key("sk-proj-1234567890abcdef");
        assert!(long.starts_with("sk-p"), "{long}");
        assert!(long.ends_with("cdef"), "{long}");
        assert!(!long.contains("1234567890"), "không được lộ phần giữa: {long}");
    }

    #[test]
    fn builtin_can_khop_ca_id_va_url() {
        let presets = vec![preset("openai", "https://api.openai.com/v1")];
        let p = provider("OpenAI", "https://api.openai.com/v1", "k");
        assert!(is_builtin("openai", &p, &presets));
        // Đúng id nhưng URL khác → custom (người dùng trỏ sang proxy riêng).
        let p2 = provider("OpenAI", "https://my-proxy.local/v1", "k");
        assert!(!is_builtin("openai", &p2, &presets));
        // Đúng URL nhưng id khác → custom.
        assert!(!is_builtin("openai_2", &p, &presets));
    }

    #[test]
    fn builtin_bo_qua_khac_biet_duoi_dang_path_thua() {
        // normalize_base_url cắt path thừa nên vẫn nhận ra là built-in.
        let presets = vec![preset("openai", "https://api.openai.com/v1")];
        let p = provider("OpenAI", "https://api.openai.com/v1/chat/completions", "k");
        assert!(is_builtin("openai", &p, &presets));
    }

    #[test]
    fn merge_them_provider_tu_auth() {
        let presets = vec![preset("openai", "https://api.openai.com/v1")];
        let mut config = empty_config();
        let mut auth = AuthConfig::new();
        auth.insert(
            "openai".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "sk-from-auth".into(),
            },
        );

        let added = merge_auth_into_providers(&mut config, &auth, &presets);
        assert_eq!(added.len(), 1);
        let p = config.provider.get("openai").expect("phải được thêm");
        assert_eq!(p.options.api_key, "sk-from-auth");
        assert_eq!(p.options.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn merge_khong_ghi_de_key_dang_co() {
        // Key trong opencode.json là nguồn ưu tiên — ghi đè bằng auth.json sẽ
        // âm thầm đổi key mà người dùng vừa nhập.
        let presets = vec![preset("openai", "https://api.openai.com/v1")];
        let mut config = empty_config();
        config.provider.insert(
            "openai".into(),
            provider("OpenAI", "https://api.openai.com/v1", "sk-in-config"),
        );
        let mut auth = AuthConfig::new();
        auth.insert(
            "openai".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "sk-from-auth".into(),
            },
        );

        merge_auth_into_providers(&mut config, &auth, &presets);
        assert_eq!(config.provider["openai"].options.api_key, "sk-in-config");
    }

    #[test]
    fn merge_dien_key_khi_dang_trong() {
        let presets = vec![preset("openai", "https://api.openai.com/v1")];
        let mut config = empty_config();
        config
            .provider
            .insert("openai".into(), provider("OpenAI", "https://api.openai.com/v1", ""));
        let mut auth = AuthConfig::new();
        auth.insert(
            "openai".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "sk-from-auth".into(),
            },
        );

        merge_auth_into_providers(&mut config, &auth, &presets);
        assert_eq!(config.provider["openai"].options.api_key, "sk-from-auth");
    }

    #[test]
    fn merge_bo_qua_entry_khong_phai_api_hoac_rong() {
        let presets = vec![preset("openai", "https://api.openai.com/v1")];
        let mut config = empty_config();
        let mut auth = AuthConfig::new();
        auth.insert(
            "openai".into(),
            AuthEntry {
                auth_type: "oauth".into(),
                key: "tok".into(),
            },
        );
        assert!(merge_auth_into_providers(&mut config, &auth, &presets).is_empty());
        assert!(config.provider.is_empty());

        let mut auth2 = AuthConfig::new();
        auth2.insert(
            "openai".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: String::new(),
            },
        );
        assert!(merge_auth_into_providers(&mut config, &auth2, &presets).is_empty());
        assert!(config.provider.is_empty());
    }

    #[test]
    fn merge_bo_qua_auth_id_khong_co_preset() {
        // Không biết base_url thì không dựng được provider — bỏ qua thay vì
        // tạo provider với URL rỗng (sẽ luôn báo lỗi kết nối, gây hoang mang).
        let presets: Vec<DynamicPreset> = vec![];
        let mut config = empty_config();
        let mut auth = AuthConfig::new();
        auth.insert(
            "provider_la".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "k".into(),
            },
        );
        assert!(merge_auth_into_providers(&mut config, &auth, &presets).is_empty());
        assert!(config.provider.is_empty());
    }

    #[test]
    fn detect_duplicate_theo_url_chuan_hoa_va_key() {
        let mut config = empty_config();
        config
            .provider
            .insert("a".into(), provider("A", "https://api.x.com/v1", "same-key"));

        // URL viết thừa path nhưng chuẩn hoá về cùng → vẫn là trùng.
        let dup = detect_duplicate(&config, "https://api.x.com/v1/chat/completions", "same-key", None);
        assert_eq!(dup.map(|(id, _)| id), Some("a".to_string()));

        // Khác key → không trùng.
        assert!(detect_duplicate(&config, "https://api.x.com/v1", "other", None).is_none());
        // Bỏ qua chính nó khi sửa.
        assert!(detect_duplicate(&config, "https://api.x.com/v1", "same-key", Some("a")).is_none());
        // Thiếu dữ liệu → không kết luận.
        assert!(detect_duplicate(&config, "", "same-key", None).is_none());
        assert!(detect_duplicate(&config, "https://api.x.com/v1", "", None).is_none());
    }

    #[test]
    fn unique_id_tang_dan() {
        let mut config = empty_config();
        assert_eq!(unique_id(&config, "openai"), "openai");
        config.provider.insert("openai".into(), provider("A", "u", "k"));
        assert_eq!(unique_id(&config, "openai"), "openai_2");
        config.provider.insert("openai_2".into(), provider("B", "u", "k"));
        assert_eq!(unique_id(&config, "openai"), "openai_3");
    }

    /// Hồi quy: save_split không được xoá entry auth.json mà merge không quản
    /// lý (OAuth, id ngoài preset) — chúng do app khác ghi, xoá là mất dữ liệu.
    /// Entry builtin đã bị xoá provider thì PHẢI bị dọn (nếu không sống lại).
    #[test]
    fn save_split_khong_xoa_auth_entry_merge_khong_quan_ly() {
        // save_split ghi file thật theo home → phải khoá + cô lập home test.
        let _guard = crate::test_support::TEST_ENV_LOCK.lock().unwrap();
        let test_dir = crate::test_support::isolate_home("store_save_split");

        let presets = vec![
            preset("openai", "https://api.openai.com/v1"),
            preset("anthropic", "https://api.anthropic.com/v1"),
            preset("groq", "https://api.groq.com/openai/v1"),
        ];
        let mut config = empty_config();

        let mut auth = AuthConfig::new();
        // 1. OAuth entry (opencode native) — phải GIỮ.
        auth.insert(
            "github".into(),
            AuthEntry {
                auth_type: "oauth".into(),
                key: "tok".into(),
            },
        );
        // 2. Id ngoài preset — phải GIỮ (merge bỏ qua, không phải mồ côi).
        auth.insert(
            "provider_la".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "k".into(),
            },
        );
        // 3. Builtin đã có provider — phải GIỮ (được sync key).
        auth.insert(
            "openai".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "sk-live".into(),
            },
        );
        // 4. Builtin nhưng provider đã bị xoá → mồ côi thật, phải XOÁ.
        auth.insert(
            "anthropic".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "sk-removed".into(),
            },
        );
        // 5. Provider còn nhưng URL trỏ đi proxy riêng (không còn built-in) →
        //    entry auth giữ khoá cũ là dữ liệu TREO, phải XOÁ (khoá thật nằm
        //    trong opencode.json của provider custom).
        auth.insert(
            "groq".into(),
            AuthEntry {
                auth_type: "api".into(),
                key: "sk-stale".into(),
            },
        );
        config.provider.insert(
            "openai".into(),
            provider("OpenAI", "https://api.openai.com/v1", "sk-live"),
        );
        config.provider.insert(
            "groq".into(),
            provider("Groq proxy", "https://my-groq-proxy.local/v1", "sk-in-config"),
        );

        save_split(&config, &mut auth, &presets).unwrap();

        assert!(auth.contains_key("github"), "entry oauth bị xoá oan");
        assert!(auth.contains_key("provider_la"), "entry ngoài preset bị xoá oan");
        assert!(auth.contains_key("openai"));
        assert!(!auth.contains_key("anthropic"), "mồ côi thật phải bị dọn");
        assert!(!auth.contains_key("groq"), "entry treo (URL đã rời preset) phải bị dọn");
        assert_eq!(auth["openai"].key, "sk-live");

        // Đọc lại từ đĩa: file auth.json phải phản ánh đúng những gì vừa giữ.
        let on_disk = AuthEntry::load_config().unwrap();
        assert!(on_disk.contains_key("github"));
        assert!(on_disk.contains_key("provider_la"));
        assert!(!on_disk.contains_key("anthropic"));
        assert!(!on_disk.contains_key("groq"), "entry treo phải biến mất khỏi file");
        // Provider custom (URL rời preset) phải nằm trong opencode.json với đủ khoá.
        let file_cfg = OpencodeConfig::load().unwrap();
        assert_eq!(
            file_cfg.provider["groq"].options.api_key, "sk-in-config",
            "khoá của provider custom phải được ghi vào opencode.json"
        );

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn build_views_sap_xep_va_che_key() {
        let presets = vec![preset("openai", "https://api.openai.com/v1")];
        let mut config = empty_config();
        config.provider.insert(
            "zzz".into(),
            provider("Z", "https://z.com/v1", "sk-proj-1234567890abcdef"),
        );
        config.provider.insert(
            "openai".into(),
            provider("OpenAI", "https://api.openai.com/v1", "sk-openai-1234567890"),
        );

        let views = build_views(&config, &presets);
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].id, "openai", "phải sắp theo id");
        assert!(views[0].is_builtin);
        assert!(!views[1].is_builtin);
        // Không có view nào chứa key thật.
        for v in &views {
            assert!(!v.api_key_masked.contains("1234567890"), "{}", v.api_key_masked);
            assert!(v.has_api_key);
        }
    }

    #[test]
    fn build_views_dem_model_va_sap_xep_model() {
        let presets: Vec<DynamicPreset> = vec![];
        let mut config = empty_config();
        let mut p = provider("A", "https://a.com/v1", "k");
        for m in ["zeta", "alpha", "mid"] {
            p.models.insert(
                m.to_string(),
                opencode_manager::config::ModelEntry {
                    name: m.to_string(),
                    limit: None,
                    modalities: None,
                    ..Default::default()
                },
            );
        }
        config.provider.insert("a".into(), p);

        let views = build_views(&config, &presets);
        assert_eq!(views[0].model_count, 3);
        assert_eq!(views[0].models, vec!["alpha", "mid", "zeta"]);
    }
}
