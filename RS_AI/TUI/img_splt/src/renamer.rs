use inquire::{CustomType, Select};
use std::fs;
use std::path::PathBuf;

pub fn rename_files(files: &mut Vec<PathBuf>) {
    println!("----------------------------------------------------");
    println!("BƯỚC 1: CHẾ ĐỘ ĐỔI TÊN FILE");

    let options = vec![
        "1. Giữ X ký tự đầu",
        "2. Giữ X ký tự cuối",
        "3. Lấy ký tự từ khoảng X đến Y",
        "4. Đánh số thứ tự mới hoàn toàn",
        "5. Bỏ qua đổi tên",
    ];

    let choice = Select::new("Lựa chọn của bạn:", options)
        .prompt()
        .unwrap_or("5. Bỏ qua");

    if choice.starts_with("5") {
        return;
    }

    let mut new_files = Vec::new();
    let total_files = files.len();

    if choice.starts_with("4") {
        let start_idx = CustomType::<usize>::new("Nhập số thứ tự bắt đầu:")
            .with_error_message("Vui lòng nhập một số nguyên hợp lệ.")
            .prompt()
            .unwrap_or(1);

        let mut padding = total_files.to_string().len();
        if padding < 3 {
            padding = 3;
        }

        println!("   -> Đang đánh số thứ tự mới...");
        let mut current_idx = start_idx;
        for file in files.iter() {
            if let Some(ext) = file.extension() {
                let new_name = format!("{:0width$}.{}", current_idx, ext.to_string_lossy(), width = padding);
                let new_path = file.with_file_name(&new_name);

                if file != &new_path {
                    if fs::rename(file, &new_path).is_ok() {
                        new_files.push(new_path);
                    } else {
                        new_files.push(file.clone());
                    }
                } else {
                    new_files.push(file.clone());
                }
                current_idx += 1;
            }
        }
    } else {
        let (mode, x, y) = if choice.starts_with("1") {
            let x = CustomType::<usize>::new("Nhập số lượng ký tự (X) muốn giữ lại từ đầu:")
                .with_error_message("Vui lòng nhập một số dương.")
                .prompt()
                .unwrap_or(0);
            (1, x, 0)
        } else if choice.starts_with("2") {
            let x = CustomType::<usize>::new("Nhập số lượng ký tự (X) muốn giữ lại từ cuối:")
                .with_error_message("Vui lòng nhập một số dương.")
                .prompt()
                .unwrap_or(0);
            (2, x, 0)
        } else {
            let x = CustomType::<usize>::new("Nhập vị trí bắt đầu (X):")
                .with_error_message("Vui lòng nhập một số dương.")
                .prompt()
                .unwrap_or(0);
            let y = CustomType::<usize>::new("Nhập vị trí kết thúc (Y):")
                .with_error_message("Vui lòng nhập một số dương.")
                .prompt()
                .unwrap_or(x);
            (3, x, y)
        };

        println!("   -> Đang cắt chuỗi và xử lý trùng lặp...");
        for file in files.iter() {
            if let Some(ext) = file.extension() {
                let file_name = file.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let chars: Vec<char> = file_name.chars().collect();
                let char_len = chars.len();

                let base_new = match mode {
                    1 => {
                        let take = x.min(char_len);
                        chars[..take].iter().collect::<String>()
                    }
                    2 => {
                        let take = x.min(char_len);
                        chars[char_len - take..].iter().collect::<String>()
                    }
                    3 => {
                        let start = x.saturating_sub(1).min(char_len);
                        let end = y.min(char_len);
                        if start < end {
                            chars[start..end].iter().collect::<String>()
                        } else {
                            file_name.clone()
                        }
                    }
                    _ => file_name.clone(),
                };

                let mut suffix = String::new();
                let mut counter = 1;
                let mut new_path = file.with_file_name(format!("{}{}.{}", base_new, suffix, ext.to_string_lossy()));

                while new_path.exists() && &new_path != file {
                    suffix = format!("_{:03}", counter);
                    new_path = file.with_file_name(format!("{}{}.{}", base_new, suffix, ext.to_string_lossy()));
                    counter += 1;
                }

                if file != &new_path {
                    if fs::rename(file, &new_path).is_ok() {
                        new_files.push(new_path);
                    } else {
                        new_files.push(file.clone());
                    }
                } else {
                    new_files.push(file.clone());
                }
            }
        }
    }

    *files = new_files;
}
