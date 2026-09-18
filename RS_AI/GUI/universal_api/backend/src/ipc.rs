use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Empty {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Req<T> {
    pub schema_version: u8,
    #[serde(deserialize_with = "present_nullable")]
    pub request_id: Option<String>,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Res<T> {
    pub schema_version: u8,
    #[serde(deserialize_with = "present_nullable")]
    pub request_id: Option<String>,
    pub data: T,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(deserialize_with = "present_nullable")]
    pub details: Option<serde_json::Value>,
}

pub type IpcResult<T> = Result<Res<T>, IpcError>;

pub(crate) fn present_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

impl<T> Req<T> {
    pub fn validate(self) -> Result<(Option<String>, T), IpcError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(IpcError::new(
                IpcErrorCode::InvalidArgument,
                format!("schema_version must be {SCHEMA_VERSION}"),
            ));
        }
        Ok((self.request_id, self.payload))
    }
}

impl IpcError {
    pub fn new(code: IpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: matches!(code, IpcErrorCode::Unavailable | IpcErrorCode::Io),
            details: None,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(IpcErrorCode::Internal, message)
    }
}

pub fn respond<T>(request_id: Option<String>, data: T) -> Res<T> {
    Res {
        schema_version: SCHEMA_VERSION,
        request_id,
        data,
    }
}

pub fn from_string(message: String) -> IpcError {
    let lower = message.to_lowercase();
    let code = if lower.contains("not found") {
        IpcErrorCode::NotFound
    } else if lower.contains("invalid") {
        IpcErrorCode::InvalidArgument
    } else if lower.contains("unauthorized") {
        IpcErrorCode::Unauthorized
    } else {
        IpcErrorCode::Internal
    };
    IpcError::new(code, message)
}
