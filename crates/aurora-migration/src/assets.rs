use crate::diagnostics::DiagnosticCollector;
use crate::model::{
    MigrationAsset, MigrationDiagnosticSeverity, MigrationPhase, MigrationStatus,
    ResourceProvenance,
};
use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_mdl::export_glb_with_texture_uris;
use aurora_project::{
    convert_texture_png, preflight_texture_bytes, resolve_model_for_export_with_dependencies,
};
use aurora_resource::{ResolvedResource, ResourceCatalog, ResourceManager, ResourceVersion};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub(crate) struct PlannedFile {
    pub path: String,
    pub role: String,
    pub scratch_path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
}

pub(crate) const MAX_TEMPORARY_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct MaterializationBudget {
    payload_bytes: u64,
    temporary_bytes: u64,
}

impl MaterializationBudget {
    pub(crate) fn reserve_input(&mut self, bytes: usize, label: &str) -> AppResult<()> {
        let bytes = u64::try_from(bytes).map_err(|_| size_limit_error(label.to_owned()))?;
        let next = self.temporary_bytes.checked_add(bytes).ok_or_else(|| {
            size_limit_error(format!("temporary allocation overflow for {label}"))
        })?;
        if next > MAX_TEMPORARY_BYTES {
            return Err(size_limit_error(format!(
                "temporary allocation for {label} exceeds {MAX_TEMPORARY_BYTES} bytes"
            )));
        }
        self.temporary_bytes = next;
        Ok(())
    }

    pub(crate) fn release_input(&mut self, bytes: usize) {
        self.temporary_bytes = self
            .temporary_bytes
            .saturating_sub(u64::try_from(bytes).unwrap_or(u64::MAX));
    }

    pub(crate) fn reserve_payload(&mut self, bytes: usize, label: &str) -> AppResult<()> {
        let bytes = u64::try_from(bytes).map_err(|_| size_limit_error(label.to_owned()))?;
        let next = self
            .payload_bytes
            .checked_add(bytes)
            .ok_or_else(|| size_limit_error(format!("payload allocation overflow for {label}")))?;
        if next > crate::model::MAX_BUNDLE_BYTES {
            return Err(size_limit_error(format!(
                "payload allocation for {label} exceeds {} bytes",
                crate::model::MAX_BUNDLE_BYTES
            )));
        }
        self.payload_bytes = next;
        Ok(())
    }
}

pub(crate) fn materialize_file(
    scratch_root: &Path,
    path: String,
    role: &str,
    bytes: Vec<u8>,
    budget: &mut MaterializationBudget,
) -> AppResult<PlannedFile> {
    budget.reserve_payload(bytes.len(), &path)?;
    budget.reserve_input(bytes.len(), &path)?;
    let digest = hash_bytes(&bytes);
    let scratch_path = path
        .split('/')
        .fold(scratch_root.to_path_buf(), |mut root, part| {
            root.push(part);
            root
        });
    if let Some(parent) = scratch_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            Box::new(AppError::io(
                "create migration scratch directory",
                "scratch",
                &error,
            ))
        })?;
    }
    let result = fs::write(&scratch_path, &bytes).map_err(|error| {
        Box::new(AppError::io(
            "write migration scratch payload",
            "scratch",
            &error,
        ))
    });
    budget.release_input(bytes.len());
    result.map(|()| PlannedFile {
        path,
        role: role.to_owned(),
        scratch_path,
        size_bytes: bytes.len() as u64,
        sha256: digest,
    })
}

#[derive(Debug, Default)]
pub(crate) struct AssetPlan {
    pub model_path_by_resref: BTreeMap<String, String>,
    pub texture_path_by_resref: BTreeMap<String, String>,
    pub assets: Vec<MigrationAsset>,
    pub files: Vec<PlannedFile>,
    pub provenance: Vec<ResourceProvenance>,
}

#[derive(Debug, Default)]
pub(crate) struct AssetAudit {
    pub model_resrefs: BTreeSet<String>,
    pub texture_resrefs: BTreeSet<String>,
    pub navigation_count: usize,
}

