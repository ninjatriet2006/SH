pub mod font_api;
pub mod lang_api;
pub mod models;
pub mod package_api;
pub mod payment_api;
pub mod settings_api;
pub mod storage;
pub mod subscription_api;
pub mod theme_api;
pub mod transaction_api;
pub mod user_api;
pub mod utils;

use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u8 = 1;

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

fn ipc_error(code: IpcErrorCode, message: impl Into<String>, retryable: bool) -> IpcError {
    IpcError {
        code,
        message: message.into(),
        retryable,
        details: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Create,
    Read,
    Update,
    Delete,
    DeletePackage,
    Persist,
}

impl Operation {
    fn error_code(self) -> IpcErrorCode {
        match self {
            Self::Create => IpcErrorCode::Validation,
            Self::Read => IpcErrorCode::Io,
            Self::Update | Self::Delete => IpcErrorCode::NotFound,
            Self::DeletePackage => IpcErrorCode::Conflict,
            Self::Persist => IpcErrorCode::Io,
        }
    }
}

fn command_result<T, P>(
    request: Req<P>,
    operation: Operation,
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
            let code = operation.error_code();
            ipc_error(code, message, code == IpcErrorCode::Io)
        })
}

macro_rules! request_dto {
    ($name:ident { $($(#[$attr:meta])* $field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $name {
            $($(#[$attr])* pub $field: $ty),*
        }
    };
}

request_dto!(AddUserRequest {
    username: String,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    email: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    phone: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    contact_url: Option<String>,
});
request_dto!(UpdateUserRequest {
    id: String,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    username: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    email: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    phone: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    contact_url: Option<String>,
});
request_dto!(IdRequest { id: String });
request_dto!(PageRequest {
    #[serde(deserialize_with = "deserialize_present_nullable")]
    page: Option<u32>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    limit: Option<u32>,
});
request_dto!(AdjustUserBalanceRequest {
    id: String,
    delta: i64,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    note: Option<String>,
});
request_dto!(AddPackageRequest {
    name: String,
    duration_days: u32,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    description: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    price: Option<i64>,
});
request_dto!(UpdatePackageRequest {
    id: String,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    name: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    duration_days: Option<u32>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    description: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    price: Option<i64>,
});
request_dto!(AddSubscriptionRequest {
    user_id: String,
    package_id: String,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    custom_expiration_date: Option<String>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    amount: Option<i64>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    auto_renew: Option<bool>,
});
request_dto!(UpdateSubscriptionRequest {
    subscription_id: String,
    new_expiration_date: String,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    amount: Option<i64>,
    #[serde(deserialize_with = "deserialize_present_nullable")]
    auto_renew: Option<bool>,
});
request_dto!(SubscriptionIdRequest {
    subscription_id: String,
});
request_dto!(UserIdRequest { user_id: String });
request_dto!(SetAutoRenewRequest {
    subscription_id: String,
    auto_renew: bool,
});
request_dto!(IssuePaymentRefRequest {
    user_id: String,
    transaction_ids: Vec<String>,
});
request_dto!(LookupPaymentRefRequest { input: String });
request_dto!(ListPaymentRefsRequest {
    #[serde(deserialize_with = "deserialize_present_nullable")]
    user_id: Option<String>,
});
request_dto!(SettlePaymentRefRequest {
    code: String,
    settled: bool,
});
request_dto!(SaveSettingsRequest {
    language: String,
    timezone: String,
    theme_id: String,
    font_id: String,
});
request_dto!(ExportBackupRequest { destination: String });
request_dto!(LangContentRequest { lang_code: String });

mod commands {
    use crate::*;

    fn optional_non_negative(value: Option<i64>, field: &str) -> Result<Option<u64>, String> {
        value
            .map(|value| u64::try_from(value).map_err(|_| format!("{field} must be non-negative")))
            .transpose()
    }

    fn timestamp(value: String, field: &str) -> Result<i64, String> {
        value
            .parse()
            .map_err(|_| format!("{field} must be a base-10 millisecond timestamp"))
    }

    fn optional_timestamp(value: Option<String>, field: &str) -> Result<Option<i64>, String> {
        value.map(|value| timestamp(value, field)).transpose()
    }

    macro_rules! command {
        ($wrapper:ident, $name:literal, $request:ty, $response:ty, $operation:expr, |$payload:ident| $body:expr) => {
            #[tauri::command(rename_all = "snake_case", rename = $name)]
            pub fn $wrapper(request: Req<$request>) -> IpcResult<$response> {
                command_result(request, $operation, |$payload| $body)
            }
        };
    }

    command!(
        ipc_add_user,
        "add_user",
        AddUserRequest,
        models::User,
        Operation::Create,
        |p| user_api::add_user(p.username, p.email, p.phone, p.contact_url)
    );
    command!(
        ipc_update_user,
        "update_user",
        UpdateUserRequest,
        models::User,
        Operation::Update,
        |p| user_api::update_user(p.id, p.username, p.email, p.phone, p.contact_url)
    );
    command!(ipc_delete_user, "delete_user", IdRequest, (), Operation::Delete, |p| {
        user_api::delete_user(p.id)
    });
    command!(
        ipc_list_users,
        "list_users",
        PageRequest,
        Vec<models::User>,
        Operation::Read,
        |p| user_api::list_users(p.page, p.limit)
    );
    command!(
        ipc_adjust_user_balance,
        "adjust_user_balance",
        AdjustUserBalanceRequest,
        models::User,
        Operation::Update,
        |p| user_api::adjust_user_balance(p.id, p.delta, p.note)
    );

    command!(
        ipc_add_package,
        "add_package",
        AddPackageRequest,
        models::Package,
        Operation::Create,
        |p| {
            package_api::add_package(
                p.name,
                p.duration_days,
                p.description,
                optional_non_negative(p.price, "price")?,
            )
        }
    );
    command!(
        ipc_update_package,
        "update_package",
        UpdatePackageRequest,
        models::Package,
        Operation::Update,
        |p| {
            package_api::update_package(
                p.id,
                p.name,
                p.duration_days,
                p.description,
                optional_non_negative(p.price, "price")?,
            )
        }
    );
    command!(
        ipc_delete_package,
        "delete_package",
        IdRequest,
        (),
        Operation::DeletePackage,
        |p| package_api::delete_package(p.id)
    );
    command!(
        ipc_list_packages,
        "list_packages",
        PageRequest,
        Vec<models::Package>,
        Operation::Read,
        |p| package_api::list_packages(p.page, p.limit)
    );

    command!(
        ipc_add_subscription_to_user,
        "add_subscription_to_user",
        AddSubscriptionRequest,
        models::Subscription,
        Operation::Create,
        |p| {
            subscription_api::add_subscription_to_user(
                p.user_id,
                p.package_id,
                optional_timestamp(p.custom_expiration_date, "custom_expiration_date")?,
                optional_non_negative(p.amount, "amount")?,
                p.auto_renew,
            )
        }
    );
    command!(
        ipc_update_subscription_expiry,
        "update_subscription_expiry",
        UpdateSubscriptionRequest,
        models::Subscription,
        Operation::Update,
        |p| {
            subscription_api::update_subscription_expiry(
                p.subscription_id,
                timestamp(p.new_expiration_date, "new_expiration_date")?,
                optional_non_negative(p.amount, "amount")?,
                p.auto_renew,
            )
        }
    );
    command!(
        ipc_remove_subscription_from_user,
        "remove_subscription_from_user",
        SubscriptionIdRequest,
        (),
        Operation::Delete,
        |p| subscription_api::remove_subscription_from_user(p.subscription_id)
    );
    command!(
        ipc_list_user_subscriptions,
        "list_user_subscriptions",
        UserIdRequest,
        Vec<models::Subscription>,
        Operation::Read,
        |p| subscription_api::list_user_subscriptions(p.user_id)
    );
    command!(
        ipc_check_subscription_status,
        "check_subscription_status",
        SubscriptionIdRequest,
        bool,
        Operation::Read,
        |p| subscription_api::check_subscription_status(p.subscription_id)
    );
    command!(
        ipc_list_all_subscriptions,
        "list_all_subscriptions",
        Empty,
        Vec<models::Subscription>,
        Operation::Read,
        |_p| subscription_api::list_all_subscriptions()
    );
    command!(
        ipc_process_auto_renewals,
        "process_auto_renewals",
        Empty,
        subscription_api::AutoRenewReport,
        Operation::Persist,
        |_p| subscription_api::process_auto_renewals()
    );
    command!(
        ipc_set_subscription_auto_renew,
        "set_subscription_auto_renew",
        SetAutoRenewRequest,
        models::Subscription,
        Operation::Update,
        |p| subscription_api::set_subscription_auto_renew(p.subscription_id, p.auto_renew)
    );

    command!(
        ipc_list_user_transactions,
        "list_user_transactions",
        UserIdRequest,
        Vec<models::Transaction>,
        Operation::Read,
        |p| transaction_api::list_user_transactions(p.user_id)
    );
    command!(
        ipc_list_all_transactions,
        "list_all_transactions",
        Empty,
        Vec<models::Transaction>,
        Operation::Read,
        |_p| transaction_api::list_all_transactions()
    );
    command!(
        ipc_delete_transaction,
        "delete_transaction",
        IdRequest,
        (),
        Operation::Delete,
        |p| transaction_api::delete_transaction(p.id)
    );
    command!(
        ipc_issue_payment_ref,
        "issue_payment_ref",
        IssuePaymentRefRequest,
        models::PaymentRef,
        Operation::Create,
        |p| payment_api::issue_payment_ref(p.user_id, p.transaction_ids)
    );
    command!(
        ipc_lookup_payment_ref,
        "lookup_payment_ref",
        LookupPaymentRefRequest,
        payment_api::PaymentLookup,
        Operation::Read,
        |p| payment_api::lookup_payment_ref(p.input)
    );
    command!(
        ipc_list_payment_refs,
        "list_payment_refs",
        ListPaymentRefsRequest,
        Vec<models::PaymentRef>,
        Operation::Read,
        |p| payment_api::list_payment_refs(p.user_id)
    );
    command!(
        ipc_settle_payment_ref,
        "settle_payment_ref",
        SettlePaymentRefRequest,
        models::PaymentRef,
        Operation::Update,
        |p| payment_api::settle_payment_ref(p.code, p.settled)
    );

    command!(
        ipc_get_settings,
        "get_settings",
        Empty,
        settings_api::Settings,
        Operation::Read,
        |_p| settings_api::get_settings()
    );
    command!(
        ipc_save_settings,
        "save_settings",
        SaveSettingsRequest,
        (),
        Operation::Persist,
        |p| settings_api::save_settings(p.language, p.timezone, p.theme_id, p.font_id)
    );
    command!(
        ipc_export_backup,
        "export_backup",
        ExportBackupRequest,
        (),
        Operation::Persist,
        |p| storage::export_backup(p.destination)
    );
    command!(
        ipc_get_available_langs,
        "get_available_langs",
        Empty,
        Vec<String>,
        Operation::Read,
        |_p| lang_api::get_available_langs()
    );
    command!(
        ipc_get_lang_content,
        "get_lang_content",
        LangContentRequest,
        serde_json::Value,
        Operation::Read,
        |p| lang_api::get_lang_content(p.lang_code)
    );
    command!(
        ipc_get_available_themes,
        "get_available_themes",
        Empty,
        Vec<models::Theme>,
        Operation::Read,
        |_p| theme_api::get_available_themes()
    );
    command!(
        ipc_get_available_fonts,
        "get_available_fonts",
        Empty,
        Vec<models::FontInfo>,
        Operation::Read,
        |_p| font_api::get_available_fonts()
    );
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;

            if let Ok(resource_dir) = app.path().resource_dir() {
                std::env::set_var("SUBSCRIPTION_MANAGER_RESOURCE_DIR", resource_dir);
            }
            utils::init_time_sync();
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::ipc_add_user,
            commands::ipc_update_user,
            commands::ipc_delete_user,
            commands::ipc_list_users,
            commands::ipc_adjust_user_balance,
            commands::ipc_add_package,
            commands::ipc_update_package,
            commands::ipc_delete_package,
            commands::ipc_list_packages,
            commands::ipc_add_subscription_to_user,
            commands::ipc_update_subscription_expiry,
            commands::ipc_remove_subscription_from_user,
            commands::ipc_list_user_subscriptions,
            commands::ipc_check_subscription_status,
            commands::ipc_list_all_subscriptions,
            commands::ipc_process_auto_renewals,
            commands::ipc_set_subscription_auto_renew,
            commands::ipc_list_user_transactions,
            commands::ipc_list_all_transactions,
            commands::ipc_delete_transaction,
            commands::ipc_issue_payment_ref,
            commands::ipc_lookup_payment_ref,
            commands::ipc_list_payment_refs,
            commands::ipc_settle_payment_ref,
            commands::ipc_get_settings,
            commands::ipc_save_settings,
            commands::ipc_export_backup,
            commands::ipc_get_available_langs,
            commands::ipc_get_lang_content,
            commands::ipc_get_available_themes,
            commands::ipc_get_available_fonts,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use serde_json::json;

    const REGISTERED_COMMANDS: &[&str] = &[
        "add_user",
        "update_user",
        "delete_user",
        "list_users",
        "adjust_user_balance",
        "add_package",
        "update_package",
        "delete_package",
        "list_packages",
        "add_subscription_to_user",
        "update_subscription_expiry",
        "remove_subscription_from_user",
        "list_user_subscriptions",
        "check_subscription_status",
        "list_all_subscriptions",
        "process_auto_renewals",
        "set_subscription_auto_renew",
        "list_user_transactions",
        "list_all_transactions",
        "delete_transaction",
        "issue_payment_ref",
        "lookup_payment_ref",
        "list_payment_refs",
        "settle_payment_ref",
        "get_settings",
        "save_settings",
        "export_backup",
        "get_available_langs",
        "get_lang_content",
        "get_available_themes",
        "get_available_fonts",
    ];

    #[test]
    fn registered_command_names_are_preserved() {
        let source = include_str!("lib.rs");
        let handler = source
            .split_once("generate_handler![")
            .expect("registered handler")
            .1
            .split_once(']')
            .expect("closed handler")
            .0;

        for command in REGISTERED_COMMANDS {
            assert!(
                handler.contains(&format!("commands::ipc_{command}")),
                "missing registered wrapper for {command}"
            );
        }

        for command in REGISTERED_COMMANDS {
            assert!(source.contains(&format!("ipc_{command},")));
        }
    }

    #[test]
    fn registered_wrappers_return_typed_ipc_results() {
        let source = include_str!("lib.rs");
        for command in REGISTERED_COMMANDS {
            assert!(
                source.contains(&format!("ipc_{command},")),
                "missing command wrapper for {command}"
            );
        }
        let macro_body = source
            .split_once("macro_rules! command")
            .expect("command macro")
            .1
            .split_once("command!(ipc_add_user")
            .expect("first wrapper")
            .0;
        assert!(macro_body.contains("request: Req<$request>"));
        assert!(macro_body.contains("-> IpcResult<$response>"));
    }

    #[test]
    fn envelope_and_error_match_phase_2_a1() {
        let response = Res {
            schema_version: SCHEMA_VERSION,
            request_id: None,
            data: Empty {},
        };
        assert_eq!(
            serde_json::to_value(response).expect("serialize response"),
            json!({"schema_version": 1, "request_id": null, "data": {}})
        );

        let error = ipc_error(IpcErrorCode::InvalidArgument, "bad request", false);
        assert_eq!(
            serde_json::to_value(error).expect("serialize error"),
            json!({
                "code": "invalid_argument",
                "message": "bad request",
                "retryable": false,
                "details": null
            })
        );
    }

    #[test]
    fn request_requires_nullable_request_id_to_be_present() {
        assert!(serde_json::from_value::<Req<Empty>>(json!({
            "schema_version": 1,
            "payload": {}
        }))
        .is_err());
        assert!(serde_json::from_value::<Req<Empty>>(json!({
            "schema_version": 1,
            "request_id": null,
            "payload": {}
        }))
        .is_ok());
    }

    #[test]
    fn command_result_echoes_request_id_and_rejects_unknown_schema() {
        let request = Req {
            schema_version: SCHEMA_VERSION,
            request_id: Some("request-1".to_owned()),
            payload: Empty {},
        };
        let response = command_result(request, Operation::Read, |_| Ok::<_, String>(42)).expect("valid response");
        assert_eq!(response.request_id.as_deref(), Some("request-1"));
        assert_eq!(response.data, 42);

        let invalid_request = Req {
            schema_version: 2,
            request_id: None,
            payload: Empty {},
        };
        let error =
            command_result(invalid_request, Operation::Read, |_| Ok::<_, String>(())).expect_err("invalid schema");
        assert_eq!(error.code, IpcErrorCode::InvalidArgument);
    }

    #[test]
    fn operation_error_mapping_does_not_depend_on_message_text() {
        let request = Req {
            schema_version: SCHEMA_VERSION,
            request_id: None,
            payload: Empty {},
        };
        let error = command_result(request, Operation::Delete, |_| Err::<(), _>("arbitrary".to_owned()))
            .expect_err("delete failure");
        assert_eq!(error.code, IpcErrorCode::NotFound);
        assert!(!error.retryable);
    }

    #[test]
    fn nullable_dto_fields_must_be_present() {
        assert!(serde_json::from_value::<AddUserRequest>(json!({
            "username": "user",
            "phone": null,
            "contact_url": null
        }))
        .is_err());
        assert!(serde_json::from_value::<AddUserRequest>(json!({
            "username": "user",
            "email": null,
            "phone": null,
            "contact_url": null
        }))
        .is_ok());
    }
}
