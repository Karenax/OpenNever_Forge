use crate::assets::{AssetAudit, AssetPlan, audit_assets, hash_bytes, plan_assets, provenance};
use crate::bundle::{BundleDraft, write_bundle_atomic};
use crate::diagnostics::DiagnosticCollector;
use crate::extract::{AreaExtraction, assemble_area_document, extract_area, list_candidates};
use crate::model::{
    AreaMigrationCandidate, AreaMigrationExportRequest, AreaMigrationExportResult,
    AreaMigrationPreview, AreaMigrationSource, BUNDLE_CLASSIFICATION, BUNDLE_REDISTRIBUTION,
    BUNDLE_SCHEMA_VERSION, BundleManifest, CoordinateConvention, DependencyProvenance,
    MigrationCounts, MigrationDiagnostic, MigrationDiagnosticSeverity, MigrationPhase,
    MigrationProgress, MigrationReport, MigrationStatus, ResourceProvenance,
};
use crate::navigation::{NavigationPlan, plan_navigation};
use crate::sources::{ensure_source_snapshot, verify_source_snapshot};
use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_project::hash_module_file;
use aurora_resource::ResourceManager;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const EXPORTER_NAME: &str = concat!("opennever-area-migration/", env!("CARGO_PKG_VERSION"));

struct PreparedMigration {
    _scratch: tempfile::TempDir,
    extraction: AreaExtraction,
    area: crate::model::MigrationAreaDocument,
    identity_map: crate::model::IdentityMap,
    assets: AssetPlan,
    navigation: NavigationPlan,
    diagnostics: Vec<MigrationDiagnostic>,
    counts: MigrationCounts,
    complete: bool,
}

pub fn list_area_migration_candidates(source: &AreaMigrationSource) -> Vec<AreaMigrationCandidate> {
    list_candidates(source)
}

pub fn audit_area_migration(
    source: &AreaMigrationSource,
    area_resref: &str,
    cancelled: &AtomicBool,
) -> AppResult<AreaMigrationPreview> {
    let mut diagnostics = DiagnosticCollector::default();
    diagnose_dependencies(source, &mut diagnostics);
    let extraction = extract_area(source, area_resref, &mut diagnostics)?;
    ensure_source_snapshot(source, &extraction, cancelled)?;
    let assets = audit_assets(
        &source.resource_catalog,
        &extraction.requested_models,
        &extraction.navigation_requests,
        cancelled,
        &mut diagnostics,
    )?;
    let diagnostics = diagnostics.into_sorted();
    let counts = calculate_audit_counts(&extraction, &assets, &diagnostics);
    let complete = calculate_complete(&counts, &diagnostics);
    Ok(AreaMigrationPreview {
        schema_version: BUNDLE_SCHEMA_VERSION.to_owned(),
        area_resref: extraction.area.resref.clone(),
        area_name: crate::model::display_area_name(&extraction.area),
        suggested_directory_name: suggested_directory_name(&extraction.area.resref),
        ready: is_export_ready(&counts, &diagnostics),
        complete,
        counts,
        diagnostics,
        classification: BUNDLE_CLASSIFICATION.to_owned(),
        redistribution: BUNDLE_REDISTRIBUTION.to_owned(),
        navigation_status: "preserved-not-converted".to_owned(),
    })
}

