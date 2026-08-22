use crate::assets::{MaterializationBudget, PlannedFile, hash_bytes, materialize_file, provenance};
use crate::diagnostics::DiagnosticCollector;
use crate::model::{
    MigrationAsset, MigrationDiagnosticSeverity, MigrationPhase, MigrationStatus,
    ResourceProvenance,
};
use aurora_core::{AppError, AppResult, ResourceKey};
use aurora_mdl::parse_mdl;
use aurora_resource::{ResourceCatalog, ResourceManager};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
pub(crate) struct NavigationPlan {
    pub paths_by_model: BTreeMap<String, Vec<String>>,
    pub assets: Vec<MigrationAsset>,
    pub files: Vec<PlannedFile>,
    pub provenance: Vec<ResourceProvenance>,
}

pub(crate) fn plan_navigation(
    catalog: &ResourceCatalog,
    requests: &BTreeMap<String, BTreeSet<u16>>,
    scratch_root: &Path,
    cancelled: &AtomicBool,
    diagnostics: &mut DiagnosticCollector,
    mut on_progress: impl FnMut(usize, usize, &str),
) -> AppResult<NavigationPlan> {
    let total = requests.values().map(BTreeSet::len).sum::<usize>();
    let mut ordinal = 0;
    let mut plan = NavigationPlan::default();
    let mut files_by_path = BTreeMap::<String, PlannedFile>::new();
    let mut resources_by_path = BTreeMap::<String, Vec<String>>::new();
    let mut surface_ids_by_path = BTreeMap::<String, BTreeSet<i32>>::new();
    let mut budget = MaterializationBudget::default();

    for (model_resref, resource_types) in requests {
        for resource_type in resource_types {
            if cancelled.load(Ordering::Relaxed) {
                return Err(Box::new(AppError::job_cancelled(model_resref)));
            }
            on_progress(ordinal, total, model_resref);
            ordinal += 1;
            let key = ResourceKey::new(model_resref, *resource_type);
            let Some(resource) = catalog.get(&key) else {
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Missing,
                    MigrationPhase::Navigation,
                    "MIGRATION_NAVIGATION_SOURCE_MISSING",
                    format!("La source de navigation {key} n'est pas résolue."),
                    Some(key.to_string()),
                    Some(model_resref.clone()),
                );
                continue;
            };
            if resource.selected.size > crate::assets::MAX_TEMPORARY_BYTES {
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Unsupported,
                    MigrationPhase::Navigation,
                    "MIGRATION_BUNDLE_LIMIT_EXCEEDED",
                    "La source de navigation dépasse la taille temporaire bornée.",
                    Some(key.to_string()),
                    Some(model_resref.clone()),
                );
                continue;
            }
            match ResourceManager::read(&resource.selected, cancelled) {
                Ok(bytes) => {
                    let digest = hash_bytes(&bytes);
                    let extension = key.extension().unwrap_or("nav");
                    let path = format!(
                        "assets/source-navigation/{extension}-{}.{extension}",
                        &digest[..24]
                    );
                    match parse_mdl(&bytes) {
                        Ok(model) => {
                            surface_ids_by_path
                                .entry(path.clone())
                                .or_default()
                                .extend(
                                    model
                                        .nodes
                                        .iter()
                                        .filter_map(|node| node.mesh.as_ref())
                                        .flat_map(|mesh| mesh.surface_ids.iter().copied()),
                                );
                        }
                        Err(error) => diagnostics.push(
                            MigrationDiagnosticSeverity::Warning,
                            MigrationStatus::Unsupported,
                            MigrationPhase::Navigation,
                            "MIGRATION_NAVIGATION_SURFACE_INDEX_UNAVAILABLE",
                            format!(
                                "{key} est préservé, mais ses identifiants de surface n'ont pas pu être indexés : {}",
                                error.message
                            ),
                            Some(key.to_string()),
                            Some(model_resref.clone()),
                        ),
                    }
                    if !files_by_path.contains_key(&path) {
                        files_by_path.insert(
                            path.clone(),
                            materialize_file(
                                scratch_root,
                                path.clone(),
                                "navigation-source",
                                bytes,
                                &mut budget,
                            )?,
                        );
                    }
                    resources_by_path
                        .entry(path.clone())
                        .or_default()
                        .push(key.to_string());
                    plan.paths_by_model
                        .entry(model_resref.clone())
                        .or_default()
                        .push(path.clone());
                    plan.provenance.push(provenance(
                        resource,
                        "preserved-navigation-source",
                        Some(digest),
                    ));
                    diagnostics.push(
                        MigrationDiagnosticSeverity::Info,
                        MigrationStatus::Manual,
                        MigrationPhase::Navigation,
                        "MIGRATION_NAVIGATION_PRESERVED_NOT_CONVERTED",
                        format!("{key} est préservé byte-for-byte mais n'est pas converti."),
                        Some(key.to_string()),
                        Some(model_resref.clone()),
                    );
                }
                Err(error) if error.code == "JOB_CANCELLED" => return Err(error),
                Err(error) => diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    if error.code.contains("LIMIT") {
                        MigrationStatus::Unsupported
                    } else {
                        MigrationStatus::Missing
                    },
                    MigrationPhase::Navigation,
                    &error.code,
                    error.user_message,
                    Some(key.to_string()),
                    Some(model_resref.clone()),
                ),
            }
        }
    }

    for paths in plan.paths_by_model.values_mut() {
        paths.sort();
        paths.dedup();
    }
    for (path, file) in &files_by_path {
        let digest = file.sha256.clone();
        let mut resource_keys = resources_by_path.remove(path).unwrap_or_default();
        resource_keys.sort();
        resource_keys.dedup();
        plan.assets.push(MigrationAsset {
            id: format!("asset:{}", &digest[..24]),
            kind: "navigation-source".to_owned(),
            resource_keys,
            path: path.clone(),
            size_bytes: file.size_bytes,
            sha256: digest,
            status: MigrationStatus::Manual,
            texture_paths: Vec::new(),
            navigation_paths: Vec::new(),
            surface_ids: surface_ids_by_path
                .remove(path)
                .unwrap_or_default()
                .into_iter()
                .collect(),
        });
    }
    plan.files.extend(files_by_path.into_values());
    plan.files.sort_by(|left, right| left.path.cmp(&right.path));
    plan.assets
        .sort_by(|left, right| left.path.cmp(&right.path));
    plan.provenance
        .sort_by(|left, right| left.resource_key.cmp(&right.resource_key));
    plan.provenance
        .dedup_by(|left, right| left.resource_key == right.resource_key);
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_resource::{
        ResolvedResource, ResourceLocation, ResourceSourceKind, ResourceVersion,
    };
    use sha2::{Digest, Sha256};
    use std::fs;

    #[test]
    fn preserves_wok_pwk_and_dwk_byte_for_byte_with_explicit_status() {
        let root = tempfile::tempdir().expect("root");
        let sources = [
            ("tile_a", 2016, "wok", b"synthetic WOK".as_slice()),
            ("place_a", 2053, "pwk", b"synthetic PWK".as_slice()),
            ("door_a", 2052, "dwk", b"synthetic DWK".as_slice()),
        ];
        let mut entries = Vec::new();
        let mut requests = BTreeMap::<String, BTreeSet<u16>>::new();
        for (resref, resource_type, extension, bytes) in sources {
            let path = root.path().join(format!("{resref}.{extension}"));
            fs::write(&path, bytes).expect("synthetic navigation");
            let key = ResourceKey::new(resref, resource_type);
            entries.push(ResolvedResource {
                key: key.clone(),
                selected: ResourceVersion {
                    key,
                    source_kind: ResourceSourceKind::Development,
                    source_name: format!("{resref}.{extension}"),
                    source_path: path.display().to_string(),
                    priority: 1,
                    offset: 0,
                    size: bytes.len() as u64,
                    sha256: Some(format!("{:x}", Sha256::digest(bytes))),
                    location: ResourceLocation::File {
                        path: path.display().to_string(),
                    },
                },
                shadowed: Vec::new(),
            });
            requests
                .entry(resref.to_owned())
                .or_default()
                .insert(resource_type);
        }
        entries.sort_by(|left, right| left.key.cmp(&right.key));
        let catalog = ResourceCatalog {
            version_count: entries.len(),
            shadowed_count: 0,
            entries,
            diagnostics: Vec::new(),
        };
        let mut diagnostics = DiagnosticCollector::default();
        let plan = plan_navigation(
            &catalog,
            &requests,
            root.path(),
            &AtomicBool::new(false),
            &mut diagnostics,
            |_, _, _| {},
        )
        .expect("navigation plan");

        assert_eq!(plan.files.len(), 3);
        assert!(
            plan.files
                .iter()
                .all(|file| file.path.starts_with("assets/source-navigation/"))
        );
        assert_eq!(
            plan.files
                .iter()
                .map(|file| fs::read(&file.scratch_path).expect("scratch navigation"))
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                b"synthetic WOK".to_vec(),
                b"synthetic PWK".to_vec(),
                b"synthetic DWK".to_vec(),
            ])
        );
        assert_eq!(
            diagnostics
                .into_sorted()
                .iter()
                .filter(
                    |diagnostic| diagnostic.code == "MIGRATION_NAVIGATION_PRESERVED_NOT_CONVERTED"
                )
                .count(),
            3
        );
    }
}
