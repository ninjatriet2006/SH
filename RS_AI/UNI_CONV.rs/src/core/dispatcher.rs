use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::core::scanner::{ScannedFile, FileType};
use crate::core::watchdog;
use crate::config::ConfigManager;
use crate::engine;

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

    // 2. Setup MultiProgress bars
    let mp = Arc::new(MultiProgress::new());
    
    // Concurrency throttle: max 3 concurrent tasks to prevent disk bottleneck (Disk Thrashing)
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

        let pb = mp_clone.add(ProgressBar::new(100));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}% - {msg}")
                .unwrap()
                .progress_chars("#>-")
        );

        let filename = f_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
        pb.set_message(filename.clone());

        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            
            // Watchdog check CPU usage before working
            watchdog::wait_if_overloaded();

            let res = match f_type {
                FileType::Directory => {
                    match dir_mode {
                        ProcessMode::Default(ext) => {
                            let out_path = f_path.with_extension(ext);
                            engine::archive::compress_files(&[f_path.clone()], &out_path, None, &pb)
                        }
                        ProcessMode::YamlConfig => {
                            let out_path = f_path.with_extension(&arc_cfg_clone.format);
                            engine::archive::compress_files(&[f_path.clone()], &out_path, None, &pb)
                        }
                        ProcessMode::Skip => {
                            pb.finish_with_message("Đã bỏ qua");
                            Ok(())
                        }
                    }
                }
                FileType::Video => {
                    match v_mode {
                        ProcessMode::Default(ext) => {
                            let out_path = f_path.with_extension(ext);
                            engine::media::convert_video(&f_path, &out_path, None, &pb)
                        }
                        ProcessMode::YamlConfig => {
                            let out_path = f_path.with_extension(&v_cfg_clone.format);
                            engine::media::convert_video(&f_path, &out_path, Some(&v_cfg_clone), &pb)
                        }
                        ProcessMode::Skip => {
                            pb.finish_with_message("Đã bỏ qua");
                            Ok(())
                        }
                    }
                }
                FileType::Audio => {
                    match a_mode {
                        ProcessMode::Default(ext) => {
                            let out_path = f_path.with_extension(ext);
                            engine::media::convert_audio(&f_path, &out_path, None, &pb)
                        }
                        ProcessMode::YamlConfig => {
                            let out_path = f_path.with_extension(&a_cfg_clone.format);
                            engine::media::convert_audio(&f_path, &out_path, Some(&a_cfg_clone), &pb)
                        }
                        ProcessMode::Skip => {
                            pb.finish_with_message("Đã bỏ qua");
                            Ok(())
                        }
                    }
                }
                FileType::Image => {
                    match i_mode {
                        ProcessMode::Default(ext) => {
                            let out_path = f_path.with_extension(ext);
                            engine::media::convert_image(&f_path, &out_path, None, &pb)
                        }
                        ProcessMode::YamlConfig => {
                            let out_path = f_path.with_extension(&i_cfg_clone.format);
                            engine::media::convert_image(&f_path, &out_path, Some(&i_cfg_clone), &pb)
                        }
                        ProcessMode::Skip => {
                            pb.finish_with_message("Đã bỏ qua");
                            Ok(())
                        }
                    }
                }
                FileType::Document => {
                    match d_mode {
                        ProcessMode::Default(ext) => {
                            let out_dir = f_path.parent().unwrap_or_else(|| Path::new("."));
                            engine::document::convert_document(&f_path, out_dir, &ext, &pb)
                        }
                        ProcessMode::YamlConfig => {
                            let out_dir = f_path.parent().unwrap_or_else(|| Path::new("."));
                            engine::document::convert_document(&f_path, out_dir, &d_cfg_clone.format, &pb)
                        }
                        ProcessMode::Skip => {
                            pb.finish_with_message("Đã bỏ qua");
                            Ok(())
                        }
                    }
                }
                FileType::Archive => {
                    let pass = passwords.get(&f_path).map(|s| s.as_str());
                    
                    // Skip password-protected files that failed verification in prompt
                    let needs_pass = engine::archive::is_password_protected(&f_path);
                    if needs_pass && pass.is_none() {
                        pb.finish_with_message("Bỏ qua (không có mật khẩu hợp lệ)");
                        Ok(())
                    } else {
                        match &arc_mode {
                            ArchiveMode::ExtractHere => {
                                let out_dir = f_path.parent().unwrap_or_else(|| Path::new("."));
                                engine::archive::extract_archive(&f_path, out_dir, pass, &pb)
                            }
                            ArchiveMode::ExtractToFolder => {
                                let stem = f_path.file_stem().unwrap_or_default();
                                let out_dir = f_path.parent().unwrap_or_else(|| Path::new(".")).join(stem);
                                let _ = std::fs::create_dir_all(&out_dir);
                                engine::archive::extract_archive(&f_path, &out_dir, pass, &pb)
                            }
                            ArchiveMode::ExtractToCustom(dest) => {
                                let _ = std::fs::create_dir_all(dest);
                                engine::archive::extract_archive(&f_path, dest, pass, &pb)
                            }
                            ArchiveMode::ConvertFormat(ext) => {
                                // Extract to temp then compress to target format
                                let temp_dir = std::env::temp_dir().join(format!("uni_conv_arc_{}", std::process::id()));
                                let _ = std::fs::create_dir_all(&temp_dir);
                                
                                let ext_res = engine::archive::extract_archive(&f_path, &temp_dir, pass, &pb);
                                if ext_res.is_ok() {
                                    pb.set_position(50);
                                    pb.set_message(format!("{} -> Nén lại thành {}", filename, ext));
                                    let target_path = f_path.with_extension(ext);
                                    let comp_res = engine::archive::compress_files(&[temp_dir.clone()], &target_path, None, &pb);
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