pub fn export_area_migration(
    source: &AreaMigrationSource,
    request: &AreaMigrationExportRequest,
    cancelled: &AtomicBool,
    mut on_progress: impl FnMut(MigrationProgress),
) -> AppResult<AreaMigrationExportResult> {
    on_progress(progress(MigrationPhase::Preparing, 1.0, None));
    let source_before = fingerprint_file(&source.module_path, cancelled)?;
    if !source_before.0.eq_ignore_ascii_case(&source.module_sha256)
        || source_before.1 != source.module_size_bytes
    {
        return Err(Box::new(
            AppError::new(
                "MIGRATION_SOURCE_CHANGED",
                "Le module source a changé depuis son analyse.",
                format!(
                    "analysis expected {} / {} bytes, current source is {} / {} bytes",
                    source.module_sha256,
                    source.module_size_bytes,
                    source_before.0,
                    source_before.1
                ),
                ErrorSeverity::Error,
            )
            .with_resource("module.mod")
            .with_import_stage("area_migration_precondition")
            .with_suggestion("Relancez l'analyse du module avant l'export."),
        ));
    }

    let prepared = prepare(
        source,
        &request.area_resref,
        cancelled,
        |phase, ratio, current| {
            let (start, span) = match phase {
                MigrationPhase::Audit => (5.0, 10.0),
                MigrationPhase::Models => (15.0, 38.0),
                MigrationPhase::Textures => (53.0, 20.0),
                MigrationPhase::Navigation => (73.0, 10.0),
                _ => (5.0, 0.0),
            };
            on_progress(progress(phase, start + ratio * span, current));
        },
    )?;
    if !is_export_ready(&prepared.counts, &prepared.diagnostics) {
        return Err(export_blocked_error(&prepared.diagnostics));
    }
    check_cancelled(cancelled, &request.area_resref)?;
    on_progress(progress(
        MigrationPhase::Bundle,
        85.0,
        Some("Écriture atomique"),
    ));

    let dependencies = dependency_provenance(source);
    let mut resources = prepared.assets.provenance.clone();
    resources.extend(prepared.navigation.provenance.clone());
    resources.extend(area_provenance(source, &prepared.extraction, cancelled));
    resources.sort_by(|left, right| {
        (&left.resource_key, &left.purpose).cmp(&(&right.resource_key, &right.purpose))
    });
    resources.dedup_by(|left, right| {
        left.resource_key == right.resource_key && left.purpose == right.purpose
    });
    let report = MigrationReport {
        schema_version: BUNDLE_SCHEMA_VERSION.to_owned(),
        area_resref: prepared.extraction.area.resref.clone(),
        complete: prepared.complete,
        counts: prepared.counts.clone(),
        navigation_converted: false,
        navigation_status: "preserved-not-converted".to_owned(),
        diagnostics_file: "diagnostics.jsonl".to_owned(),
        bundle_is_local_only: true,
        source_module_immutable: true,
        payload_file_count: 0,
        payload_size_bytes: 0,
    };
    let manifest = BundleManifest {
        schema_version: BUNDLE_SCHEMA_VERSION.to_owned(),
        exporter: EXPORTER_NAME.to_owned(),
        module_sha256: source.module_sha256.to_ascii_lowercase(),
        module_size_bytes: source.module_size_bytes,
        area_resref: prepared.extraction.area.resref.clone(),
        classification: BUNDLE_CLASSIFICATION.to_owned(),
        redistribution: BUNDLE_REDISTRIBUTION.to_owned(),
        coordinate_system: CoordinateConvention::default(),
        resolution_policy: "OpenNever Resource Manager selected version; shadowed versions retained as sanitized provenance".to_owned(),
        dependencies,
        resources,
        counts: prepared.counts.clone(),
        files: Vec::new(),
        integrity_scope: "files lists every payload file with final byte size and SHA-256; manifest.json is self-excluded and its final hash is returned by the exporter".to_owned(),
    };
    let mut asset_files = prepared.assets.files;
    asset_files.extend(prepared.navigation.files);
    asset_files.sort_by(|left, right| left.path.cmp(&right.path));
    asset_files.dedup_by(|left, right| left.path == right.path);
    let module_path = source.module_path.clone();
    let source_for_verification = source.clone();
    let expected_hash = source_before.0;
    let expected_size = source_before.1;
    let written = write_bundle_atomic(
        &request.destination,
        &protected_sources(source),
        cancelled,
        BundleDraft {
            area: prepared.area,
            identity_map: prepared.identity_map,
            diagnostics: prepared.diagnostics.clone(),
            report,
            manifest,
            asset_files,
        },
        || {
            let after = fingerprint_file(&module_path, cancelled)?;
            if after != (expected_hash, expected_size) {
                return Err(Box::new(
                    AppError::new(
                        "MIGRATION_SOURCE_MUTATED_DURING_EXPORT",
                        "Le module source a changé pendant l'export.",
                        "source SHA-256 or size changed before atomic bundle publication",
                        ErrorSeverity::Error,
                    )
                    .with_resource("module.mod")
                    .with_import_stage("area_migration_immutability_check"),
                ));
            }
            verify_source_snapshot(&source_for_verification, cancelled)?;
            Ok(())
        },
    )?;
    on_progress(progress(
        MigrationPhase::Verifying,
        100.0,
        Some("Bundle vérifié"),
    ));
    Ok(AreaMigrationExportResult {
        bundle_path: written.path.display().to_string(),
        manifest_file: written.manifest_file,
        report: written.report,
        diagnostics: prepared.diagnostics,
    })
}

