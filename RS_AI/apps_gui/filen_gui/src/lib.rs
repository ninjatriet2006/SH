// Library facade for the filen_gui core crate.
//
// Exposes the framework-agnostic core modules (operations + transfer) so the
// Tauri shell (`src-tauri`) can depend on them via `filen_gui = { path = "../.." }`.
// This file does NOT modify operations.rs / transfer.rs — it only re-exports them.
pub mod operations;
pub mod transfer;