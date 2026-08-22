use crate::coordinates::{canonical_position, canonical_transform};
use crate::diagnostics::DiagnosticCollector;
use crate::model::{
    AreaMigrationCandidate, AreaMigrationSource, IdentityEntry, IdentityMap, MigrationAreaDocument,
    MigrationAsset, MigrationDiagnosticSeverity, MigrationInstance, MigrationPhase,
    MigrationSpawnPoint, MigrationStatus, MigrationTile, SourceTransform, display_area_name,
};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_world::{AreaMap, SceneManifest};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub(crate) struct AreaExtraction {
    pub area: AreaMap,
    pub scene: SceneManifest,
    pub requested_models: BTreeSet<String>,
    pub navigation_requests: BTreeMap<String, BTreeSet<u16>>,
}

pub fn list_candidates(source: &AreaMigrationSource) -> Vec<AreaMigrationCandidate> {
    let mut candidates = source
        .world_index
        .areas
        .iter()
        .map(|area| AreaMigrationCandidate {
            resref: area.resref.clone(),
            name: display_area_name(area),
            width: area.width,
            height: area.height,
            tile_count: area.tiles.len(),
            instance_count: area.instances.len(),
            source_diagnostic_count: area.diagnostics.len(),
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.resref.cmp(&right.resref));
    candidates
}

pub(crate) fn extract_area(
    source: &AreaMigrationSource,
    area_resref: &str,
    diagnostics: &mut DiagnosticCollector,
) -> AppResult<AreaExtraction> {
    let requested = area_resref.trim().to_ascii_lowercase();
    if requested.is_empty()
        || requested.len() > 64
        || !requested
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-'))
    {
        return Err(Box::new(
            AppError::new(
                "MIGRATION_AREA_RESREF_INVALID",
                "Le ResRef de zone n'est pas valide.",
                format!("invalid area ResRef {area_resref:?}"),
                ErrorSeverity::Error,
            )
            .with_resource(area_resref)
            .with_import_stage("area_migration_audit"),
        ));
    }
    let area = source
        .world_index
        .areas
        .iter()
        .find(|area| area.resref.eq_ignore_ascii_case(&requested))
        .cloned()
        .ok_or_else(|| {
            Box::new(
                AppError::new(
                    "MIGRATION_AREA_NOT_FOUND",
                    "La zone sélectionnée n'existe pas dans l'analyse.",
                    format!("WorldIndex has no area matching {requested}"),
                    ErrorSeverity::Error,
                )
                .with_resource(requested.clone())
                .with_import_stage("area_migration_audit"),
            )
        })?;
    let scene = source
        .world_index
        .scenes
        .iter()
        .find(|scene| scene.area.eq_ignore_ascii_case(&requested))
        .cloned()
        .ok_or_else(|| {
            Box::new(
                AppError::new(
                    "MIGRATION_SCENE_NOT_FOUND",
                    "Le manifeste de scène de cette zone est absent.",
                    format!("WorldIndex has no scene matching {requested}"),
                    ErrorSeverity::Error,
                )
                .with_resource(requested.clone())
                .with_import_stage("area_migration_audit"),
            )
        })?;

    diagnostics.extend_world(&area.diagnostics);
    diagnostics.extend_world(&scene.diagnostics);
    if area.width == 0 || area.height == 0 {
        diagnostics.push(
            MigrationDiagnosticSeverity::Error,
            MigrationStatus::Missing,
            MigrationPhase::Audit,
            "MIGRATION_AREA_DIMENSIONS_INVALID",
            "La zone ne possède pas de dimensions exportables.",
            Some(area.resref.clone()),
            None,
        );
    }
    let expected_tiles = u64::from(area.width) * u64::from(area.height);
    if usize::try_from(expected_tiles).ok() != Some(area.tiles.len()) {
        diagnostics.push(
            MigrationDiagnosticSeverity::Error,
            MigrationStatus::Missing,
            MigrationPhase::Audit,
            "MIGRATION_TILE_COUNT_MISMATCH",
            format!(
                "La grille {}×{} exige {expected_tiles} tuiles mais l'analyse en contient {}.",
                area.width,
                area.height,
                area.tiles.len()
            ),
            Some(area.are_source.clone()),
            None,
        );
    }
    diagnostics.push(
        MigrationDiagnosticSeverity::Info,
        MigrationStatus::Manual,
        MigrationPhase::Audit,
        "MIGRATION_AREA_KIND_UNSPECIFIED",
        "Le modèle métier courant ne qualifie pas encore la zone comme intérieure ou extérieure ; areaKind reste unknown.",
        Some(area.are_source.clone()),
        None,
    );

    let mut requested_models = BTreeSet::new();
    let mut navigation_requests = BTreeMap::<String, BTreeSet<u16>>::new();
    for object in scene.objects.iter().chain(&scene.overlays) {
        for model in &object.model_resrefs {
            let model = normalize_resref(model);
            if model.is_empty() {
                continue;
            }
            requested_models.insert(model.clone());
            let navigation_type = match object.kind.as_str() {
                "tile" => Some(2016),
                "door" => Some(2052),
                "placeable" => Some(2053),
                _ => None,
            };
            if let Some(resource_type) = navigation_type {
                navigation_requests
                    .entry(model)
                    .or_default()
                    .insert(resource_type);
            }
        }
    }
    Ok(AreaExtraction {
        area,
        scene,
        requested_models,
        navigation_requests,
    })
}

pub(crate) fn assemble_area_document(
    source: &AreaMigrationSource,
    extraction: &AreaExtraction,
    model_path_by_resref: &BTreeMap<String, String>,
    navigation_paths_by_model: &BTreeMap<String, Vec<String>>,
    mut assets: Vec<MigrationAsset>,
    diagnostics: &mut DiagnosticCollector,
) -> (MigrationAreaDocument, IdentityMap) {
    let object_by_id = extraction
        .scene
        .objects
        .iter()
        .chain(&extraction.scene.overlays)
        .map(|object| (object.id.as_str(), object))
        .collect::<BTreeMap<_, _>>();
    let mut identities = Vec::new();
    let mut tiles = Vec::with_capacity(extraction.area.tiles.len());
    for tile in &extraction.area.tiles {
        let source_identity = format!("tile:{}:{}", tile.x, tile.y);
        let id = stable_id(
            &source.module_sha256,
            &extraction.area.resref,
            "tile",
            &tile.tile_id.to_string(),
            &source_identity,
        );
        let object = object_by_id.get(source_identity.as_str()).copied();
        let model_resref = object.and_then(|value| value.model_resref.clone());
        let model_asset = model_resref
            .as_deref()
            .map(normalize_resref)
            .and_then(|resref| model_path_by_resref.get(&resref).cloned());
        let status = if model_asset.is_some() {
            MigrationStatus::Converted
        } else {
            MigrationStatus::Missing
        };
        if status == MigrationStatus::Missing {
            let (code, message, resource) = match model_resref.as_deref() {
                Some(resref) => (
                    "MIGRATION_TILE_MODEL_NOT_EXPORTED",
                    format!(
                        "Le modèle de tuile {resref} n'a pas pu être exporté en GLB ; la tuile reste explicitement sans asset."
                    ),
                    format!("{}.mdl", normalize_resref(resref)),
                ),
                None => (
                    "MIGRATION_TILE_MODEL_UNRESOLVED",
                    format!(
                        "Le SET ne fournit aucun modèle résolu pour la tuile {} en ({}, {}).",
                        tile.tile_id, tile.x, tile.y
                    ),
                    extraction
                        .area
                        .tileset
                        .as_deref()
                        .map(|tileset| format!("{tileset}.set"))
                        .unwrap_or_else(|| extraction.area.resref.clone()),
                ),
            };
            diagnostics.push(
                MigrationDiagnosticSeverity::Warning,
                MigrationStatus::Missing,
                MigrationPhase::Models,
                code,
                message,
                Some(resource),
                Some(id.clone()),
            );
        }
        let source_position = [tile.x as f32 * 10.0 + 5.0, tile.y as f32 * 10.0 + 5.0, 0.0];
        let yaw = (tile.orientation % 4) as f32 * std::f32::consts::FRAC_PI_2;
        let transform = canonical_transform(source_position, yaw)
            .expect("bounded tile coordinates and orientation are finite");
        identities.push(IdentityEntry {
            stable_id: id.clone(),
            source_kind: "tile".to_owned(),
            resource_key: model_resref
                .as_deref()
                .map(|value| format!("{}.mdl", normalize_resref(value)))
                .unwrap_or_default(),
            instance_identity: source_identity,
        });
        tiles.push(MigrationTile {
            id,
            source: tile.clone(),
            source_transform: SourceTransform::from_values(source_position, yaw),
            transform,
            model_resref,
            model_asset,
            status,
        });
    }

    let mut instances = Vec::with_capacity(extraction.area.instances.len());
    for instance in &extraction.area.instances {
        let source_resource = portable_instance_source(&extraction.area.resref, instance);
        let resource_resref = instance.template_resref.as_deref().unwrap_or("");
        let id = stable_id(
            &source.module_sha256,
            &extraction.area.resref,
            &instance.category,
            resource_resref,
            &instance.id,
        );
        let object = object_by_id.get(instance.id.as_str()).copied();
        let model_resrefs = object
            .map(|value| value.model_resrefs.clone())
            .unwrap_or_default();
        let model_assets = model_resrefs
            .iter()
            .filter_map(|resref| model_path_by_resref.get(&normalize_resref(resref)).cloned())
            .collect::<Vec<_>>();
        let source_position = [instance.x, instance.y, instance.z];
        let (source_yaw, orientation_diagnostic) = instance_yaw(instance);
        if let Some((code, message)) = orientation_diagnostic {
            diagnostics.push(
                MigrationDiagnosticSeverity::Warning,
                MigrationStatus::Manual,
                MigrationPhase::Audit,
                code,
                message,
                Some(source_resource.clone()),
                Some(id.clone()),
            );
        }
        let transform = source_yaw.and_then(|yaw| canonical_transform(source_position, yaw));
        if transform.is_none() {
            diagnostics.push(
                MigrationDiagnosticSeverity::Error,
                MigrationStatus::Missing,
                MigrationPhase::Audit,
                "MIGRATION_INSTANCE_TRANSFORM_INVALID",
                "L'instance possède une position ou rotation non finie.",
                Some(source_resource.clone()),
                Some(id.clone()),
            );
        }
        let canonical_geometry = instance
            .geometry
            .iter()
            .filter_map(|point| canonical_position([point.x, point.y, point.z]))
            .collect::<Vec<_>>();
        if canonical_geometry.len() != instance.geometry.len() {
            diagnostics.push(
                MigrationDiagnosticSeverity::Error,
                MigrationStatus::Missing,
                MigrationPhase::Audit,
                "MIGRATION_GEOMETRY_COORDINATE_INVALID",
                "Au moins un point de géométrie n'est pas fini.",
                Some(source_resource.clone()),
                Some(id.clone()),
            );
        }
        let canonical_spawn_points = instance
            .spawn_points
            .iter()
            .filter_map(|point| {
                canonical_transform([point.x, point.y, point.z], point.orientation).map(
                    |transform| MigrationSpawnPoint {
                        source: point.clone(),
                        transform,
                    },
                )
            })
            .collect::<Vec<_>>();
        if canonical_spawn_points.len() != instance.spawn_points.len() {
            diagnostics.push(
                MigrationDiagnosticSeverity::Error,
                MigrationStatus::Missing,
                MigrationPhase::Audit,
                "MIGRATION_SPAWN_COORDINATE_INVALID",
                "Au moins un point d'apparition n'est pas fini.",
                Some(source_resource.clone()),
                Some(id.clone()),
            );
        }
        let (classification, status, status_reason) = classify_instance(instance, &model_assets);
        diagnostics.push(
            if status == MigrationStatus::Unsupported {
                MigrationDiagnosticSeverity::Warning
            } else {
                MigrationDiagnosticSeverity::Info
            },
            status,
            MigrationPhase::Audit,
            format!("MIGRATION_INSTANCE_{}", status_code(status)),
            status_reason.clone(),
            Some(source_resource.clone()),
            Some(id.clone()),
        );
        identities.push(IdentityEntry {
            stable_id: id.clone(),
            source_kind: instance.category.clone(),
            resource_key: instance
                .template_resref
                .clone()
                .unwrap_or_else(|| source_resource.clone()),
            instance_identity: instance.id.clone(),
        });
        instances.push(MigrationInstance {
            id,
            source_identity: instance.id.clone(),
            source: {
                let mut source = instance.clone();
                source.source_path = source_resource;
                source
            },
            source_transform: transform
                .as_ref()
                .zip(source_yaw)
                .map(|(_, yaw)| SourceTransform::from_values(source_position, yaw)),
            transform,
            canonical_geometry,
            canonical_spawn_points,
            model_resrefs,
            model_assets,
            classification,
            status,
            status_reason,
        });
    }

    for asset in &mut assets {
        if asset.kind == "model" {
            asset.navigation_paths = asset
                .resource_keys
                .iter()
                .filter_map(|key| key.strip_suffix(".mdl"))
                .flat_map(|model_resref| {
                    navigation_paths_by_model
                        .get(model_resref)
                        .into_iter()
                        .flatten()
                        .cloned()
                })
                .collect();
            asset.navigation_paths.sort();
            asset.navigation_paths.dedup();
        }
        asset.id = stable_id(
            &source.module_sha256,
            &extraction.area.resref,
            "asset",
            &asset.sha256,
            &asset.resource_keys.join("|"),
        );
        identities.push(IdentityEntry {
            stable_id: asset.id.clone(),
            source_kind: asset.kind.clone(),
            resource_key: asset.resource_keys.join("|"),
            instance_identity: asset.path.clone(),
        });
    }
    identities.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    assets.sort_by(|left, right| left.path.cmp(&right.path));
    // Keep only logical resource names in the portable bundle. The full local paths remain
    // available to the in-memory analysis but must not leak through area.json.
    let mut source_files = vec![format!("{}.are", extraction.area.resref)];
    if extraction.area.git_source.is_some() {
        source_files.push(format!("{}.git", extraction.area.resref));
    }
    if extraction.area.gic_source.is_some() {
        source_files.push(format!("{}.gic", extraction.area.resref));
    }
    source_files.sort();
    source_files.dedup();
    (
        MigrationAreaDocument {
            schema_version: crate::model::BUNDLE_SCHEMA_VERSION.to_owned(),
            resref: extraction.area.resref.clone(),
            name: display_area_name(&extraction.area),
            area_kind: "unknown".to_owned(),
            dimensions: [extraction.area.width, extraction.area.height],
            grid_size_meters: 10.0,
            tileset: extraction.area.tileset.clone(),
            tiles,
            instances,
            assets,
            source_files,
        },
        IdentityMap {
            schema_version: crate::model::BUNDLE_SCHEMA_VERSION.to_owned(),
            module_sha256: source.module_sha256.to_ascii_lowercase(),
            area_resref: extraction.area.resref.clone(),
            entries: identities,
        },
    )
}

pub(crate) fn classify_instance(
    instance: &aurora_world::AreaInstance,
    model_assets: &[String],
) -> (String, MigrationStatus, String) {
    match instance.category.as_str() {
        "door" => (
            "door-dynamic".to_owned(),
            if model_assets.is_empty() {
                MigrationStatus::Missing
            } else {
                MigrationStatus::Placeholder
            },
            if model_assets.is_empty() {
                "Porte conservée comme placeholder ; son modèle et son comportement ne sont pas convertis."
            } else {
                "Géométrie de porte convertie ; scripts, animation et transition restent des placeholders."
            }
            .to_owned(),
        ),
        "placeable" => (
            "placeable-dynamic".to_owned(),
            if model_assets.is_empty() {
                MigrationStatus::Missing
            } else {
                MigrationStatus::Placeholder
            },
            "Plaçable conservé séparément ; inventaire, scripts et comportement ne sont pas convertis."
                .to_owned(),
        ),
        "creature" => (
            "creature-placeholder".to_owned(),
            if model_assets.is_empty() {
                MigrationStatus::Missing
            } else {
                MigrationStatus::Placeholder
            },
            "Créature conservée comme placeholder sans règles, IA ni animation garantie.".to_owned(),
        ),
        "trigger" => (
            "trigger-volume".to_owned(),
            if instance.geometry.is_empty() {
                MigrationStatus::Placeholder
            } else {
                MigrationStatus::Approximated
            },
            "Volume de trigger conservé ; scripts et logique de déclenchement demandent une reprise manuelle."
                .to_owned(),
        ),
        "encounter" => (
            "encounter-placeholder".to_owned(),
            MigrationStatus::Placeholder,
            "Rencontre et points d'apparition conservés sans règles de gameplay.".to_owned(),
        ),
        "sound" => (
            "sound-placeholder".to_owned(),
            MigrationStatus::Placeholder,
            "Émetteur sonore conservé sans conversion de l'asset audio ni de ses règles.".to_owned(),
        ),
        "waypoint" => (
            "waypoint-anchor".to_owned(),
            MigrationStatus::Exact,
            "Ancre spatiale conservée ; ses consommateurs NWScript ne sont pas convertis.".to_owned(),
        ),
        "store" | "item" => (
            format!("{}-placeholder", instance.category),
            MigrationStatus::Placeholder,
            "Identité et données source conservées sans conversion de l'économie ou de l'inventaire."
                .to_owned(),
        ),
        _ => (
            "instance-unsupported".to_owned(),
            MigrationStatus::Unsupported,
            "Catégorie d'instance inconnue ; données source conservées pour reprise manuelle.".to_owned(),
        ),
    }
}

fn status_code(status: MigrationStatus) -> &'static str {
    match status {
        MigrationStatus::Exact => "EXACT",
        MigrationStatus::Converted => "CONVERTED",
        MigrationStatus::Approximated => "APPROXIMATED",
        MigrationStatus::Placeholder => "PLACEHOLDER",
        MigrationStatus::Manual => "MANUAL",
        MigrationStatus::Unsupported => "UNSUPPORTED",
        MigrationStatus::Missing => "MISSING",
        MigrationStatus::LicenseBlocked => "LICENSE_BLOCKED",
    }
}

