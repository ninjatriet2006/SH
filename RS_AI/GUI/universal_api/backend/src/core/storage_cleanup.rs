//! Instance Storage & Cache Cleanup Engine.
//!
//! Scans managed IDE profile directories (`~/.cockpit_tools/instances/*`), detects orphan directories
//! no longer tracked in instance store, measures heavy cache folders (GPUCache, Code Cache, Service Worker),
//! and provides 1-click cleanups to recover SSD disk space safely without touching user credentials.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::core::profiles::store::get_platform_file_name;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrphanInstanceItem {
    pub path: String,
    pub platform: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheFolderItem {
    pub instance_path: String,
    pub cache_path: String,
    pub platform: String,
    pub folder_name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageScanReport {
    pub orphan_directories: Vec<OrphanInstanceItem>,
    pub cache_folders: Vec<CacheFolderItem>,
    pub total_orphan_bytes: u64,
    pub total_cache_bytes: u64,
    pub total_reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCleanReport {
    pub freed_bytes: u64,
    pub deleted_paths: Vec<String>,
    pub failed_paths: Vec<String>,
}

fn get_cockpit_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".cockpit_tools")
}

pub fn calculate_dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += calculate_dir_size(&p);
            }
        }
    }
    total
}

pub fn scan_storage() -> StorageScanReport {
    let cockpit_dir = get_cockpit_dir();
    let instances_base = cockpit_dir.join("instances");

    let mut orphans = Vec::new();
    let mut caches = Vec::new();
    let mut total_orphan_bytes = 0;
    let mut total_cache_bytes = 0;

    let platforms = [
        "antigravity", "cursor", "github_copilot", "windsurf", "trae", "zed", "codebuddy",
        "workbuddy", "kiro", "qoder", "zcode", "claude", "codex", "grok"
    ];

    for platform in platforms {
        let registry_file_name = get_platform_file_name(platform);
        let registry_path = cockpit_dir.join(&registry_file_name);

        let registered_dirs: HashSet<PathBuf> = if let Ok(content) = fs::read_to_string(&registry_path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                v.get("instances")
                    .and_then(|arr| arr.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|item| {
                                item.get("userDataDir")
                                    .or_else(|| item.get("user_data_dir"))
                                    .and_then(|x| x.as_str())
                                    .map(|s| PathBuf::from(s).canonicalize().unwrap_or_else(|_| PathBuf::from(s)))
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                HashSet::new()
            }
        } else {
            HashSet::new()
        };

        let platform_inst_dir = instances_base.join(platform);
        if platform_inst_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&platform_inst_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let canon = p.canonicalize().unwrap_or_else(|_| p.clone());
                        let is_registered = registered_dirs.iter().any(|reg| reg == &canon || reg == &p);

                        if !is_registered {
                            let size = calculate_dir_size(&p);
                            total_orphan_bytes += size;
                            orphans.push(OrphanInstanceItem {
                                path: p.to_string_lossy().to_string(),
                                platform: platform.to_string(),
                                size_bytes: size,
                            });
                        } else {
                            // Check for heavy cache folders inside registered instance
                            scan_instance_caches(&p, platform, &mut caches, &mut total_cache_bytes);
                        }
                    }
                }
            }
        }

        // Also check caches inside registered dirs even if located elsewhere
        for reg_dir in &registered_dirs {
            if reg_dir.is_dir() && !reg_dir.starts_with(&platform_inst_dir) {
                scan_instance_caches(reg_dir, platform, &mut caches, &mut total_cache_bytes);
            }
        }
    }

    StorageScanReport {
        orphan_directories: orphans,
        cache_folders: caches,
        total_orphan_bytes,
        total_cache_bytes,
        total_reclaimable_bytes: total_orphan_bytes + total_cache_bytes,
    }
}

fn scan_instance_caches(
    instance_dir: &Path,
    platform: &str,
    caches: &mut Vec<CacheFolderItem>,
    total_cache_bytes: &mut u64,
) {
    let cache_dir_names = [
        "Cache", "Code Cache", "GPUCache", "Service Worker", "Crashpad", "logs"
    ];

    // Check direct subdirs and User/ subdirs
    let candidate_roots = [
        instance_dir.to_path_buf(),
        instance_dir.join("User"),
        instance_dir.join("Cache"),
    ];

    for root in candidate_roots {
        if !root.is_dir() {
            continue;
        }
        for name in cache_dir_names {
            let target = root.join(name);
            if target.is_dir() {
                let size = calculate_dir_size(&target);
                if size > 1024 * 1024 { // Only report if > 1MB
                    *total_cache_bytes += size;
                    caches.push(CacheFolderItem {
                        instance_path: instance_dir.to_string_lossy().to_string(),
                        cache_path: target.to_string_lossy().to_string(),
                        platform: platform.to_string(),
                        folder_name: name.to_string(),
                        size_bytes: size,
                    });
                }
            }
        }
    }
}

pub fn execute_cleanup(delete_orphans: bool, clean_caches: bool) -> StorageCleanReport {
    let scan = scan_storage();
    let mut freed_bytes = 0;
    let mut deleted_paths = Vec::new();
    let mut failed_paths = Vec::new();

    if delete_orphans {
        for orphan in scan.orphan_directories {
            let p = Path::new(&orphan.path);
            if p.is_dir() {
                match fs::remove_dir_all(p) {
                    Ok(_) => {
                        freed_bytes += orphan.size_bytes;
                        deleted_paths.push(orphan.path);
                    }
                    Err(_) => {
                        failed_paths.push(orphan.path);
                    }
                }
            }
        }
    }

    if clean_caches {
        for cache in scan.cache_folders {
            let p = Path::new(&cache.cache_path);
            if p.is_dir() {
                match fs::remove_dir_all(p) {
                    Ok(_) => {
                        freed_bytes += cache.size_bytes;
                        deleted_paths.push(cache.cache_path);
                    }
                    Err(_) => {
                        failed_paths.push(cache.cache_path);
                    }
                }
            }
        }
    }

    StorageCleanReport {
        freed_bytes,
        deleted_paths,
        failed_paths,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_dir_size() {
        let temp_dir = std::env::temp_dir().join(format!("cockpit_test_storage_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let sub_dir = temp_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();

        fs::write(temp_dir.join("file1.txt"), b"12345").unwrap();
        fs::write(sub_dir.join("file2.txt"), b"abcdefghij").unwrap();

        let size = calculate_dir_size(&temp_dir);
        assert_eq!(size, 15);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
