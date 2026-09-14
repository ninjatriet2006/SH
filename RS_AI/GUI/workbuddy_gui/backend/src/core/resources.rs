use std::path::PathBuf;
use std::sync::OnceLock;

static RESOURCE_BASE: OnceLock<PathBuf> = OnceLock::new();

pub const RESOURCE_ANCHOR: &str = "langs";

fn detect_resource_base() -> PathBuf {
    let has_anchor = |p: &std::path::Path| p.join(RESOURCE_ANCHOR).is_dir();

    #[cfg(debug_assertions)]
    {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        if let Some(app_root) = manifest.parent() {
            if has_anchor(app_root) {
                return app_root.to_path_buf();
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        if has_anchor(&cwd) {
            return cwd;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut cur = exe.parent();
        while let Some(dir) = cur {
            if has_anchor(dir) {
                return dir.to_path_buf();
            }
            cur = dir.parent();
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn configured_resource_base() -> Option<PathBuf> {
    std::env::var_os("WORKBUDDY_RESOURCE_DIR")
        .map(PathBuf::from)
        .filter(|path| path.join(RESOURCE_ANCHOR).is_dir())
}

pub fn resource_base() -> &'static PathBuf {
    RESOURCE_BASE.get_or_init(|| configured_resource_base().unwrap_or_else(detect_resource_base))
}

pub fn resource_dir(name: &str) -> PathBuf {
    resource_base().join(name)
}
