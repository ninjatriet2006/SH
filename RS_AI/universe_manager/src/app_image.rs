use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::detector::DesktopMetadata;
use tempfile::TempDir;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;


/// Extracts the desktop file and icon from an AppImage using its `--appimage-extract` CLI option.
/// Returns the parsed metadata, and optionally the extracted icon's bytes/path.
pub fn extract_metadata(appimage_path: &Path) -> Result<(DesktopMetadata, Option<PathBuf>), std::io::Error> {
    // Ensure the AppImage has executable permission first, otherwise we cannot execute it to extract
    #[cfg(unix)]
    if let Ok(metadata) = fs::metadata(appimage_path) {
        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        if mode & 0o111 == 0 {
            permissions.set_mode(mode | 0o111); // Add +x
            let _ = fs::set_permissions(appimage_path, permissions);
        }
    }

    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Run <appimage> --appimage-extract to extract the contents
    let status = Command::new(appimage_path)
        .arg("--appimage-extract")
        .current_dir(temp_path)
        .status()?;

    if !status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Không thể trích xuất AppImage bằng tham số --appimage-extract",
        ));
    }

    let squashfs_root = temp_path.join("squashfs-root");
    if !squashfs_root.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Thư mục squashfs-root không được tạo sau khi trích xuất",
        ));
    }

    // 1. Locate the desktop file in squashfs-root (usually named *.desktop in the root)
    let mut desktop_meta = DesktopMetadata::default();

    if let Ok(entries) = fs::read_dir(&squashfs_root) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e.to_ascii_lowercase()) == Some(std::ffi::OsStr::new("desktop").to_ascii_lowercase()) {
                desktop_meta = DesktopMetadata::parse_file(&path);
                break;
            }
        }
    }

    // 2. Locate the icon. The desktop metadata tells us the icon name (e.g. Icon=myapp).
    // AppImage usually has a png or svg in the root dir named after the icon or matching the metadata Icon key.
    let mut extracted_icon_path = None;
    if let Some(ref icon_name) = desktop_meta.icon {
        // Look in root directory for icon_name.png or icon_name.svg
        let png_name = format!("{}.png", icon_name);
        let svg_name = format!("{}.svg", icon_name);
        let root_png = squashfs_root.join(&png_name);
        let root_svg = squashfs_root.join(&svg_name);

        if root_svg.exists() {
            extracted_icon_path = Some(root_svg);
        } else if root_png.exists() {
            extracted_icon_path = Some(root_png);
        }
    }

    // If still not found, search the root directory for any png or svg
    if extracted_icon_path.is_none() {
        if let Ok(entries) = fs::read_dir(&squashfs_root) {
            let mut found_png = None;
            let mut found_svg = None;

            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().map(|e| e.to_ascii_lowercase());
                    if ext == Some(std::ffi::OsStr::new("svg").to_ascii_lowercase()) {
                        found_svg = Some(path);
                        break; // SVG is preferred
                    } else if ext == Some(std::ffi::OsStr::new("png").to_ascii_lowercase()) {
                        found_png = Some(path);
                    }
                }
            }

            extracted_icon_path = found_svg.or(found_png);
        }
    }

    // Copy the extracted icon out to a permanent path in the same directory as the AppImage
    // or return it so the integrator can copy it.
    let mut final_icon_local_path = None;
    if let Some(ref icon_path) = extracted_icon_path {
        let appimage_parent = appimage_path.parent().unwrap_or_else(|| Path::new("."));
        let icon_ext = icon_path.extension().unwrap_or_default().to_string_lossy();
        
        // Target icon path: next to the AppImage, named <appimage_name>.<ext>
        let appimage_stem = appimage_path.file_stem().unwrap_or_default().to_string_lossy();
        let target_icon = appimage_parent.join(format!("{}.{}", appimage_stem, icon_ext));

        if fs::copy(icon_path, &target_icon).is_ok() {
            final_icon_local_path = Some(target_icon);
        }
    }

    Ok((desktop_meta, final_icon_local_path))
}