fn normalize_resref(value: &str) -> String {
    let normalized = value.trim().trim_matches('"').to_ascii_lowercase();
    normalized
        .strip_suffix(".mdl")
        .unwrap_or(&normalized)
        .to_owned()
}

fn instance_yaw(
    instance: &aurora_world::AreaInstance,
) -> (Option<f32>, Option<(&'static str, &'static str)>) {
    if let Some(bearing) = instance.bearing {
        return if bearing.is_finite() {
            (Some(bearing), None)
        } else {
            (
                None,
                Some((
                    "MIGRATION_INSTANCE_BEARING_INVALID",
                    "Bearing est non fini et ne peut pas produire une rotation canonique.",
                )),
            )
        };
    }
    match (instance.x_orientation, instance.y_orientation) {
        (None, None) => (Some(0.0), None),
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() && x.hypot(y) > f32::EPSILON => {
            let yaw = y.atan2(x);
            if yaw.is_finite() {
                (Some(yaw), None)
            } else {
                (
                    None,
                    Some((
                        "MIGRATION_INSTANCE_ORIENTATION_INVALID",
                        "Le vecteur XOrientation/YOrientation produit une rotation non finie.",
                    )),
                )
            }
        }
        (Some(x), Some(y)) if !x.is_finite() || !y.is_finite() => (
            None,
            Some((
                "MIGRATION_INSTANCE_ORIENTATION_INVALID",
                "XOrientation ou YOrientation est non fini.",
            )),
        ),
        _ => (
            None,
            Some((
                "MIGRATION_INSTANCE_ORIENTATION_INVALID",
                "XOrientation et YOrientation doivent former un vecteur non nul.",
            )),
        ),
    }
}

