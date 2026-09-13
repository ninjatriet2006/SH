use crate::contract::Preferences;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static WRITE_ID: AtomicU64 = AtomicU64::new(0);
static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub fn load_or_migrate(path: &Path, legacy_path: Option<&Path>) -> io::Result<Preferences> {
    match fs::read(path) {
        Ok(bytes) => {
            let parsed = serde_json::from_slice::<Preferences>(&bytes).ok();
            let normalized = parsed.clone().map(normalize).unwrap_or_default();
            if parsed.as_ref() != Some(&normalized) {
                atomic_write(path, &normalized)?;
            }
            Ok(normalized)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let preferences = legacy_path
                .and_then(|legacy| import_legacy(legacy).ok())
                .map(normalize)
                .unwrap_or_default();
            atomic_write(path, &preferences)?;
            Ok(preferences)
        }
        Err(error) => Err(error),
    }
}

pub fn legacy_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("universe_manager_gui/preferences.conf"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|path| path.join("universe_manager_gui/preferences.conf"))
    }
}

fn import_legacy(path: &Path) -> io::Result<Preferences> {
    let mut preferences = Preferences::default();
    for line in fs::read_to_string(path)?.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "language" => preferences.language = value.trim().to_owned(),
            "theme" => preferences.theme = value.trim().to_owned(),
            "font" => preferences.font_id = legacy_font_id(value.trim()),
            _ => {}
        }
    }
    Ok(preferences)
}

fn legacy_font_id(value: &str) -> String {
    if value.is_empty() || value.eq_ignore_ascii_case("default") {
        "system-default".into()
    } else if Path::new(value)
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("DejaVuSans") || name.eq_ignore_ascii_case("DejaVu Sans"))
    {
        "dejavusans".into()
    } else {
        value.to_owned()
    }
}

fn normalize(mut value: Preferences) -> Preferences {
    let defaults = Preferences::default();
    if !matches!(value.language.as_str(), "vi" | "en") {
        value.language = defaults.language;
    }
    if !matches!(value.theme.as_str(), "system" | "light" | "dark") {
        value.theme = defaults.theme;
    }
    if !matches!(value.font_id.as_str(), "system-default" | "dejavusans") {
        value.font_id = defaults.font_id;
    }
    value
}

pub fn atomic_write(path: &Path, value: &Preferences) -> io::Result<()> {
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|_| io::Error::other("preferences lock poisoned"))?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = unique_sibling(path, "tmp");
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    write_and_sync(&temporary, &bytes)?;
    if path.exists() {
        let backup = path.with_extension("json.bak");
        let backup_temporary = unique_sibling(path, "bak.tmp");
        if let Err(error) = fs::copy(path, &backup_temporary)
            .and_then(|_| fs::File::open(&backup_temporary)?.sync_all())
            .and_then(|()| atomic_replace(&backup_temporary, &backup))
        {
            let _ = fs::remove_file(&temporary);
            let _ = fs::remove_file(&backup_temporary);
            return Err(error);
        }
    }
    if let Err(error) = atomic_replace(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    sync_parent(path)
}

fn unique_sibling(path: &Path, suffix: &str) -> PathBuf {
    let id = WRITE_ID.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.{suffix}-{}-{id}", std::process::id()))
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(target_os = "windows")]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW, REPLACEFILE_WRITE_THROUGH, ReplaceFileW,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    // Both APIs atomically replace files on one volume; buffers are live and NUL-terminated.
    let succeeded = unsafe {
        if destination.exists() {
            ReplaceFileW(
                destination_wide.as_ptr(),
                source.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null(),
                std::ptr::null(),
            )
        } else {
            MoveFileExW(
                source.as_ptr(),
                destination_wide.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        }
    };
    if succeeded == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> io::Result<()> {
    fs::File::open(
        path.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing parent"))?,
    )?
    .sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "universe-preferences-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).expect("clock").as_nanos()
        ))
    }

    #[test]
    fn defaults_and_invalid_values_are_normalized() {
        assert_eq!(
            Preferences::default(),
            Preferences {
                language: "vi".into(),
                theme: "system".into(),
                font_id: "system-default".into()
            }
        );
        let value = normalize(Preferences {
            language: "fr".into(),
            theme: "neon".into(),
            font_id: "/host/font.ttf".into(),
        });
        assert_eq!(value, Preferences::default());
    }

    #[test]
    fn imports_legacy_once_and_keeps_backup_on_update() {
        let root = directory();
        fs::create_dir_all(&root).expect("test directory");
        let legacy = root.join("preferences.conf");
        let canonical = root.join("data/preferences.json");
        fs::write(&legacy, "language=en\ntheme=dark\nfont=/tmp/DejaVuSans.ttf\n").expect("legacy");
        let imported = load_or_migrate(&canonical, Some(&legacy)).expect("migration");
        assert_eq!(
            imported,
            Preferences {
                language: "en".into(),
                theme: "dark".into(),
                font_id: "dejavusans".into()
            }
        );
        fs::write(&legacy, "language=vi\ntheme=light\nfont=\n").expect("changed legacy");
        assert_eq!(
            load_or_migrate(&canonical, Some(&legacy)).expect("canonical wins"),
            imported
        );
        atomic_write(&canonical, &Preferences::default()).expect("update");
        let backup: Preferences =
            serde_json::from_slice(&fs::read(canonical.with_extension("json.bak")).expect("backup"))
                .expect("backup json");
        assert_eq!(backup, imported);
        fs::remove_dir_all(root).expect("cleanup test directory");
    }
}