fn prepare(
    source: &AreaMigrationSource,
    area_resref: &str,
    cancelled: &AtomicBool,
    mut on_progress: impl FnMut(MigrationPhase, f64, Option<&str>),
) -> AppResult<PreparedMigration> {
    check_cancelled(cancelled, area_resref)?;
    let mut diagnostics = DiagnosticCollector::default();
    diagnose_dependencies(source, &mut diagnostics);
    on_progress(MigrationPhase::Audit, 0.0, Some(area_resref));
    let extraction = extract_area(source, area_resref, &mut diagnostics)?;
    ensure_source_snapshot(source, &extraction, cancelled)?;
    let scratch = tempfile::tempdir().map_err(|error| {
        Box::new(AppError::io(
            "create migration scratch directory",
            "scratch",
            &error,
        ))
    })?;
    let assets = plan_assets(
        &source.resource_catalog,
        &extraction.requested_models,
        scratch.path(),
        cancelled,
        &mut diagnostics,
        |phase, current, total, resource| {
            let ratio = if total == 0 {
                1.0
            } else {
                current as f64 / total as f64
            };
            on_progress(phase, ratio, Some(resource));
        },
    )?;
    let navigation = plan_navigation(
        &source.resource_catalog,
        &extraction.navigation_requests,
        scratch.path(),
        cancelled,
        &mut diagnostics,
        |current, total, resource| {
            let ratio = if total == 0 {
                1.0
            } else {
                current as f64 / total as f64
            };
            on_progress(MigrationPhase::Navigation, ratio, Some(resource));
        },
    )?;
    let mut migration_assets = assets.assets.clone();
    migration_assets.extend(navigation.assets.clone());
    let (area, identity_map) = assemble_area_document(
        source,
        &extraction,
        &assets.model_path_by_resref,
        &navigation.paths_by_model,
        migration_assets,
        &mut diagnostics,
    );
    let diagnostics = diagnostics.into_sorted();
    let counts = calculate_counts(&area, &assets, &navigation, &diagnostics);
    let complete = calculate_complete(&counts, &diagnostics);
    Ok(PreparedMigration {
        _scratch: scratch,
        extraction,
        area,
        identity_map,
        assets,
        navigation,
        diagnostics,
        counts,
        complete,
    })
}

fn calculate_audit_counts(
    extraction: &AreaExtraction,
    assets: &AssetAudit,
    diagnostics: &[MigrationDiagnostic],
) -> MigrationCounts {
    let object_by_id = extraction
        .scene
        .objects
        .iter()
        .chain(&extraction.scene.overlays)
        .map(|object| (object.id.as_str(), object))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut counts = MigrationCounts {
        tiles: extraction.area.tiles.len(),
        instances: extraction.area.instances.len(),
        unique_models: assets.model_resrefs.len(),
        textures: assets.texture_resrefs.len(),
        preserved_navigation: assets.navigation_count,
        diagnostics: diagnostics.len(),
        warnings: diagnostics
            .iter()
            .filter(|value| value.severity == MigrationDiagnosticSeverity::Warning)
            .count(),
        errors: diagnostics
            .iter()
            .filter(|value| value.severity == MigrationDiagnosticSeverity::Error)
            .count(),
        ..MigrationCounts::default()
    };
    for tile in &extraction.area.tiles {
        let identity = format!("tile:{}:{}", tile.x, tile.y);
        let status = object_by_id
            .get(identity.as_str())
            .and_then(|object| object.model_resref.as_deref())
            .filter(|resref| assets.model_resrefs.contains(&normalize_resref(resref)))
            .map(|_| MigrationStatus::Converted)
            .unwrap_or(MigrationStatus::Missing);
        counts.record_status(status);
    }
    for instance in &extraction.area.instances {
        let models = object_by_id
            .get(instance.id.as_str())
            .map(|object| {
                object
                    .model_resrefs
                    .iter()
                    .filter(|resref| assets.model_resrefs.contains(&normalize_resref(resref)))
                    .map(|_| "audit-model".to_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let (_, status, _) = crate::extract::classify_instance(instance, &models);
        counts.record_status(status);
    }
    for diagnostic in diagnostics {
        counts.record_status(diagnostic.status);
    }
    counts
}

fn calculate_complete(counts: &MigrationCounts, diagnostics: &[MigrationDiagnostic]) -> bool {
    is_export_ready(counts, diagnostics)
        && !counts.by_status.keys().any(|status| {
            matches!(
                status,
                MigrationStatus::Approximated
                    | MigrationStatus::Placeholder
                    | MigrationStatus::Manual
            )
        })
}

pub(crate) fn is_export_ready(
    counts: &MigrationCounts,
    diagnostics: &[MigrationDiagnostic],
) -> bool {
    !diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == MigrationDiagnosticSeverity::Error
            || diagnostic.code == "MIGRATION_DEPENDENCY_UNCHECKED"
            || matches!(
                diagnostic.status,
                MigrationStatus::Missing
                    | MigrationStatus::Unsupported
                    | MigrationStatus::LicenseBlocked
            )
    }) && !counts.by_status.keys().any(|status| {
        matches!(
            status,
            MigrationStatus::Missing
                | MigrationStatus::Unsupported
                | MigrationStatus::LicenseBlocked
        )
    })
}

fn export_blocked_error(diagnostics: &[MigrationDiagnostic]) -> Box<AppError> {
    let reason = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.severity == MigrationDiagnosticSeverity::Error
                || matches!(
                    diagnostic.status,
                    MigrationStatus::Missing
                        | MigrationStatus::Unsupported
                        | MigrationStatus::LicenseBlocked
                )
        })
        .map(|diagnostic| diagnostic.code.as_str())
        .unwrap_or("MIGRATION_READINESS");
    Box::new(
        AppError::new(
            "MIGRATION_EXPORT_BLOCKED",
            "L'export est bloqué par l'audit de la migration.",
            format!("migration readiness contract rejected export: {reason}"),
            ErrorSeverity::Error,
        )
        .with_resource(reason)
        .with_import_stage("area_migration_readiness")
        .with_suggestion("Corrigez ou acceptez les sources manquantes, puis relancez l'audit."),
    )
}

