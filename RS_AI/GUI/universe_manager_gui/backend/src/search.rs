use crate::path_security::AllowedRoots;
use crate::{BackendError, CancellationToken, ErrorKind, ManagerConfig, Progress, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchReport {
    pub query: String,
    pub results: Vec<SearchResult>,
}

pub fn search_apps(
    config: &ManagerConfig,
    managed_roots: Vec<PathBuf>,
    query: &str,
    cancellation: &CancellationToken,
    mut progress: impl FnMut(Progress),
) -> Result<SearchReport> {
    let query = query.trim();
    if query.is_empty() || query.len() > 256 {
        return Err(BackendError::new(
            ErrorKind::Validation,
            "search query must contain 1 to 256 characters",
        ));
    }
    let roots = AllowedRoots::new(managed_roots, "managed directory")?;
    let managed = roots.resolve_directory(Path::new(&config.settings.managed_dir))?;
    let needle = query.to_lowercase();
    let total = u64::try_from(config.apps.len()).unwrap_or(u64::MAX);
    let mut results = Vec::new();
    for (index, app) in config.apps.iter().enumerate() {
        cancellation.check()?;
        let install_path = roots.resolve(Path::new(&app.install_path))?;
        if !install_path.starts_with(&managed) {
            return Err(BackendError::at_path(
                ErrorKind::Forbidden,
                "registered application is outside managed_dir",
                install_path,
            ));
        }
        if app.name.to_lowercase().contains(&needle) || app.id.to_lowercase().contains(&needle) {
            results.push(SearchResult {
                name: app.name.clone(),
                id: app.id.clone(),
                version: app.version.clone().unwrap_or_default(),
                source: app
                    .inventory_sources
                    .first()
                    .cloned()
                    .or_else(|| app.package_type.clone())
                    .unwrap_or_else(|| "Applications".to_string()),
            });
        }
        progress(Progress {
            completed: u64::try_from(index + 1).unwrap_or(u64::MAX),
            total: Some(total),
            message: app.name.clone(),
        });
    }
    results.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(SearchReport {
        query: query.to_string(),
        results,
    })
}
