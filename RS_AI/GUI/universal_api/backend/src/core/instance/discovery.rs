//! Editor Binary Discovery Module.
//!
//! Tự động quét và tìm kiếm file thực thi của các IDE (Cursor, VS Code, Windsurf, CodeBuddy, Zed).

use std::path::PathBuf;

/// Danh sách binary gợi ý cho từng nền tảng IDE.
pub fn candidate_binary_names<'a>(platform_id: &'a str) -> Vec<&'a str> {
    match platform_id.to_lowercase().as_str() {
        "antigravity" | "antigravity_ide" | "antigravity_desktop" => {
            vec!["antigravity-ide", "antigravity", "Antigravity", "antigravity_ide"]
        }
        "codex" => vec!["codex", "Codex"],
        "claude" => vec!["claude", "Claude"],
        "cursor" => vec!["cursor", "Cursor"],
        "windsurf" => vec!["windsurf", "Windsurf"],
        "codebuddy_cn" => vec!["codebuddy-cn", "codebuddy", "CodeBuddy", "coding-copilot"],
        "codebuddy" | "codebuddy_global" => vec!["codebuddy", "CodeBuddy"],
        "zed" => vec!["zed", "zed-editor", "Zed"],
        "trae" | "trae_cn" => vec!["trae", "Trae"],
        "vscode" | "github_copilot" => vec!["code", "vscode", "code-oss"],
        _ => vec![platform_id],
    }
}

/// Tìm kiếm binary trong biến môi trường PATH hoặc các thư mục hệ thống chuẩn.
pub fn find_executable(platform_id: &str) -> Option<PathBuf> {
    let names = candidate_binary_names(platform_id);

    // 1. Kiểm tra trong PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for name in &names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    // 2. Kiểm tra các thư mục mặc định trên Linux
    #[cfg(target_os = "linux")]
    {
        let standard_dirs = [
            "/usr/bin",
            "/usr/local/bin",
            "/opt",
            "/var/lib/flatpak/exports/bin",
            "/snap/bin",
        ];
        for dir in &standard_dirs {
            for name in &names {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            let local_bins = [
                home.join(".local/bin"),
                home.join(".local/share/applications"),
                home.join(".cargo/bin"),
            ];
            for dir in &local_bins {
                for name in &names {
                    let candidate = dir.join(name);
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }

            // Quét ~/Applications (AppImage hoặc binary bundle)
            let apps_dir = home.join("Applications");
            if apps_dir.is_dir() {
                for name in &names {
                    let direct = apps_dir.join(name);
                    if direct.is_file() {
                        return Some(direct);
                    }
                    let nested = apps_dir.join(name).join(name);
                    if nested.is_file() {
                        return Some(nested);
                    }
                }
            }
        }
    }

    None
}
