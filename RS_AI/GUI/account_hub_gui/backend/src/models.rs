use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCriterion {
    pub key: String,
    pub label: String,
    pub value_type: String, // "boolean" | "text"
    pub value: String,       // "true"/"false" or custom text
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Website {
    pub id: String,
    pub name: String,
    pub url: String,
    pub category: String,
    pub has_daily_checkin: bool,
    pub can_cheat_account: bool,
    pub requires_kyc: bool,
    pub requires_proxy: bool,
    pub custom_criteria: Vec<CustomCriterion>,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAccount {
    pub id: String,
    pub email: String,
    pub owner: String,
    pub recovery_email: String,
    pub phone: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Live,
    Die,
}

impl Default for AccountStatus {
    fn default() -> Self {
        AccountStatus::Live
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationRecord {
    pub id: String,
    pub email_id: String,
    pub website_id: String,
    pub is_registered: bool,
    pub status: AccountStatus, // Live / Die
    pub is_checked_in: bool,   // Đã điểm danh HÔM NAY chưa (tự reset khi sang ngày mới)
    pub registered_at: Option<String>,
    pub checkin_streak: u32,
    pub last_checkin_at: Option<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub lang: String,
    pub theme: String,
    pub font: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            lang: "vi".to_string(),
            theme: "default".to_string(),
            font: "DejaVuSans".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppDatabase {
    pub emails: Vec<EmailAccount>,
    pub websites: Vec<Website>,
    pub registrations: Vec<RegistrationRecord>,
    pub settings: AppSettings,
}
