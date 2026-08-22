use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_mdl::{MdlModel, export_glb_with_texture_uris};
use aurora_project::{
    ModuleAnalysis, ResourceCatalog, ResourceManager, resolve_model_for_export_with_dependencies,
};
use aurora_world::{AssetRecord, AssetSupport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const ASSET_EXPORT_SCHEMA_VERSION: &str = "opennever-asset-export@1.0.0";
pub const ASSET_EXPORT_CLASSIFICATION: &str = "local_only_proprietary";
pub const ASSET_EXPORT_REDISTRIBUTION: &str = "not_redistributable_without_separate_rights";
pub const MAX_ASSET_TEXTURES: usize = 4_096;
pub const MAX_ASSET_EXPORT_BYTES: u64 = 1024 * 1024 * 1024;

const MODEL_RESOURCE_TYPE: u16 = 2002;
const TEXTURE_PRIORITY: [u16; 4] = [2033, 3, 2080, 6];

#[derive(Debug, Clone)]
pub struct AssetExportSource {
    pub module_path: PathBuf,
    pub module_sha256: String,
    pub resource_catalog: ResourceCatalog,
    pub assets: Vec<AssetRecord>,
    pub protected_roots: Vec<PathBuf>,
}

impl AssetExportSource {
    pub fn from_analysis_with_roots(
        analysis: &ModuleAnalysis,
        module_path: impl AsRef<Path>,
        mut protected_roots: Vec<PathBuf>,
    ) -> Self {
        let module_path = module_path.as_ref().to_path_buf();
        if let Some(parent) = module_path.parent() {
            protected_roots.push(parent.to_path_buf());
        }
        for resource in &analysis.resource_catalog.entries {
            add_source_root(&mut protected_roots, &resource.selected.source_path);
            for shadowed in &resource.shadowed {
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
            resource_catalog: analysis.resource_catalog.clone(),
            assets: analysis.world_index.assets.assets.clone(),
            protected_roots,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportCandidate {
    pub resref: String,
    pub format: String,
    pub source: String,
    pub exportable: bool,
    pub declared_animation_count: usize,
    pub declared_animations: Vec<String>,
    pub mesh_count: usize,
    pub triangle_count: usize,
    pub skin_count: usize,
    pub texture_count: usize,
    pub diagnostic_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetAnimationSummary {
    pub name: String,
    pub length_seconds: f32,
    pub transition_seconds: f32,
    pub root_node: String,
    pub track_count: usize,
    pub event_count: usize,
    pub exported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetTextureSummary {
    pub resref: String,
    pub resource_type: Option<u16>,
    pub output_path: Option<String>,
    pub status: String,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetExportMode {
    Static,
    Animated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportPreview {
    pub schema_version: String,
    pub resref: String,
    pub mode: AssetExportMode,
    pub ready: bool,
    pub suggested_directory_name: String,
    pub node_count: usize,
    pub mesh_count: usize,
    pub primitive_count: usize,
    pub skin_count: usize,
    pub animation_count: usize,
    pub animations: Vec<AssetAnimationSummary>,
    pub textures: Vec<AssetTextureSummary>,
    pub warnings: Vec<String>,
    pub classification: String,
    pub redistribution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportFile {
    pub path: String,
    pub role: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportManifest {
    pub schema_version: String,
    pub generator: String,
    pub classification: String,
    pub redistribution: String,
    pub source_module_sha256: String,
    pub source_model: String,
    pub source_model_sha256: String,
    pub source_dependencies: BTreeMap<String, String>,
    pub mode: AssetExportMode,
    pub animations: Vec<AssetAnimationSummary>,
    pub textures: Vec<AssetTextureSummary>,
    pub warnings: Vec<String>,
    pub files: Vec<AssetExportFile>,
    pub source_module_immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetExportResult {
    pub schema_version: String,
    pub destination: String,
    pub resref: String,
    pub mode: AssetExportMode,
    pub glb_path: String,
    pub glb_sha256: String,
    pub glb_size_bytes: u64,
    pub animation_count: usize,
    pub texture_count: usize,
    pub warnings: Vec<String>,
    pub manifest: AssetExportManifest,
}

pub fn list_asset_export_candidates(source: &AssetExportSource) -> Vec<AssetExportCandidate> {
    let mut values = source
        .assets
        .iter()
        .filter(|asset| asset.key.resource_type == MODEL_RESOURCE_TYPE)
        .map(|asset| AssetExportCandidate {
            resref: asset.key.resref.clone(),
            format: asset.format.clone(),
            source: asset.source.clone(),
            exportable: asset.glb_preview && asset.support == AssetSupport::Preview,
            declared_animation_count: asset.animations.len(),
            declared_animations: asset.animations.clone(),
            mesh_count: asset.mesh_count,
            triangle_count: asset.triangle_count,
            skin_count: asset.skin_count,
            texture_count: asset.textures.len(),
            diagnostic_count: asset.diagnostics.len(),
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| left.resref.cmp(&right.resref));
    values
}

pub fn preview_asset_export(
    source: &AssetExportSource,
    resref: &str,
    cancelled: &AtomicBool,
) -> AppResult<AssetExportPreview> {
    check_cancelled(cancelled, resref)?;
    let normalized = normalize_resref(resref)?;
    let resolved = resolve_model_for_export_with_dependencies(
        &source.resource_catalog,
        &normalized,
        cancelled,
    )?;
    let textures = plan_textures(&source.resource_catalog, &resolved.model)?;
    let artifact = export_glb_with_texture_uris(&resolved.model, &BTreeMap::new())
        .map_err(|error| glb_error(&normalized, error.code, error.message))?;
    let (animations, warnings) = summarize_animations(&resolved.model, artifact.animation_count);
    Ok(AssetExportPreview {
        schema_version: ASSET_EXPORT_SCHEMA_VERSION.to_owned(),
        resref: normalized.clone(),
        mode: mode_for_count(artifact.animation_count),
        ready: true,
        suggested_directory_name: format!("{normalized}.asset-export-v1"),
        node_count: artifact.node_count,
        mesh_count: artifact.mesh_count,
        primitive_count: artifact.primitive_count,
        skin_count: artifact.skin_count,
        animation_count: artifact.animation_count,
        animations,
        textures,
        warnings,
        classification: ASSET_EXPORT_CLASSIFICATION.to_owned(),
        redistribution: ASSET_EXPORT_REDISTRIBUTION.to_owned(),
    })
}

pub fn export_asset(
    source: &AssetExportSource,
    resref: &str,
    destination: &Path,
    cancelled: &AtomicBool,
) -> AppResult<AssetExportResult> {
    let destination = validate_asset_export_destination(destination, &source.protected_roots)?;
    check_cancelled(cancelled, resref)?;
    let normalized = normalize_resref(resref)?;
    let resolved = resolve_model_for_export_with_dependencies(
        &source.resource_catalog,
        &normalized,
        cancelled,
    )?;
    let before = capture_dependencies(
        &source.resource_catalog,
        &resolved.resource_resrefs,
        cancelled,
    )?;
    let parent = destination
        .parent()
        .expect("validated asset destination has a parent");
    let staging = tempfile::Builder::new()
        .prefix(".opennever-asset-export-")
        .tempdir_in(parent)
        .map_err(|error| {
            Box::new(AppError::io(
                "create asset export staging directory",
                parent.display().to_string(),
                &error,
            ))
        })?;
    let staging_root = staging.path();
    let mut texture_summaries = plan_textures(&source.resource_catalog, &resolved.model)?;
    let mut texture_uri_map = BTreeMap::new();
    let mut files = Vec::new();
    let mut warnings = Vec::new();
    let mut total_bytes = 0_u64;

    for texture in &mut texture_summaries {
        check_cancelled(cancelled, &texture.resref)?;
        let Some(resource_type) = texture.resource_type else {
            if let Some(diagnostic) = &texture.diagnostic {
                warnings.push(diagnostic.clone());
            }
            continue;
        };
        match aurora_project::build_texture_png(
            &source.resource_catalog,
            &texture.resref,
            resource_type,
            cancelled,
        ) {
            Ok(image) => {
                let path = format!("textures/{}.png", texture.resref);
                let record = write_payload(staging_root, &path, "texture", &image.bytes)?;
                reserve_export_bytes(&mut total_bytes, record.size_bytes, &path)?;
                texture.output_path = Some(path.clone());
                texture.status = "converted".to_owned();
                texture_uri_map.insert(texture.resref.clone(), path);
                files.push(record);
            }
            Err(error) => {
                texture.status = "fallback".to_owned();
                texture.diagnostic = Some(error.user_message.clone());
                warnings.push(format!("{}: {}", texture.resref, error.user_message));
            }
        }
    }

    let artifact = export_glb_with_texture_uris(&resolved.model, &texture_uri_map)
        .map_err(|error| glb_error(&normalized, error.code, error.message))?;
    let glb_path = format!("{normalized}.glb");
    let glb_file = write_payload(staging_root, &glb_path, "model", &artifact.bytes)?;
    reserve_export_bytes(&mut total_bytes, glb_file.size_bytes, &glb_path)?;
    let glb_sha256 = glb_file.sha256.clone();
    let glb_size_bytes = glb_file.size_bytes;
    files.push(glb_file);
    let (animations, animation_warnings) =
        summarize_animations(&resolved.model, artifact.animation_count);
    warnings.extend(animation_warnings);
    warnings.sort();
    warnings.dedup();

    let after = capture_dependencies(
        &source.resource_catalog,
        &resolved.resource_resrefs,
        cancelled,
    )?;
    if before != after {
        return Err(Box::new(
            AppError::new(
                "ASSET_EXPORT_SOURCE_CHANGED",
                "Une ressource source a changé pendant l'export.",
                "resolved model dependency hashes changed during asset export",
                ErrorSeverity::Error,
            )
            .with_resource(normalized)
            .with_import_stage("asset_export_source_check"),
        ));
    }

    let source_model_sha256 = before
        .get(&format!("{}.mdl", normalized))
        .cloned()
        .unwrap_or_else(|| resolved.model.source_sha256.clone());
    let manifest = AssetExportManifest {
        schema_version: ASSET_EXPORT_SCHEMA_VERSION.to_owned(),
        generator: "OpenNever Forge aurora-asset-export 0.1".to_owned(),
        classification: ASSET_EXPORT_CLASSIFICATION.to_owned(),
        redistribution: ASSET_EXPORT_REDISTRIBUTION.to_owned(),
        source_module_sha256: source.module_sha256.clone(),
        source_model: format!("{}.mdl", normalized),
        source_model_sha256,
        source_dependencies: before,
        mode: mode_for_count(artifact.animation_count),
        animations,
        textures: texture_summaries,
        warnings: warnings.clone(),
        files: files.clone(),
        source_module_immutable: true,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| {
        Box::new(
            AppError::new(
                "ASSET_EXPORT_MANIFEST_FAILED",
                "Le manifeste de l'asset n'a pas pu être généré.",
                error.to_string(),
                ErrorSeverity::Error,
            )
            .with_resource(normalized.clone())
            .with_import_stage("asset_export_manifest"),
        )
    })?;
    let manifest_file = write_payload(staging_root, "manifest.json", "manifest", &manifest_bytes)?;
    reserve_export_bytes(&mut total_bytes, manifest_file.size_bytes, "manifest.json")?;
    check_cancelled(cancelled, &normalized)?;
    let staging_path = staging.keep();
    if let Err(error) = fs::rename(&staging_path, &destination) {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(Box::new(AppError::io(
            "publish asset export",
            destination.display().to_string(),
            &error,
        )));
    }

    Ok(AssetExportResult {
        schema_version: ASSET_EXPORT_SCHEMA_VERSION.to_owned(),
        destination: destination.display().to_string(),
        resref: normalized,
        mode: mode_for_count(artifact.animation_count),
        glb_path,
        glb_sha256,
        glb_size_bytes,
        animation_count: artifact.animation_count,
        texture_count: manifest
            .textures
            .iter()
            .filter(|texture| texture.output_path.is_some())
            .count(),
        warnings,
        manifest,
    })
}

pub fn validate_asset_export_destination(
    destination: &Path,
    protected_roots: &[PathBuf],
) -> AppResult<PathBuf> {
    if !destination.is_absolute() {
        return Err(path_error(destination, "destination must be absolute"));
    }
    if destination.exists() {
        return Err(path_error(destination, "destination already exists"));
    }
    if !matches!(destination.file_name(), Some(name) if matches!(Path::new(name).components().collect::<Vec<_>>().as_slice(), [Component::Normal(_)]))
    {
        return Err(path_error(destination, "destination name is invalid"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| path_error(destination, "destination has no parent"))?;
    if !parent.is_dir() {
        return Err(path_error(parent, "destination parent is not a directory"));
    }
    let parent_metadata = fs::symlink_metadata(parent).map_err(|error| {
        Box::new(AppError::io(
            "inspect asset destination parent",
            parent.display().to_string(),
            &error,
        ))
    })?;
    if is_link_metadata(&parent_metadata) || contains_link_component(parent) {
        return Err(path_error(
            parent,
            "destination parent must not traverse a symbolic link or junction",
        ));
    }
    let canonical_parent = parent.canonicalize().map_err(|error| {
        Box::new(AppError::io(
            "canonicalize asset destination parent",
            parent.display().to_string(),
            &error,
        ))
    })?;
    let normalized = canonical_parent.join(destination.file_name().expect("validated file name"));
    let candidate = normalized_path(&normalized);
    for protected in protected_roots {
        let Ok(protected) = protected.canonicalize() else {
            continue;
        };
        if is_same_or_descendant(&candidate, &normalized_path(&protected)) {
            return Err(path_error(
                &normalized,
                "destination is inside a protected NWN source root",
            ));
        }
    }
    Ok(normalized)
}

fn plan_textures(
    catalog: &ResourceCatalog,
    model: &MdlModel,
) -> AppResult<Vec<AssetTextureSummary>> {
    let resrefs = model
        .nodes
        .iter()
        .filter_map(|node| node.mesh.as_ref())
        .flat_map(|mesh| mesh.material.textures.iter())
        .filter_map(|value| normalize_texture_resref(value))
        .collect::<BTreeSet<_>>();
    if resrefs.len() > MAX_ASSET_TEXTURES {
        return Err(Box::new(
            AppError::new(
                "ASSET_EXPORT_TEXTURE_LIMIT_EXCEEDED",
                "Le modèle référence trop de textures pour un export borné.",
                format!(
                    "model references {} textures; limit is {MAX_ASSET_TEXTURES}",
                    resrefs.len()
                ),
                ErrorSeverity::Error,
            )
            .with_import_stage("asset_export_limits"),
        ));
    }
    Ok(resrefs
        .into_iter()
        .map(|resref| {
            let resource_type = TEXTURE_PRIORITY.into_iter().find(|resource_type| {
                catalog
                    .get(&ResourceKey::new(&resref, *resource_type))
                    .is_some()
            });
            AssetTextureSummary {
                resref: resref.clone(),
                resource_type,
                output_path: None,
                status: if resource_type.is_some() {
                    "planned"
                } else {
                    "missing"
                }
                .to_owned(),
                diagnostic: resource_type.is_none().then(|| {
                    format!("La texture {resref} n'est pas résolue dans un format PNG exportable.")
                }),
            }
        })
        .collect())
}

fn summarize_animations(
    model: &MdlModel,
    exported_count: usize,
) -> (Vec<AssetAnimationSummary>, Vec<String>) {
    let node_names = model
        .nodes
        .iter()
        .map(|node| node.name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let animations = model
        .animations
        .iter()
        .map(|animation| {
            let exported = animation
                .tracks
                .iter()
                .any(|track| node_names.contains(&track.node.to_ascii_lowercase()));
            AssetAnimationSummary {
                name: animation.name.clone(),
                length_seconds: animation.length,
                transition_seconds: animation.transition,
                root_node: animation.root_node.clone(),
                track_count: animation.tracks.len(),
                event_count: animation.events.len(),
                exported,
            }
        })
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if animations
        .iter()
        .filter(|animation| animation.exported)
        .count()
        != exported_count
    {
        warnings.push(
            "Le nombre de clips Aurora transformables diffère du nombre de clips écrits dans le GLB."
                .to_owned(),
        );
    }
    if !model.animations.is_empty() && exported_count == 0 {
        warnings.push(
            "Des animations sont déclarées, mais aucune piste transformable n'a pu être écrite dans le GLB."
                .to_owned(),
        );
    }
    (animations, warnings)
}

fn capture_dependencies(
    catalog: &ResourceCatalog,
    resrefs: &[String],
    cancelled: &AtomicBool,
) -> AppResult<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for resref in resrefs {
        check_cancelled(cancelled, resref)?;
        let key = ResourceKey::new(resref, MODEL_RESOURCE_TYPE);
        let resource = catalog.get(&key).ok_or_else(|| {
            Box::new(
                AppError::new(
                    "ASSET_EXPORT_DEPENDENCY_MISSING",
                    "Une dépendance du modèle n'est plus disponible.",
                    format!("Resource Manager did not resolve {key}"),
                    ErrorSeverity::Error,
                )
                .with_resource(key.to_string())
                .with_import_stage("asset_export_source_check"),
            )
        })?;
        values.insert(
            key.file_name(),
            ResourceManager::hash(&resource.selected, cancelled)?,
        );
    }
    Ok(values)
}

fn write_payload(
    root: &Path,
    relative: &str,
    role: &str,
    bytes: &[u8],
) -> AppResult<AssetExportFile> {
    let path = relative
        .split('/')
        .fold(root.to_path_buf(), |mut path, part| {
            path.push(part);
            path
        });
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            Box::new(AppError::io(
                "create asset export directory",
                parent.display().to_string(),
                &error,
            ))
        })?;
    }
    let mut file = fs::File::create(&path).map_err(|error| {
        Box::new(AppError::io(
            "create asset export payload",
            path.display().to_string(),
            &error,
        ))
    })?;
    file.write_all(bytes).map_err(|error| {
        Box::new(AppError::io(
            "write asset export payload",
            path.display().to_string(),
            &error,
        ))
    })?;
    file.sync_all().map_err(|error| {
        Box::new(AppError::io(
            "flush asset export payload",
            path.display().to_string(),
            &error,
        ))
    })?;
    Ok(AssetExportFile {
        path: relative.to_owned(),
        role: role.to_owned(),
        size_bytes: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    })
}

fn reserve_export_bytes(total: &mut u64, bytes: u64, label: &str) -> AppResult<()> {
    *total = total.checked_add(bytes).ok_or_else(|| size_error(label))?;
    if *total > MAX_ASSET_EXPORT_BYTES {
        return Err(size_error(label));
    }
    Ok(())
}

fn normalize_resref(value: &str) -> AppResult<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(Box::new(
            AppError::new(
                "ASSET_EXPORT_RESREF_INVALID",
                "Le ResRef de l'asset n'est pas valide.",
                format!("unsafe asset resref {value:?}"),
                ErrorSeverity::Warning,
            )
            .with_import_stage("asset_export_validation"),
        ));
    }
    Ok(value)
}

fn normalize_texture_resref(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('"').to_ascii_lowercase();
    (!value.is_empty()
        && value != "null"
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')))
    .then_some(value)
}

fn mode_for_count(animation_count: usize) -> AssetExportMode {
    if animation_count > 0 {
        AssetExportMode::Animated
    } else {
        AssetExportMode::Static
    }
}

fn check_cancelled(cancelled: &AtomicBool, resource: &str) -> AppResult<()> {
    if cancelled.load(Ordering::Relaxed) {
        Err(AppError::job_cancelled(resource.to_owned()).into())
    } else {
        Ok(())
    }
}

fn add_source_root(roots: &mut Vec<PathBuf>, source_path: &str) {
    let container = source_path
        .split_once("::")
        .map_or(source_path, |(path, _)| path);
    if let Some(parent) = Path::new(container).parent() {
        roots.push(parent.to_path_buf());
    }
}

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

fn is_link_metadata(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn contains_link_component(path: &Path) -> bool {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if let Ok(metadata) = fs::symlink_metadata(&current)
            && is_link_metadata(&metadata)
        {
            return true;
        }
    }
    false
}

fn normalized_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    value
        .strip_prefix("//?/")
        .or_else(|| value.strip_prefix("//./"))
        .unwrap_or(&value)
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn is_same_or_descendant(candidate: &str, protected: &str) -> bool {
    candidate == protected
        || candidate
            .strip_prefix(protected)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn glb_error(resref: &str, code: String, message: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Le modèle n'a pas pu être exporté en GLB.",
            message,
            ErrorSeverity::Error,
        )
        .with_resource(ResourceKey::new(resref, MODEL_RESOURCE_TYPE).to_string())
        .with_import_stage("asset_glb_export"),
    )
}

fn path_error(path: &Path, technical: &str) -> Box<AppError> {
    Box::new(
        AppError::new(
            "ASSET_EXPORT_DESTINATION_INVALID",
            "La destination de l'export d'asset n'est pas valide.",
            format!("{}: {technical}", path.display()),
            ErrorSeverity::Warning,
        )
        .with_import_stage("asset_export_path_validation"),
    )
}

fn size_error(label: &str) -> Box<AppError> {
    Box::new(
        AppError::new(
            "ASSET_EXPORT_SIZE_LIMIT_EXCEEDED",
            "L'asset dépasse la taille maximale d'export.",
            format!("asset export exceeds {MAX_ASSET_EXPORT_BYTES} bytes while writing {label}"),
            ErrorSeverity::Error,
        )
        .with_import_stage("asset_export_limits"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_mdl::{
        AnimationTrack, MdlAnimation, TrackPath, export_glb_with_texture_uris, parse_mdl,
    };
    use aurora_resource::{
        ResolvedResource, ResourceLocation, ResourceSourceKind, ResourceVersion,
    };

    #[test]
    fn exports_static_model_with_texture_and_manifest() {
        let source_root = tempfile::tempdir().expect("source root");
        let export_root = tempfile::tempdir().expect("export root");
        let model_path = source_root.path().join("crate.mdl");
        let texture_path = source_root.path().join("stone.png");
        fs::write(
            &model_path,
            b"newmodel crate\nnode trimesh body\nbitmap stone\nverts 3\n0 0 0\n1 0 0\n0 1 0\ntverts 3\n0 0\n1 0\n0 1\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\ndonemodel crate\n",
        )
        .expect("model");
        fs::write(&texture_path, png_pixel()).expect("texture");
        let catalog = ResourceCatalog {
            entries: vec![
                loose_resource("crate", MODEL_RESOURCE_TYPE, &model_path),
                loose_resource("stone", 2080, &texture_path),
            ],
            version_count: 2,
            shadowed_count: 0,
            diagnostics: Vec::new(),
        };
        let source = AssetExportSource {
            module_path: source_root.path().join("fixture.mod"),
            module_sha256: "module-sha".to_owned(),
            resource_catalog: catalog,
            assets: Vec::new(),
            protected_roots: vec![source_root.path().to_path_buf()],
        };
        let destination = export_root.path().join("crate.asset-export-v1");
        let result =
            export_asset(&source, "crate", &destination, &AtomicBool::new(false)).expect("export");
        assert_eq!(result.mode, AssetExportMode::Static);
        assert_eq!(result.texture_count, 1);
        assert!(destination.join("crate.glb").is_file());
        assert!(destination.join("textures/stone.png").is_file());
        assert!(destination.join("manifest.json").is_file());
        assert_eq!(result.manifest.files[0].path, "textures/stone.png");
        let glb = fs::read(destination.join("crate.glb")).expect("GLB");
        let json_length = u32::from_le_bytes(glb[12..16].try_into().expect("JSON length"));
        let document: serde_json::Value = serde_json::from_slice(
            &glb[20..20 + usize::try_from(json_length).expect("length fits")],
        )
        .expect("GLB JSON");
        assert_eq!(document["images"][0]["uri"], "textures/stone.png");
    }

    #[test]
    fn animation_summary_matches_a_real_glb_clip() {
        let mut model = parse_mdl(
            b"newmodel actor\nnode dummy root\nendnode\nnode trimesh body\nparent root\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n",
        )
        .expect("model");
        model.animations.push(MdlAnimation {
            name: "walk".to_owned(),
            root_node: "root".to_owned(),
            length: 1.0,
            transition: 0.2,
            events: Vec::new(),
            tracks: vec![AnimationTrack {
                node: "root".to_owned(),
                path: TrackPath::Translation,
                times: vec![0.0, 1.0],
                values: vec![[0.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]],
            }],
        });
        let artifact = export_glb_with_texture_uris(&model, &BTreeMap::new()).expect("GLB");
        let (animations, warnings) = summarize_animations(&model, artifact.animation_count);
        assert_eq!(artifact.animation_count, 1);
        assert_eq!(animations[0].name, "walk");
        assert!(animations[0].exported);
        assert!(warnings.is_empty());
    }

    #[test]
    fn protected_source_root_rejects_destination() {
        let root = tempfile::tempdir().expect("root");
        let destination = root.path().join("asset.asset-export-v1");
        let error = validate_asset_export_destination(&destination, &[root.path().to_path_buf()])
            .expect_err("protected destination");
        assert_eq!(error.code, "ASSET_EXPORT_DESTINATION_INVALID");
    }

    fn loose_resource(resref: &str, resource_type: u16, path: &Path) -> ResolvedResource {
        let size = fs::metadata(path).expect("metadata").len();
        let key = ResourceKey::new(resref, resource_type);
        ResolvedResource {
            key: key.clone(),
            selected: ResourceVersion {
                key,
                source_kind: ResourceSourceKind::Standalone,
                source_name: path.file_name().unwrap().to_string_lossy().to_string(),
                source_path: path.display().to_string(),
                priority: 0,
                offset: 0,
                size,
                sha256: None,
                location: ResourceLocation::File {
                    path: path.display().to_string(),
                },
            },
            shadowed: Vec::new(),
        }
    }

    fn png_pixel() -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("PNG header");
            writer
                .write_image_data(&[255, 255, 255, 255])
                .expect("PNG pixel");
        }
        bytes
    }
}
