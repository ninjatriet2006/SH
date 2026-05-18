use crate::config::Settings;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Select;
use rayon::prelude::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub fn process_files(files: &[PathBuf], settings: &Settings) -> Vec<PathBuf> {
    println!("----------------------------------------------------");
    println!("BƯỚC 2 & 3: CHUẨN HÓA ĐỊNH DẠNG & KIỂM TRA ĐỘ PHÂN GIẢI");
    
    let options = vec![
        "1. jpg", "2. png", "3. webp", "4. avif", "5. heic", "6. bmp", "7. tiff", "8. Bỏ qua (Giữ nguyên định dạng gốc)"
    ];
    let choice = Select::new("Lựa chọn định dạng đích:", options).prompt().unwrap_or("8. Bỏ qua");
    
    let target_ext = match choice.chars().next().unwrap() {
        '1' => "jpg",
        '2' => "png",
        '3' => "webp",
        '4' => "avif",
        '5' => "heic",
        '6' => "bmp",
        '7' => "tiff",
        _ => "skip",
    };

    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let dir_name = current_dir.file_name().unwrap_or_default().to_string_lossy();
    let process_dir = current_dir.join(format!("{}_process", dir_name));

    if !process_dir.exists() {
        fs::create_dir_all(&process_dir).expect("Không thể tạo thư mục _process");
    }

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("   -> Tiến trình: [{bar:50.cyan/blue}] {percent}% ({pos}/{len} file) {msg}").unwrap()
        .progress_chars("#>-"));

    let failed_count = Arc::new(AtomicUsize::new(0));

    let processed_files: Vec<PathBuf> = files.par_iter().filter_map(|file| {
        let final_ext = if target_ext == "skip" {
            file.extension().unwrap_or_default().to_string_lossy().to_string()
        } else {
            target_ext.to_string()
        };

        let file_stem = file.file_stem().unwrap_or_default().to_string_lossy();
        let out_path = process_dir.join(format!("{}.{}", file_stem, final_ext));

        let mut needs_upscale = false;
        
        // ffprobe check width
        if let Ok(output) = Command::new("ffprobe")
            .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width", "-of", "csv=s=x:p=0"])
            .arg(file)
            .output() 
        {
            let out_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(width) = out_str.trim().parse::<u32>() {
                if width < settings.min_upscale_width {
                    needs_upscale = true;
                }
            }
        }

        let mut success = false;
        
        if !needs_upscale && target_ext == "skip" {
            // Không làm gì cả, chỉ copy
            if fs::copy(file, &out_path).is_ok() {
                success = true;
            }
        } else {
            // Cần ffmpeg
            for _retry in 0..settings.max_retries {
                let mut cmd = Command::new("ffmpeg");
                cmd.arg("-nostdin").arg("-i").arg(file);
                
                if needs_upscale {
                    cmd.arg("-vf").arg(format!("scale={}:-1", settings.target_upscale_width));
                }
                
                cmd.arg(&out_path).arg("-y").arg("-loglevel").arg("quiet");
                
                if let Ok(status) = cmd.status() {
                    if status.success() {
                        if let Ok(metadata) = fs::metadata(&out_path) {
                            if metadata.len() > 0 {
                                success = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        pb.inc(1);

        if success {
            Some(out_path)
        } else {
            failed_count.fetch_add(1, Ordering::SeqCst);
            None
        }
    }).collect();

    // Chủ động giải phóng bộ nhớ RAM của danh sách Beta ngay lập tức
    // để giữ cho luồng làm việc (workflow) sạch sẽ và tối ưu nhất
    drop(processed_files);

    pb.finish_with_message("Xong!");

    let failed = failed_count.load(Ordering::SeqCst);
    if failed > 0 {
        println!("Cảnh báo: Có {} file bị lỗi không thể convert/upscale.", failed);
    }

    // Xóa các file rỗng trong _process (nếu lọt lưới)
    if let Ok(entries) = fs::read_dir(&process_dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.len() == 0 {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    // Thu thập lại danh sách file thực tế trong _process
    let mut final_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&process_dir) {
        for entry in entries.flatten() {
            if entry.path().is_file() {
                final_files.push(entry.path());
            }
        }
    }
    
    final_files.sort_by(crate::scanner::natural_sort);
    
    final_files
}

pub fn swap_directories(process_dir: &Path, original_files: &[PathBuf]) {
    // Delete original files to make room
    for file in original_files {
        let _ = fs::remove_file(file);
    }
    
    // Move from _process to current dir
    let current_dir = env::current_dir().unwrap();
    if let Ok(entries) = fs::read_dir(process_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let dest = current_dir.join(path.file_name().unwrap());
            let _ = fs::rename(&path, &dest);
        }
    }
    
    let _ = fs::remove_dir(process_dir);
}
