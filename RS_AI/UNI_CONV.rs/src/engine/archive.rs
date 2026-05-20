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
    let mut cmd = Command::new("7z");
    cmd.arg("x").arg("-y");

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
