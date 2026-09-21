/*
[INTEGRITY NOTES]
- Mục đích: Bao thư IPC duy nhất (Req/Res/IpcError) + lính gác cổng (validate)
  cho mọi Tauri command của rclone_gui.
- Trách nhiệm: Điểm giao tiếp duy nhất api ↔ frontend. Không đổi tên lệnh
  Tauri, payload JSON hay IpcError — dời nguyên ngữ nghĩa từ `ipc.rs` sang.
*/
// UNIVERSAL: bao thư + lính gác cổng — dời nguyên từ `ipc.rs` (xóa), macro
// dùng `$crate::` để gọi chéo giữa các cụm `api::*` mà không vỡ đường dẫn.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct Req<T> {
    pub schema_version: u8,
    pub request_id: Option<String>,
    pub payload: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Res<T> {
    pub schema_version: u8,
    pub request_id: Option<String>,
    pub data: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
}

pub type IpcResult<T> = Result<Res<T>, IpcError>;

pub(crate) fn validate<T>(request: &Req<T>) -> Result<Option<String>, IpcError> {
    if request.schema_version != 1 {
        return Err(error(IpcErrorCode::InvalidArgument, "schema_version must be 1"));
    }
    if request.request_id.is_some() {
        return Err(error(
            IpcErrorCode::InvalidArgument,
            "request_id must be null for non-job commands",
        ));
    }
    Ok(request.request_id.clone())
}

pub(crate) fn success<T>(request_id: Option<String>, data: T) -> Res<T> {
    Res {
        schema_version: 1,
        request_id,
        data,
    }
}

pub(crate) fn error(code: IpcErrorCode, message: impl Into<String>) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable: matches!(code, IpcErrorCode::Unavailable | IpcErrorCode::Io),
        details: None,
    }
}

pub(crate) fn backend_error(message: String) -> IpcError {
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("not found") || lower.contains("không tìm thấy") {
        IpcErrorCode::NotFound
    } else if lower.contains("invalid") || lower.contains("không hợp lệ") || lower.contains("thiếu ") {
        IpcErrorCode::InvalidArgument
    } else if lower.contains("cancel") || lower.contains("huỷ") {
        IpcErrorCode::Cancelled
    } else if lower.contains("permission") || lower.contains("quyền") || lower.contains("forbidden") {
        IpcErrorCode::Forbidden
    } else {
        IpcErrorCode::Io
    };
    error(code, message)
}

/// UNIVERSAL: payload rỗng dùng chung cho lệnh không tham số — dời từ `ipc.rs`.
#[derive(Deserialize)]
pub struct Empty {}

#[macro_export]
macro_rules! payload {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        #[derive(serde::Deserialize)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

#[macro_export]
macro_rules! async_command {
    ($name:ident, $payload:ty, $output:ty, $target:path, ($($field:ident),* $(,)?)) => {
        #[tauri::command]
        pub async fn $name(
            request: $crate::api::envelope::Req<$payload>,
        ) -> $crate::api::envelope::IpcResult<$output> {
            let request_id = $crate::api::envelope::validate(&request)?;
            #[allow(unused_variables)]
            let payload = request.payload;
            $target($(payload.$field),*)
                .await
                .map(|data| $crate::api::envelope::success(request_id, data))
                .map_err($crate::api::envelope::backend_error)
        }
    };
}

#[macro_export]
macro_rules! sync_command {
    ($name:ident, $payload:ty, $output:ty, $target:path, ($($field:ident),* $(,)?)) => {
        #[tauri::command]
        pub fn $name(
            request: $crate::api::envelope::Req<$payload>,
        ) -> $crate::api::envelope::IpcResult<$output> {
            let request_id = $crate::api::envelope::validate(&request)?;
            #[allow(unused_variables)]
            let payload = request.payload;
            $target($(payload.$field),*)
                .map(|data| $crate::api::envelope::success(request_id, data))
                .map_err($crate::api::envelope::backend_error)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serializes_exact_contract() {
        let value = serde_json::to_value(success(None, true)).expect("serialize response");
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version": 1,
                "request_id": null,
                "data": true
            })
        );

        let failure = error(IpcErrorCode::InvalidArgument, "bad request");
        assert_eq!(
            serde_json::to_value(failure).expect("serialize error"),
            serde_json::json!({
                "code": "invalid_argument",
                "message": "bad request",
                "retryable": false,
                "details": null
            })
        );
    }

    #[test]
    fn rejects_wrong_schema_and_non_job_request_id() {
        let wrong_schema = Req {
            schema_version: 2,
            request_id: None,
            payload: Empty {},
        };
        assert_eq!(
            validate(&wrong_schema).expect_err("schema must fail").code,
            IpcErrorCode::InvalidArgument
        );

        let job_id = Req {
            schema_version: 1,
            request_id: Some("unexpected".into()),
            payload: Empty {},
        };
        assert_eq!(
            validate(&job_id).expect_err("request id must fail").code,
            IpcErrorCode::InvalidArgument
        );
    }
}
