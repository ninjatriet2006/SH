//! Profile Management & Hardware Emulation Module.

pub mod model;
pub mod spoofing;
pub mod store;

pub use model::{HardwareFingerprint, InstanceProfile};
pub use spoofing::apply_hardware_spoofing;
pub use store::{
    clone_profile, create_profile, delete_profile, get_instances_root_dir, load_profiles,
    save_profiles, update_profile,
};
