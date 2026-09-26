/*
[INTEGRITY NOTES]
- Mục đích: Cung cấp giao thức Enveloped IPC Pattern (A.1 Contract) chuẩn hóa theo `subscription_manager_gui`.
- Trách nhiệm: Đóng gói và bóc tách `Req<T>` / `Res<T>`, quản lý mã lỗi máy học được `IpcErrorCode`.
- Tương tác: Dùng trong toàn bộ tầng `api` và định tuyến tại `lib.rs`.
*/

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Req<T> {
    pub schema_version: u8,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub request_id: Option<String>,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Res<T> {
    pub schema_version: u8,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub request_id: Option<String>,
    pub data: T,
}

pub fn deserialize_present_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcError {
    pub code: IpcErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    pub details: Option<serde_json::Value>,
}

pub type IpcResult<T> = Result<Res<T>, IpcError>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Empty {}

pub fn ipc_error(code: IpcErrorCode, message: impl Into<String>, retryable: bool) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable,
        details: None,
    }
}

pub fn command_result<T, P>(
    request: Req<P>,
    default_err_code: IpcErrorCode,
    run: impl FnOnce(P) -> Result<T, String>,
) -> IpcResult<T> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(ipc_error(
            IpcErrorCode::InvalidArgument,
            format!("Unsupported IPC schema version: {}", request.schema_version),
            false,
        ));
    }

    let request_id = request.request_id;
    run(request.payload)
        .map(|data| Res {
            schema_version: SCHEMA_VERSION,
            request_id,
            data,
        })
        .map_err(|message| {
            ipc_error(default_err_code, message, default_err_code == IpcErrorCode::Io)
        })
}

pub async fn async_command_result<T, P, F, Fut>(
    request: Req<P>,
    default_err_code: IpcErrorCode,
    run: F,
) -> IpcResult<T>
where
    F: FnOnce(P) -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    if request.schema_version != SCHEMA_VERSION {
        return Err(ipc_error(
            IpcErrorCode::InvalidArgument,
            format!("Unsupported IPC schema version: {}", request.schema_version),
            false,
        ));
    }

    let request_id = request.request_id;
    run(request.payload)
        .await
        .map(|data| Res {
            schema_version: SCHEMA_VERSION,
            request_id,
            data,
        })
        .map_err(|message| {
            ipc_error(default_err_code, message, default_err_code == IpcErrorCode::Io)
        })
}
