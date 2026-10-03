//! In-place Credential Injection Module.

pub mod vscode_db;

pub use vscode_db::{
    inject_codebuddy_cn, inject_codebuddy_global, inject_github_copilot, read_item_table_value,
    write_item_table_value,
};
