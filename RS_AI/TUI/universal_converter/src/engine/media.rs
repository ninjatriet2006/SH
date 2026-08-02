use crate::config::models::{AudioConfig, ImageConfig, VideoConfig};
use crate::core::traits::MediaConverter;
use crate::core::watchdog;
use indicatif::ProgressBar;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct MediaEngine;

impl MediaEngine {
    pub fn new() -> Self {
        Self
    }
}

// Query duration using ffprobe
pub fn get_duration(input: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
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

impl MediaConverter for MediaEngine {
    fn convert_video(
        &self,
        input: &Path,
        output: &Path,
        config: Option<&VideoConfig>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        let duration = get_duration(input).unwrap_or(1.0);

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y").arg("-i").arg(input);

        // Check if output target format is an audio format (Audio Extraction)
        let out_ext = output.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let is_target_audio = matches!(out_ext.as_str(), "mp3" | "wav" | "flac" | "m4a" | "ogg" | "wma" | "aac");

        if is_target_audio {
            cmd.arg("-vn"); // Strip video track (Audio Extraction)
            cmd.arg("-acodec").arg("libmp3lame"); // default codec for mp3
        } else if let Some(cfg) = config {
            // Apply Hardware Acceleration / Codec
            match cfg.hardware_accel.as_str() {
                "nvidia_h264" => {
                    cmd.arg("-c:v").arg("h264_nvenc");
                }
                "nvidia_hevc" => {
                    cmd.arg("-c:v").arg("hevc_nvenc");
                }
                "apple_vt" => {
                    cmd.arg("-c:v").arg("h264_videotoolbox");
                }
                "intel_qsv" => {
                    cmd.arg("-c:v").arg("h264_qsv");
                }
                _ => {
                    if cfg.use_gpu {
                        cmd.arg("-c:v").arg("h264_nvenc");
                    } else if cfg.codec != "original" {
                        cmd.arg("-c:v").arg(&cfg.codec);
                    }
                }
            }

            // Quality/Resolution scaling
            match cfg.quality.as_str() {
                "720p" => {
                    cmd.arg("-vf").arg("scale=-2:720");
                }
                "1080p" => {
                    cmd.arg("-vf").arg("scale=-2:1080");
                }
                "2k" => {
                    cmd.arg("-vf").arg("scale=-2:1440");
                }
                "4k" => {
                    cmd.arg("-vf").arg("scale=-2:2160");
                }
                "480p" => {
                    cmd.arg("-vf").arg("scale=-2:480");
                }
                _ => {} // keep original resolution
            }

            // Limit FPS
            if let Some(fps) = cfg.fps {
                cmd.arg("-r").arg(fps.to_string());
            }

            // Remove subtitles
            if cfg.remove_subtitles {
                cmd.arg("-sn");
            }
        } else {
            // Default mode: Copy streams if possible (fastest)
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
            for l in reader.lines().map_while(Result::ok) {
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

        let status = child.wait()?;
        watchdog::deregister_child(pid);

        if status.success() {
            pb.finish_with_message("Hoàn thành");
            Ok(())
        } else {
            anyhow::bail!("FFmpeg video conversion failed.")
        }
    }

    fn convert_audio(
        &self,
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

            if let Some(ref sr) = cfg.sample_rate {
                cmd.arg("-ar").arg(sr);
            }
            if let Some(ch) = cfg.channels {
                cmd.arg("-ac").arg(ch.to_string());
            }
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
            for l in reader.lines().map_while(Result::ok) {
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

        let status = child.wait()?;
        watchdog::deregister_child(pid);

        if status.success() {
            pb.finish_with_message("Hoàn thành");
            Ok(())
        } else {
            anyhow::bail!("FFmpeg audio conversion failed.")
        }
    }

    fn convert_image(
        &self,
        input: &Path,
        output: &Path,
        config: Option<&ImageConfig>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y").arg("-i").arg(input);

        if let Some(cfg) = config {
            // Quality controls
            cmd.arg("-q:v").arg((cfg.quality / 10).to_string());

            // Image resizing
            if let Some(ref size) = cfg.resize {
                cmd.arg("-vf").arg(format!("scale={}", size));
            }
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
            anyhow::bail!("FFmpeg image conversion failed.")
        }
    }
}
