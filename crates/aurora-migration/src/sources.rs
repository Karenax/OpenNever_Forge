use crate::extract::AreaExtraction;
use crate::model::{
    AreaMigrationSource, CapturedFileFingerprint, CapturedResourceFingerprint,
    MigrationSourceSnapshot,
};
use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_project::hash_module_file;
use aurora_project::resolve_model_for_export_with_dependencies;
use aurora_resource::{ResourceManager, ResourceVersion};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

pub(crate) fn ensure_source_snapshot(
    source: &AreaMigrationSource,
    extraction: &AreaExtraction,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    if source
        .source_snapshot
        .lock()
        .expect("source snapshot poisoned")
        .is_some()
    {
        return Ok(());
    }

    let keys = migration_resource_keys(source, extraction);
    let mut snapshot = MigrationSourceSnapshot::default();
    for key in keys {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(AppError::job_cancelled(key.to_string()).into());
        }
        let Some(resource) = source.resource_catalog.get(&key) else {
            continue;
        };
        let bytes = ResourceManager::read(&resource.selected, cancelled).map_err(|_| {
            source_fingerprint_error("MIGRATION_SOURCE_FINGERPRINT_FAILED", key.to_string())
        })?;
        let digest = hash_bytes(&bytes);
        snapshot.resources.insert(
            key.clone(),
            CapturedResourceFingerprint {
                key,
                selected: ResourceVersion {
                    sha256: Some(digest.clone()),
                    ..resource.selected.clone()
                },
                content_sha256: digest,
                content_size_bytes: bytes.len() as u64,
            },
        );
    }

    for dependency in &source.dependency_report.dependencies {
        let Some(path) = dependency.selected_path.as_deref() else {
            continue;
        };
        let fingerprint = hash_file(Path::new(path), cancelled).map_err(|_| {
            source_fingerprint_error(
                "MIGRATION_DEPENDENCY_FINGERPRINT_FAILED",
                dependency.logical_name.clone(),
            )
        })?;
        snapshot.dependencies.insert(
            dependency.logical_name.to_ascii_lowercase(),
            CapturedFileFingerprint {
                path: PathBuf::from(path),
                size_bytes: fingerprint.1,
                content_sha256: fingerprint.0,
            },
        );
    }

    *source
        .source_snapshot
        .lock()
        .expect("source snapshot poisoned") = Some(snapshot);
    Ok(())
}

pub(crate) fn verify_source_snapshot(
    source: &AreaMigrationSource,
    cancelled: &AtomicBool,
) -> AppResult<()> {
    let snapshot = source
        .source_snapshot
        .lock()
        .expect("source snapshot poisoned")
        .clone()
        .ok_or_else(|| {
            source_fingerprint_error(
                "MIGRATION_SOURCE_SNAPSHOT_MISSING",
                "migration-source".to_owned(),
            )
        })?;

    for (key, expected) in snapshot.resources {
        let Some(current) = source.resource_catalog.get(&key) else {
            return Err(source_changed_error(
                key.to_string(),
                "selected version disappeared",
            ));
        };
        if !same_selected_version(&expected.selected, &current.selected) {
            return Err(source_changed_error(
                key.to_string(),
                "selected source version changed",
            ));
        }
        let bytes = ResourceManager::read(&current.selected, cancelled).map_err(|_| {
            source_changed_error(key.to_string(), "selected source is no longer readable")
        })?;
        if bytes.len() as u64 != expected.content_size_bytes
            || hash_bytes(&bytes) != expected.content_sha256
        {
            return Err(source_changed_error(
                key.to_string(),
                "selected source content changed",
            ));
        }
    }

    for (logical_name, expected) in snapshot.dependencies {
        let current = hash_file(&expected.path, cancelled).map_err(|_| {
            source_changed_error(logical_name.clone(), "selected dependency disappeared")
        })?;
        if current != (expected.content_sha256, expected.size_bytes) {
            return Err(source_changed_error(
                logical_name,
                "selected dependency content changed",
            ));
        }
    }
    Ok(())
}

