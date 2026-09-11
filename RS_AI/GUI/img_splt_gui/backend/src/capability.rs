use crate::CancellationToken;
use crate::{BackendError, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::io;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolStatus {
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub ffmpeg: ToolStatus,
    pub ffprobe: ToolStatus,
}

pub trait CapabilityProbe: Sync {
    fn version_output(&self, program: &str) -> io::Result<std::process::Output>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemCapabilityProbe;

impl CapabilityProbe for SystemCapabilityProbe {
    fn version_output(&self, program: &str) -> io::Result<std::process::Output> {
        Command::new(program)
            .arg("-version")
            .stdin(Stdio::null())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
    }
}

pub fn check_capabilities(probe: &dyn CapabilityProbe, cancellation: &CancellationToken) -> Result<CapabilityReport> {
    cancellation.check()?;
    let ffmpeg = probe_tool(probe, "ffmpeg")?;
    cancellation.check()?;
    let ffprobe = probe_tool(probe, "ffprobe")?;
    Ok(CapabilityReport { ffmpeg, ffprobe })
}

fn probe_tool(probe: &dyn CapabilityProbe, program: &str) -> Result<ToolStatus> {
    match probe.version_output(program) {
        Ok(output) => {
            let text = if output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stderr)
            } else {
                String::from_utf8_lossy(&output.stdout)
            };
            let version = text
                .lines()
                .next()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned);
            Ok(ToolStatus {
                available: output.status.success(),
                version,
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ToolStatus {
            available: false,
            version: None,
        }),
        Err(error) => Err(BackendError::new(
            ErrorKind::Unavailable,
            format!("cannot probe {program}: {error}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    struct FakeProbe {
        versions: HashMap<String, String>,
    }

    impl CapabilityProbe for FakeProbe {
        fn version_output(&self, program: &str) -> io::Result<std::process::Output> {
            let version = self
                .versions
                .get(program)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "missing"))?;
            Ok(std::process::Output {
                status: successful_status(),
                stdout: version.as_bytes().to_vec(),
                stderr: Vec::new(),
            })
        }
    }

    #[cfg(unix)]
    fn successful_status() -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(0)
    }

    #[cfg(windows)]
    fn successful_status() -> std::process::ExitStatus {
        std::process::ExitStatus::from_raw(0)
    }

    #[test]
    #[cfg(unix)]
    fn reports_available_and_missing_tools_without_installing() {
        let probe = FakeProbe {
            versions: HashMap::from([("ffmpeg".to_owned(), "ffmpeg version 7.0\n".to_owned())]),
        };
        let report = check_capabilities(&probe, &CancellationToken::new()).expect("capability report");
        assert!(report.ffmpeg.available);
        assert_eq!(report.ffmpeg.version.as_deref(), Some("ffmpeg version 7.0"));
        assert!(!report.ffprobe.available);
    }

    #[test]
    fn cancellation_stops_before_probe() {
        let token = CancellationToken::new();
        token.cancel();
        let probe = FakeProbe {
            versions: HashMap::new(),
        };
        assert_eq!(
            check_capabilities(&probe, &token).expect_err("cancelled").kind(),
            ErrorKind::Cancelled
        );
    }
}
