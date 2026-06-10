use std::process::Command;

pub fn run_as_admin(command: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // On Linux, run sudo sh -c "command"
        let status = Command::new("sudo")
            .arg("sh")
            .arg("-c")
            .arg(command)
            .status()?;
        
        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Thực thi với sudo thất bại.")
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, start powershell as Admin to execute command
        // Start-Process powershell -Verb RunAs -ArgumentList "-Command ... "
        let args = format!("-NoProfile -ExecutionPolicy Bypass -Command \"{}\"", command);
        let status = Command::new("powershell")
            .arg("-Command")
            .arg(format!("Start-Process powershell -Verb RunAs -Wait -ArgumentList '{}'", args))
            .status()?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Thực thi với quyền Administrator thất bại.")
        }
    }
}
