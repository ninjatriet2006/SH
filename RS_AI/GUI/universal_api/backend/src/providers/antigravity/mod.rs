//! Antigravity Provider module.
//! Sub-tasks:
//! - accounts: CRUD, tags, notes, reorder, import/export
//! - switch: active account switching and switch history
//! - quota: quota checking and refresh
//! - oauth: Google OAuth flow
//! - ide: Antigravity IDE injection

pub mod accounts;
pub mod ide;
pub mod oauth;
pub mod quota;
pub mod switch;
