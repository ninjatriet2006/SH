use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use chrono::Local;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelLimit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelModalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<ModelLimit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modalities: Option<ModelModalities>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProviderOptions {
    #[serde(rename = "baseURL")]
    pub base_url: String,
    #[serde(rename = "apiKey")]
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Provider {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm: Option<String>,
    pub name: String,
    pub options: ProviderOptions,
    #[serde(default)]
    pub models: HashMap<String, ModelEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpencodeConfig {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default)]
    pub provider: HashMap<String, Provider>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthEntry {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub key: String,
}

pub type AuthConfig = HashMap<String, AuthEntry>;

impl OpencodeConfig {
    pub fn file_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".config").join("opencode").join("opencode.json")
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::file_path();
        if !path.exists() {
            return Ok(OpencodeConfig {
                schema: Some("https://opencode.ai/config.json".to_string()),
                provider: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Không thể đọc file opencode.json: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Lỗi parse JSON opencode.json: {}", e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path();
        
        // Tạo thư mục cha nếu chưa có
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Không thể tạo thư mục cấu hình: {}", e))?;
        }

        // Tạo bản backup
        if path.exists() {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = path.with_extension(format!("json.bak_{}", timestamp));
            let _ = fs::copy(&path, &backup_path);
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Không thể serialize cấu hình: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Không thể ghi file opencode.json: {}", e))?;

        Ok(())
    }
}

impl AuthEntry {
    pub fn file_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home"));
        home.join(".local").join("share").join("opencode").join("auth.json")
    }

    pub fn load_config() -> Result<AuthConfig, String> {
        let path = Self::file_path();
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Không thể đọc file auth.json: {}", e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("Lỗi parse JSON auth.json: {}", e))
    }

    pub fn save_config(config: &AuthConfig) -> Result<(), String> {
        let path = Self::file_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Không thể tạo thư mục chứa auth.json: {}", e))?;
        }

        // Tạo bản backup
        if path.exists() {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
            let backup_path = path.with_extension(format!("json.bak_{}", timestamp));
            let _ = fs::copy(&path, &backup_path);
        }

        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Không thể serialize auth.json: {}", e))?;

        fs::write(&path, content)
            .map_err(|e| format!("Không thể ghi file auth.json: {}", e))?;

        Ok(())
    }
}