fn migration_resource_keys(
    source: &AreaMigrationSource,
    extraction: &AreaExtraction,
) -> BTreeSet<ResourceKey> {
    let mut keys = BTreeSet::new();
    let area = &extraction.area;
    keys.extend([
        ResourceKey::new(&area.resref, 2012),
        ResourceKey::new(&area.resref, 2023),
        ResourceKey::new(&area.resref, 2046),
    ]);
    if let Some(tileset) = &area.tileset {
        keys.insert(ResourceKey::new(tileset, 2013));
    }
    for instance in &area.instances {
        if let Some(resref) = &instance.template_resref
            && let Some(resource_type) = blueprint_type(&instance.category)
        {
            keys.insert(ResourceKey::new(resref, resource_type));
        }
    }
    for model in &extraction.requested_models {
        keys.insert(ResourceKey::new(model, 2002));
        for resource_type in [2016, 2052, 2053] {
            if extraction
                .navigation_requests
                .get(model)
                .is_some_and(|types| types.contains(&resource_type))
            {
                keys.insert(ResourceKey::new(model, resource_type));
            }
        }
    }

    // The analysis already indexed model texture references. Use those references to snapshot
    // every candidate that the deterministic fallback may inspect, not only the preferred type.
    for asset in &source.world_index.assets.assets {
        if asset.key.resource_type == 2002
            && extraction.requested_models.contains(&asset.key.resref)
        {
            for texture in &asset.textures {
                for resource_type in [2033, 3, 2080, 6] {
                    keys.insert(ResourceKey::new(texture, resource_type));
                }
            }
            for dependency in asset
                .referenced_models
                .iter()
                .chain(asset.supermodel.iter())
            {
                keys.insert(ResourceKey::new(dependency, 2002));
            }
        }
    }
    // Some synthetic or older WorldIndex snapshots do not carry the model texture index. Re-read
    // the bounded model graph so every candidate that the exporter can actually inspect receives
    // the same immutable baseline as the module and dependency files.
    for model_resref in &extraction.requested_models {
        if let Ok(resolved) = resolve_model_for_export_with_dependencies(
            &source.resource_catalog,
            model_resref,
            &AtomicBool::new(false),
        ) {
            for dependency in &resolved.resource_resrefs {
                keys.insert(ResourceKey::new(dependency, 2002));
            }
            for texture in resolved
                .model
                .nodes
                .iter()
                .filter_map(|node| node.mesh.as_ref())
                .flat_map(|mesh| mesh.material.textures.iter())
                .filter_map(|value| normalize_resref(value))
            {
                for resource_type in [2033, 3, 2080, 6] {
                    keys.insert(ResourceKey::new(&texture, resource_type));
                }
            }
        }
    }
    keys.retain(|key| source.resource_catalog.get(key).is_some());
    keys
}

fn blueprint_type(category: &str) -> Option<u16> {
    match category {
        "item" => Some(2025),
        "creature" => Some(2027),
        "trigger" => Some(2032),
        "sound" => Some(2035),
        "encounter" => Some(2040),
        "door" => Some(2042),
        "placeable" => Some(2044),
        "store" => Some(2051),
        "waypoint" => Some(2058),
        _ => None,
    }
}

fn same_selected_version(expected: &ResourceVersion, current: &ResourceVersion) -> bool {
    expected.source_kind == current.source_kind
        && expected.priority == current.priority
        && expected.offset == current.offset
        && expected.size == current.size
        && normalize_path(&expected.source_path) == normalize_path(&current.source_path)
}

fn hash_file(path: &Path, cancelled: &AtomicBool) -> AppResult<(String, u64)> {
    let fingerprint = hash_module_file(path, cancelled, |_| {})?;
    Ok((
        fingerprint.sha256.to_ascii_lowercase(),
        fingerprint.size_bytes,
    ))
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn normalize_resref(value: &str) -> Option<String> {
    let value = value
        .trim()
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let stem = value
        .rsplit_once('.')
        .map_or(value.as_str(), |(stem, _)| stem)
        .trim();
    (!stem.is_empty() && stem != "null").then(|| stem.to_owned())
}

fn source_fingerprint_error(code: &str, resource: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Une source utilisée par la migration n'a pas pu être empreintée.",
            "the selected migration source could not be fingerprinted",
            ErrorSeverity::Error,
        )
        .with_resource(resource)
        .with_import_stage("area_migration_source_fingerprint"),
    )
}

fn source_changed_error(resource: String, detail: &str) -> Box<AppError> {
    Box::new(
        AppError::new(
            "MIGRATION_SOURCE_CHANGED",
            "Une source utilisée par la migration a changé depuis l'audit.",
            detail,
            ErrorSeverity::Error,
        )
        .with_resource(resource)
        .with_import_stage("area_migration_source_fingerprint")
        .with_suggestion("Relancez l'analyse de la zone avant l'export."),
    )
}
