use crate::ResourceCatalog;
use aurora_core::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

const CATALOG_CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceCatalogCacheState {
    #[default]
    Disabled,
    Hit,
    Miss,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCatalogCacheSummary {
    pub state: ResourceCatalogCacheState,
    pub signature: Option<String>,
    pub path: Option<String>,
    pub game_resource_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedGameCatalog {
    schema_version: u32,
    source_signature: String,
    catalog: ResourceCatalog,
}

pub(crate) fn load(path: &Path, expected_signature: &str) -> Option<ResourceCatalog> {
    let bytes = fs::read(path).ok()?;
    let cached = serde_json::from_slice::<CachedGameCatalog>(&bytes).ok()?;
    (cached.schema_version == CATALOG_CACHE_SCHEMA_VERSION
        && cached.source_signature == expected_signature)
        .then_some(cached.catalog)
}

pub(crate) fn store(
    path: &Path,
    source_signature: &str,
    catalog: &ResourceCatalog,
) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::io(
                "create resource catalog cache directory",
                parent.display().to_string(),
                &error,
            )
        })?;
    }
    let bytes = serde_json::to_vec(&CachedGameCatalog {
        schema_version: CATALOG_CACHE_SCHEMA_VERSION,
        source_signature: source_signature.to_owned(),
        catalog: catalog.clone(),
    })
    .map_err(|error| {
        AppError::database(
            path.display().to_string(),
            format!("cannot serialize resource catalog cache: {error}"),
        )
    })?;
    fs::write(path, bytes).map_err(|error| {
        AppError::io(
            "write resource catalog cache",
            path.display().to_string(),
            &error,
        )
        .into()
    })
}

pub(crate) fn game_source_signature(
    game_root: &Path,
    language_root: Option<&Path>,
    cancelled: &AtomicBool,
) -> AppResult<String> {
    let mut sources = Vec::new();
    collect_matching_files(game_root, &["key"], &mut sources, cancelled)?;
    collect_matching_files(
        &game_root.join("data"),
        &["key", "bif"],
        &mut sources,
        cancelled,
    )?;
    collect_matching_files(&game_root.join("ovr"), &[], &mut sources, cancelled)?;
    if let Some(language_root) = language_root {
        collect_matching_files(
            &language_root.join("data"),
            &["key", "bif"],
            &mut sources,
            cancelled,
        )?;
        collect_matching_files(
            &language_root.join("data/ovr"),
            &[],
            &mut sources,
            cancelled,
        )?;
    }
    sources.sort();

    let mut digest = Sha256::new();
    digest.update(game_root.display().to_string().to_ascii_lowercase());
    for source in sources {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(game_root.display().to_string()).into());
        }
        let metadata = fs::metadata(&source).map_err(|error| {
            AppError::io(
                "read resource catalog source metadata",
                source.display().to_string(),
                &error,
            )
        })?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        digest.update(source.display().to_string().to_ascii_lowercase());
        digest.update(metadata.len().to_le_bytes());
        digest.update(modified.to_le_bytes());
    }
    Ok(hex::encode_upper(digest.finalize()))
}

fn collect_matching_files(
    directory: &Path,
    extensions: &[&str],
    output: &mut Vec<PathBuf>,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        AppError::io(
            "enumerate resource catalog cache sources",
            directory.display().to_string(),
            &error,
        )
    })?;
    for entry in entries {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AppError::job_cancelled(directory.display().to_string()).into());
        }
        let entry = entry.map_err(|error| {
            AppError::io(
                "enumerate resource catalog cache source",
                directory.display().to_string(),
                &error,
            )
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if extensions.is_empty()
            || path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| {
                    extensions
                        .iter()
                        .any(|extension| value.eq_ignore_ascii_case(extension))
                })
        {
            output.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    #[test]
    fn cache_round_trip_requires_the_current_signature() {
        let root = tempdir().expect("temporary directory");
        let path = root.path().join("catalog.json");
        let catalog = ResourceCatalog::default();
        store(&path, "CURRENT", &catalog).expect("store cache");

        assert_eq!(load(&path, "CURRENT"), Some(catalog));
        assert_eq!(load(&path, "STALE"), None);
    }

    #[test]
    fn source_signature_changes_when_a_game_resource_changes() {
        let root = tempdir().expect("temporary directory");
        let game = root.path().join("game");
        fs::create_dir_all(game.join("data")).expect("game data");
        let bif = game.join("data/base.bif");
        fs::write(&bif, b"one").expect("first BIF");
        let cancelled = AtomicBool::new(false);
        let before = game_source_signature(&game, None, &cancelled).expect("first signature");
        fs::write(&bif, b"different size").expect("changed BIF");
        let after = game_source_signature(&game, None, &cancelled).expect("second signature");

        assert_ne!(before, after);
    }
}
