use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::config::ConfigManager;
use crate::core::scanner::{FileType, ScannedFile};
use crate::core::traits::{Compressor, DocConverter, MediaConverter};
use crate::core::watchdog;
use crate::engine::{ArchiveEngine, DocEngine, MediaEngine};

#[derive(Clone)]
pub enum ProcessMode {
    Default(String), // target extension
    YamlConfig,
    Skip,
}

#[derive(Clone)]
pub enum ArchiveMode {
    ExtractHere,
    ExtractToFolder,
    ExtractToCustom(PathBuf),
    ConvertFormat(String), // target format e.g. "zip", "7z"
    Skip,
}

#[allow(clippy::too_many_arguments)]
pub async fn run_batch_processing(
    files: Vec<ScannedFile>,
    video_mode: ProcessMode,
    audio_mode: ProcessMode,
    image_mode: ProcessMode,
    doc_mode: ProcessMode,
    archive_mode: ArchiveMode,
    directory_mode: ProcessMode,
    archive_passwords: HashMap<PathBuf, String>,
) -> anyhow::Result<()> {
    let config_mgr = ConfigManager::new();

    // Load configs beforehand (once for the whole batch)
    let v_cfg = config_mgr.load_video_config().unwrap_or_default();
    let a_cfg = config_mgr.load_audio_config().unwrap_or_default();
    let i_cfg = config_mgr.load_image_config().unwrap_or_default();
    let d_cfg = config_mgr.load_doc_config().unwrap_or_default();
    let arc_cfg = config_mgr.load_archive_config().unwrap_or_default();

    // Setup Engines via Traits
    let archive_engine: Arc<dyn Compressor> = Arc::new(ArchiveEngine::new());
    let media_engine: Arc<dyn MediaConverter> = Arc::new(MediaEngine::new());
    let doc_engine: Arc<dyn DocConverter> = Arc::new(DocEngine::new());

    // 2. Setup MultiProgress bars
    let mp = Arc::new(MultiProgress::new());

    // Concurrency throttle: max 3 concurrent tasks to prevent disk bottleneck
    let semaphore = Arc::new(Semaphore::new(3));

    let mut tasks = Vec::new();

    for f in files {
        let f_path = f.path.clone();
        let f_type = f.file_type.clone();

        let v_mode = video_mode.clone();
        let a_mode = audio_mode.clone();
        let i_mode = image_mode.clone();
        let d_mode = doc_mode.clone();
        let arc_mode = archive_mode.clone();
        let dir_mode = directory_mode.clone();
        let passwords = archive_passwords.clone();
        let sem = semaphore.clone();
        let mp_clone = mp.clone();

        let v_cfg_clone = v_cfg.clone();
        let a_cfg_clone = a_cfg.clone();
        let i_cfg_clone = i_cfg.clone();
        let d_cfg_clone = d_cfg.clone();
        let arc_cfg_clone = arc_cfg.clone();

        let arc_eng = archive_engine.clone();
        let med_eng = media_engine.clone();
        let doc_eng = doc_engine.clone();

        let pb = mp_clone.add(ProgressBar::new(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% - {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        let filename = f_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        pb.set_message(filename.clone());

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            // Watchdog check CPU usage before working
            watchdog::wait_if_overloaded();

            let res = match f_type {
                FileType::Directory => match dir_mode {
                    ProcessMode::Default(ext) => {
                        let out_path = f_path.with_extension(ext);
                        arc_eng.compress_files(std::slice::from_ref(&f_path), &out_path, None, &pb)
                    }
                    ProcessMode::YamlConfig => {
                        let out_path = f_path.with_extension(&arc_cfg_clone.format);
                        arc_eng.compress_files(std::slice::from_ref(&f_path), &out_path, None, &pb)
                    }
                    ProcessMode::Skip => {
                        pb.finish_with_message("Đã bỏ qua");
                        Ok(())
                    }
                },
                FileType::Video => match v_mode {
                    ProcessMode::Default(ext) => {
                        let out_path = f_path.with_extension(ext);
                        med_eng.convert_video(&f_path, &out_path, None, &pb)
                    }
                    ProcessMode::YamlConfig => {
                        let out_path = f_path.with_extension(&v_cfg_clone.format);
                        med_eng.convert_video(&f_path, &out_path, Some(&v_cfg_clone), &pb)
                    }
                    ProcessMode::Skip => {
                        pb.finish_with_message("Đã bỏ qua");
                        Ok(())
                    }
                },
                FileType::Audio => match a_mode {
                    ProcessMode::Default(ext) => {
                        let out_path = f_path.with_extension(ext);
                        med_eng.convert_audio(&f_path, &out_path, None, &pb)
                    }
                    ProcessMode::YamlConfig => {
                        let out_path = f_path.with_extension(&a_cfg_clone.format);
                        med_eng.convert_audio(&f_path, &out_path, Some(&a_cfg_clone), &pb)
                    }
                    ProcessMode::Skip => {
                        pb.finish_with_message("Đã bỏ qua");
                        Ok(())
                    }
                },
                FileType::Image => match i_mode {
                    ProcessMode::Default(ext) => {
                        let out_path = f_path.with_extension(ext);
                        med_eng.convert_image(&f_path, &out_path, None, &pb)
                    }
                    ProcessMode::YamlConfig => {
                        let out_path = f_path.with_extension(&i_cfg_clone.format);
                        med_eng.convert_image(&f_path, &out_path, Some(&i_cfg_clone), &pb)
                    }
                    ProcessMode::Skip => {
                        pb.finish_with_message("Đã bỏ qua");
                        Ok(())
                    }
                },
                FileType::Document => match d_mode {
                    ProcessMode::Default(ext) => {
                        let out_dir = f_path.parent().unwrap_or_else(|| Path::new("."));
                        doc_eng.convert_document(&f_path, out_dir, &ext, &pb)
                    }
                    ProcessMode::YamlConfig => {
                        let out_dir = f_path.parent().unwrap_or_else(|| Path::new("."));
                        doc_eng.convert_document(&f_path, out_dir, &d_cfg_clone.format, &pb)
                    }
                    ProcessMode::Skip => {
                        pb.finish_with_message("Đã bỏ qua");
                        Ok(())
                    }
                },
                FileType::Archive => {
                    let pass = passwords.get(&f_path).map(|s| s.as_str());
                    let needs_pass = arc_eng.is_password_protected(&f_path);
                    if needs_pass && pass.is_none() {
                        pb.finish_with_message("Bỏ qua (không có mật khẩu hợp lệ)");
                        Ok(())
                    } else {
                        match &arc_mode {
                            ArchiveMode::ExtractHere => {
                                let out_dir = f_path.parent().unwrap_or_else(|| Path::new("."));
                                arc_eng.extract_archive(&f_path, out_dir, pass, &pb)
                            }
                            ArchiveMode::ExtractToFolder => {
                                let folder_name = get_archive_folder_name(&f_path);
                                let out_dir = f_path.parent().unwrap_or_else(|| Path::new(".")).join(folder_name);
                                let _ = std::fs::create_dir_all(&out_dir);
                                arc_eng.extract_archive(&f_path, &out_dir, pass, &pb)
                            }
                            ArchiveMode::ExtractToCustom(dest) => {
                                let _ = std::fs::create_dir_all(dest);
                                arc_eng.extract_archive(&f_path, dest, pass, &pb)
                            }
                            ArchiveMode::ConvertFormat(ext) => {
                                // Extract to temp then compress to target format
                                let temp_dir = std::env::temp_dir()
                                    .join(format!("universal_converter_arc_{}", std::process::id()));
                                let _ = std::fs::create_dir_all(&temp_dir);

                                let ext_res = arc_eng.extract_archive(&f_path, &temp_dir, pass, &pb);
                                if ext_res.is_ok() {
                                    pb.set_position(50);
                                    pb.set_message(format!("{} -> Nén lại thành {}", filename, ext));
                                    let target_path = f_path.with_extension(ext);
                                    let comp_res = arc_eng.compress_files(
                                        std::slice::from_ref(&temp_dir),
                                        &target_path,
                                        None,
                                        &pb,
                                    );
                                    let _ = std::fs::remove_dir_all(&temp_dir);
                                    comp_res
                                } else {
                                    let _ = std::fs::remove_dir_all(&temp_dir);
                                    ext_res
                                }
                            }
                            ArchiveMode::Skip => {
                                pb.finish_with_message("Đã bỏ qua");
                                Ok(())
                            }
                        }
                    }
                }
                FileType::Unknown => {
                    pb.finish_with_message("Không hỗ trợ");
                    Ok(())
                }
            };

            if let Err(e) = res {
                pb.finish_with_message(format!("Lỗi: {}", e));
            }
        });
        tasks.push(task);
    }

    // Wait for all processes to complete
    for t in tasks {
        let _ = t.await;
    }

    println!("\n[✅] Đã xử lý xong loạt file!");
    Ok(())
}

fn get_archive_folder_name(path: &Path) -> String {
    let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let filename_lower = filename.to_lowercase();

    if filename_lower.ends_with(".tar.gz") {
        filename[..filename.len() - 7].to_string()
    } else if filename_lower.ends_with(".tar.bz2") {
        filename[..filename.len() - 8].to_string()
    } else if filename_lower.ends_with(".tar.xz") {
        filename[..filename.len() - 7].to_string()
    } else if filename_lower.ends_with(".tgz") {
        filename[..filename.len() - 4].to_string()
    } else if filename_lower.ends_with(".tbz2") {
        filename[..filename.len() - 5].to_string()
    } else if filename_lower.ends_with(".txz") {
        filename[..filename.len() - 4].to_string()
    } else {
        path.file_stem().unwrap_or_default().to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_get_archive_folder_name() {
        assert_eq!(get_archive_folder_name(Path::new("my_app.tar.gz")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.tar.bz2")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.tar.xz")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.tgz")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.tbz2")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.txz")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.zip")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.7z")), "my_app");
        assert_eq!(get_archive_folder_name(Path::new("my_app.tar")), "my_app");
    }
}
