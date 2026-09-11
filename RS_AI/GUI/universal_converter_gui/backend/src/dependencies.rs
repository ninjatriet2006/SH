use crate::CancellationToken;
use std::io;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyReport {
    pub is_ok: bool,
    pub missing: Vec<String>,
}

pub trait ExecutableProbe {
    fn exists_on_path(&self, program: &str, arguments: &[&str], cancellation: &CancellationToken) -> io::Result<bool>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemExecutableProbe;

impl ExecutableProbe for SystemExecutableProbe {
    fn exists_on_path(&self, program: &str, arguments: &[&str], cancellation: &CancellationToken) -> io::Result<bool> {
        cancellation.check()?;
        let mut child = match Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error),
        };
        loop {
            if cancellation.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                return cancellation.check().map(|()| false);
            }
            if child.try_wait()?.is_some() {
                return Ok(true);
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

pub fn check_dependencies() -> io::Result<DependencyReport> {
    check_dependencies_cancellable(&CancellationToken::new())
}

pub fn check_dependencies_cancellable(cancellation: &CancellationToken) -> io::Result<DependencyReport> {
    check_dependencies_with(&SystemExecutableProbe, cancellation)
}

pub fn check_dependencies_with(
    probe: &impl ExecutableProbe,
    cancellation: &CancellationToken,
) -> io::Result<DependencyReport> {
    let mut missing = Vec::new();
    if !probe.exists_on_path("ffmpeg", &["-version"], cancellation)? {
        missing.push("ffmpeg".to_owned());
    }
    if !probe.exists_on_path("7z", &["--help"], cancellation)? && !probe.exists_on_path("7z", &[], cancellation)? {
        missing.push("7z (p7zip)".to_owned());
    }
    if !probe.exists_on_path("soffice", &["--version"], cancellation)? {
        missing.push("libreoffice (soffice)".to_owned());
    }

    Ok(DependencyReport {
        is_ok: missing.is_empty(),
        missing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeProbe;

    impl ExecutableProbe for FakeProbe {
        fn exists_on_path(
            &self,
            program: &str,
            _arguments: &[&str],
            cancellation: &CancellationToken,
        ) -> io::Result<bool> {
            cancellation.check()?;
            Ok(program == "ffmpeg")
        }
    }

    #[test]
    fn reports_all_missing_tools_without_installing_them() {
        let report = check_dependencies_with(&FakeProbe, &CancellationToken::new()).expect("fake probe must succeed");
        assert!(!report.is_ok);
        assert_eq!(report.missing, ["7z (p7zip)", "libreoffice (soffice)"]);
    }

    #[test]
    fn cancellation_stops_before_probe() {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        assert_eq!(
            check_dependencies_with(&FakeProbe, &cancellation)
                .expect_err("cancelled dependency check")
                .kind(),
            io::ErrorKind::Interrupted
        );
    }
}
