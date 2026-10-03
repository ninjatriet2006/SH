//! Instance Discovery & Process Controls Module.

pub mod discovery;
pub mod launcher;

pub use discovery::find_executable;
pub use launcher::{kill_instance, launch_instance, LaunchOptions};
