//! [INTEGRITY NOTES]
//! Mục đích: Tương tác với hệ thống file cục bộ (Local Filesystem).
//! Trách nhiệm: Chứa các hàm đọc, sao chép, di chuyển, xóa file trên ổ cứng máy tính thật, sinh thumbnail.
//! Tương tác: Giao tiếp với std::fs, trash, thư viện image/ffmpeg.
//!
//! [KHỐI LOCAL_FS]

use std::path::Path;
use crate::models::*;

pub fn list_local(path: &str) -> Result<Vec<FileItem>, String> {
    let dir = Path::new(path);
    if !dir.is_dir() {
        return Err("Không phải là thư mục".to_string());
    }
    let mut items = Vec::new();
    if let Some(parent) = dir.parent()
        && parent != dir
    {
        items.push(FileItem {
            name: "..".to_string(),
            is_dir: true,
            size: 0,
            mod_time: String::new(),
            owner: None,
            group: None,
            permissions: None,
        });
    }
    
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    let read_dir = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;
        let is_dir = metadata.is_dir();
        let size = if is_dir { 0 } else { metadata.len() };
        let mod_time = metadata
            .modified()
            .ok()
            .map(|t| {
                let datetime: chrono::DateTime<chrono::Local> = t.into();
                datetime.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| "N/A".to_string());

        let (owner, group, permissions) = {
            #[cfg(unix)]
            {
                let mode = metadata.mode();
                (
                    Some(metadata.uid().to_string()),
                    Some(metadata.gid().to_string()),
                    Some(format!("{:04o}", mode & 0o777))
                )
            }
            #[cfg(not(unix))]
            {
                (None, None, None)
            }
        };

        items.push(FileItem {
            name,
            is_dir,
            size,
            mod_time,
            owner,
            group,
            permissions,
        });
    }
    items.sort_by(|a, b| {
        if a.name == ".." {
            std::cmp::Ordering::Less
        } else if b.name == ".." {
            std::cmp::Ordering::Greater
        } else {
            b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name))
        }
    });
    Ok(items)
}

// Tạo thư mục mới (mkdir)

pub fn copy_local(from: &str, to: &str, overwrite: bool) -> Result<(), String> {
    let from_path = Path::new(from);
    let to_path = Path::new(to);
    if to_path.exists() && !overwrite {
        return Err("Destination exists and overwrite is false".to_string());
    }
    if from_path.is_dir() {
        let mut options = fs_extra::dir::CopyOptions::new();
        options.overwrite = overwrite;
        fs_extra::dir::copy(from, to, &options).map_err(|e| e.to_string())?;
    } else {
        std::fs::copy(from, to).map_err(|e| e.to_string())?;
    }
    Ok(())
}


pub fn move_local(from: &str, to: &str) -> Result<(), String> {
    std::fs::rename(from, to).map_err(|e| e.to_string())
}


pub fn delete_local(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    trash::delete(p).map_err(|e| e.to_string())
}


pub fn list_trash_local() -> Result<Vec<TrashItemLocal>, String> {
    let items = trash::os_limited::list().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for item in items {
        let id = item.id.to_string_lossy().to_string();
        let name = item.name.to_string_lossy().to_string();
        let original_path = item.original_parent.to_string_lossy().to_string();
        
        result.push(TrashItemLocal {
            id,
            name,
            original_path,
            time_deleted: item.time_deleted.to_string(),
        });
    }
    Ok(result)
}


pub fn trash_restore_local(item_id: &str) -> Result<(), String> {
    let items = trash::os_limited::list().map_err(|e| e.to_string())?;
    if let Some(item) = items.into_iter().find(|i| i.id.to_string_lossy() == item_id) {
        trash::os_limited::restore_all(vec![item]).map_err(|e| e.to_string())
    } else {
        Err("Không tìm thấy mục trong thùng rác cục bộ".to_string())
    }
}


pub fn trash_empty_local() -> Result<(), String> {
    let items = trash::os_limited::list().map_err(|e| e.to_string())?;
    trash::os_limited::purge_all(items).map_err(|e| e.to_string())
}


pub fn copy_local_batch(srcs: &[String], dst_dir: &str, overwrite: bool) -> Result<(), String> {
    for src in srcs {
        let src_path = Path::new(src);
        if let Some(name) = src_path.file_name() {
            let dst_path = Path::new(dst_dir).join(name);
            crate::local_fs::copy_local(src, &dst_path.to_string_lossy(), overwrite)?;
        }
    }
    Ok(())
}


pub fn chown_local(path: &str, uid: u32, gid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::chown;
        chown(path, Some(uid), Some(gid)).map_err(|e| e.to_string())
    }
    #[cfg(not(unix))]
    {
        Err("chown is only supported on Unix systems".to_string())
    }
}


pub fn get_thumbnail(path: &str) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use std::io::Cursor;
    use std::path::Path;

    let ext = Path::new(path).extension().unwrap_or_default().to_string_lossy().to_lowercase();
    
    match ext.as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "webm" => {
            // [TODO]: Đang phụ thuộc vào Terminal (ffmpegthumbnailer). Tương lai có thể làm logic _integrate.
            let output = std::process::Command::new("ffmpegthumbnailer")
                .args(["-i", path, "-o", "-", "-s", "64", "-c", "jpeg", "-f"])
                .output();
            if let Ok(out) = output {
                if out.status.success() {
                    let base64_str = STANDARD.encode(&out.stdout);
                    return Ok(format!("data:image/jpeg;base64,{}", base64_str));
                }
            }
        },
        "pdf" => {
            // [TODO]: Đang phụ thuộc vào Terminal (pdftoppm). Tương lai có thể làm logic _integrate.
            let output = std::process::Command::new("pdftoppm")
                .args(["-jpeg", "-f", "1", "-l", "1", "-singlefile", "-scale-to", "64", path])
                .output();
            if let Ok(out) = output {
                if out.status.success() {
                    let base64_str = STANDARD.encode(&out.stdout);
                    return Ok(format!("data:image/jpeg;base64,{}", base64_str));
                }
            }
        },
        _ => {}
    }

    // Fallback for normal images
    let img = image::open(path).map_err(|e| format!("Lỗi mở ảnh: {}", e))?;
    let thumb = img.thumbnail(64, 64);
    
    let mut buffer = Cursor::new(Vec::new());
    thumb.write_to(&mut buffer, image::ImageFormat::Jpeg).map_err(|e| format!("Lỗi tạo thumb: {}", e))?;
    
    let base64_str = STANDARD.encode(buffer.into_inner());
    Ok(format!("data:image/jpeg;base64,{}", base64_str))
}

