use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoConfig {
    #[serde(default = "default_video_format")]
    pub format: String,
    #[serde(default = "default_video_codec")]
    pub codec: String,
    #[serde(default = "default_video_quality")]
    pub quality: String, // original, 1080p, 720p
    #[serde(default = "default_false")]
    pub use_gpu: bool,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            format: default_video_format(),
            codec: default_video_codec(),
            quality: default_video_quality(),
            use_gpu: default_false(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioConfig {
    #[serde(default = "default_audio_format")]
    pub format: String,
    #[serde(default = "default_audio_bitrate")]
    pub bitrate: String,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            format: default_audio_format(),
            bitrate: default_audio_bitrate(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageConfig {
    #[serde(default = "default_image_format")]
    pub format: String,
    #[serde(default = "default_image_quality")]
    pub quality: u8,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            format: default_image_format(),
            quality: default_image_quality(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocConfig {
    #[serde(default = "default_doc_format")]
    pub format: String,
}

impl Default for DocConfig {
    fn default() -> Self {
        Self {
            format: default_doc_format(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArchiveConfig {
    #[serde(default = "default_archive_format")]
    pub format: String,
    #[serde(default = "default_archive_action")]
    pub action: String, // extract_here, extract_to_folder, compress
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            format: default_archive_format(),
            action: default_archive_action(),
        }
    }
}

// Defaults for Serde Deserialization
fn default_video_format() -> String { "mp4".to_string() }
fn default_video_codec() -> String { "libx264".to_string() }
fn default_video_quality() -> String { "original".to_string() }
fn default_audio_format() -> String { "mp3".to_string() }
fn default_audio_bitrate() -> String { "320k".to_string() }
fn default_image_format() -> String { "webp".to_string() }
fn default_image_quality() -> u8 { 90 }
fn default_doc_format() -> String { "pdf".to_string() }
fn default_archive_format() -> String { "zip".to_string() }
fn default_archive_action() -> String { "extract_to_folder".to_string() }
fn default_false() -> bool { false }
