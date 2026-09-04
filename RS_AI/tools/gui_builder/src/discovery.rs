/*
[INTEGRITY NOTES]
- Mục đích: Phát hiện động các project có thể build trong workspace RS_AI.
- Trách nhiệm: Đọc `cargo metadata`, phân loại Tauri / Cargo thuần, tìm tên binary,
  tên xuất release và thư mục target. Không hard-code danh sách project.
- Tương tác: Dùng bởi `app.rs` (dựng danh sách) và `builder.rs` (chạy build).

Vì sao không hard-code: `build_release.sh` cũ liệt kê tay 11 project và bỏ sót
`rclone_gui` — thêm project mới vào workspace là quên cập nhật script.

Ba trường hợp đặc biệt phải xử lý (đều có thật trong repo này):
  1. Package Tauri có `name` khác tên sản phẩm (ví dụ `name = "app"` của template).
     → Lấy `productName` trong tauri.conf.json làm tên xuất release.
  2. Crate Tauri nằm trong workspace lồng (`[workspace]` riêng) nên
     `cargo metadata` ở root KHÔNG thấy — phải quét thêm `tauri.conf.json`.
  3. Crate ở workspace lồng có `target/` riêng, không dùng target chung.
*/

use std::collections::HashSet;
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
    /// Tên binary cargo sinh ra trong `<target_dir>/release/`.
    pub bin_name: String,
    /// Tên thư mục + file khi xuất vào `release/`.
    /// Với app Tauri là `productName`, nên thư mục mang tên sản phẩm thật thay vì
    /// tên crate mặc định của template (`app`, `*_tauri`).
    pub release_name: String,
    /// Đường dẫn thư mục chứa Cargo.toml, tương đối với workspace root.
    pub rel_dir: String,
    pub kind: BuildKind,
    /// Thư mục `target` chứa binary sau khi build. Crate ở workspace lồng có
    /// target riêng, không dùng target chung của workspace gốc.
    pub target_dir: PathBuf,
    /// Thư mục gốc của app (chứa `frontend/`, `langs/`, `themes/`…) để copy
    /// tài nguyên chạy kèm. `None` với crate không phải app Tauri.
    pub app_root: Option<PathBuf>,
}

