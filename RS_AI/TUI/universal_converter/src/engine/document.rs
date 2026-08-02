use crate::core::traits::DocConverter;
use crate::core::watchdog;
use indicatif::ProgressBar;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct DocEngine;

impl DocEngine {
    pub fn new() -> Self {
        Self
    }
}

impl DocConverter for DocEngine {
    fn convert_document(
        &self,
        input: &Path,
        output_dir: &Path,
        target_format: &str,
        pb: &ProgressBar,
    ) -> anyhow::Result<()> {
        // Construct temp user profile URL for LibreOffice
        let temp_dir = std::env::temp_dir();
        let temp_dir_str = temp_dir.to_string_lossy().replace('\\', "/");
        let user_installation = format!(
            "file:///{}/universal_converter_soffice_{}",
            temp_dir_str,
            std::process::id()
        );

        let mut cmd = Command::new("soffice");
        cmd.arg("--headless")
            .arg(format!("-env:UserInstallation={}", user_installation))
            .arg("--convert-to")
            .arg(target_format)
            .arg("--outdir")
            .arg(output_dir)
            .arg(input);

        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        let mut child = cmd.spawn()?;
        let pid = child.id();
        watchdog::register_child(pid);

        let status = child.wait()?;
        watchdog::deregister_child(pid);

        // Clean up the temp user profile directory
        let profile_path = temp_dir.join(format!("universal_converter_soffice_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(profile_path);

        if status.success() {
            pb.set_position(100);
            pb.finish_with_message("Hoàn thành");
            Ok(())
        } else {
            anyhow::bail!("LibreOffice (soffice) document conversion failed.")
        }
    }
}
