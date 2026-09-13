//! Tauri-free domain services for Universe Manager.
//!
//! All paths and filesystem roots are supplied by the caller. This crate never
//! prompts, elevates privileges, exits, changes the working directory, or
//! executes a command through a shell.

mod cancellation;
mod config;
mod discovery;
mod error;
mod lifecycle;
mod path_security;
mod search;

pub use cancellation::CancellationToken;
pub use config::{AppEntry, ConfigStore, InstallType, ManagerConfig, ManagerSettings, load_config};
pub use discovery::{DetectionReport, DiscoveryService, Progress, stable_app_id};
pub use error::{BackendError, ErrorKind, Result};
pub use lifecycle::{LifecycleManager, OperationResult, ProcessIdentity};
pub use search::{SearchReport, SearchResult, search_apps};
