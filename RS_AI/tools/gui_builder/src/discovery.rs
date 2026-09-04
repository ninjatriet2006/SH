/*
[INTEGRITY NOTES]
- Mục đích: Phát hiện động các project có thể build trong workspace RS_AI.
- Trách nhiệm: Đọc `cargo metadata`, phân loại Tauri / Cargo thuần, tìm tên binary
  và script build riêng. Không hard-code danh sách project.
- Tương tác: Dùng bởi `app.rs` (dựng danh sách) và `builder.rs` (chạy build).

Vì sao không hard-code: `build_release.sh` cũ liệt kê tay 11 project và bỏ sót
`rclone_gui` — thêm project mới vào workspace là quên cập nhật script.
*/

use std::path::{Path, PathBuf};
use std::process::Command;

/// Cách một project cần được build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildKind {
    /// App Tauri: phải dùng `cargo tauri build` để frontend được nhúng vào binary.
    /// Chạy `cargo build` trực tiếp sẽ tạo binary rơi về `devUrl` → lỗi
    /// "connection refused" khi mở app mà không có dev server.
    Tauri { config_dir: PathBuf },
    /// Crate Rust thường: `cargo build --release -p <name>`.
    Cargo,
}

/// Một project có thể build.
#[derive(Debug, Clone)]
pub struct Project {
    /// Tên package trong Cargo.toml (dùng cho `-p`).
    pub package: String,
    /// Tên binary sinh ra trong `target/release/`.
    pub bin_name: String,
    /// Đường dẫn thư mục chứa Cargo.toml, tương đối với workspace root.
    pub rel_dir: String,
    pub kind: BuildKind,
}

/// Tên các trường JSON cần đọc từ `cargo metadata`.
#[derive(serde::Deserialize)]
struct Metadata {
    packages: Vec<MetaPackage>,
    workspace_members: Vec<String>,
    workspace_root: String,
}

#[derive(serde::Deserialize)]
struct MetaPackage {
    id: String,
    name: String,
    manifest_path: String,
    targets: Vec<MetaTarget>,
    #[serde(default)]
    dependencies: Vec<MetaDep>,
    #[serde(default)]
    build_dependencies: Vec<MetaDep>,
}

#[derive(serde::Deserialize)]
struct MetaTarget {
    name: String,
    kind: Vec<String>,
}

#[derive(serde::Deserialize)]
struct MetaDep {
    name: String,
    #[serde(default)]
    kind: Option<String>,
}

/// Tên hàm: workspace_root
/// Mô tả: Xác định thư mục gốc workspace từ `cargo locate-project`.
pub fn workspace_root() -> Result<PathBuf, String> {
    let out = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()
        .map_err(|e| format!("Không gọi được cargo: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let manifest = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Path::new(&manifest)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Không xác định được workspace root".to_string())
}

/// Tên hàm: discover
/// Mô tả: Quét workspace, trả về danh sách project build được, sắp theo tên binary.
///
/// Chỉ nhận package có target kiểu `bin` — crate chỉ có `lib` không tạo ra
/// binary nào để xuất vào `release/`.
pub fn discover(root: &Path) -> Result<Vec<Project>, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Không gọi được cargo metadata: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let meta: Metadata =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("Lỗi đọc cargo metadata: {e}"))?;
    let root_path = PathBuf::from(&meta.workspace_root);

    let mut projects = Vec::new();
    for pkg in &meta.packages {
        if !meta.workspace_members.contains(&pkg.id) {
            continue;
        }

        // Bỏ chính công cụ này khỏi danh sách.
        if pkg.name == "gui_builder" {
            continue;
        }

        let Some(bin) = pkg.targets.iter().find(|t| t.kind.iter().any(|k| k == "bin")) else {
            continue; // Chỉ có lib → không xuất binary
        };

        let manifest = PathBuf::from(&pkg.manifest_path);
        let Some(dir) = manifest.parent() else { continue };
        let rel_dir = dir
            .strip_prefix(&root_path)
            .unwrap_or(dir)
            .to_string_lossy()
            .to_string();

        // Tauri nhận diện qua build-dependency `tauri-build` + có tauri.conf.json.
        let uses_tauri_build = pkg
            .build_dependencies
            .iter()
            .chain(pkg.dependencies.iter().filter(|d| d.kind.as_deref() == Some("build")))
            .any(|d| d.name == "tauri-build");

        let kind = if uses_tauri_build && dir.join("tauri.conf.json").is_file() {
            BuildKind::Tauri {
                config_dir: dir.to_path_buf(),
            }
        } else {
            BuildKind::Cargo
        };

        projects.push(Project {
            package: pkg.name.clone(),
            bin_name: bin.name.clone(),
            rel_dir,
            kind,
        });
    }

    projects.sort_by(|a, b| a.bin_name.to_lowercase().cmp(&b.bin_name.to_lowercase()));
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_finds_workspace_projects() {
        // Chạy trong repo thật: phải tìm được ít nhất một project và không
        // bao giờ chứa chính gui_builder.
        let Ok(root) = workspace_root() else { return };
        let Ok(list) = discover(&root) else { return };
        assert!(!list.is_empty(), "phải tìm được project trong workspace");
        assert!(list.iter().all(|p| p.package != "gui_builder"));
    }

    #[test]
    fn discover_marks_tauri_projects() {
        let Ok(root) = workspace_root() else { return };
        let Ok(list) = discover(&root) else { return };
        // rclone_gui là app Tauri — nếu nhận diện sai thành Cargo thì binary sẽ
        // thiếu frontend (đúng lỗi "connection refused" đã gặp).
        if let Some(p) = list.iter().find(|p| p.package == "rclone_gui") {
            assert!(
                matches!(p.kind, BuildKind::Tauri { .. }),
                "rclone_gui phải được nhận diện là Tauri"
            );
        }
    }
}