const TEXTURE_PRIORITY: [u16; 4] = [2033, 3, 2080, 6];

pub(crate) fn audit_assets(
    catalog: &ResourceCatalog,
    requested_models: &BTreeSet<String>,
    navigation_requests: &BTreeMap<String, BTreeSet<u16>>,
    cancelled: &AtomicBool,
    diagnostics: &mut DiagnosticCollector,
) -> AppResult<AssetAudit> {
    let mut audit = AssetAudit::default();
    let mut texture_resrefs = BTreeSet::new();

    for resref in requested_models {
        check_cancelled(cancelled, resref)?;
        let key = ResourceKey::new(resref, 2002);
        let Some(resource) = catalog.get(&key) else {
            push_missing_model(diagnostics, &key);
            continue;
        };
        if resource.selected.size > MAX_TEMPORARY_BYTES {
            diagnostics.push(
                MigrationDiagnosticSeverity::Error,
                MigrationStatus::Unsupported,
                MigrationPhase::Models,
                "MIGRATION_BUNDLE_LIMIT_EXCEEDED",
                "Le modèle dépasse la taille d'entrée bornée avant toute allocation.",
                Some(key.to_string()),
                None,
            );
            continue;
        }
        match resolve_model_for_export_with_dependencies(catalog, resref, cancelled) {
            Ok(resolved) => {
                audit.model_resrefs.insert(resref.clone());
                texture_resrefs.extend(
                    resolved
                        .model
                        .nodes
                        .iter()
                        .filter_map(|node| node.mesh.as_ref())
                        .flat_map(|mesh| mesh.material.textures.iter())
                        .filter_map(|texture| normalize_asset_resref(texture)),
                );
            }
            Err(error) => diagnostics.push(
                MigrationDiagnosticSeverity::Warning,
                MigrationStatus::Unsupported,
                MigrationPhase::Models,
                &error.code,
                error.user_message,
                Some(key.to_string()),
                None,
            ),
        }
    }

    for resref in texture_resrefs {
        check_cancelled(cancelled, &resref)?;
        let mut resolved = false;
        let mut had_candidate = false;
        for resource_type in TEXTURE_PRIORITY {
            let key = ResourceKey::new(&resref, resource_type);
            let Some(resource) = catalog.get(&key) else {
                continue;
            };
            had_candidate = true;
            if resource.selected.size > MAX_TEMPORARY_BYTES {
                push_texture_rejected(
                    diagnostics,
                    &key,
                    "MIGRATION_BUNDLE_LIMIT_EXCEEDED",
                    "Le candidat texture dépasse la taille temporaire bornée ; le candidat suivant sera essayé.",
                );
                continue;
            }
            let bytes = match ResourceManager::read(&resource.selected, cancelled) {
                Ok(bytes) => bytes,
                Err(error) if error.code == "JOB_CANCELLED" => return Err(error),
                Err(_) => {
                    push_texture_rejected(
                        diagnostics,
                        &key,
                        "MIGRATION_TEXTURE_CANDIDATE_REJECTED",
                        "Le candidat texture n'a pas pu être lu ; le candidat suivant sera essayé.",
                    );
                    continue;
                }
            };
            match preflight_texture_bytes(&bytes, &key, resource_type) {
                Ok(_) => {
                    audit.texture_resrefs.insert(resref.clone());
                    resolved = true;
                    break;
                }
                Err(error) => {
                    let code = if error.code == "TEXTURE_DECODED_SIZE_LIMIT" {
                        "MIGRATION_TEXTURE_DECODE_LIMIT"
                    } else {
                        "MIGRATION_TEXTURE_CANDIDATE_REJECTED"
                    };
                    push_texture_rejected(
                        diagnostics,
                        &key,
                        code,
                        "Le candidat texture a été refusé par le préflight borné ; le candidat suivant sera essayé.",
                    );
                }
            }
        }
        if !resolved && !had_candidate {
            diagnostics.push(
                MigrationDiagnosticSeverity::Warning,
                MigrationStatus::Missing,
                MigrationPhase::Textures,
                "MIGRATION_TEXTURE_MISSING",
                format!("La texture {resref} n'est résolue dans aucun format pris en charge."),
                Some(resref),
                None,
            );
        } else if !resolved {
            diagnostics.push(
                MigrationDiagnosticSeverity::Warning,
                MigrationStatus::Unsupported,
                MigrationPhase::Textures,
                "MIGRATION_TEXTURE_UNSUPPORTED",
                format!("Aucun candidat texture de {resref} n'est convertible."),
                Some(resref),
                None,
            );
        }
    }

    for (resref, types) in navigation_requests {
        for resource_type in types {
            let key = ResourceKey::new(resref, *resource_type);
            let Some(resource) = catalog.get(&key) else {
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Missing,
                    MigrationPhase::Navigation,
                    "MIGRATION_NAVIGATION_SOURCE_MISSING",
                    format!("La source de navigation {key} n'est pas résolue."),
                    Some(key.to_string()),
                    Some(resref.clone()),
                );
                continue;
            };
            if resource.selected.size > MAX_TEMPORARY_BYTES {
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Unsupported,
                    MigrationPhase::Navigation,
                    "MIGRATION_BUNDLE_LIMIT_EXCEEDED",
                    "La source de navigation dépasse la taille temporaire bornée.",
                    Some(key.to_string()),
                    Some(resref.clone()),
                );
                continue;
            }
            audit.navigation_count += 1;
        }
    }
    Ok(audit)
}

