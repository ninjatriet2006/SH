//! UI-independent image splitter domain services.
//!
//! Every filesystem entry point requires an explicit absolute path. This crate
//! never changes the process working directory, prompts, elevates privileges,
//! or terminates the process.

mod cancellation;
mod capability;
mod distribution;
#[cfg(target_os = "linux")]
mod distribution_linux;
mod error;
mod path_security;
mod processing;
#[cfg(target_os = "linux")]
mod processing_linux;
mod scan;
mod settings;

pub use cancellation::CancellationToken;
pub use capability::{CapabilityProbe, CapabilityReport, SystemCapabilityProbe, ToolStatus, check_capabilities};
pub use distribution::{
    DistributionMode, DistributionOptions, DistributionProgress, DistributionReport, distribute_images,
};
pub use error::{BackendError, ErrorKind, PartialRollback, Result, RollbackFailure};
pub use processing::{
    MediaToolRunner, ProcessOptions, ProcessProgress, ProcessReport, SystemMediaToolRunner, process_images,
};
pub use scan::{ImageScanReport, ScanProgress, is_supported_image, natural_compare, scan_images};
pub use settings::{ImageSettings, load_settings, save_settings};
