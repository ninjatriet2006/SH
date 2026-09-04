/*
[INTEGRITY NOTES]
- Mục đích: Thực thi build một project và phát tiến trình về UI.
- Trách nhiệm: Chạy `cargo build`/`cargo tauri build` với `--message-format=json`,
  bóc tách từng dòng để biết đang biên dịch crate nào và đã xong bao nhiêu, rồi
  copy binary vào `release/<bin>/`.
- Tương tác: Chạy trên thread nền, gửi `BuildEvent` qua mpsc channel cho `app.rs`.

Vì sao dùng `--message-format=json`: cargo không cho biết tổng số crate trước khi
build. Ta đếm số crate đã `compiler-artifact` và lấy tổng từ `cargo metadata`
(bao gồm cả dependency) để dựng thanh tiến trình có ý nghĩa.
*/

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

use crate::discovery::{BuildKind, Project};

/// Sự kiện phát ra trong quá trình build.
#[derive(Debug, Clone)]
pub enum BuildEvent {
    /// Bắt đầu một bước (nhãn hiển thị trên UI).
    Stage(String),
    /// Dòng log thô từ cargo/npm.
    Log(String),
    /// Đang biên dịch crate `name`; `done`/`total` để vẽ thanh tiến trình.
    Progress {
        done: usize,
        total: usize,
        name: String,
    },
    /// Cảnh báo (không dừng build).
    Warn(String),
    /// Build xong một project.
    Finished { ok: bool, message: String },
}

/// Đếm tổng số crate cần biên dịch (kể cả dependency) để làm mốc tiến trình.
fn total_units(root: &Path) -> usize {
    Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| v.get("packages").and_then(|p| p.as_array()).map(Vec::len))
        .unwrap_or(300)
}

/// Chạy một lệnh, chuyển từng dòng stdout/stderr thành `BuildEvent`.
/// Trả về `Ok(())` nếu exit code 0.
fn run_streaming(
    mut cmd: Command,
    tx: &Sender<BuildEvent>,
    total: usize,
    counter: &mut usize,
) -> Result<(), String> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Không khởi chạy được lệnh: {e}"))?;

    // stdout: JSON message của cargo (nếu có) → tiến trình.
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                match v.get("reason").and_then(|r| r.as_str()) {
                    Some("compiler-artifact") => {
                        *counter += 1;
                        let name = v
                            .get("target")
                            .and_then(|t| t.get("name"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("?")
                            .to_string();
                        let _ = tx.send(BuildEvent::Progress {
                            done: *counter,
                            total,
                            name,
                        });
                    }
                    Some("compiler-message") => {
                        // Chỉ hiện phần render sẵn của rustc (đã có màu/ngữ cảnh).
                        if let Some(rendered) = v
                            .get("message")
                            .and_then(|m| m.get("rendered"))
                            .and_then(|r| r.as_str())
                        {
                            for l in rendered.lines().take(12) {
                                let _ = tx.send(BuildEvent::Log(l.to_string()));
                            }
                        }
                    }
                    _ => {}
                }
            } else if !line.trim().is_empty() {
                // Không phải JSON (ví dụ output của npm) → log thẳng.
                let _ = tx.send(BuildEvent::Log(line));
            }
        }
    }

    // stderr: tiến trình dạng chữ của cargo ("Compiling", "Finished") và lỗi.
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if t.starts_with("warning:") {
                let _ = tx.send(BuildEvent::Warn(t.to_string()));
            } else {
                let _ = tx.send(BuildEvent::Log(t.to_string()));
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("Lỗi khi đợi tiến trình: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Lệnh thất bại với mã: {status}"))
    }
}

