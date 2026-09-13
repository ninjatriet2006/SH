use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const DEJAVU_HASH: &str = "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280";
const LICENSE_HASH: &str = "63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9";
const FONT_MANIFEST: &str = "ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280  DejaVuSans.ttf\n63d3ba759d12804c5b31a9d5940d855c1820d1f5999e6b0872eb1c7ff045fbc9  LICENSE.txt\n";

pub fn initialization_script(resource_root: &Path) -> String {
    let root = fs::canonicalize(resource_root).ok();
    script_for_root(root.as_deref())
}

pub fn fallback_initialization_script() -> String {
    script_for_root(None)
}

fn script_for_root(root: Option<&Path>) -> String {
    let paths = serde_json::json!({
        "languages": {
            "en": optional_resource(root, "langs/en.json"),
            "vi": optional_resource(root, "langs/vi.json"),
        },
        "themes": {
            "system": optional_resource(root, "themes/system.json"),
            "light": optional_resource(root, "themes/light.json"),
            "dark": optional_resource(root, "themes/dark.json"),
        },
        "fonts": {"primary": root.and_then(verified_font)},
    });
    format!("window.__UNIVERSE_MANAGER_RESOURCES__={paths};")
}

fn optional_resource(root: Option<&Path>, relative: &str) -> Option<PathBuf> {
    root.and_then(|root| contained(root, relative).ok())
}

fn verified_font(root: &Path) -> Option<PathBuf> {
    let font = contained(root, "fonts/DejaVuSans.ttf").ok()?;
    let license = contained(root, "fonts/LICENSE.txt").ok()?;
    let manifest = contained(root, "fonts/manifest.sha256").ok()?;
    verify_hash(&font, DEJAVU_HASH).ok()?;
    verify_hash(&license, LICENSE_HASH).ok()?;
    (fs::read_to_string(manifest).ok()? == FONT_MANIFEST).then_some(font)
}

fn contained(root: &Path, relative: &str) -> io::Result<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid resource ID"));
    }
    let mut candidate = root.to_owned();
    for part in relative.components() {
        candidate.push(part.as_os_str());
        if fs::symlink_metadata(&candidate)?.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "resource symlink denied",
            ));
        }
    }
    let candidate = fs::canonicalize(candidate)?;
    if !candidate.starts_with(root) || !candidate.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "resource escapes bundle",
        ));
    }
    Ok(candidate)
}

fn verify_hash(path: &Path, expected: &str) -> io::Result<()> {
    if format!("{:x}", Sha256::digest(fs::read(path)?)) == expected {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, "resource hash mismatch"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    fn fixture(name: &str) -> PathBuf {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("universe-resource-{name}-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).expect("create resource fixture");
        root
    }

    fn paths(script: &str) -> serde_json::Value {
        let json = script
            .strip_prefix("window.__UNIVERSE_MANAGER_RESOURCES__=")
            .and_then(|value| value.strip_suffix(';'))
            .expect("resource initialization script");
        serde_json::from_str(json).expect("resource paths JSON")
    }

    #[test]
    fn missing_languages_themes_and_font_use_frontend_fallbacks() {
        let value = paths(&initialization_script(&fixture("missing")));
        assert_eq!(value["languages"], serde_json::json!({"en": null, "vi": null}));
        assert_eq!(
            value["themes"],
            serde_json::json!({"system": null, "light": null, "dark": null})
        );
        assert!(value["fonts"]["primary"].is_null());
    }

    #[test]
    fn startup_fallback_does_not_search_the_process_working_directory() {
        let value = paths(&fallback_initialization_script());
        assert!(value["languages"]["en"].is_null());
        assert!(value["themes"]["system"].is_null());
        assert!(value["fonts"]["primary"].is_null());
    }

    #[test]
    fn corrupt_bundled_font_selects_system_default_without_hiding_other_resources() {
        let root = fixture("corrupt-font");
        let app_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("app root");
        fs::create_dir_all(root.join("langs")).expect("langs fixture");
        fs::create_dir_all(root.join("fonts")).expect("fonts fixture");
        fs::copy(app_root.join("langs/en.json"), root.join("langs/en.json")).expect("copy language");
        fs::write(root.join("fonts/DejaVuSans.ttf"), b"corrupt font").expect("write corrupt font");
        fs::copy(app_root.join("fonts/LICENSE.txt"), root.join("fonts/LICENSE.txt")).expect("copy license");
        fs::copy(
            app_root.join("fonts/manifest.sha256"),
            root.join("fonts/manifest.sha256"),
        )
        .expect("copy manifest");

        let value = paths(&initialization_script(&root));
        assert!(value["languages"]["en"].as_str().is_some());
        assert!(value["fonts"]["primary"].is_null());
    }

    #[cfg(unix)]
    #[test]
    fn resources_cannot_be_sourced_through_cross_bundle_symlinks() {
        use std::os::unix::fs::symlink;

        let root = fixture("contained");
        let external = fixture("external");
        fs::create_dir_all(root.join("langs")).expect("langs fixture");
        fs::write(external.join("en.json"), b"{}").expect("external language");
        symlink(external.join("en.json"), root.join("langs/en.json")).expect("resource symlink");

        let value = paths(&initialization_script(&root));
        assert!(value["languages"]["en"].is_null());
        assert!(contained(&root, "../external/en.json").is_err());
    }
}