#[derive(Debug)]
struct ModelMetadata {
    resref: String,
    key: ResourceKey,
    resource_keys: Vec<ResourceKey>,
    texture_resrefs: BTreeSet<String>,
}

pub(crate) fn plan_assets(
    catalog: &ResourceCatalog,
    requested_models: &BTreeSet<String>,
    scratch_root: &Path,
    cancelled: &AtomicBool,
    diagnostics: &mut DiagnosticCollector,
    mut on_progress: impl FnMut(MigrationPhase, usize, usize, &str),
) -> AppResult<AssetPlan> {
    let mut plan = AssetPlan::default();
    let mut metadata = Vec::new();
    let mut all_textures = BTreeSet::new();
    let mut budget = MaterializationBudget::default();

    // Pass 1 only keeps bounded model metadata. The parsed model is dropped before the next one.
    for (index, resref) in requested_models.iter().enumerate() {
        check_cancelled(cancelled, resref)?;
        on_progress(
            MigrationPhase::Models,
            index,
            requested_models.len(),
            resref,
        );
        let key = ResourceKey::new(resref, 2002);
        let Some(resource) = catalog.get(&key) else {
            push_missing_model(diagnostics, &key);
            continue;
        };
        let resolved = match resolve_model_for_export_with_dependencies(catalog, resref, cancelled)
        {
            Ok(resolved) => resolved,
            Err(error) => {
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Unsupported,
                    MigrationPhase::Models,
                    &error.code,
                    error.user_message,
                    Some(key.to_string()),
                    None,
                );
                continue;
            }
        };
        let selected_bytes =
            match read_resource_bounded(&resource.selected, &key.to_string(), cancelled) {
                Ok(bytes) => bytes,
                Err(error) => {
                    diagnostics.push(
                        MigrationDiagnosticSeverity::Warning,
                        MigrationStatus::Missing,
                        MigrationPhase::Models,
                        &error.code,
                        error.user_message,
                        Some(key.to_string()),
                        None,
                    );
                    continue;
                }
            };
        plan.provenance.push(provenance(
            resource,
            "model",
            Some(hash_bytes(&selected_bytes)),
        ));
        let mut resource_keys = resolved
            .resource_resrefs
            .iter()
            .map(|dependency| ResourceKey::new(dependency, 2002))
            .collect::<Vec<_>>();
        resource_keys.sort();
        resource_keys.dedup();
        for dependency_key in resource_keys.iter().filter(|value| **value != key) {
            if let Some(dependency) = catalog.get(dependency_key) {
                match read_resource_bounded(
                    &dependency.selected,
                    &dependency_key.to_string(),
                    cancelled,
                ) {
                    Ok(bytes) => plan.provenance.push(provenance(
                        dependency,
                        "model-resolution-dependency",
                        Some(hash_bytes(&bytes)),
                    )),
                    Err(error) => diagnostics.push(
                        MigrationDiagnosticSeverity::Warning,
                        MigrationStatus::Missing,
                        MigrationPhase::Models,
                        &error.code,
                        error.user_message,
                        Some(dependency_key.to_string()),
                        Some(key.to_string()),
                    ),
                }
            }
        }
        let texture_resrefs = resolved
            .model
            .nodes
            .iter()
            .filter_map(|node| node.mesh.as_ref())
            .flat_map(|mesh| mesh.material.textures.iter())
            .filter_map(|texture| normalize_asset_resref(texture))
            .collect::<BTreeSet<_>>();
        all_textures.extend(texture_resrefs.iter().cloned());
        for diagnostic in &resolved.model.diagnostics {
            diagnostics.push(
                MigrationDiagnosticSeverity::Warning,
                MigrationStatus::Manual,
                MigrationPhase::Models,
                &diagnostic.code,
                &diagnostic.message,
                Some(key.to_string()),
                diagnostic.node.clone(),
            );
        }
        metadata.push(ModelMetadata {
            resref: resref.clone(),
            key,
            resource_keys,
            texture_resrefs,
        });
    }

    let mut texture_files = BTreeMap::<String, PlannedFile>::new();
    let mut texture_status_by_path = BTreeMap::<String, MigrationStatus>::new();
    for (index, resref) in all_textures.iter().enumerate() {
        check_cancelled(cancelled, resref)?;
        on_progress(MigrationPhase::Textures, index, all_textures.len(), resref);
        match resolve_and_convert_texture(catalog, resref, cancelled, diagnostics)? {
            TextureResolution::Resolved {
                key,
                resource,
                source_bytes,
                preview,
            } => {
                plan.provenance.push(provenance(
                    resource,
                    "base-color-texture",
                    Some(hash_bytes(&source_bytes)),
                ));
                let digest = hash_bytes(&preview.bytes);
                let path = format!("assets/textures/texture-{}.png", &digest[..24]);
                let status = match key.resource_type {
                    2080 => MigrationStatus::Exact,
                    6 => MigrationStatus::Approximated,
                    _ => MigrationStatus::Converted,
                };
                if status == MigrationStatus::Approximated {
                    diagnostics.push(
                        MigrationDiagnosticSeverity::Warning,
                        status,
                        MigrationPhase::Textures,
                        "MIGRATION_PLT_APPROXIMATED",
                        "La PLT est conservée comme aperçu PNG sans composition d'apparence complète.",
                        Some(key.to_string()),
                        None,
                    );
                }
                if !texture_files.contains_key(&path) {
                    texture_files.insert(
                        path.clone(),
                        materialize_file(
                            scratch_root,
                            path.clone(),
                            "texture",
                            preview.bytes,
                            &mut budget,
                        )?,
                    );
                }
                texture_status_by_path.entry(path.clone()).or_insert(status);
                plan.texture_path_by_resref.insert(resref.clone(), path);
            }
            TextureResolution::Rejected => {}
            TextureResolution::Missing => {
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Missing,
                    MigrationPhase::Textures,
                    "MIGRATION_TEXTURE_MISSING",
                    format!("La texture {resref} n'est résolue dans aucun format pris en charge."),
                    Some(resref.clone()),
                    None,
                );
            }
        }
    }

    let texture_uri_map = plan
        .texture_path_by_resref
        .iter()
        .map(|(resref, path)| {
            let file_name = path.rsplit('/').next().unwrap_or(path);
            (resref.clone(), format!("../textures/{file_name}"))
        })
        .collect::<BTreeMap<_, _>>();
    let mut model_files = BTreeMap::<String, PlannedFile>::new();
    let mut model_resources_by_path = BTreeMap::<String, Vec<String>>::new();
    let mut model_textures_by_path = BTreeMap::<String, BTreeSet<String>>::new();
    let mut model_status_by_path = BTreeMap::<String, MigrationStatus>::new();

    // Pass 2 converts one model at a time and immediately moves its GLB to scratch storage.
    for parsed in metadata {
        check_cancelled(cancelled, &parsed.resref)?;
        let resolved =
            match resolve_model_for_export_with_dependencies(catalog, &parsed.resref, cancelled) {
                Ok(resolved) => resolved,
                Err(error) => {
                    diagnostics.push(
                        MigrationDiagnosticSeverity::Warning,
                        MigrationStatus::Unsupported,
                        MigrationPhase::Models,
                        &error.code,
                        error.user_message,
                        Some(parsed.key.to_string()),
                        None,
                    );
                    continue;
                }
            };
        let model = resolved.model;
        let mut model_status = MigrationStatus::Converted;
        for node in &model.nodes {
            let Some(mesh) = &node.mesh else {
                continue;
            };
            let material_resrefs = mesh
                .material
                .textures
                .iter()
                .filter_map(|texture| normalize_asset_resref(texture))
                .collect::<BTreeSet<_>>();
            let mapped_count = material_resrefs
                .iter()
                .filter(|resref| plan.texture_path_by_resref.contains_key(*resref))
                .count();
            if material_resrefs.len() > 1 {
                model_status = MigrationStatus::Approximated;
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Approximated,
                    MigrationPhase::Textures,
                    "MIGRATION_MATERIAL_CHANNELS_APPROXIMATED",
                    "Le matériau Aurora référence plusieurs textures ; le GLB v1 lie une seule URI de couleur de base.",
                    Some(parsed.key.to_string()),
                    Some(node.name.clone()),
                );
            }
            if !material_resrefs.is_empty() && mapped_count == 0 {
                model_status = MigrationStatus::Approximated;
                diagnostics.push(
                    MigrationDiagnosticSeverity::Warning,
                    MigrationStatus::Approximated,
                    MigrationPhase::Textures,
                    "MIGRATION_MATERIAL_COLOR_FALLBACK",
                    "Aucune texture utilisable n'a été liée à ce matériau ; le facteur de couleur reste le fallback.",
                    Some(parsed.key.to_string()),
                    Some(node.name.clone()),
                );
            }
        }
        match export_glb_with_texture_uris(&model, &texture_uri_map) {
            Ok(glb) => {
                let path = format!("assets/models/model-{}.glb", &glb.glb_sha256[..24]);
                if !model_files.contains_key(&path) {
                    model_files.insert(
                        path.clone(),
                        materialize_file(
                            scratch_root,
                            path.clone(),
                            "model",
                            glb.bytes,
                            &mut budget,
                        )?,
                    );
                }
                model_resources_by_path
                    .entry(path.clone())
                    .or_default()
                    .extend(parsed.resource_keys.iter().map(ToString::to_string));
                model_textures_by_path
                    .entry(path.clone())
                    .or_default()
                    .extend(
                        parsed
                            .texture_resrefs
                            .iter()
                            .filter_map(|resref| plan.texture_path_by_resref.get(resref).cloned()),
                    );
                model_status_by_path
                    .entry(path.clone())
                    .and_modify(|status| *status = (*status).max(model_status))
                    .or_insert(model_status);
                plan.model_path_by_resref.insert(parsed.resref, path);
            }
            Err(error) => diagnostics.push(
                MigrationDiagnosticSeverity::Warning,
                MigrationStatus::Unsupported,
                MigrationPhase::Models,
                &error.code,
                error.message,
                Some(parsed.key.to_string()),
                None,
            ),
        }
    }

    for (path, file) in &texture_files {
        plan.assets.push(MigrationAsset {
            id: format!("asset:{}", &file.sha256[..24]),
            kind: "texture".to_owned(),
            resource_keys: plan
                .texture_path_by_resref
                .iter()
                .filter(|(_, value)| *value == path)
                .map(|(resref, _)| resref.clone())
                .collect(),
            path: path.clone(),
            size_bytes: file.size_bytes,
            sha256: file.sha256.clone(),
            status: *texture_status_by_path
                .get(path)
                .unwrap_or(&MigrationStatus::Converted),
            texture_paths: Vec::new(),
            navigation_paths: Vec::new(),
            surface_ids: Vec::new(),
        });
    }
    for (path, file) in &model_files {
        let mut resource_keys = model_resources_by_path.remove(path).unwrap_or_default();
        resource_keys.sort();
        resource_keys.dedup();
        plan.assets.push(MigrationAsset {
            id: format!("asset:{}", &file.sha256[..24]),
            kind: "model".to_owned(),
            resource_keys,
            path: path.clone(),
            size_bytes: file.size_bytes,
            sha256: file.sha256.clone(),
            status: model_status_by_path
                .remove(path)
                .unwrap_or(MigrationStatus::Converted),
            texture_paths: model_textures_by_path
                .remove(path)
                .unwrap_or_default()
                .into_iter()
                .collect(),
            navigation_paths: Vec::new(),
            surface_ids: Vec::new(),
        });
    }
    plan.files.extend(texture_files.into_values());
    plan.files.extend(model_files.into_values());
    plan.files.sort_by(|left, right| left.path.cmp(&right.path));
    plan.assets
        .sort_by(|left, right| left.path.cmp(&right.path));
    plan.provenance.sort_by(|left, right| {
        (&left.resource_key, &left.purpose).cmp(&(&right.resource_key, &right.purpose))
    });
    plan.provenance.dedup_by(|left, right| {
        left.resource_key == right.resource_key && left.purpose == right.purpose
    });
    Ok(plan)
}

