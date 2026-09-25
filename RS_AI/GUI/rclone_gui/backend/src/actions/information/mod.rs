//! Nhóm Information Actions: kiểm tra dung lượng (`about`) và kích thước (`size`).
//! Bọc qua fastlane tại `api::remote_manager`.

pub mod about;
pub mod size;

pub use about::{AboutPlan, about, execute_about, plan_about};
pub use size::{SizePlan, execute_size, plan_size, size};
