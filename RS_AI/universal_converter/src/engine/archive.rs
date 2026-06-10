use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use indicatif::ProgressBar;
use zip::write::FileOptions;
use crate::core::traits::Compressor;
use crate::core::watchdog;

pub struct ArchiveEngine;

impl ArchiveEngine {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for ArchiveEngine {
    fn is_password_protected(&self, archive: &Path) -> bool {
        let filename = archive.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if filename.ends_with(".zip") {
            if let Ok(file) = File::open(archive) {
                if let Ok(mut zip) = zip::ZipArchive::new(file) {
                    for i in 0..zip.len() {
                        match zip.by_index(i) {
                            Err(zip::result::ZipError::UnsupportedArchive(zip::result::ZipError::PASSWORD_REQUIRED)) => {
                                return true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else if filename.ends_with(".7z") {
            // Check via sevenz-rust or CLI fallback
            if let Ok(file) = File::open(archive) {
                if let Ok(len) = file.metadata().map(|m| m.len()) {
                    let file_mut = file;
                    if let Err(sevenz_rust::Error::PasswordRequired) = sevenz_rust::SevenZReader::new(file_mut, len, sevenz_rust::Password::empty()) {
                        return true;
                    }
                }
            }

            
            // CLI Fallback
            let output = std::process::Command::new("7z")
                .arg("t")
                .arg("-p_universal_converter_dummy_pass")
                .arg(archive)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output();

            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                let combined = format!("{} {}", stdout, stderr);
                if combined.contains("Wrong password") || combined.contains("Encrypted") || combined.contains("Enter password") {
                    return true;
                }
            }
        }
        false
    }

    fn verify_password(&self, archive: &Path, password: &str) -> bool {
        let filename = archive.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        if filename.ends_with(".zip") {
            if let Ok(file) = File::open(archive) {
                if let Ok(mut zip) = zip::ZipArchive::new(file) {
                    for i in 0..zip.len() {
                        if let Ok(Ok(mut file)) = zip.by_index_decrypt(i, password.as_bytes()) {
                            let mut buffer = [0; 1024];
                            if file.read(&mut buffer).is_err() {
                                return false;
                            }
                            return true; // Password works
                        }
                    }
                }
            }
            return false;
        } else if filename.ends_with(".7z") {
            // Try decompressing a single file index with password using sevenz-rust
            let res = sevenz_rust::decompress_file_with_password(archive, std::env::temp_dir(), password.into());
            if res.is_ok() {
                return true;
            }

            // CLI Fallback
            let output = std::process::Command::new("7z")
                .arg("t")
                .arg(format!("-p{}", password))
                .arg(archive)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output();

            if let Ok(out) = output {
                return out.status.success();
            }
        }
        false
    }

    fn extract_archive(
        &self,
        archive: &Path,
        output_dir: &Path,
        password: Option<&str>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        let filename = archive.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let _ = fs::create_dir_all(output_dir);

        if filename.ends_with(".zip") && password.is_none() {
            pb.set_message("Đang giải nén ZIP...");
            let file = File::open(archive)?;
            let mut zip = zip::ZipArchive::new(file)?;
            let total = zip.len();
            for i in 0..total {
                let mut file = zip.by_index(i)?;
                let outpath = match file.enclosed_name() {
                    Some(path) => output_dir.join(path),
                    None => continue,
                };

                if (*file.name()).ends_with('/') {
                    fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        if !p.exists() {
                            fs::create_dir_all(p)?;
                        }
                    }
                    let mut outfile = File::create(&outpath)?;
                    io::copy(&mut file, &mut outfile)?;
                }
                pb.set_position(((i + 1) as f32 / total as f32 * 100.0) as u64);
            }
            pb.finish_with_message("Hoàn thành giải nén ZIP");
            return Ok(());
        }

        if filename.ends_with(".7z") {
            pb.set_message("Đang giải nén 7Z...");
            let pass_str = password.unwrap_or("");
            let res = if !pass_str.is_empty() {
                sevenz_rust::decompress_file_with_password(archive, output_dir, pass_str.into())
            } else {
                sevenz_rust::decompress_file(archive, output_dir)
            };

            if res.is_ok() {
                pb.set_position(100);
                pb.finish_with_message("Hoàn thành giải nén 7Z");
                return Ok(());
            }
            // Fall back to CLI if library fails
        }

        if (filename.ends_with(".tar.gz") || filename.ends_with(".tgz")) && password.is_none() {
            pb.set_message("Đang giải nén TAR.GZ (Native)...");
            let file = File::open(archive)?;
            let gz = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(gz);
            tar.unpack(output_dir)?;
            pb.set_position(100);
            pb.finish_with_message("Hoàn thành giải nén TAR.GZ");
            return Ok(());
        }

        // CLI general fallback (for tar.gz, double archives, etc.)
        let is_tar_gz = filename.ends_with(".tar.gz") || filename.ends_with(".tgz");
        let is_tar_bz2 = filename.ends_with(".tar.bz2") || filename.ends_with(".tbz2");
        let is_tar_xz = filename.ends_with(".tar.xz") || filename.ends_with(".txz");
        let is_double_archive = is_tar_gz || is_tar_bz2 || is_tar_xz;

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
            pb.set_message("Giải nén kép (.tar.gz/bz2/xz) qua 7z...");
            let mut decompress = std::process::Command::new("7z");
            decompress.arg("x").arg("-so").arg("-y");
            if let Some(pass) = password {
                decompress.arg(format!("-p{}", pass));
            }
            decompress.arg(archive);
            decompress.stdout(std::process::Stdio::piped());
            decompress.stderr(std::process::Stdio::null());

            let mut decompress_child = decompress.spawn()?;
            let decompress_pid = decompress_child.id();
            watchdog::register_child(decompress_pid);

            let mut extract = std::process::Command::new("7z");
            extract.arg("x").arg("-si").arg("-ttar").arg("-y");
            extract.arg(format!("-o{}", final_output.to_string_lossy()));
            extract.stdin(decompress_child.stdout.take().unwrap());
            extract.stdout(std::process::Stdio::piped());
            extract.stderr(std::process::Stdio::null());

            let mut extract_child = extract.spawn()?;
            let extract_pid = extract_child.id();
            watchdog::register_child(extract_pid);

            let decompress_status = decompress_child.wait()?;
            watchdog::deregister_child(decompress_pid);

            let extract_status = extract_child.wait()?;
            watchdog::deregister_child(extract_pid);

            if decompress_status.success() && extract_status.success() {
                pb.set_position(100);
                pb.finish_with_message("Hoàn thành");
                Ok(())
            } else {
                anyhow::bail!("Giải nén kép thất bại.")
            }
        } else {
            let mut cmd = std::process::Command::new("7z");
            cmd.arg("x").arg("-y");
            cmd.arg(format!("-o{}", final_output.to_string_lossy()));

            if let Some(pass) = password {
                cmd.arg(format!("-p{}", pass));
            }

            cmd.arg(archive);
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::null());

            let mut child = cmd.spawn()?;
            let pid = child.id();
            watchdog::register_child(pid);

            if let Some(stdout) = child.stdout.take() {
                let reader = io::BufReader::new(stdout);
                for line in io::BufRead::lines(reader) {
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
                anyhow::bail!("7z giải nén thất bại.")
            }
        }
    }

    fn compress_files(
        &self,
        files: &[PathBuf],
        archive_out: &Path,
        password: Option<&str>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        let filename = archive_out.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        
        // Native ZIP compression (pure Rust) if no password or simple ZIP
        if filename.ends_with(".zip") && password.is_none() {
            pb.set_message("Đang nén ZIP (Native)...");
            let file = File::create(archive_out)?;
            let mut zip = zip::ZipWriter::new(file);
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);

            let mut total_files = 0;
            for path in files {
                if path.is_file() {
                    total_files += 1;
                } else if path.is_dir() {
                    for entry in walkdir::WalkDir::new(path) {
                        if entry.is_ok() {
                            total_files += 1;
                        }
                    }
                }
            }

            let mut processed = 0;
            for path in files {
                if path.is_file() {
                    let name = path.file_name().unwrap().to_string_lossy();
                    zip.start_file(name, options)?;
                    let mut f = File::open(path)?;
                    let mut buffer = Vec::new();
                    f.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                    processed += 1;
                    pb.set_position((processed * 100 / total_files) as u64);
                } else if path.is_dir() {
                    let prefix = path.parent().unwrap();
                    for entry in walkdir::WalkDir::new(path) {
                        let entry = entry?;
                        let entry_path = entry.path();
                        let name = entry_path.strip_prefix(prefix)?.to_string_lossy().into_owned();
                        if entry_path.is_file() {
                            zip.start_file(name, options)?;
                            let mut f = File::open(entry_path)?;
                            let mut buffer = Vec::new();
                            f.read_to_end(&mut buffer)?;
                            zip.write_all(&buffer)?;
                        } else if entry_path.is_dir() {
                            zip.add_directory(name, options)?;
                        }
                        processed += 1;
                        pb.set_position((processed * 100 / total_files) as u64);
                    }
                }
            }
            zip.finish()?;
            pb.finish_with_message("Hoàn thành nén ZIP");
            return Ok(());
        }

        // CLI fallback for 7z compression (supports passwords, compression levels, threads)
        pb.set_message("Đang nén qua 7z CLI...");
        let mut cmd = std::process::Command::new("7z");
        cmd.arg("a").arg("-y");

        if let Some(pass) = password {
            cmd.arg(format!("-p{}", pass));
        }

        cmd.arg(archive_out);
        for f in files {
            cmd.arg(f);
        }

        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::null());

        let mut child = cmd.spawn()?;
        let pid = child.id();
        watchdog::register_child(pid);

        if let Some(stdout) = child.stdout.take() {
            let reader = io::BufReader::new(stdout);
            for line in io::BufRead::lines(reader) {
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
            pb.finish_with_message("Hoàn thành");
            Ok(())
        } else {
            anyhow::bail!("7z nén tệp thất bại.")
        }
    }

    fn list_archive_contents(&self, archive: &Path) -> anyhow::Result<Vec<String>> {
        let filename = archive.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let mut file_list = Vec::new();

        if filename.ends_with(".zip") {
            let file = File::open(archive)?;
            let mut zip = zip::ZipArchive::new(file)?;
            for i in 0..zip.len() {
                if let Ok(f) = zip.by_index(i) {
                    file_list.push(f.name().to_string());
                }
            }
            return Ok(file_list);
        }

        if filename.ends_with(".7z") {
            if let Ok(file) = File::open(archive) {
                if let Ok(len) = file.metadata().map(|m| m.len()) {
                    let mut file_mut = file;
                    if let Ok(sevenz) = sevenz_rust::Archive::read(&mut file_mut, len, &[]) {
                        for entry in &sevenz.files {
                            file_list.push(entry.name.to_string());
                        }
                        return Ok(file_list);
                    }
                }
            }
        }

        // CLI fallback
        let output = std::process::Command::new("7z")
            .arg("l")
            .arg(archive)
            .output()?;

        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut parse = false;
            for line in text.lines() {
                if line.contains("-------------------") {
                    parse = !parse;
                    continue;
                }
                if parse {
                    // Extract name from the 7z listing output line (the last column)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let name = parts[4..].join(" ");
                        if !name.is_empty() {
                            file_list.push(name);
                        }
                    }
                }
            }
        }

        Ok(file_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::traits::Compressor;
    use std::fs::{self, File};
    use std::io::Write;
    use indicatif::ProgressBar;

    #[test]
    fn test_extract_tar_gz() -> anyhow::Result<()> {
        let test_dir = Path::new("test_extract_src");
        let _ = fs::remove_dir_all(test_dir);
        let _ = fs::remove_dir_all("test_extracted_output");
        let _ = fs::remove_file("test_archive.tar.gz");

        fs::create_dir_all(test_dir)?;
        let sub_dir = test_dir.join("inner_folder");
        fs::create_dir_all(&sub_dir)?;
        
        let mut f1 = File::create(sub_dir.join("hello.txt"))?;
        f1.write_all(b"Hello World from hello.txt")?;
        
        let mut f2 = File::create(test_dir.join("root.txt"))?;
        f2.write_all(b"Hello from root.txt")?;
        
        let status = std::process::Command::new("tar")
            .args(&["-czf", "test_archive.tar.gz", "-C", "test_extract_src", "inner_folder", "root.txt"])
            .status()?;
        assert!(status.success());
        
        let engine = ArchiveEngine::new();
        let out_dir = Path::new("test_extracted_output");
        let pb = ProgressBar::hidden();
        
        engine.extract_archive(Path::new("test_archive.tar.gz"), out_dir, None, &pb)?;
        
        // Assert files exist in test_extracted_output
        assert!(out_dir.join("inner_folder/hello.txt").exists());
        assert!(out_dir.join("root.txt").exists());
        
        // Clean up
        let _ = fs::remove_dir_all(test_dir);
        let _ = fs::remove_file("test_archive.tar.gz");
        let _ = fs::remove_dir_all("test_extracted_output");
        Ok(())
    }
}

