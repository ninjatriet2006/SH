/*
[INTEGRITY NOTES]
- Mục đích: Thêm nhanh nhiều provider — một endpoint + N API key.
- Trách nhiệm: Tách/lọc key, bỏ key trùng (cả trùng trong lần dán lẫn trùng với
  provider đã có), tạo provider với id ngẫu nhiên không xung đột.
- Tương tác: `core::store`, crate `opencode_manager` (sinh tên, chuẩn hoá URL).

Vì sao cần chức năng này: khi mua nhiều key cùng một nhà cung cấp, thêm từng cái
qua form là rất lâu. Điểm dễ sai là trùng lặp — dán lại danh sách cũ sẽ tạo ra
hàng loạt provider trùng, nên hàm này lọc theo cặp (endpoint đã chuẩn hoá, key).
*/

use crate::core::store::{load_merged, save_split};
use crate::ipc::{from_string, respond, IpcResult, Req};
use opencode_manager::app::DynamicPreset;
use opencode_manager::ckey::generate_provider_name;
use opencode_manager::config::{normalize_base_url, OpencodeConfig, Provider, ProviderOptions};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct BulkAddResult {
    /// Số provider đã tạo.
    pub added: usize,
    /// Số key bị bỏ vì trùng với provider đã có.
    pub skipped_existing: usize,
    /// Số key bị bỏ vì lặp trong chính lần dán này.
    pub skipped_duplicate_input: usize,
    /// Id các provider vừa tạo (để UI trỏ tới).
    pub created_ids: Vec<String>,
    /// Endpoint sau khi chuẩn hoá — hiện lại cho người dùng thấy nếu bị sửa.
    pub normalized_endpoint: String,
}

/// Kiểm tra endpoint có dạng `scheme://host...`.
///
/// Bắt buộc vì `normalize_base_url` cố ý trả nguyên chuỗi khi thiếu scheme; nếu
/// không chặn ở đây, ta sẽ tạo ra hàng loạt provider có URL vô nghĩa và mọi lần
/// kiểm tra kết nối đều lỗi mà không rõ lý do.
fn endpoint_valid(endpoint: &str) -> bool {
    match endpoint.split_once("://") {
        Some((scheme, rest)) => !scheme.is_empty() && !rest.is_empty() && !rest.starts_with('/'),
        None => false,
    }
}

/// Tách danh sách key từ ô nhập nhiều dòng: trim, bỏ dòng rỗng, bỏ lặp.
/// Trả về `(keys, số dòng lặp bị bỏ)`.
pub fn parse_keys(raw: &str) -> (Vec<String>, usize) {
    let mut seen = HashSet::new();
    let mut keys = Vec::new();
    let mut dup = 0usize;
    for line in raw.lines() {
        // Cho phép dán cả dạng "tên: key" hoặc có dấu phẩy cuối dòng.
        let k = line.trim().trim_end_matches(',').trim().to_string();
        if k.is_empty() {
            continue;
        }
        if !seen.insert(k.clone()) {
            dup += 1;
            continue;
        }
        keys.push(k);
    }
    (keys, dup)
}

/// Hàm THUẦN: thêm các key vào config, trả về báo cáo. Không chạm file.
pub fn bulk_add_into(config: &mut OpencodeConfig, endpoint_raw: &str, keys_raw: &str) -> Result<BulkAddResult, String> {
    let endpoint = normalize_base_url(endpoint_raw.trim());
    if endpoint.is_empty() {
        return Err("Vui lòng nhập endpoint.".to_string());
    }
    if !endpoint_valid(&endpoint) {
        return Err("Endpoint không hợp lệ: phải có dạng https://host/...".to_string());
    }

    let (keys, skipped_duplicate_input) = parse_keys(keys_raw);
    if keys.is_empty() {
        return Err("Vui lòng dán ít nhất 1 API key (mỗi dòng 1 key).".to_string());
    }

    let mut existing_ids: HashSet<String> = config.provider.keys().cloned().collect();
    let mut added = 0usize;
    let mut skipped_existing = 0usize;
    let mut created_ids = Vec::new();

    for key in keys {
        // Cặp (endpoint, key) đã có provider → bỏ qua, không tạo bản trùng.
        let exists = config
            .provider
            .values()
            .any(|p| normalize_base_url(&p.options.base_url) == endpoint && p.options.api_key.trim() == key);
        if exists {
            skipped_existing += 1;
            continue;
        }

        let id = generate_provider_name(&existing_ids);
        existing_ids.insert(id.clone());
        config.provider.insert(
            id.clone(),
            Provider {
                npm: Some("@ai-sdk/openai-compatible".to_string()),
                name: id.clone(),
                options: ProviderOptions {
                    base_url: endpoint.clone(),
                    api_key: key,
                    ..Default::default()
                },
                models: Default::default(),
                ..Default::default()
            },
        );
        created_ids.push(id);
        added += 1;
    }

    Ok(BulkAddResult {
        added,
        skipped_existing,
        skipped_duplicate_input,
        created_ids,
        normalized_endpoint: endpoint,
    })
}

fn presets() -> Vec<DynamicPreset> {
    opencode_manager::app::App::load_dynamic_presets()
}

#[tauri::command(rename_all = "snake_case")]
pub fn bulk_add_providers(request: Req<BulkAddProvidersRequest>) -> IpcResult<BulkAddResult> {
    let (request_id, payload) = request.validate()?;
    bulk_add_providers_inner(payload.endpoint, payload.keys)
        .map(|data| respond(request_id, data))
        .map_err(from_string)
}

