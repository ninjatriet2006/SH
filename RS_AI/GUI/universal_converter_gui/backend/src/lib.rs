//! UI-independent domain services for Universal Converter.
//!
//! This crate intentionally has no prompt, privilege-escalation, process-exit,
//! current-working-directory, or UI call paths. Callers must provide absolute
//! input and output paths.

mod cancellation;
mod classification;
mod conversion;
mod dependencies;
mod native;
mod path_security;

pub use cancellation::CancellationToken;
pub use classification::{
    Classification, FileType, ScanProgress, ScanReport, classify_file, scan_directory, scan_directory_cancellable,
};
pub use conversion::{
    BatchConvertReport, BatchConvertRequest, ConversionProgress, ConversionRoots, Converter, SystemToolRunner,
    ToolRunner, preflight_batch_conversion,
};
pub use dependencies::{
    DependencyReport, ExecutableProbe, SystemExecutableProbe, check_dependencies, check_dependencies_cancellable,
    check_dependencies_with,
};
pub use native::{
    InstallationRecord, NativeInstallRequest, NativeManager, NativeOperation, NativeOperationResult, NativeProgress,
    NativeUninstallRequest, UserInstallRoots,
};
