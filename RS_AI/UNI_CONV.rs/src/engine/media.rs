use std::process::{Command, Stdio};
use std::path::Path;
use std::io::{BufRead, BufReader};
use indicatif::ProgressBar;
use crate::core::watchdog;
use crate::config::models::{VideoConfig, AudioConfig, ImageConfig};

// Query duration using ffprobe
pub fn get_duration(input: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(input)
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Ok(dur) = text.trim().parse::<f64>() {
            return Some(dur);
        }
    }
    None
}

pub fn convert_video(
    input: &Path,
    output: &Path,
    config: Option<&VideoConfig>,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    let duration = get_duration(input).unwrap_or(1.0);
    
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(input);

    // Apply configuration
    if let Some(cfg) = config {
        // Video Codec / GPU Acceleration
        if cfg.use_gpu {
            // Check if nvidia is available
            cmd.arg("-c:v").arg("h264_nvenc");
        } else if cfg.codec != "original" {
            cmd.arg("-c:v").arg(&cfg.codec);
        }

        // Quality/Resolution scaling
        match cfg.quality.as_str() {
            "720p" => {
                cmd.arg("-vf").arg("scale=-2:720");
            }
            "1080p" => {
                cmd.arg("-vf").arg("scale=-2:1080");
            }
            _ => {} // keep original resolution
        }
    } else {
        // Default mode: Copy streams if possible (fastest) or copy codecs
        cmd.arg("-c").arg("copy");
    }

    // Force progress reporting to stdout/pipe
    cmd.arg("-progress").arg("-");
    cmd.arg(output);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    let pid = child.id();
    watchdog::register_child(pid);

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                if l.starts_with("out_time_us=") {
                    let us_str = l.trim_start_matches("out_time_us=");
                    if let Ok(us) = us_str.parse::<f64>() {
                        let current_secs = us / 1_000_000.0;
                        let pct = (current_secs / duration * 100.0).min(100.0) as u64;
                        pb.set_position(pct);
                    }
                }
                if l.starts_with("progress=end") {
                    break;
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
        anyhow::bail!("FFmpeg kết thúc với lỗi.")
    }
}

pub fn convert_audio(
    input: &Path,
    output: &Path,
    config: Option<&AudioConfig>,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    let duration = get_duration(input).unwrap_or(1.0);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(input);

    if let Some(cfg) = config {
        cmd.arg("-b:a").arg(&cfg.bitrate);
    } else {
        cmd.arg("-c:a").arg("copy");
    }

    cmd.arg("-progress").arg("-");
    cmd.arg(output);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    let pid = child.id();
    watchdog::register_child(pid);

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                if l.starts_with("out_time_us=") {
                    let us_str = l.trim_start_matches("out_time_us=");
                    if let Ok(us) = us_str.parse::<f64>() {
                        let current_secs = us / 1_000_000.0;
                        let pct = (current_secs / duration * 100.0).min(100.0) as u64;
                        pb.set_position(pct);
                    }
                }
                if l.starts_with("progress=end") {
                    break;
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
        anyhow::bail!("FFmpeg kết thúc với lỗi.")
    }
}

pub fn convert_image(
    input: &Path,
    output: &Path,
    config: Option<&ImageConfig>,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y").arg("-i").arg(input);

    if let Some(cfg) = config {
        // Quality controls (for jpeg / webp)
        cmd.arg("-q:v").arg((cfg.quality / 10).to_string()); // scale 1-100 to 1-10 for ffmpeg quality
    }

    cmd.arg(output);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    let pid = child.id();
    watchdog::register_child(pid);

    let status = child.wait()?;
    watchdog::deregister_child(pid);

    if status.success() {
        pb.set_position(100);
        pb.finish_with_message("Hoàn thành");
        Ok(())
    } else {
        anyhow::bail!("FFmpeg convert ảnh thất bại.")
    }
}
