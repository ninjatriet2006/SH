use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u8 = 1;

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

pub fn validate<T>(request: Req<T>) -> Result<(Option<String>, T), IpcError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(error(IpcErrorCode::InvalidArgument, "schema_version must be 1"));
    }
    if request.request_id.is_some() {
        return Err(error(
            IpcErrorCode::InvalidArgument,
            "request_id must be null for this command",
        ));
    }
    Ok((request.request_id, request.payload))
}

pub fn success<T>(request_id: Option<String>, data: T) -> Res<T> {
    Res {
        schema_version: SCHEMA_VERSION,
        request_id,
        data,
    }
}

pub fn backend(message: String) -> IpcError {
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("not found") || lower.contains("không tìm thấy") {
        IpcErrorCode::NotFound
    } else if lower.contains("permission") || lower.contains("phân quyền") {
        IpcErrorCode::Forbidden
    } else if lower.contains("cancel") || lower.contains("huỷ") {
        IpcErrorCode::Cancelled
    } else {
        IpcErrorCode::Io
    };
    error(code, message)
}

pub fn invalid(message: impl Into<String>) -> IpcError {
    error(IpcErrorCode::InvalidArgument, message)
}

pub fn forbidden(message: impl Into<String>) -> IpcError {
    error(IpcErrorCode::Forbidden, message)
}

pub fn internal(message: impl Into<String>) -> IpcError {
    error(IpcErrorCode::Internal, message)
}

pub fn unavailable(message: impl Into<String>) -> IpcError {
    error(IpcErrorCode::Unavailable, message)
}

fn error(code: IpcErrorCode, message: impl Into<String>) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable: false,
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nullable_fields_must_be_present() {
        assert!(serde_json::from_value::<Req<serde_json::Value>>(json!({
            "schema_version": 1, "payload": {}
        }))
        .is_err());
        assert!(serde_json::from_value::<IpcError>(json!({
            "code":"io", "message":"x", "retryable":false
        }))
        .is_err());
    }

    #[test]
    fn rejects_wrong_version_and_non_null_request_id() {
        let request = Req {
            schema_version: 2,
            request_id: None,
            payload: (),
        };
        assert_eq!(validate(request).unwrap_err().code, IpcErrorCode::InvalidArgument);
        let request = Req {
            schema_version: 1,
            request_id: Some("forged".into()),
            payload: (),
        };
        assert_eq!(validate(request).unwrap_err().code, IpcErrorCode::InvalidArgument);
    }
}
