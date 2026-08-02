use crate::config::Settings;
use inquire::{CustomType, Select};
use regex::Regex;
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn distribute_files(files: &[PathBuf], settings: &Settings) {
    if files.is_empty() {
        println!("LỖI: Không tìm thấy file ảnh hợp lệ để xử lý!");
        return;
    }
    let total_final = files.len();
    println!("====================================================");
    println!("TỔNG HỢP: Có {} file ảnh hợp lệ.", total_final);

    let chapter_x = CustomType::<usize>::new("Nhập số Chương (X) [Nhập 0 cho Oneshot]:")
        .with_error_message("Vui lòng nhập một số nguyên.")
        .prompt()
        .unwrap_or(0);

    let is_oneshot = chapter_x == 0;

    let options = vec![
        "1. Balanced (Chia đều số lượng file)",
        "2. Greedy (Nhồi đầy số file tối đa)",
        "3. Fixed (Chia theo số thư mục cố định)",
    ];
    let default_idx = match settings.default_distribution_mode.as_str() {
        "greedy" => 1,
        "fixed" => 2,
        _ => 0,
    };
    let choice = Select::new("Chọn thuật toán chia thư mục:", options)
        .with_starting_cursor(default_idx)
        .prompt()
        .unwrap_or("1");

    let (num_folders, files_per_folder) = if choice.starts_with("3") {
        let mut n = settings.fixed_folder_count;
        if n == 0 {
            n = 1;
        }
        let per_folder = (total_final as f64 / n as f64).ceil() as usize;
        (n, per_folder)
    } else if choice.starts_with("2") {
        let per_folder = settings.max_files_per_folder;
        let mut n = total_final / per_folder;
        if !total_final.is_multiple_of(per_folder) {
            n += 1;
        }
        (n, per_folder)
    } else {
        // Balanced mode
        let mut n = 1;
        while total_final.div_ceil(n) > settings.max_files_per_folder {
            n += 1;
        }
        let per_folder = total_final.div_ceil(n);
        (n, per_folder)
    };

    if is_oneshot && num_folders > 1 {
        println!(
            "Cảnh báo: Số lượng file ({}) vượt mức 1 thư mục Oneshot. Sẽ chia thành {} thư mục Oneshot. Part",
            total_final, num_folders
        );
    }

    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut next_y = 1;
    if !is_oneshot {
        // Scan for existing sub-chapters
        let re = Regex::new(&format!(r"^Chapter\s+{}\.(\d+)$", chapter_x)).unwrap();
        let mut max_y = 0;

        if let Ok(entries) = fs::read_dir(&current_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    if let Some(caps) = re.captures(&dir_name)
                        && let Ok(y) = caps[1].parse::<usize>()
                        && y > max_y
                    {
                        max_y = y;
                    }
                }
            }
        }
        next_y = max_y + 1;
    }

    let mut current_folder_idx = 0;
    let mut current_file_count = 0;

    for file in files {
        if choice.starts_with("2") {
            // Greedy
            if current_file_count >= files_per_folder {
                current_folder_idx += 1;
                current_file_count = 0;
            }
        } else {
            // Balanced or Fixed
            if current_file_count >= files_per_folder {
                current_folder_idx += 1;
                current_file_count = 0;
            }
        }

        let folder_name = if is_oneshot {
            if num_folders > 1 {
                format!("Oneshot Part {}", current_folder_idx + 1)
            } else {
                "Oneshot".to_string()
            }
        } else {
            format!("Chapter {}.{}", chapter_x, next_y + current_folder_idx)
        };

        let target_dir = current_dir.join(&folder_name);
        if !target_dir.exists() {
            fs::create_dir_all(&target_dir).expect("Không thể tạo thư mục đích");
        }

        let target_path = target_dir.join(file.file_name().unwrap());

        // Handle collisions if the target already has the file
        let mut final_target = target_path.clone();
        let mut collision_counter = 1;
        while final_target.exists() {
            let stem = file.file_stem().unwrap_or_default().to_string_lossy();
            let ext = file.extension().unwrap_or_default().to_string_lossy();
            let new_name = format!("{}_{:03}.{}", stem, collision_counter, ext);
            final_target = target_dir.join(new_name);
            collision_counter += 1;
        }

        let _ = fs::rename(file, &final_target);
        current_file_count += 1;
    }

    println!("----------------------------------------------------");
    println!("HOÀN TẤT: Quy trình phân chia đã xong!");
}
