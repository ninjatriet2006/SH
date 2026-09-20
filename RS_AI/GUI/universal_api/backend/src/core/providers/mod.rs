//! Native provider blocks — mỗi provider ngoài có Account + Configuration riêng
//! (theo mẫu CodeBuddy), KHÔNG scheduler. Khối đầu tiên: Zed.
//!
//! Tương lai: `copilot.rs`, `codex.rs`... cùng pattern: auth + models + chat,
//! đăng ký kind trong `ProviderEntry` (`external.rs`).

pub mod zed;