enum TextureResolution<'a> {
    Resolved {
        key: ResourceKey,
        resource: &'a ResolvedResource,
        source_bytes: Vec<u8>,
        preview: aurora_project::AssetPreview,
    },
    Rejected,
    Missing,
}

fn resolve_and_convert_texture<'a>(
    catalog: &'a ResourceCatalog,
    resref: &str,
    cancelled: &AtomicBool,
    diagnostics: &mut DiagnosticCollector,
) -> AppResult<TextureResolution<'a>> {
    let mut had_candidate = false;
    for resource_type in TEXTURE_PRIORITY {
        let key = ResourceKey::new(resref, resource_type);
        let Some(resource) = catalog.get(&key) else {
            continue;
        };
        had_candidate = true;
        let source_bytes =
            match read_resource_bounded(&resource.selected, &key.to_string(), cancelled) {
                Ok(bytes) => bytes,
                Err(error) if error.code == "JOB_CANCELLED" => return Err(error),
                Err(_) => {
                    push_texture_rejected(
                        diagnostics,
                        &key,
                        "MIGRATION_TEXTURE_CANDIDATE_REJECTED",
                        "Le candidat texture n'a pas pu être lu ; le candidat suivant sera essayé.",
                    );
                    continue;
                }
            };
        if let Err(error) = preflight_texture_bytes(&source_bytes, &key, resource_type) {
            let code = if error.code == "TEXTURE_DECODED_SIZE_LIMIT" {
                "MIGRATION_TEXTURE_DECODE_LIMIT"
            } else {
                "MIGRATION_TEXTURE_CANDIDATE_REJECTED"
            };
            push_texture_rejected(
                diagnostics,
                &key,
                code,
                "Le candidat texture a été refusé par le préflight borné ; le candidat suivant sera essayé.",
            );
            continue;
        }
        match convert_texture_png(&source_bytes, &key, resource_type) {
            Ok(preview) => {
                return Ok(TextureResolution::Resolved {
                    key,
                    resource,
                    source_bytes,
                    preview,
                });
            }
            Err(error) if error.code == "JOB_CANCELLED" => return Err(error),
            Err(_error) => {
                push_texture_rejected(
                    diagnostics,
                    &key,
                    "MIGRATION_TEXTURE_CANDIDATE_REJECTED",
                    "Le candidat texture n'est pas convertible ; le candidat suivant sera essayé.",
                );
            }
        }
    }
    if had_candidate {
        diagnostics.push(
            MigrationDiagnosticSeverity::Warning,
            MigrationStatus::Unsupported,
            MigrationPhase::Textures,
            "MIGRATION_TEXTURE_UNSUPPORTED",
            format!("Aucun candidat texture de {resref} n'est convertible."),
            Some(resref.to_owned()),
            None,
        );
        Ok(TextureResolution::Rejected)
    } else {
        Ok(TextureResolution::Missing)
    }
}