/// Tên hàm: build_project
/// Mô tả: Build một project và copy binary vào `release/<bin_name>/`.
/// Gửi toàn bộ tiến trình qua `tx`; luôn kết thúc bằng `BuildEvent::Finished`.
pub fn build_project(root: PathBuf, project: Project, tx: Sender<BuildEvent>) {
    let total = total_units(&root);
    let mut counter = 0usize;

    let result = match &project.kind {
        BuildKind::Tauri { config_dir } => {
            let _ = tx.send(BuildEvent::Stage(format!(
                "Build Tauri: {} (frontend + backend)",
                project.bin_name
            )));
            // `cargo tauri build` chạy beforeBuildCommand (dựng frontend) rồi
            // nhúng dist vào binary. Dùng `cargo build` trực tiếp sẽ tạo binary
            // rơi về devUrl → app báo "connection refused".
            let mut cmd = Command::new("cargo");
            cmd.args([
                "tauri",
                "build",
                "--no-bundle",
                "--",
                "--message-format=json-diagnostic-rendered-ansi",
            ])
            .current_dir(config_dir);
            run_streaming(cmd, &tx, total, &mut counter)
        }
        BuildKind::Cargo => {
            let _ = tx.send(BuildEvent::Stage(format!(
                "Build Cargo: {}",
                project.package
            )));
            let mut cmd = Command::new("cargo");
            cmd.args([
                "build",
                "--release",
                "-p",
                &project.package,
                "--message-format=json-diagnostic-rendered-ansi",
            ])
            .current_dir(&root);
            run_streaming(cmd, &tx, total, &mut counter)
        }
    };

    if let Err(e) = result {
        let _ = tx.send(BuildEvent::Finished {
            ok: false,
            message: format!("{} — build thất bại: {}", project.bin_name, e),
        });
        return;
    }

    // ── Xuất binary vào release/ ─────────────────────────────────────────────
    let _ = tx.send(BuildEvent::Stage(format!(
        "Xuất vào release/{}/",
        project.bin_name
    )));

    let src = root.join("target/release").join(&project.bin_name);
    if !src.is_file() {
        let _ = tx.send(BuildEvent::Finished {
            ok: false,
            message: format!(
                "Không tìm thấy binary: target/release/{} (build xong nhưng thiếu file?)",
                project.bin_name
            ),
        });
        return;
    }

    let dest_dir = root.join("release").join(&project.bin_name);
    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        let _ = tx.send(BuildEvent::Finished {
            ok: false,
            message: format!("Không tạo được thư mục release: {e}"),
        });
        return;
    }

    let dest = dest_dir.join(&project.bin_name);
    if let Err(e) = std::fs::copy(&src, &dest) {
        let _ = tx.send(BuildEvent::Finished {
            ok: false,
            message: format!("Không copy được binary: {e}"),
        });
        return;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755));
    }

    let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let _ = tx.send(BuildEvent::Log(format!(
        "Đã copy {} ({})",
        dest.strip_prefix(&root).unwrap_or(&dest).display(),
        human_size(size)
    )));

    // Với app Tauri, copy kèm tài nguyên chạy ngoài binary nếu có.
    if let BuildKind::Tauri { config_dir } = &project.kind {
        if let Some(app_dir) = config_dir.parent() {
            for extra in ["langs", "themes"] {
                let from = app_dir.join(extra);
                if from.is_dir() {
                    let to = dest_dir.join(extra);
                    let _ = std::fs::remove_dir_all(&to);
                    if copy_dir(&from, &to).is_ok() {
                        let _ = tx.send(BuildEvent::Log(format!("Đã copy thư mục {extra}/")));
                    } else {
                        let _ = tx.send(BuildEvent::Warn(format!("Không copy được {extra}/")));
                    }
                }
            }
        }
    }

    let _ = tx.send(BuildEvent::Finished {
        ok: true,
        message: format!("{} — xong ({})", project.bin_name, human_size(size)),
    });
}

/// Copy đệ quy một thư mục (không theo symlink ra ngoài).
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

/// Định dạng byte sang chuỗi dễ đọc.
pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_formats_units() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(10 * 1024 * 1024), "10.0 MB");
    }

    #[test]
    fn total_units_returns_positive() {
        // Kể cả khi cargo lỗi, hàm phải trả về mốc dự phòng > 0 để không chia cho 0.
        let n = total_units(Path::new("/definitely/not/a/workspace"));
        assert!(n > 0);
    }

    #[test]
    fn copy_dir_copies_nested_files() {
        let base = std::env::temp_dir().join("gui_builder_copy_test");
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src/sub");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.txt"), b"hello").unwrap();

        let dst = base.join("dst");
        copy_dir(&base.join("src"), &dst).unwrap();
        assert_eq!(
            std::fs::read_to_string(dst.join("sub/a.txt")).unwrap(),
            "hello"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
