//! Profile Management & Hardware Emulation Module.

pub mod model;
pub mod spoofing;
pub mod store;

pub use model::{HardwareFingerprint, InstanceProfile};
pub use spoofing::apply_hardware_spoofing;
pub use store::{
    clone_profile, create_platform_instance, create_profile, delete_platform_instance,
    delete_profile, get_instances_root_dir, is_pid_alive, load_all_instances,
    load_platform_instances, load_profiles, record_platform_instance_launch,
    record_platform_instance_stop, update_platform_instance, update_profile,
};
