use serde::{Deserialize, Serialize};

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
pub struct PartialRollbackDetails {
    pub failures: Vec<RollbackFailureDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackFailureDetails {
    pub from: PathRef,
    pub to: PathRef,
    pub cause: String,
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
pub struct AppEvent<T> {
    pub schema_version: u8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImagePickerKind {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagePickerSelectRequest {
    pub kind: ImagePickerKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagePickerSelectResult {
    pub kind: ImagePickerKind,
    pub path: PathRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionMode {
    Balanced,
    Greedy,
    Fixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSettings {
    pub default_distribution_mode: DistributionMode,
    pub max_files_per_folder: u64,
    pub fixed_folder_count: u64,
    pub max_retries: u64,
    pub min_upscale_width: u32,
    pub target_upscale_width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub ffmpeg: ToolStatus,
    pub ffprobe: ToolStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanImagesRequest {
    pub directory: PathRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageScanReport {
    pub directory: PathRef,
    pub images: Vec<PathRef>,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessImagesRequest {
    pub input_directory: PathRef,
    pub files: Vec<PathRef>,
    pub output_directory: PathRef,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub output_format: Option<String>,
    pub upscale: bool,
    pub settings: ImageSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessReport {
    pub output_directory: PathRef,
    pub processed: Vec<PathRef>,
    pub failed: Vec<PathRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributeRequest {
    pub input_directory: PathRef,
    pub files: Vec<PathRef>,
    pub output_directory: PathRef,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub chapter: Option<u32>,
    pub mode: DistributionMode,
    pub max_files_per_folder: u64,
    pub fixed_folder_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributionReport {
    pub output_directory: PathRef,
    pub folders: Vec<PathRef>,
    pub distributed: u64,
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
            language: "en".to_owned(),
            theme: "dark".to_owned(),
            font_id: "system-default".to_owned(),
        }
    }
}