fn read_resource_bounded(
    version: &ResourceVersion,
    label: &str,
    cancelled: &AtomicBool,
) -> AppResult<Vec<u8>> {
    if version.size > MAX_TEMPORARY_BYTES {
        return Err(size_limit_error(format!(
            "source {label} exceeds the temporary allocation budget"
        )));
    }
    let bytes = ResourceManager::read(version, cancelled)?;
    if bytes.len() as u64 > MAX_TEMPORARY_BYTES {
        return Err(size_limit_error(format!(
            "source {label} exceeds the temporary allocation budget"
        )));
    }
    Ok(bytes)
}

fn push_missing_model(diagnostics: &mut DiagnosticCollector, key: &ResourceKey) {
    diagnostics.push(
        MigrationDiagnosticSeverity::Warning,
        MigrationStatus::Missing,
        MigrationPhase::Models,
        "MDL_RESOURCE_NOT_FOUND",
        format!("Le modèle {key} n'est pas résolu par le Resource Manager."),
        Some(key.to_string()),
        None,
    );
}

fn push_texture_rejected(
    diagnostics: &mut DiagnosticCollector,
    key: &ResourceKey,
    code: &str,
    message: &str,
) {
    diagnostics.push(
        MigrationDiagnosticSeverity::Warning,
        if matches!(
            code,
            "MIGRATION_TEXTURE_DECODE_LIMIT" | "MIGRATION_BUNDLE_LIMIT_EXCEEDED"
        ) {
            MigrationStatus::Unsupported
        } else {
            MigrationStatus::Manual
        },
        MigrationPhase::Textures,
        code,
        message,
        Some(key.to_string()),
        None,
    );
}