fn normalize_resref(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .strip_suffix(".mdl")
        .unwrap_or(value.trim().trim_matches('"'))
        .to_ascii_lowercase()
}

fn diagnose_dependencies(source: &AreaMigrationSource, diagnostics: &mut DiagnosticCollector) {
    for dependency in &source.dependency_report.dependencies {
        use aurora_project::ModuleDependencyState;

        let (status, code, message) = match dependency.state {
            ModuleDependencyState::Resolved => continue,
            ModuleDependencyState::Missing => (
                MigrationStatus::Missing,
                "MIGRATION_DEPENDENCY_MISSING",
                format!(
                    "La dépendance {} {} est absente du catalogue analysé.",
                    dependency_kind_label(dependency.kind),
                    dependency.logical_name
                ),
            ),
            ModuleDependencyState::Unchecked => (
                MigrationStatus::Manual,
                "MIGRATION_DEPENDENCY_UNCHECKED",
                format!(
                    "La dépendance {} {} n'a pas été vérifiée : fournissez explicitement les racines d'installation et de données utilisateur.",
                    dependency_kind_label(dependency.kind),
                    dependency.logical_name
                ),
            ),
            ModuleDependencyState::Invalid => (
                MigrationStatus::Unsupported,
                "MIGRATION_DEPENDENCY_INVALID",
                format!(
                    "La dépendance {} {} possède un nom ou un emplacement non valide.",
                    dependency_kind_label(dependency.kind),
                    dependency.logical_name
                ),
            ),
        };
        diagnostics.push(
            MigrationDiagnosticSeverity::Warning,
            status,
            MigrationPhase::Audit,
            code,
            message,
            Some(dependency.logical_name.clone()),
            None,
        );
    }
}

fn calculate_counts(
    area: &crate::model::MigrationAreaDocument,
    assets: &AssetPlan,
    navigation: &NavigationPlan,
    diagnostics: &[MigrationDiagnostic],
) -> MigrationCounts {
    let mut counts = MigrationCounts {
        tiles: area.tiles.len(),
        instances: area.instances.len(),
        unique_models: assets
            .assets
            .iter()
            .filter(|asset| asset.kind == "model")
            .count(),
        textures: assets
            .assets
            .iter()
            .filter(|asset| asset.kind == "texture")
            .count(),
        preserved_navigation: navigation.files.len(),
        diagnostics: diagnostics.len(),
        warnings: diagnostics
            .iter()
            .filter(|value| value.severity == MigrationDiagnosticSeverity::Warning)
            .count(),
        errors: diagnostics
            .iter()
            .filter(|value| value.severity == MigrationDiagnosticSeverity::Error)
            .count(),
        ..MigrationCounts::default()
    };
    for tile in &area.tiles {
        counts.record_status(tile.status);
    }
    for instance in &area.instances {
        counts.record_status(instance.status);
    }
    for asset in &area.assets {
        counts.record_status(asset.status);
    }
    for diagnostic in diagnostics.iter().filter(|diagnostic| {
        matches!(
            diagnostic.status,
            MigrationStatus::Missing
                | MigrationStatus::Unsupported
                | MigrationStatus::LicenseBlocked
        )
    }) {
        counts.record_status(diagnostic.status);
    }
    counts
}

