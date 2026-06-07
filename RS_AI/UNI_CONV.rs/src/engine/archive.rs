use std::process::{Command, Stdio};
use std::path::{Path, PathBuf};
use std::io::{BufRead, BufReader};
use indicatif::ProgressBar;
use crate::core::watchdog;

pub fn is_password_protected(archive: &Path) -> bool {
    // Run test command with a dummy password to see if it complains about password
    let output = Command::new("7z")
        .arg("t")
        .arg("-p_uni_conv_dummy_pass")
        .arg(archive)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let combined = format!("{} {}", stdout, stderr);
        
        if combined.contains("Wrong password") || combined.contains("Encrypted") || combined.contains("Enter password") {
            return true;
        }
    }
    false
}

pub fn verify_password(archive: &Path, password: &str) -> bool {
    // Test extraction with the given password to validate it
    let output = Command::new("7z")
        .arg("t")
        .arg(format!("-p{}", password))
        .arg(archive)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Ok(out) = output {
        out.status.success()
    } else {
        false
    }
}

pub fn extract_archive(
    archive: &Path,
    output_dir: &Path,
    password: Option<&str>,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    let filename = archive.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let is_tar_gz = filename.ends_with(".tar.gz") || filename.ends_with(".tgz");
    let is_tar_bz2 = filename.ends_with(".tar.bz2") || filename.ends_with(".tbz2");
    let is_tar_xz = filename.ends_with(".tar.xz") || filename.ends_with(".txz");
    let is_double_archive = is_tar_gz || is_tar_bz2 || is_tar_xz;

    // Handle Long Path prefix on Windows Explorer
    #[cfg(target_os = "windows")]
    let final_output = {
        let path_str = output_dir.to_string_lossy();
        if !path_str.starts_with("\\\\?\\") {
            PathBuf::from(format!("\\\\?\\{}", path_str))
        } else {
            output_dir.to_path_buf()
        }
    };
    #[cfg(not(target_os = "windows"))]
    let final_output = output_dir.to_path_buf();

    if is_double_archive {
        pb.set_message("Giải nén nén kép (.tar.gz/.tar.bz2/.tar.xz)...");
        
        let mut decompress = Command::new("7z");
        decompress.arg("x").arg("-so").arg("-y");
        if let Some(pass) = password {
            decompress.arg(format!("-p{}", pass));
        }
        decompress.arg(archive);
        decompress.stdout(Stdio::piped());
        decompress.stderr(Stdio::null());

        let mut decompress_child = decompress.spawn()?;
        let decompress_pid = decompress_child.id();
        watchdog::register_child(decompress_pid);

        let mut extract = Command::new("7z");
        extract.arg("x").arg("-si").arg("-ttar").arg("-y");
        extract.arg(format!("-o{}", final_output.to_string_lossy()));
        extract.stdin(decompress_child.stdout.take().unwrap());
        extract.stdout(Stdio::piped());
        extract.stderr(Stdio::null());

        let mut extract_child = extract.spawn()?;
        let extract_pid = extract_child.id();
        watchdog::register_child(extract_pid);

        // Read stdout of extract command to keep progress/watchdog alive and show extraction is in progress
        if let Some(stdout) = extract_child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut line_count = 0;
            for _line in reader.lines() {
                line_count += 1;
                if line_count % 100 == 0 {
                    pb.set_message(format!("Giải nén: đã xử lý {} file/folder...", line_count));
                }
            }
        }

        let decompress_status = decompress_child.wait()?;
        watchdog::deregister_child(decompress_pid);

        let extract_status = extract_child.wait()?;
        watchdog::deregister_child(extract_pid);

        if decompress_status.success() && extract_status.success() {
            pb.set_position(100);
            pb.finish_with_message("Hoàn thành");
            Ok(())
        } else {
            anyhow::bail!("7z giải nén thất bại (Có thể sai mật khẩu hoặc tệp hỏng).")
        }
    } else {
        let mut cmd = Command::new("7z");
        cmd.arg("x").arg("-y");
        cmd.arg(format!("-o{}", final_output.to_string_lossy()));

        if let Some(pass) = password {
            cmd.arg(format!("-p{}", pass));
        }

        cmd.arg(archive);
        
        // We can parse 7z progress output (it outputs progress to stdout if we read lines)
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let mut child = cmd.spawn()?;
        let pid = child.id();
        watchdog::register_child(pid);

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    // 7z outputs lines with percentages like " 43%" or similar during operation
                    let trimmed = l.trim();
                    if trimmed.ends_with('%') {
                        let pct_str = trimmed.trim_end_matches('%');
                        if let Ok(pct) = pct_str.parse::<u64>() {
                            pb.set_position(pct);
                        }
                    }
                }
            }
        }

        let status = child.wait()?;
        watchdog::deregister_child(pid);

        if status.success() {
            pb.set_position(100);
            pb.finish_with_message("Hoàn thành");
            Ok(())
        } else {
            anyhow::bail!("7z giải nén thất bại (Có thể sai mật khẩu hoặc file hỏng).")
        }
    }
}

pub fn compress_files(
    files: &[PathBuf],
    archive_out: &Path,
    password: Option<&str>,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("7z");
    cmd.arg("a").arg("-y");

    if let Some(pass) = password {
        cmd.arg(format!("-p{}", pass));
    }

    cmd.arg(archive_out);

    for f in files {
        cmd.arg(f);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    let pid = child.id();
    watchdog::register_child(pid);

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                let trimmed = l.trim();
                if trimmed.ends_with('%') {
                    let pct_str = trimmed.trim_end_matches('%');
                    if let Ok(pct) = pct_str.parse::<u64>() {
                        pb.set_position(pct);
                    }
                }
            }
        }
    }

    let status = child.wait()?;
    watchdog::deregister_child(pid);

    if status.success() {
        pb.set_position(100);
        pb.finish_with_message("Hoàn thành");
        Ok(())
    } else {
        anyhow::bail!("7z nén file thất bại.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indicatif::ProgressBar;
    
    #[test]
    fn test_tar_gz_extraction() {
        let archive_str = shellexpand::tilde("~/Downloads/Antigravity IDE.tar.gz").to_string();
        let archive = Path::new(&archive_str);
        if !archive.exists() {
            return;
        }
        let output_dir_str = shellexpand::tilde("~/.gemini/antigravity-ide/scratch/test_verify").to_string();
        let output_dir = Path::new(&output_dir_str);
        let _ = std::fs::remove_dir_all(&output_dir);
        std::fs::create_dir_all(&output_dir).unwrap();
        
        let pb = ProgressBar::hidden();
        let res = extract_archive(archive, &output_dir, None, &pb);
        assert!(res.is_ok());
        
        // Verify output dir contains extracted folder and files
        let inner_folder = output_dir.join("Antigravity IDE");
        assert!(inner_folder.exists());
        assert!(inner_folder.is_dir());
        
        let _ = std::fs::remove_dir_all(&output_dir);
    }
}