pub(crate) fn provenance(
    resource: &ResolvedResource,
    purpose: &str,
    selected_content_sha256: Option<String>,
) -> ResourceProvenance {
    let mut selected = crate::model::MigrationResourceVersion::sanitized(&resource.selected);
    if selected_content_sha256.is_some() {
        selected.content_sha256 = selected_content_sha256;
    }
    ResourceProvenance {
        resource_key: resource.key.to_string(),
        selected,
        shadowed: resource
            .shadowed
            .iter()
            .map(crate::model::MigrationResourceVersion::sanitized)
            .collect(),
        purpose: purpose.to_owned(),
    }
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn normalize_asset_resref(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim_matches('"')
        .to_ascii_lowercase();
    if normalized.is_empty() || normalized == "null" {
        return None;
    }
    let stem = normalized
        .rsplit_once('.')
        .map_or(normalized.as_str(), |(stem, _)| stem)
        .trim();
    (!stem.is_empty()).then(|| stem.to_owned())
}

fn check_cancelled(cancelled: &AtomicBool, resource: &str) -> AppResult<()> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(Box::new(AppError::job_cancelled(resource)));
    }
    Ok(())
}

pub(crate) fn size_limit_error(detail: impl Into<String>) -> Box<AppError> {
    Box::new(
        AppError::new(
            "MIGRATION_BUNDLE_LIMIT_EXCEEDED",
            "Le bundle dépasse les limites de sécurité.",
            detail,
            ErrorSeverity::Error,
        )
        .with_import_stage("area_migration"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_material_texture_names_without_paths_or_extensions() {
        assert_eq!(
            normalize_asset_resref("Textures\\Stone_A.TGA"),
            Some("stone_a".to_owned())
        );
        assert_eq!(normalize_asset_resref("null"), None);
        assert_eq!(normalize_asset_resref("  "), None);
    }

    #[test]
    fn cumulative_payload_budget_rejects_before_the_next_payload_is_written() {
        let mut budget = MaterializationBudget::default();
        budget
            .reserve_payload(crate::model::MAX_BUNDLE_BYTES as usize, "first")
            .expect("first reservation");
        assert!(budget.reserve_payload(1, "second").is_err());
    }
}
