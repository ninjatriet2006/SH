use crate::config::models::{AudioConfig, ImageConfig, VideoConfig};
use indicatif::ProgressBar;
use std::path::{Path, PathBuf};

pub trait Compressor: Send + Sync {
    fn is_password_protected(&self, archive: &Path) -> bool;

    fn verify_password(&self, archive: &Path, password: &str) -> bool;

    fn extract_archive(
        &self,
        archive: &Path,
        output_dir: &Path,
        password: Option<&str>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()>;

    fn compress_files(
        &self,
        files: &[PathBuf],
        archive_out: &Path,
        password: Option<&str>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()>;

    fn list_archive_contents(&self, archive: &Path) -> anyhow::Result<Vec<String>>;
}

pub trait MediaConverter: Send + Sync {
    fn convert_video(
        &self,
        input: &Path,
        output: &Path,
        config: Option<&VideoConfig>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()>;

    fn convert_audio(
        &self,
        input: &Path,
        output: &Path,
        config: Option<&AudioConfig>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()>;

    fn convert_image(
        &self,
        input: &Path,
        output: &Path,
        config: Option<&ImageConfig>,
        pb: &ProgressBar,
    ) -> anyhow::Result<()>;
}

pub trait DocConverter: Send + Sync {
    fn convert_document(
        &self,
        input: &Path,
        output_dir: &Path,
        target_format: &str,
        pb: &ProgressBar,
    ) -> anyhow::Result<()>;
}
