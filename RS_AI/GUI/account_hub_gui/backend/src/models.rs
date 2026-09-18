use serde::{Deserialize, Serialize};

/// Current schema version (v2 per plan-criteria-refactor Contract).
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

fn default_schema_version_v1() -> u32 {
    1
}

/// A reusable criterion that can be assigned to websites (e.g. KYC, Proxy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: String,
}

/// A login method that can be assigned to websites (e.g. Email, Wallet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginMethod {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Website {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub has_daily_checkin: bool,
    #[serde(default)]
    pub criterion_ids: Vec<String>,
    #[serde(default)]
    pub login_method_ids: Vec<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDatabase {
    pub emails: Vec<EmailAccount>,
    pub websites: Vec<Website>,
    pub registrations: Vec<RegistrationRecord>,
    pub settings: AppSettings,
    #[serde(default)]
    pub criteria: Vec<Criterion>,
    #[serde(default)]
    pub login_methods: Vec<LoginMethod>,
    #[serde(default = "default_schema_version_v1")]
    pub schema_version: u32,
}

impl Default for AppDatabase {
    fn default() -> Self {
        Self {
            emails: vec![],
            websites: vec![],
            registrations: vec![],
            settings: AppSettings::default(),
            criteria: vec![],
            login_methods: vec![],
            schema_version: CURRENT_SCHEMA_VERSION,
        }
    }
}

/// Legacy v1 shapes — ONLY used by the v1→v2 migration.
/// A5 exception: old field names may appear here and in `storage::migrate_*`.
pub mod legacy_v1 {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CustomCriterionV1 {
        pub key: String,
        pub label: String,
        pub value_type: String,
        pub value: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WebsiteV1 {
        pub id: String,
        #[serde(default)]
        pub name: String,
        #[serde(default)]
        pub url: String,
        #[serde(default)]
        pub category: String,
        #[serde(default)]
        pub has_daily_checkin: bool,
        #[serde(default)]
        pub can_cheat_account: bool,
        #[serde(default)]
        pub requires_kyc: bool,
        #[serde(default)]
        pub requires_proxy: bool,
        #[serde(default)]
        pub custom_criteria: Vec<CustomCriterionV1>,
        #[serde(default)]
        pub notes: String,
        #[serde(default)]
        pub created_at: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DatabaseV1 {
        #[serde(default)]
        pub emails: Vec<crate::models::EmailAccount>,
        #[serde(default)]
        pub websites: Vec<WebsiteV1>,
        #[serde(default)]
        pub registrations: Vec<crate::models::RegistrationRecord>,
        #[serde(default)]
        pub settings: Option<crate::models::AppSettings>,
    }
}