#[derive(serde::Deserialize)]
struct Metadata {
    packages: Vec<MetaPackage>,
    workspace_members: Vec<String>,
    workspace_root: String,
    target_directory: String,
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

/// Đọc `productName` trong tauri.conf.json. Đây là tên sản phẩm thật, dùng làm
/// tên xuất release thay cho tên crate (template Tauri hay để `name = "app"`).
fn product_name(config_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(config_dir.join("tauri.conf.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    v.get("productName")
        .and_then(|p| p.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

/// Chạy `cargo metadata --no-deps` tại `dir` và trả về JSON đã parse.
fn read_metadata(dir: &Path) -> Result<Metadata, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("Không gọi được cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("Lỗi đọc cargo metadata: {e}"))
}

/// Package có dùng `tauri-build` trong build-dependencies?
fn uses_tauri_build(pkg: &MetaPackage) -> bool {
    pkg.build_dependencies
        .iter()
        .chain(
            pkg.dependencies
                .iter()
                .filter(|d| d.kind.as_deref() == Some("build")),
        )
        .any(|d| d.name == "tauri-build")
}

/// Dựng `Project` từ một package đã biết là thuộc workspace nào đó.
fn make_project(
    pkg: &MetaPackage,
    root_for_rel: &Path,
    target_dir: PathBuf,
) -> Option<Project> {
    let bin = pkg
        .targets
        .iter()
        .find(|t| t.kind.iter().any(|k| k == "bin"))?;

    let manifest = PathBuf::from(&pkg.manifest_path);
    let dir = manifest.parent()?;
    let rel_dir = dir
        .strip_prefix(root_for_rel)
        .unwrap_or(dir)
        .to_string_lossy()
        .to_string();

    let is_tauri = uses_tauri_build(pkg) && dir.join("tauri.conf.json").is_file();

    let (kind, release_name, app_root) = if is_tauri {
        let product = product_name(dir).unwrap_or_else(|| bin.name.clone());
        (
            BuildKind::Tauri {
                config_dir: dir.to_path_buf(),
            },
            product,
            dir.parent().map(Path::to_path_buf),
        )
    } else {
        (BuildKind::Cargo, bin.name.clone(), None)
    };

    Some(Project {
        package: pkg.name.clone(),
        bin_name: bin.name.clone(),
        release_name,
        rel_dir,
        kind,
        target_dir,
        app_root,
    })
}

/// Quét thêm các crate Tauri nằm trong workspace lồng (`[workspace]` riêng), vì
/// `cargo metadata` ở root không liệt kê chúng.
fn discover_nested(root: &Path, known_dirs: &HashSet<PathBuf>) -> Vec<Project> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    // Không đi vào các thư mục nặng/không liên quan.
    const SKIP: [&str; 6] = ["target", "node_modules", ".git", "dist", "release", ".ua"];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if SKIP.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }

            // Là crate Tauri chưa được workspace gốc quản lý?
            if path.join("tauri.conf.json").is_file()
                && path.join("Cargo.toml").is_file()
                && !known_dirs.contains(&path)
            {
                if let Ok(meta) = read_metadata(&path) {
                    let target_dir = PathBuf::from(&meta.target_directory);
                    let members: HashSet<&String> = meta.workspace_members.iter().collect();
                    for pkg in &meta.packages {
                        if !members.contains(&pkg.id) {
                            continue;
                        }
                        if let Some(p) = make_project(pkg, root, target_dir.clone()) {
                            found.push(p);
                        }
                    }
                }
                continue; // Không đi sâu thêm vào crate đã xử lý
            }

            stack.push(path);
        }
    }
    found
}

/// Tên hàm: discover
/// Mô tả: Quét workspace, trả về danh sách project build được, sắp theo tên xuất release.
///
/// Chỉ nhận package có target kiểu `bin` — crate chỉ có `lib` không tạo ra
/// binary nào để xuất vào `release/`.
pub fn discover(root: &Path) -> Result<Vec<Project>, String> {
    let meta = read_metadata(root)?;
    let root_path = PathBuf::from(&meta.workspace_root);
    let workspace_target = PathBuf::from(&meta.target_directory);

    let members: HashSet<&String> = meta.workspace_members.iter().collect();
    let mut projects = Vec::new();
    let mut known_dirs: HashSet<PathBuf> = HashSet::new();

    for pkg in &meta.packages {
        if !members.contains(&pkg.id) {
            continue;
        }
        // Bỏ chính công cụ này khỏi danh sách.
        if pkg.name == "gui_builder" {
            continue;
        }
        if let Some(dir) = PathBuf::from(&pkg.manifest_path).parent() {
            known_dirs.insert(dir.to_path_buf());
        }
        if let Some(p) = make_project(pkg, &root_path, workspace_target.clone()) {
            projects.push(p);
        }
    }

    projects.extend(discover_nested(&root_path, &known_dirs));

    projects.sort_by(|a, b| {
        a.release_name
            .to_lowercase()
            .cmp(&b.release_name.to_lowercase())
    });
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_finds_workspace_projects() {
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
            assert!(p.app_root.is_some(), "app Tauri phải biết app_root");
        }
    }

    #[test]
    fn tauri_release_name_uses_product_name() {
        let Ok(root) = workspace_root() else { return };
        let Ok(list) = discover(&root) else { return };
        // Trước đây crate này tên "app" (template Tauri) nên release/ sinh ra
        // thư mục "app" — không nhận ra là project nào.
        if let Some(p) = list.iter().find(|p| p.package == "subscription_manager_gui") {
            assert_eq!(
                p.release_name, "subscription_manager_gui",
                "release_name phải lấy từ productName trong tauri.conf.json"
            );
            assert_ne!(p.release_name, "app", "không được dùng tên crate template");
        }
    }

    #[test]
    fn discovers_nested_workspace_tauri_crate() {
        let Ok(root) = workspace_root() else { return };
        let Ok(list) = discover(&root) else { return };
        // filen_gui_tauri nằm ở GUI/filen_gui/bridge với [workspace] riêng nên
        // `cargo metadata` ở root không thấy — phải được quét bổ sung.
        if root.join("GUI/filen_gui/bridge/tauri.conf.json").is_file() {
            let p = list
                .iter()
                .find(|p| p.package == "filen_gui_tauri")
                .expect("phải tìm được crate Tauri ở workspace lồng");
            assert_eq!(p.release_name, "filen_gui", "dùng productName, không phải tên crate");
            // Target riêng, không phải target chung của workspace gốc.
            assert!(
                p.target_dir.starts_with(root.join("GUI/filen_gui/bridge")),
                "crate workspace lồng phải dùng target riêng: {:?}",
                p.target_dir
            );
        }
    }

    #[test]
    fn no_duplicate_release_names() {
        let Ok(root) = workspace_root() else { return };
        let Ok(list) = discover(&root) else { return };
        let mut seen = HashSet::new();
        for p in &list {
            assert!(
                seen.insert(p.release_name.clone()),
                "trùng tên xuất release: {} — hai project sẽ ghi đè nhau",
                p.release_name
            );
        }
    }
}
