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

impl<T> Req<T> {
    pub fn validate(self) -> Result<(Option<String>, T), IpcError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(IpcError::new(
                IpcErrorCode::InvalidArgument,
                format!("schema_version phải là {SCHEMA_VERSION}"),
            ));
        }
        if self.request_id.is_some() {
            return Err(IpcError::new(
                IpcErrorCode::InvalidArgument,
                "request_id phải là null cho command không tạo job",
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
    let code = if lower.contains("không tìm thấy") || lower.contains("không tồn tại") {
        IpcErrorCode::NotFound
    } else if lower.contains("không hợp lệ")
        || lower.contains("không được để trống")
        || lower.contains("vui lòng")
        || lower.contains("cần cả")
        || lower.contains("chưa chọn")
    {
        IpcErrorCode::InvalidArgument
    } else if lower.contains("đã được dùng") || lower.contains("trùng") {
        IpcErrorCode::Conflict
    } else if lower.contains("http") || lower.contains("kết nối") || lower.contains("api") {
        IpcErrorCode::Unavailable
    } else if lower.contains("đọc") || lower.contains("ghi") || lower.contains("file") {
        IpcErrorCode::Io
    } else {
        IpcErrorCode::Internal
    };
    IpcError::new(code, message)
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Empty {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_a1_error_shape_and_codes() {
        let error = IpcError::new(IpcErrorCode::InvalidArgument, "bad request");
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!({
                "code": "invalid_argument",
                "message": "bad request",
                "retryable": false,
                "details": null
            })
        );
    }

    #[test]
    fn envelope_keeps_null_request_id() {
        let response = respond(None, 7_u8);
        assert_eq!(
            serde_json::to_value(response).unwrap(),
            serde_json::json!({"schema_version": 1, "request_id": null, "data": 7})
        );
    }

    #[test]
    fn nullable_fields_must_be_present() {
        assert!(serde_json::from_value::<Req<Empty>>(serde_json::json!({
            "schema_version": 1, "payload": {}
        }))
        .is_err());
        assert!(serde_json::from_value::<IpcError>(serde_json::json!({
            "code": "internal", "message": "x", "retryable": false
        }))
        .is_err());
        assert!(
            serde_json::from_value::<crate::api::models::SetPrimaryModelRequest>(serde_json::json!({
                "provider_id": null
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<crate::api::models::SetPrimaryModelRequest>(serde_json::json!({
                "provider_id": null, "model_id": null
            }))
            .is_ok()
        );
    }

    #[test]
    fn request_id_is_propagated_and_schema_is_validated() {
        let request = Req {
            schema_version: SCHEMA_VERSION,
            request_id: Some("req-42".to_string()),
            payload: Empty {},
        };
        assert_eq!(request.validate().unwrap_err().code, IpcErrorCode::InvalidArgument);

        let invalid = Req {
            schema_version: 2,
            request_id: None,
            payload: Empty {},
        }
        .validate()
        .unwrap_err();
        assert_eq!(invalid.code, IpcErrorCode::InvalidArgument);
    }
}
