use serde::{Deserialize, Serialize};

pub use universe_manager_backend::{AppEntry, ManagerConfig};

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Req<T> {
    pub schema_version: u8,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub request_id: Option<String>,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Res<T> {
    pub schema_version: u8,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub request_id: Option<String>,
    pub data: T,
}

fn deserialize_present_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcErrorCode {
    InvalidArgument,
    NotFound,
    Conflict,
    Unauthorized,
    Forbidden,
    Unavailable,
    Io,
    Validation,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JobResult<T> {
    Completed { request_id: String, value: T },
    Failed { request_id: String, error: IpcError },
    Cancelled { request_id: String, error: IpcError },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Started,
    Progress,
    Completed,
    Failed,
    Cancelled,
}

impl JobState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobEvent<T> {
    pub schema_version: u8,
    pub job_id: String,
    pub seq: u64,
    pub state: JobState,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct JobProgress<T> {
    pub request_id: String,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub completed: Option<u64>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub total: Option<u64>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub message: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub result: Option<JobResult<T>>,
}

pub type IpcResult<T> = Result<Res<T>, IpcError>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Empty {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathRef {
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UniversePickerKind {
    Managed,
    Source,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversePickerSelectRequest {
    pub kind: UniversePickerKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UniversePickerSelectResult {
    pub kind: UniversePickerKind,
    pub path: PathRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectAppRequest {
    pub path: PathRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionReport {
    pub is_appimage: bool,
    pub suggested_name: String,
    pub executables: Vec<PathRef>,
    pub icons: Vec<PathRef>,
    pub desktop_templates: Vec<PathRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppActionRequest {
    pub app_id: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationResult {
    pub app_id: String,
    pub operation: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchAppsRequest {
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchReport {
    pub query: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub language: String,
    pub theme: String,
    pub font_id: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: "vi".to_owned(),
            theme: "system".to_owned(),
            font_id: "system-default".to_owned(),
        }
    }
}