fn area_provenance(
    source: &AreaMigrationSource,
    extraction: &AreaExtraction,
    cancelled: &AtomicBool,
) -> Vec<ResourceProvenance> {
    let mut keys = vec![
        (
            ResourceKey::new(&extraction.area.resref, 2012),
            "area-definition",
        ),
        (
            ResourceKey::new(&extraction.area.resref, 2023),
            "area-instances",
        ),
        (
            ResourceKey::new(&extraction.area.resref, 2046),
            "area-toolset-data",
        ),
    ];
    if let Some(tileset) = &extraction.area.tileset {
        keys.push((ResourceKey::new(tileset, 2013), "tileset-model-resolution"));
    }
    for instance in &extraction.area.instances {
        let Some(resref) = &instance.template_resref else {
            continue;
        };
        let resource_type = match instance.category.as_str() {
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
        };
        if let Some(resource_type) = resource_type {
            keys.push((
                ResourceKey::new(resref, resource_type),
                "instance-blueprint",
            ));
        }
    }
    let mut result = keys
        .into_iter()
        .filter_map(|(key, purpose)| {
            source.resource_catalog.get(&key).map(|resource| {
                let digest = ResourceManager::read(&resource.selected, cancelled)
                    .ok()
                    .map(|bytes| hash_bytes(&bytes));
                provenance(resource, purpose, digest)
            })
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| left.resource_key.cmp(&right.resource_key));
    result.dedup_by(|left, right| left.resource_key == right.resource_key);
    result
}

fn dependency_provenance(source: &AreaMigrationSource) -> Vec<DependencyProvenance> {
    let mut dependencies = source
        .dependency_report
        .dependencies
        .iter()
        .map(|dependency| DependencyProvenance {
            kind: dependency.kind,
            logical_name: dependency.logical_name.clone(),
            state: dependency.state,
            selected_path_sha256: dependency.selected_path.as_deref().map(hash_path),
            selected_size_bytes: dependency
                .fingerprint
                .as_ref()
                .map(|value| value.size_bytes),
            selected_content_sha256: dependency
                .fingerprint
                .as_ref()
                .map(|value| value.sha256.to_ascii_lowercase()),
            shadowed_version_count: dependency.shadowed_paths.len(),
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| {
        (dependency_kind_order(left.kind), &left.logical_name)
            .cmp(&(dependency_kind_order(right.kind), &right.logical_name))
    });
    dependencies
}

fn protected_sources(source: &AreaMigrationSource) -> Vec<PathBuf> {
    let mut paths = vec![source.module_path.clone()];
    paths.extend(source.protected_roots.iter().cloned());
    paths.extend(
        source
            .dependency_report
            .dependencies
            .iter()
            .filter_map(|dependency| dependency.selected_path.as_deref())
            .map(PathBuf::from),
    );
    paths
}

fn dependency_kind_order(kind: aurora_project::ModuleDependencyKind) -> u8 {
    match kind {
        aurora_project::ModuleDependencyKind::Hak => 0,
        aurora_project::ModuleDependencyKind::CustomTlk => 1,
    }
}

fn dependency_kind_label(kind: aurora_project::ModuleDependencyKind) -> &'static str {
    match kind {
        aurora_project::ModuleDependencyKind::Hak => "HAK",
        aurora_project::ModuleDependencyKind::CustomTlk => "TLK",
    }
}

fn fingerprint_file(path: &Path, cancelled: &AtomicBool) -> AppResult<(String, u64)> {
    let fingerprint = hash_module_file(path, cancelled, |_| {})?;
    Ok((fingerprint.sha256, fingerprint.size_bytes))
}

fn hash_path(path: &str) -> String {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    format!("{:x}", Sha256::digest(normalized.as_bytes()))
}

fn suggested_directory_name(area_resref: &str) -> String {
    let safe = area_resref
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '_' | '-') {
                value.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("{safe}.area-migration-v1")
}

fn progress(phase: MigrationPhase, percent: f64, current: Option<&str>) -> MigrationProgress {
    MigrationProgress {
        phase,
        percent: percent.clamp(0.0, 100.0),
        current: current.map(str::to_owned),
    }
}

fn check_cancelled(cancelled: &AtomicBool, resource: &str) -> AppResult<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(Box::new(AppError::job_cancelled(resource)));
    }
    Ok(())
}

pub use crate::bundle::{validate_bundle_destination, validate_bundle_directory};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggested_directory_is_stable_and_path_neutral() {
        assert_eq!(
            suggested_directory_name("My_Area"),
            "my_area.area-migration-v1"
        );
    }
}