fn portable_instance_source(area_resref: &str, instance: &aurora_world::AreaInstance) -> String {
    let normalized = instance.source_path.replace('\\', "/");
    let fallback_container = format!("{area_resref}.git");
    let (container, locator) = normalized
        .rsplit_once("::")
        .filter(|(_, locator)| !locator.is_empty())
        .unwrap_or((fallback_container.as_str(), instance.id.as_str()));
    let container_name = container.rsplit('/').next().unwrap_or(container);
    format!("{container_name}::{locator}")
}

pub fn stable_id(
    module_sha256: &str,
    area_resref: &str,
    resource_type: &str,
    resource_resref: &str,
    instance_identity: &str,
) -> String {
    let normalized = [
        crate::model::BUNDLE_SCHEMA_VERSION.to_owned(),
        module_sha256.to_ascii_lowercase(),
        area_resref.to_ascii_lowercase(),
        resource_type.to_ascii_lowercase(),
        resource_resref.to_ascii_lowercase(),
        instance_identity.to_owned(),
    ];
    let mut hasher = Sha256::new();
    for part in normalized {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    let digest = format!("{:x}", hasher.finalize());
    format!("amv1-{}", &digest[..32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_ids_depend_on_source_identity_not_output_path() {
        let first = stable_id("ABC", "AREA", "door", "door01", "Door List[4]");
        let second = stable_id("abc", "area", "DOOR", "DOOR01", "Door List[4]");
        let other = stable_id("abc", "area", "door", "door01", "Door List[5]");
        assert_eq!(first, second);
        assert_ne!(first, other);
        assert!(first.starts_with("amv1-"));
    }

    #[test]
    fn stable_ids_use_unambiguous_length_prefixed_parts() {
        let left = stable_id("abc", "area", "door", "a\nb", "c");
        let right = stable_id("abc", "area", "door", "a", "b\nc");
        assert_ne!(left, right);
    }

    fn orientation_instance(
        bearing: Option<f32>,
        x_orientation: Option<f32>,
        y_orientation: Option<f32>,
    ) -> aurora_world::AreaInstance {
        aurora_world::AreaInstance {
            id: "area:Creature List:0".to_owned(),
            category: "creature".to_owned(),
            tag: None,
            template_resref: None,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            bearing,
            x_orientation,
            y_orientation,
            appearance: None,
            transition_destination: None,
            transition_flags: None,
            load_screen_id: None,
            geometry: Vec::new(),
            spawn_points: Vec::new(),
            inventory: Vec::new(),
            source_path: r"C:\game\area_a.git::Creature List[0]".to_owned(),
        }
    }

    #[test]
    fn orientation_uses_bearing_then_atan2_vector_and_rejects_invalid_values() {
        let bearing = orientation_instance(Some(0.25), Some(0.0), Some(1.0));
        assert_eq!(instance_yaw(&bearing).0, Some(0.25));
        let vector = orientation_instance(None, Some(0.0), Some(1.0));
        assert!(
            (instance_yaw(&vector).0.expect("vector yaw") - std::f32::consts::FRAC_PI_2).abs()
                < f32::EPSILON
        );
        let invalid = orientation_instance(None, Some(0.0), Some(0.0));
        assert!(instance_yaw(&invalid).0.is_none());
        assert_eq!(
            portable_instance_source("area_a", &vector),
            "area_a.git::Creature List[0]"
        );
    }

    #[test]
    fn required_model_instance_without_an_asset_is_missing() {
        let instance = orientation_instance(None, None, None);
        assert_eq!(
            classify_instance(&instance, &[]).1,
            MigrationStatus::Missing
        );
        assert_eq!(
            classify_instance(&instance, &["assets/models/creature.glb".to_owned()]).1,
            MigrationStatus::Placeholder
        );
    }
}
