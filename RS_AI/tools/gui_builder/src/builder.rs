/*
[INTEGRITY NOTES]
- Mục đích: Thực thi build một project và phát tiến trình về UI.
- Trách nhiệm: Chạy `cargo build`/`cargo tauri build` với `--message-format=json`,
  bóc tách từng dòng để biết đang biên dịch crate nào và đã xong bao nhiêu, rồi
  copy binary + tài nguyên chạy kèm vào `release/<release_name>/`.
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
    Progress { done: usize, total: usize, name: String },
    /// Cảnh báo (không dừng build).
    Warn(String),
    /// Build xong một project.
    Finished { ok: bool, message: String },
}

/// Tài nguyên chạy kèm cần copy sang release cho app Tauri.
/// `dist/` cần cho trường hợp webview đọc asset ngoài binary; `langs`/`themes`/
/// `fonts` là tài nguyên runtime của các app trong repo này.
const APP_RESOURCES: [&str; 3] = ["langs", "themes", "fonts"];

/// Chuẩn version Node cho vite 8: ^20.19.0 || >=22.12.0.
/// Tách hàm thuần để unit-test được.
fn node_version_ok(ver: &str) -> bool {
    let nums: Vec<u64> = ver
        .trim()
        .trim_start_matches('v')
        .split('.')
        .take(3)
        .filter_map(|p| p.parse().ok())
        .collect();
    match nums.as_slice() {
        [20, minor, ..] => *minor >= 19,
        [major, ..] if *major >= 22 => true,
        _ => false,
    }
}

/// Kiểm tra Node trước khi build Tauri: `beforeBuildCommand` chạy `npm run build`
/// (vite 8) cần Node ^20.19 || >=22.12. Mở TUI bằng double-click không load nvm
/// nên hay rớt về node hệ thống v18 → chết với lỗi khó hiểu
/// (`node:util does not provide styleText`). Chặn sớm với thông báo rõ ràng.
fn check_node_for_tauri(tx: &Sender<BuildEvent>) -> Result<(), String> {
    let out = Command::new("node")
        .arg("--version")
        .output()
        .map_err(|e| format!("Không tìm thấy `node` trên PATH: {e}"))?;
    let ver = String::from_utf8_lossy(&out.stdout).trim().to_string(); // "v20.20.2"
    if node_version_ok(&ver) {
        let _ = tx.send(BuildEvent::Log(format!("Node {ver} — đạt yêu cầu vite")));
        Ok(())
    } else {
        Err(format!(
            "Node {ver} quá cũ (vite 8 cần ^20.19 || >=22.12). \
             Mở terminal có nvm (node -v) rồi chạy lại ./build_release.sh, \
             hoặc cài Node mới từ nodejs.org"
        ))
    }
}

/// Trả về thư mục frontend mà `beforeBuildCommand` sẽ dùng. Tauri config cho
/// phép đặt CWD tương đối với backend; fallback giữ tương thích template chuẩn.
fn frontend_dir(config_dir: &Path) -> PathBuf {
    let configured = std::fs::read_to_string(config_dir.join("tauri.conf.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|config| {
            config
                .pointer("/build/beforeBuildCommand/cwd")
                .and_then(|cwd| cwd.as_str())
                .map(PathBuf::from)
        });

    match configured {
        Some(path) if path.is_absolute() => path,
        Some(path) => normalize_path(config_dir.join(path)),
        None => config_dir
            .parent()
            .map(|parent| parent.join("frontend"))
            .unwrap_or_else(|| config_dir.join("frontend")),
    }
}

/// Chuẩn hóa `..` mà không yêu cầu đường dẫn đã tồn tại, để `cwd: "../frontend"`
/// trong Tauri config luôn hiện và chạy theo đúng thư mục mong muốn.
fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

/// `npm run build` tìm local binary qua `node_modules/.bin`; thiếu `tsc` nghĩa
/// là dependencies chưa được cài hoặc node_modules không còn đồng bộ với lock.
fn frontend_dependencies_ready(frontend: &Path) -> bool {
    let bin_dir = frontend.join("node_modules/.bin");
    bin_dir.join("tsc").is_file() || bin_dir.join("tsc.cmd").is_file()
}

/// Cài dependencies chỉ khi frontend có package.json nhưng local TypeScript
/// binary đang thiếu. `npm ci` tái lập đúng package-lock; project chưa có lock
/// mới dùng `npm install` để tạo dependency tree lần đầu.
fn ensure_frontend_dependencies(config_dir: &Path, tx: &Sender<BuildEvent>, counter: &mut usize) -> Result<(), String> {
    let frontend = frontend_dir(config_dir);
    if !frontend.join("package.json").is_file() || frontend_dependencies_ready(&frontend) {
        return Ok(());
    }

    let use_ci = frontend.join("package-lock.json").is_file();
    let command = if use_ci { "npm ci" } else { "npm install" };
    let _ = tx.send(BuildEvent::Stage(format!(
        "Cài frontend dependencies: {} ({command})",
        frontend.display()
    )));
    let _ = tx.send(BuildEvent::Log(
        "Không tìm thấy node_modules/.bin/tsc; đang cài dependencies frontend.".to_string(),
    ));

    let mut npm = Command::new("npm");
    npm.arg(if use_ci { "ci" } else { "install" }).current_dir(&frontend);
    run_streaming(npm, tx, 0, counter).map_err(|error| {
        format!(
            "Không thể cài frontend dependencies trong {} bằng `{command}`: {error}",
            frontend.display()
        )
    })?;

    if frontend_dependencies_ready(&frontend) {
        Ok(())
    } else {
        Err(format!(
            "`{command}` đã xong nhưng vẫn không tìm thấy {}. Kiểm tra package.json có devDependency `typescript`.",
            frontend.join("node_modules/.bin/tsc").display()
        ))
    }
}

/// Đếm tổng số crate cần biên dịch cho RIÊNG package đang build.
///
/// Trước đây đếm `packages.len()` của cả workspace (909) trong khi build một
/// app chỉ sinh ~450 artifact → thanh gauge kẹt ở giữa dù đã xong. Giờ duyệt
/// bao đóng phụ thuộc (transitive closure) của package qua `resolve.nodes`
/// trong `cargo metadata` nên mẫu số khớp với số artifact thực tế.
fn total_units(dir: &Path, package: &str) -> usize {
    const FALLBACK: usize = 300;
    let out = Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .current_dir(dir)
        .output()
        .ok()
        .filter(|o| o.status.success());
    let Some(out) = out else { return FALLBACK };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return FALLBACK;
    };

    // Tìm id của package cần build trong workspace members.
    let members: Vec<&str> = v
        .get("workspace_members")
        .and_then(|m| m.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    let root_id = v
        .get("packages")
        .and_then(|p| p.as_array())
        .and_then(|pkgs| {
            pkgs.iter().find(|p| {
                p.get("name").and_then(|n| n.as_str()) == Some(package)
                    && p.get("id")
                        .and_then(|id| id.as_str())
                        .is_some_and(|id| members.contains(&id))
            })
        })
        .and_then(|p| p.get("id"))
        .and_then(|id| id.as_str());
    let Some(root_id) = root_id else { return FALLBACK };

    // BFS qua resolve.nodes để đếm bao đóng.
    let nodes: Vec<&serde_json::Value> = v
        .get("resolve")
        .and_then(|r| r.get("nodes"))
        .and_then(|n| n.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    if nodes.is_empty() {
        return FALLBACK;
    }
    let mut seen = std::collections::HashSet::new();
    let mut stack = vec![root_id.to_string()];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(node) = nodes
            .iter()
            .find(|n| n.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
            && let Some(deps) = node.get("deps").and_then(|d| d.as_array())
        {
            for d in deps {
                if let Some(pkg) = d.get("pkg").and_then(|p| p.as_str()) {
                    stack.push(pkg.to_string());
                }
            }
        }
    }
    if seen.is_empty() { FALLBACK } else { seen.len() }
}

/// Chạy một lệnh, chuyển từng dòng stdout/stderr thành `BuildEvent`.
/// Trả về `Ok(())` nếu exit code 0.
fn run_streaming(mut cmd: Command, tx: &Sender<BuildEvent>, total: usize, counter: &mut usize) -> Result<(), String> {
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

    let status = child.wait().map_err(|e| format!("Lỗi khi đợi tiến trình: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("Lệnh thất bại với mã: {status}"))
    }
}

/// Tên hàm: build_project
/// Mô tả: Build một project và copy binary + tài nguyên vào `release/<release_name>/`.
/// Gửi toàn bộ tiến trình qua `tx`; luôn kết thúc bằng `BuildEvent::Finished`.
pub fn build_project(root: PathBuf, project: Project, tx: Sender<BuildEvent>) {
    // Đếm tổng crate tại chính thư mục build (workspace lồng có bộ dep riêng).
    let count_dir = match &project.kind {
        BuildKind::Tauri { config_dir } => config_dir.clone(),
        BuildKind::Cargo => root.clone(),
    };
    let total = total_units(&count_dir, &project.package);
    let mut counter = 0usize;

    let result = match &project.kind {
        BuildKind::Tauri { config_dir } => {
            let _ = tx.send(BuildEvent::Stage(format!(
                "Build Tauri: {} (frontend + backend)",
                project.release_name
            )));
            // Chặn sớm nếu Node quá cũ, thay vì để npm chết giữa chừng.
            if let Err(e) = check_node_for_tauri(&tx) {
                let _ = tx.send(BuildEvent::Finished {
                    ok: false,
                    message: format!("{} — build thất bại: {}", project.release_name, e),
                });
                return;
            }
            if let Err(e) = ensure_frontend_dependencies(config_dir, &tx, &mut counter) {
                let _ = tx.send(BuildEvent::Finished {
                    ok: false,
                    message: format!("{} — build thất bại: {}", project.release_name, e),
                });
                return;
            }
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
            let _ = tx.send(BuildEvent::Stage(format!("Build Cargo: {}", project.package)));
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
            message: format!("{} — build thất bại: {}", project.release_name, e),
        });
        return;
    }

    // ── Xuất vào release/ ────────────────────────────────────────────────────
    let _ = tx.send(BuildEvent::Stage(format!("Xuất vào release/{}/", project.release_name)));

    // Binary mang tên crate; thư mục + file xuất ra mang tên sản phẩm.
    let src = project.target_dir.join("release").join(&project.bin_name);
    if !src.is_file() {
        let _ = tx.send(BuildEvent::Finished {
            ok: false,
            message: format!(
                "Không tìm thấy binary: {} (build xong nhưng thiếu file?)",
                src.display()
            ),
        });
        return;
    }

    let dest_dir = root.join("release").join(&project.release_name);
    if let Err(e) = std::fs::create_dir_all(&dest_dir) {
        let _ = tx.send(BuildEvent::Finished {
            ok: false,
            message: format!("Không tạo được thư mục release: {e}"),
        });
        return;
    }

    let dest = dest_dir.join(&project.release_name);
    // Xoá đích trước khi copy: ghi đè trực tiếp (open O_TRUNC) lên file binary
    // ĐANG CHẠY bị kernel chặn với ETXTBSY ("Text file busy", os error 26).
    // Unlink thì luôn được — tiến trình đang chạy giữ inode cũ, file mới nhận
    // inode mới. Lỗi NotFound (chưa từng xuất) thì bỏ qua.
    match std::fs::remove_file(&dest) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            let _ = tx.send(BuildEvent::Finished {
                ok: false,
                message: format!("Không xoá được binary cũ (có app đang chạy giữ file?): {e}"),
            });
            return;
        }
    }
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
        "Binary → release/{}/{} ({})",
        project.release_name,
        project.release_name,
        human_size(size)
    )));

    // Với app Tauri, copy kèm tài nguyên chạy ngoài binary.
    if let (BuildKind::Tauri { config_dir }, Some(app_root)) = (&project.kind, &project.app_root) {
        copy_app_resources(app_root, config_dir, &dest_dir, &tx);
    }

    let _ = tx.send(BuildEvent::Finished {
        ok: true,
        message: format!("{} — xong ({})", project.release_name, human_size(size)),
    });
}

/// Copy tài nguyên chạy kèm của app Tauri: `frontend/dist`, các thư mục runtime
/// và icons. Thiếu chúng thì app vẫn chạy nhưng mất theme/ngôn ngữ.
fn copy_app_resources(app_root: &Path, config_dir: &Path, dest_dir: &Path, tx: &Sender<BuildEvent>) {
    // frontend/dist — build output thuần (tên file có hash), KHÔNG chứa dữ liệu
    // người dùng nên thay trọn bộ để asset cũ không dồn lại. Nhưng thay qua
    // staging rồi mới đổi tên: copy lỗi giữa đường không làm mất bản đang chạy.
    let dist = app_root.join("frontend/dist");
    if dist.is_dir() {
        replace_dir_atomically(&dist, &dest_dir.join("dist"), "dist", tx);
    }

    // langs/themes/fonts — người dùng CÓ thả file riêng vào đây (font .ttf tự
    // tải, theme tự viết, bản dịch thêm). Vì vậy chỉ ghi đè file trùng tên và
    // giữ nguyên file lạ, KHÔNG xoá thư mục. Trước đây `remove_dir_all` làm mất
    // toàn bộ font/theme người dùng thêm sau mỗi lần build.
    for name in APP_RESOURCES {
        let from = app_root.join(name);
        if !from.is_dir() {
            continue;
        }
        let to = dest_dir.join(name);
        let kept = count_extra_files(&from, &to);
        match merge_dir(&from, &to) {
            Ok(copied) => {
                let mut msg = format!("Đã cập nhật {name}/ ({copied} file)");
                if kept > 0 {
                    msg.push_str(&format!(", giữ {kept} file riêng của bạn"));
                }
                let _ = tx.send(BuildEvent::Log(msg));
            }
            Err(e) => {
                let _ = tx.send(BuildEvent::Warn(format!("Không cập nhật được {name}/: {e}")));
            }
        }
    }

    // Icons nằm cạnh tauri.conf.json.
    let icons = config_dir.join("icons");
    if icons.is_dir() {
        let to = dest_dir.join("icons");
        let _ = std::fs::create_dir_all(&to);
        let mut copied = 0usize;
        if let Ok(entries) = std::fs::read_dir(&icons) {
            for entry in entries.flatten() {
                let p = entry.path();
                let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                // Chỉ lấy định dạng icon dùng khi chạy/đóng gói.
                if matches!(ext.as_str(), "png" | "ico" | "icns") {
                    if std::fs::copy(&p, to.join(entry.file_name())).is_ok() {
                        copied += 1;
                    }
                }
            }
        }
        if copied > 0 {
            let _ = tx.send(BuildEvent::Log(format!("Đã copy {copied} icon")));
        }
    }
}

/// Copy đệ quy một thư mục (tạo mới hoặc ghi đè file trùng tên).
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

/// Ghi đè `to` bằng nội dung `from` NHƯNG giữ lại các file chỉ có ở `to`.
/// Trả về số file đã ghi. Dùng cho tài nguyên mà người dùng có thể bổ sung.
fn merge_dir(from: &Path, to: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(to)?;
    let mut copied = 0usize;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copied += merge_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Đếm file chỉ tồn tại ở `to` (người dùng tự thêm) để báo lại cho họ biết
/// những file đó được giữ nguyên.
fn count_extra_files(from: &Path, to: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(to) else {
        return 0;
    };
    let mut n = 0usize;
    for entry in entries.flatten() {
        let dst = entry.path();
        let src = from.join(entry.file_name());
        if dst.is_dir() {
            n += count_extra_files(&src, &dst);
        } else if !src.exists() {
            n += 1;
        }
    }
    n
}

/// Thay trọn nội dung `to` bằng `from` một cách an toàn: dựng thư mục tạm cạnh
/// đích, copy xong mới hoán đổi. Nếu copy lỗi thì đích cũ vẫn còn nguyên —
/// trước đây `remove_dir_all` chạy trước copy nên lỗi giữa đường là mất trắng.
fn replace_dir_atomically(from: &Path, to: &Path, label: &str, tx: &Sender<BuildEvent>) {
    let staging = to.with_extension("new");
    let _ = std::fs::remove_dir_all(&staging);
    if let Err(e) = copy_dir(from, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        let _ = tx.send(BuildEvent::Warn(format!("Không copy được {label}/: {e}")));
        return;
    }

    let backup = to.with_extension("old");
    let _ = std::fs::remove_dir_all(&backup);
    let had_old = to.exists();
    if had_old && std::fs::rename(to, &backup).is_err() {
        // Không đổi tên được (khác filesystem/quyền): rơi về copy trực tiếp.
        let _ = std::fs::remove_dir_all(&staging);
        match copy_dir(from, to) {
            Ok(_) => {
                let _ = tx.send(BuildEvent::Log(format!("Đã cập nhật {label}/")));
            }
            Err(e) => {
                let _ = tx.send(BuildEvent::Warn(format!("Không copy được {label}/: {e}")));
            }
        }
        return;
    }

    if let Err(e) = std::fs::rename(&staging, to) {
        // Hoán đổi thất bại → phục hồi bản cũ.
        if had_old {
            let _ = std::fs::rename(&backup, to);
        }
        let _ = std::fs::remove_dir_all(&staging);
        let _ = tx.send(BuildEvent::Warn(format!("Không thay được {label}/: {e}")));
        return;
    }

    let _ = std::fs::remove_dir_all(&backup);
    let _ = tx.send(BuildEvent::Log(format!("Đã cập nhật {label}/")));
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
    fn node_version_gate() {
        // Case thật của user: node hệ thống v18 → phải chặn.
        assert!(!node_version_ok("v18.19.1"));
        assert!(!node_version_ok("v20.18.0"));
        // Vite 8 cần ^20.19 || >=22.12.
        assert!(node_version_ok("v20.19.0"));
        assert!(node_version_ok("v20.20.2"));
        assert!(node_version_ok("v22.12.0"));
        assert!(node_version_ok("v26.3.1"));
        // Rác vào → chặn an toàn.
        assert!(!node_version_ok(""));
        assert!(!node_version_ok("vXX"));
    }

    #[test]
    fn frontend_dir_dung_cwd_trong_tauri_config() {
        let base = std::env::temp_dir().join(format!("gui-builder-frontend-dir-{}", std::process::id()));
        let backend = base.join("app/backend");
        std::fs::create_dir_all(&backend).unwrap();
        std::fs::write(
            backend.join("tauri.conf.json"),
            r#"{"build":{"beforeBuildCommand":{"cwd":"../web"}}}"#,
        )
        .unwrap();

        assert_eq!(frontend_dir(&backend), base.join("app/web"));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn frontend_dependencies_can_be_detected_from_local_tsc() {
        let base = std::env::temp_dir().join(format!("gui-builder-tsc-{}", std::process::id()));
        assert!(!frontend_dependencies_ready(&base));
        std::fs::create_dir_all(base.join("node_modules/.bin")).unwrap();
        std::fs::write(base.join("node_modules/.bin/tsc"), b"").unwrap();
        assert!(frontend_dependencies_ready(&base));
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn total_units_returns_positive() {
        // Kể cả khi cargo lỗi, hàm phải trả về mốc dự phòng > 0 để không chia cho 0.
        let n = total_units(Path::new("/definitely/not/a/workspace"), "nope");
        assert!(n > 0);
    }

    /// Kiểm tra TOÀN BỘ bước copy tài nguyên (không chỉ hàm con): sau khi
    /// "build lại", font/theme người dùng thả vào release phải còn, dist phải
    /// được thay mới.
    #[test]
    fn copy_app_resources_giu_tai_nguyen_nguoi_dung() {
        let base = std::env::temp_dir().join("gui_builder_resources_test");
        let _ = std::fs::remove_dir_all(&base);

        // Cây nguồn giả lập: app_root có dist + langs + themes + fonts.
        let app_root = base.join("app");
        std::fs::create_dir_all(app_root.join("frontend/dist/assets")).unwrap();
        std::fs::write(app_root.join("frontend/dist/assets/index-new.js"), b"bundle moi").unwrap();
        for (dir, file, content) in [
            ("langs", "vi.json", "{}"),
            ("themes", "default.json", "moi"),
            ("fonts", "README.md", "huong dan"),
        ] {
            std::fs::create_dir_all(app_root.join(dir)).unwrap();
            std::fs::write(app_root.join(dir).join(file), content).unwrap();
        }
        let config_dir = app_root.join("backend");
        std::fs::create_dir_all(config_dir.join("icons")).unwrap();
        std::fs::write(config_dir.join("icons/32x32.png"), b"png").unwrap();

        // Thư mục release đang dùng: có asset cũ + file riêng của người dùng.
        let dest = base.join("release/app");
        std::fs::create_dir_all(dest.join("dist/assets")).unwrap();
        std::fs::write(dest.join("dist/assets/index-old.js"), b"bundle cu").unwrap();
        std::fs::create_dir_all(dest.join("fonts/local")).unwrap();
        std::fs::write(dest.join("fonts/local/MyFont.ttf"), b"font cua toi").unwrap();
        std::fs::create_dir_all(dest.join("themes")).unwrap();
        std::fs::write(dest.join("themes/my_theme.json"), b"theme cua toi").unwrap();
        std::fs::create_dir_all(dest.join("langs")).unwrap();
        std::fs::write(dest.join("langs/ja.json"), b"ban dich cua toi").unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        copy_app_resources(&app_root, &config_dir, &dest, &tx);
        drop(tx);
        let _: Vec<_> = rx.iter().collect();

        // Tài nguyên người dùng tự thêm: PHẢI còn.
        for (path, expect) in [
            ("fonts/local/MyFont.ttf", "font cua toi"),
            ("themes/my_theme.json", "theme cua toi"),
            ("langs/ja.json", "ban dich cua toi"),
        ] {
            assert_eq!(
                std::fs::read_to_string(dest.join(path)).unwrap_or_default(),
                expect,
                "{path} bị mất sau khi build lại"
            );
        }

        // File từ repo: được ghi đè/thêm mới.
        assert_eq!(
            std::fs::read_to_string(dest.join("themes/default.json")).unwrap(),
            "moi"
        );
        assert!(dest.join("langs/vi.json").is_file());
        assert!(dest.join("icons/32x32.png").is_file());

        // dist: thay trọn bộ, không dồn asset cũ.
        assert!(dest.join("dist/assets/index-new.js").is_file());
        assert!(
            !dest.join("dist/assets/index-old.js").exists(),
            "asset cũ phải được dọn"
        );

        let _ = std::fs::remove_dir_all(&base);
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
        assert_eq!(std::fs::read_to_string(dst.join("sub/a.txt")).unwrap(), "hello");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Bug đã gặp: mỗi lần build lại, `remove_dir_all` xoá sạch `fonts/` và
    /// `themes/` ở release nên font .ttf người dùng tự thả vào và theme tự viết
    /// đều mất. `merge_dir` phải ghi đè file trùng tên mà giữ file lạ.
    #[test]
    fn merge_dir_giu_file_nguoi_dung_them() {
        let base = std::env::temp_dir().join("gui_builder_merge_test");
        let _ = std::fs::remove_dir_all(&base);

        // Nguồn (repo): 1 theme mặc định.
        let from = base.join("from");
        std::fs::create_dir_all(from.join("local")).unwrap();
        std::fs::write(from.join("default.json"), b"moi").unwrap();

        // Đích (release đang dùng): theme mặc định bản cũ + font người dùng thả vào.
        let to = base.join("to");
        std::fs::create_dir_all(to.join("local")).unwrap();
        std::fs::write(to.join("default.json"), b"cu").unwrap();
        std::fs::write(to.join("my_theme.json"), b"cua toi").unwrap();
        std::fs::write(to.join("local/Inter.ttf"), b"font").unwrap();

        let extra = count_extra_files(&from, &to);
        let copied = merge_dir(&from, &to).unwrap();

        // File của repo được ghi đè.
        assert_eq!(std::fs::read_to_string(to.join("default.json")).unwrap(), "moi");
        // File riêng của người dùng còn nguyên — đây chính là hồi quy cần chặn.
        assert_eq!(std::fs::read_to_string(to.join("my_theme.json")).unwrap(), "cua toi");
        assert_eq!(std::fs::read_to_string(to.join("local/Inter.ttf")).unwrap(), "font");
        assert_eq!(copied, 1, "chỉ ghi file từ nguồn");
        assert_eq!(extra, 2, "phải nhận ra 2 file riêng của người dùng");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// `dist/` thay trọn bộ (tên file có hash, không phải dữ liệu người dùng)
    /// nhưng phải qua staging: file cũ chỉ bị bỏ SAU khi bản mới copy xong.
    #[test]
    fn replace_dir_atomically_thay_tron_bo() {
        let base = std::env::temp_dir().join("gui_builder_replace_test");
        let _ = std::fs::remove_dir_all(&base);

        let from = base.join("from/assets");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("index-new.js"), b"new").unwrap();

        let to = base.join("to");
        std::fs::create_dir_all(to.join("assets")).unwrap();
        std::fs::write(to.join("assets/index-old.js"), b"old").unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        replace_dir_atomically(&base.join("from"), &to, "dist", &tx);
        drop(tx);

        assert!(to.join("assets/index-new.js").is_file(), "thiếu asset mới");
        assert!(
            !to.join("assets/index-old.js").exists(),
            "asset cũ phải bị dọn để không dồn rác"
        );
        // Không để lại thư mục tạm.
        assert!(!to.with_extension("new").exists());
        assert!(!to.with_extension("old").exists());
        assert!(
            rx.iter().any(|e| matches!(e, BuildEvent::Log(_))),
            "phải báo log thành công"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Nguồn lỗi (không tồn tại) thì đích cũ PHẢI còn nguyên — trước đây xoá
    /// trước copy nên lỗi giữa đường là mất trắng.
    #[test]
    fn replace_dir_atomically_khong_lam_mat_ban_cu_khi_loi() {
        let base = std::env::temp_dir().join("gui_builder_replace_fail_test");
        let _ = std::fs::remove_dir_all(&base);

        let to = base.join("to");
        std::fs::create_dir_all(&to).unwrap();
        std::fs::write(to.join("keep.js"), b"quan trong").unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        replace_dir_atomically(&base.join("khong-ton-tai"), &to, "dist", &tx);
        drop(tx);

        assert_eq!(
            std::fs::read_to_string(to.join("keep.js")).unwrap(),
            "quan trong",
            "copy lỗi không được làm mất bản đang chạy"
        );
        assert!(!to.with_extension("new").exists(), "phải dọn thư mục tạm");
        assert!(
            rx.iter().any(|e| matches!(e, BuildEvent::Warn(_))),
            "phải cảnh báo khi lỗi"
        );

        let _ = std::fs::remove_dir_all(&base);
    }
}
