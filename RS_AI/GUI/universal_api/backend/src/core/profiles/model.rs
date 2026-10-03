//! Profile Data Models & Hardware Fingerprint Structures.
//!
//! Tách biệt và mô phỏng hoàn toàn môi trường của từng instance IDE độc lập.
//! Tương thích 1:1 với định dạng dữ liệu Cockpit Tools (`instances.json`, `defaultSettings`).

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ExtraArgsVal {
    Text(String),
    List(Vec<String>),
}

impl Serialize for ExtraArgsVal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string_lossy())
    }
}

impl Default for ExtraArgsVal {
    fn default() -> Self {
        ExtraArgsVal::Text(String::new())
    }
}

impl ExtraArgsVal {
    pub fn to_string_lossy(&self) -> String {
        match self {
            ExtraArgsVal::Text(s) => s.clone(),
            ExtraArgsVal::List(l) => l.join(" "),
        }
    }
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            ExtraArgsVal::Text(s) => s.split_whitespace().map(|x| x.to_string()).collect(),
            ExtraArgsVal::List(l) => l.clone(),
        }
    }
}

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let v = serde_json::Value::deserialize(deserializer)?;
    match v {
        serde_json::Value::Number(n) => n.as_i64().ok_or_else(|| D::Error::custom("invalid integer timestamp")),
        serde_json::Value::String(s) => {
            if let Ok(ts) = s.parse::<i64>() {
                Ok(ts)
            } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                Ok(dt.timestamp())
            } else {
                Ok(chrono::Utc::now().timestamp())
            }
        }
        _ => Ok(chrono::Utc::now().timestamp()),
    }
}

fn deserialize_optional_timestamp<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = Option::<serde_json::Value>::deserialize(deserializer)?;
    match v {
        Some(serde_json::Value::Number(n)) => Ok(n.as_i64()),
        Some(serde_json::Value::String(s)) => {
            if let Ok(ts) = s.parse::<i64>() {
                Ok(Some(ts))
            } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                Ok(Some(dt.timestamp()))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn default_now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareFingerprint {
    /// SHA-256 hash giả lập cho ID máy (`telemetry.machineId`)
    #[serde(default)]
    pub machine_id: String,
    /// SHA-256 hash giả lập cho card mạng MAC (`telemetry.macMachineId`)
    #[serde(default)]
    pub mac_machine_id: String,
    /// UUID v4 giả lập cho Device ID (`telemetry.devDeviceId`)
    #[serde(default)]
    pub dev_device_id: String,
    /// Windows SQM session identifier (`sqmId`)
    #[serde(default)]
    pub sqm_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceProfile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Định danh nền tảng IDE (ví dụ: "antigravity", "codex", "claude", "cursor", "vscode", "windsurf", "zed")
    #[serde(default, alias = "platform_id")]
    pub platform_id: String,
    /// Thư mục dữ liệu người dùng cô lập (`--user-data-dir`)
    #[serde(default, alias = "user_data_dir")]
    pub user_data_dir: PathBuf,
    /// Thư mục làm việc working dir
    #[serde(default, alias = "working_dir")]
    pub working_dir: Option<PathBuf>,
    /// ID tài khoản được gán chết cho profile này (nếu có, xuất ra bindAccountId chuẩn Cockpit)
    #[serde(default, rename = "bindAccountId", alias = "bound_account_id", alias = "bind_account_id")]
    pub bound_account_id: Option<String>,
    /// Các cờ dòng lệnh bổ sung (luôn serialize thành chuỗi cho Cockpit)
    #[serde(default, alias = "extra_args")]
    pub extra_args: ExtraArgsVal,
    /// Bộ mã máy ảo riêng biệt để chống phát hiện multi-account
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_fingerprint: Option<HardwareFingerprint>,
    /// Thời điểm khởi tạo (Unix timestamp i64, tương thích tuyệt đối Cockpit)
    #[serde(default = "default_now_timestamp", alias = "created_at", deserialize_with = "deserialize_timestamp")]
    pub created_at: i64,
    /// Thời điểm khởi chạy gần nhất
    #[serde(default, alias = "last_launched_at", deserialize_with = "deserialize_optional_timestamp")]
    pub last_launched_at: Option<i64>,
    /// PID của tiến trình gần nhất
    #[serde(default, alias = "last_pid")]
    pub last_pid: Option<u32>,
    /// Đây có phải là Default Instance đại diện cho app chính không
    #[serde(default, alias = "is_default")]
    pub is_default: bool,
    /// Trạng thái đang chạy thực tế hay đã tắt (kiểm tra qua PID)
    #[serde(default, alias = "is_running")]
    pub is_running: bool,
    /// Chế độ khởi chạy: app hoặc cli
    #[serde(default, alias = "launch_mode")]
    pub launch_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DefaultInstanceSettings {
    #[serde(default, alias = "bind_account_id", alias = "bound_account_id")]
    pub bind_account_id: Option<String>,
    #[serde(default, alias = "extra_args")]
    pub extra_args: Option<String>,
    #[serde(default, alias = "working_dir")]
    pub working_dir: Option<String>,
    #[serde(default, alias = "launch_mode")]
    pub launch_mode: Option<String>,
    #[serde(default, alias = "follow_local_account")]
    pub follow_local_account: Option<bool>,
    #[serde(default, alias = "auto_sync_threads")]
    pub auto_sync_threads: Option<bool>,
    #[serde(default, alias = "last_pid")]
    pub last_pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStore {
    #[serde(default, alias = "profiles")]
    pub instances: Vec<InstanceProfile>,
    #[serde(default, alias = "default_settings")]
    pub default_settings: Option<DefaultInstanceSettings>,
}

impl HardwareFingerprint {
    /// Sinh ngẫu nhiên một bộ định danh phần cứng giả lập mới hoàn toàn.
    pub fn random() -> Self {
        use rand::RngCore;
        use sha2::{Digest, Sha256};

        let mut rng = rand::thread_rng();

        // Sinh machineId ngẫu nhiên (64 hex characters)
        let mut b1 = [0u8; 32];
        rng.fill_bytes(&mut b1);
        let mut hasher1 = Sha256::new();
        hasher1.update(&b1);
        let machine_id = format!("{:x}", hasher1.finalize());

        // Sinh macMachineId ngẫu nhiên (64 hex characters)
        let mut b2 = [0u8; 32];
        rng.fill_bytes(&mut b2);
        let mut hasher2 = Sha256::new();
        hasher2.update(&b2);
        let mac_machine_id = format!("{:x}", hasher2.finalize());

        // Sinh devDeviceId (UUID v4 format)
        let mut u = [0u8; 16];
        rng.fill_bytes(&mut u);
        u[6] = (u[6] & 0x0f) | 0x40; // Version 4
        u[8] = (u[8] & 0x3f) | 0x80; // Variant 10
        let dev_device_id = format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            u[0], u[1], u[2], u[3], u[4], u[5], u[6], u[7], u[8], u[9], u[10], u[11], u[12], u[13], u[14], u[15]
        );

        // Sinh sqmId dạng {UUID}
        let sqm_id = format!("{{{}}}", dev_device_id.to_uppercase());

        Self {
            machine_id,
            mac_machine_id,
            dev_device_id,
            sqm_id,
        }
    }
}
