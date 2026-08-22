use crate::coordinates::CanonicalTransform;
use aurora_core::ResourceKey;
use aurora_project::{
    ModuleAnalysis, ModuleDependencyKind, ModuleDependencyReport, ModuleDependencyState,
    ResourceCatalog, WorldIndex,
};
use aurora_resource::{ResourceSourceKind, ResourceVersion};
use aurora_world::{AreaInstance, AreaMap, AreaSpawnPoint, AreaTile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const BUNDLE_SCHEMA_VERSION: &str = "area-migration-bundle@1.0.0";
pub const BUNDLE_CLASSIFICATION: &str = "local_only_proprietary";
pub const BUNDLE_REDISTRIBUTION: &str = "not_redistributable_without_separate_rights";
pub const MAX_BUNDLE_FILES: usize = 50_000;
pub const MAX_BUNDLE_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Complete in-memory read model required by the migration exporter.
///
/// The source path is used only for the before/after immutability proof. It is never serialized
/// into the bundle. Catalog locations are sanitized before becoming provenance records.
#[derive(Debug, Clone)]
pub struct AreaMigrationSource {
    pub module_path: PathBuf,
    pub module_sha256: String,
    pub module_size_bytes: u64,
    pub resource_catalog: ResourceCatalog,
    pub world_index: WorldIndex,
    pub dependency_report: ModuleDependencyReport,
    /// Roots that must never contain a published bundle. They are supplied by the analysis
    /// caller (module directory, game installation and user-data roots) and extended with the
    /// actual selected source containers used by a migration.
    pub protected_roots: Vec<PathBuf>,
    /// Preview and export share this lazily captured snapshot. Keeping it behind an Arc means a
    /// remounted Tauri view cannot accidentally create a fresh baseline after a source changed.
    pub source_snapshot: Arc<Mutex<Option<MigrationSourceSnapshot>>>,
}

impl AreaMigrationSource {
    pub fn from_analysis(analysis: &ModuleAnalysis, module_path: impl AsRef<Path>) -> Self {
        Self::from_analysis_with_roots(analysis, module_path, Vec::new())
    }

    pub fn from_analysis_with_roots(
        analysis: &ModuleAnalysis,
        module_path: impl AsRef<Path>,
        protected_roots: Vec<PathBuf>,
    ) -> Self {
        let module_path = module_path.as_ref().to_path_buf();
        let mut protected_roots = protected_roots;
        if let Some(parent) = module_path.parent() {
            protected_roots.push(parent.to_path_buf());
        }
        for entry in &analysis.resource_catalog.entries {
            add_source_root(&mut protected_roots, &entry.selected.source_path);
            for shadowed in &entry.shadowed {
                add_source_root(&mut protected_roots, &shadowed.source_path);
            }
        }
        for dependency in &analysis.dependency_report.dependencies {
            if let Some(path) = dependency.selected_path.as_deref() {
                add_source_root(&mut protected_roots, path);
            }
        }
        protected_roots.sort();
        protected_roots.dedup();
        Self {
            module_path,
            module_sha256: analysis.fingerprint.sha256.clone(),
            module_size_bytes: analysis.fingerprint.size_bytes,
            resource_catalog: analysis.resource_catalog.clone(),
            world_index: analysis.world_index.clone(),
            dependency_report: analysis.dependency_report.clone(),
            protected_roots,
            source_snapshot: Arc::new(Mutex::new(None)),
        }
    }
}

fn add_source_root(roots: &mut Vec<PathBuf>, source_path: &str) {
    let container = source_path
        .split_once("::")
        .map_or(source_path, |(path, _)| path);
    let path = Path::new(container);
    if let Some(parent) = path.parent() {
        roots.push(parent.to_path_buf());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedResourceFingerprint {
    pub key: ResourceKey,
    pub selected: aurora_resource::ResourceVersion,
    pub content_sha256: String,
    pub content_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFileFingerprint {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrationSourceSnapshot {
    pub resources: BTreeMap<ResourceKey, CapturedResourceFingerprint>,
    pub dependencies: BTreeMap<String, CapturedFileFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaMigrationCandidate {
    pub resref: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tile_count: usize,
    pub instance_count: usize,
    pub source_diagnostic_count: usize,
}

/// Explicit migration disposition. No object or asset is omitted without one of these states.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationStatus {
    Exact,
    Converted,
    Approximated,
    Placeholder,
    Manual,
    Unsupported,
    Missing,
    LicenseBlocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MigrationDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MigrationPhase {
    Preparing,
    Audit,
    Models,
    Textures,
    Navigation,
    Bundle,
    Verifying,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationDiagnostic {
    pub sequence: usize,
    pub severity: MigrationDiagnosticSeverity,
    pub status: MigrationStatus,
    pub phase: MigrationPhase,
    pub code: String,
    pub message: String,
    pub resource: Option<String>,
    pub identity: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationCounts {
    pub tiles: usize,
    pub instances: usize,
    pub unique_models: usize,
    pub textures: usize,
    pub preserved_navigation: usize,
    pub missing_items: usize,
    pub fallbacks: usize,
    pub diagnostics: usize,
    pub warnings: usize,
    pub errors: usize,
    pub by_status: BTreeMap<MigrationStatus, usize>,
}

impl MigrationCounts {
    pub(crate) fn record_status(&mut self, status: MigrationStatus) {
        *self.by_status.entry(status).or_default() += 1;
        if status == MigrationStatus::Missing {
            self.missing_items += 1;
        }
        if matches!(
            status,
            MigrationStatus::Approximated
                | MigrationStatus::Placeholder
                | MigrationStatus::Manual
                | MigrationStatus::Unsupported
                | MigrationStatus::LicenseBlocked
        ) {
            self.fallbacks += 1;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaMigrationPreview {
    pub schema_version: String,
    pub area_resref: String,
    pub area_name: String,
    pub suggested_directory_name: String,
    pub ready: bool,
    pub complete: bool,
    pub counts: MigrationCounts,
    pub diagnostics: Vec<MigrationDiagnostic>,
    pub classification: String,
    pub redistribution: String,
    pub navigation_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaMigrationExportRequest {
    pub area_resref: String,
    pub destination: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationProgress {
    pub phase: MigrationPhase,
    pub percent: f64,
    pub current: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BundleFileRecord {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResourceVersion {
    pub source_kind: ResourceSourceKind,
    pub source_name: String,
    pub source_file_name: String,
    pub source_path_sha256: String,
    pub priority: u32,
    pub offset: u64,
    pub size_bytes: u64,
    pub content_sha256: Option<String>,
}

impl MigrationResourceVersion {
    pub(crate) fn sanitized(version: &ResourceVersion) -> Self {
        use sha2::{Digest, Sha256};

        let normalized_path = version.source_path.replace('\\', "/").to_ascii_lowercase();
        let source_file_name = Path::new(&version.source_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&version.source_name)
            .to_owned();
        Self {
            source_kind: version.source_kind,
            source_name: version.source_name.clone(),
            source_file_name,
            source_path_sha256: format!("{:x}", Sha256::digest(normalized_path.as_bytes())),
            priority: version.priority,
            offset: version.offset,
            size_bytes: version.size,
            content_sha256: version.sha256.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProvenance {
    pub resource_key: String,
    pub selected: MigrationResourceVersion,
    pub shadowed: Vec<MigrationResourceVersion>,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyProvenance {
    pub kind: ModuleDependencyKind,
    pub logical_name: String,
    pub state: ModuleDependencyState,
    pub selected_path_sha256: Option<String>,
    pub selected_size_bytes: Option<u64>,
    pub selected_content_sha256: Option<String>,
    pub shadowed_version_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BundleManifest {
    pub schema_version: String,
    pub exporter: String,
    pub module_sha256: String,
    pub module_size_bytes: u64,
    pub area_resref: String,
    pub classification: String,
    pub redistribution: String,
    pub coordinate_system: CoordinateConvention,
    pub resolution_policy: String,
    pub dependencies: Vec<DependencyProvenance>,
    pub resources: Vec<ResourceProvenance>,
    pub counts: MigrationCounts,
    /// Every payload file is listed. `manifest.json` is intentionally excluded because a file
    /// cannot contain the SHA-256 of its own final bytes; its hash is returned by the job/CLI.
    pub files: Vec<BundleFileRecord>,
    pub integrity_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoordinateConvention {
    pub version: String,
    pub source: String,
    pub canonical: String,
    pub mapping: String,
    pub handedness: String,
    pub basis_rows: [[i8; 3]; 3],
}

impl Default for CoordinateConvention {
    fn default() -> Self {
        Self {
            version: "nwn-to-canonical@1".to_owned(),
            source: "NWN right-handed Z-up [x,y,z]".to_owned(),
            canonical: "right-handed Y-up [x,y,z]".to_owned(),
            mapping: "[x,y,z] -> [x,z,-y]".to_owned(),
            handedness: "basis determinant +1; GLB indices adapt NWN front faces to glTF"
                .to_owned(),
            basis_rows: [[1, 0, 0], [0, 0, 1], [0, -1, 0]],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub schema_version: String,
    pub area_resref: String,
    pub complete: bool,
    pub counts: MigrationCounts,
    pub navigation_converted: bool,
    pub navigation_status: String,
    pub diagnostics_file: String,
    pub bundle_is_local_only: bool,
    pub source_module_immutable: bool,
    pub payload_file_count: usize,
    pub payload_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaMigrationExportResult {
    pub bundle_path: String,
    pub manifest_file: BundleFileRecord,
    pub report: MigrationReport,
    pub diagnostics: Vec<MigrationDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationAreaDocument {
    pub schema_version: String,
    pub resref: String,
    pub name: String,
    pub area_kind: String,
    pub dimensions: [u32; 2],
    pub grid_size_meters: f32,
    pub tileset: Option<String>,
    pub tiles: Vec<MigrationTile>,
    pub instances: Vec<MigrationInstance>,
    pub assets: Vec<MigrationAsset>,
    pub source_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceTransform {
    pub position: [f32; 3],
    pub yaw_radians: f32,
}

impl SourceTransform {
    pub(crate) fn from_values(position: [f32; 3], yaw_radians: f32) -> Self {
        Self {
            position,
            yaw_radians,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTile {
    pub id: String,
    pub source: AreaTile,
    pub source_transform: SourceTransform,
    pub transform: CanonicalTransform,
    pub model_resref: Option<String>,
    pub model_asset: Option<String>,
    pub status: MigrationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationInstance {
    pub id: String,
    pub source_identity: String,
    pub source: AreaInstance,
    pub source_transform: Option<SourceTransform>,
    pub transform: Option<CanonicalTransform>,
    pub canonical_geometry: Vec<[f32; 3]>,
    pub canonical_spawn_points: Vec<MigrationSpawnPoint>,
    pub model_resrefs: Vec<String>,
    pub model_assets: Vec<String>,
    pub classification: String,
    pub status: MigrationStatus,
    pub status_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSpawnPoint {
    pub source: AreaSpawnPoint,
    pub transform: CanonicalTransform,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationAsset {
    pub id: String,
    pub kind: String,
    pub resource_keys: Vec<String>,
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub status: MigrationStatus,
    pub texture_paths: Vec<String>,
    pub navigation_paths: Vec<String>,
    #[serde(default)]
    pub surface_ids: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityMap {
    pub schema_version: String,
    pub module_sha256: String,
    pub area_resref: String,
    pub entries: Vec<IdentityEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityEntry {
    pub stable_id: String,
    pub source_kind: String,
    pub resource_key: String,
    pub instance_identity: String,
}

pub(crate) fn display_area_name(area: &AreaMap) -> String {
    area.name
        .text
        .clone()
        .unwrap_or_else(|| area.resref.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_v1_disposition_has_a_stable_explicit_wire_value() {
        let statuses = [
            MigrationStatus::Exact,
            MigrationStatus::Converted,
            MigrationStatus::Approximated,
            MigrationStatus::Placeholder,
            MigrationStatus::Manual,
            MigrationStatus::Unsupported,
            MigrationStatus::Missing,
            MigrationStatus::LicenseBlocked,
        ];
        let values = statuses
            .into_iter()
            .map(|status| {
                serde_json::to_value(status)
                    .expect("status")
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            [
                "exact",
                "converted",
                "approximated",
                "placeholder",
                "manual",
                "unsupported",
                "missing",
                "license-blocked",
            ]
        );
    }
}
