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
mod process_snapshot;
mod search;

pub mod sources;

pub use cancellation::CancellationToken;
pub use config::{AppEntry, ConfigStore, InstallType, ManagerConfig, ManagerSettings, load_config};
pub use discovery::{
    DetectionReport, DiscoveryService, Progress, parse_desktop_file, scan_all_applications,
    scan_system_applications, stable_app_id,
};
pub use error::{BackendError, ErrorKind, Result};
pub use lifecycle::{LifecycleManager, OperationResult, ProcessIdentity};
pub use process_snapshot::ProcessSnapshot;
pub use search::{SearchReport, SearchResult, search_apps};
pub use sources::{AppSource, SourceKind, SourceRegistry};