#[derive(Deserialize)]
pub struct BulkAddProvidersRequest {
    pub endpoint: String,
    pub keys: Vec<String>,
}

fn bulk_add_providers_inner(endpoint: String, keys: Vec<String>) -> Result<BulkAddResult, String> {
    let presets = presets();
    let (mut config, mut auth) = load_merged(&presets)?;
    let result = bulk_add_into(&mut config, &endpoint, &keys.join("\n"))?;

    // Chỉ ghi file khi thật sự có thay đổi — tránh tạo bản backup vô ích của
    // `opencode.json` mỗi lần người dùng bấm nút mà mọi key đều trùng.
    if result.added > 0 {
        save_split(&config, &mut auth, &presets)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn empty_config() -> OpencodeConfig {
        OpencodeConfig {
            schema: None,
            model: None,
            provider: HashMap::new(),
        }
    }

    #[test]
    fn parse_keys_lam_sach_va_bo_lap() {
        let (keys, dup) = parse_keys("  sk-a  \n\nsk-b,\nsk-a\n   \nsk-c");
        assert_eq!(keys, vec!["sk-a", "sk-b", "sk-c"]);
        assert_eq!(dup, 1, "sk-a lặp một lần");
    }

    #[test]
    fn endpoint_phai_co_scheme_va_host() {
        assert!(endpoint_valid("https://api.x.com/v1"));
        assert!(endpoint_valid("http://localhost:1234/v1"));
        // Thiếu scheme → không đủ thông tin, phải chặn.
        assert!(!endpoint_valid("api.x.com/v1"));
        assert!(!endpoint_valid("/v1"));
        assert!(!endpoint_valid(""));
        // Có scheme nhưng host rỗng.
        assert!(!endpoint_valid("https:///v1"));
    }

    #[test]
    fn tao_mot_provider_moi_key() {
        let mut config = empty_config();
        let r = bulk_add_into(&mut config, "https://api.x.com/v1", "sk-1\nsk-2\nsk-3").unwrap();
        assert_eq!(r.added, 3);
        assert_eq!(config.provider.len(), 3);
        // Mỗi provider phải có id KHÁC nhau, nếu không sẽ ghi đè lẫn nhau.
        assert_eq!(r.created_ids.len(), 3);
        let unique: HashSet<&String> = r.created_ids.iter().collect();
        assert_eq!(unique.len(), 3, "id phải duy nhất: {:?}", r.created_ids);
        // Mỗi provider giữ đúng key của nó.
        let keys: HashSet<String> = config.provider.values().map(|p| p.options.api_key.clone()).collect();
        assert_eq!(keys, ["sk-1", "sk-2", "sk-3"].iter().map(|s| s.to_string()).collect());
    }

    #[test]
    fn bo_key_trung_provider_da_co() {
        // Dán lại danh sách cũ không được sinh ra bản trùng.
        let mut config = empty_config();
        bulk_add_into(&mut config, "https://api.x.com/v1", "sk-1\nsk-2").unwrap();
        let r = bulk_add_into(&mut config, "https://api.x.com/v1", "sk-1\nsk-2\nsk-3").unwrap();

        assert_eq!(r.added, 1, "chỉ sk-3 là mới");
        assert_eq!(r.skipped_existing, 2);
        assert_eq!(config.provider.len(), 3);
    }

    #[test]
    fn cung_key_khac_endpoint_thi_van_them() {
        // Cùng key dùng cho hai endpoint khác nhau là hợp lệ (proxy/gateway).
        let mut config = empty_config();
        bulk_add_into(&mut config, "https://api.x.com/v1", "sk-1").unwrap();
        let r = bulk_add_into(&mut config, "https://api.y.com/v1", "sk-1").unwrap();
        assert_eq!(r.added, 1);
        assert_eq!(config.provider.len(), 2);
    }

    #[test]
    fn endpoint_duoc_chuan_hoa_truoc_khi_so_trung() {
        // `/v1/chat/completions` và `/v1` là cùng một endpoint → key thứ hai trùng.
        let mut config = empty_config();
        bulk_add_into(&mut config, "https://api.x.com/v1", "sk-1").unwrap();
        let r = bulk_add_into(&mut config, "https://api.x.com/v1/chat/completions", "sk-1").unwrap();
        assert_eq!(r.added, 0);
        assert_eq!(r.skipped_existing, 1);
        assert_eq!(r.normalized_endpoint, "https://api.x.com/v1");
    }

    #[test]
    fn tu_choi_input_rong_hoac_endpoint_sai() {
        let mut config = empty_config();
        assert!(bulk_add_into(&mut config, "", "sk-1").is_err());
        assert!(bulk_add_into(&mut config, "api.x.com/v1", "sk-1").is_err());
        assert!(bulk_add_into(&mut config, "https://api.x.com/v1", "").is_err());
        assert!(bulk_add_into(&mut config, "https://api.x.com/v1", "\n\n  \n").is_err());
        // Không được tạo gì khi lỗi.
        assert!(config.provider.is_empty());
    }

    #[test]
    fn khong_ghi_de_provider_dang_co_khi_sinh_id() {
        // Id sinh ngẫu nhiên phải tránh cả id đã có trong config.
        let mut config = empty_config();
        for i in 0..20 {
            let keys = format!("sk-{i}");
            bulk_add_into(&mut config, "https://api.x.com/v1", &keys).unwrap();
        }
        assert_eq!(config.provider.len(), 20, "không được mất provider nào do id trùng");
    }
}
