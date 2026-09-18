//! Upstream API module — communicates with the WorkBuddy/CodeBuddy backend.
//!
//! Faithful port of Go `internal/upstream` package.

pub mod client;
pub mod errors;
pub mod headers;
pub mod payload;
pub mod realms;
pub mod report;
pub mod sanitize;
pub mod sse;
pub mod thinking;
pub mod travel;

// Re-export primary types for convenience.
pub use client::Client;
pub use errors::{ErrKind, UpstreamError};
