use std::process::{Command, Stdio};
use std::path::Path;
use indicatif::ProgressBar;
use crate::core::watchdog;

pub fn convert_document(
    input: &Path,
    output_dir: &Path,
    target_format: &str,
    pb: &ProgressBar,
) -> anyhow::Result<()> {
    // Construct temp user profile URL for LibreOffice
    let temp_dir = std::env::temp_dir();
    let temp_dir_str = temp_dir.to_string_lossy().replace('\\', "/");
    let user_installation = format!("file:///{}/uni_conv_soffice_{}", temp_dir_str, std::process::id());

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

    // Clean up the temp user profile directory asynchronously
    let profile_path = temp_dir.join(format!("uni_conv_soffice_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(profile_path);

    if status.success() {
        pb.set_position(100);
        pb.finish_with_message("Hoàn thành");
        Ok(())
    } else {
        anyhow::bail!("LibreOffice (soffice) convert tài liệu thất bại.")
    }
}
