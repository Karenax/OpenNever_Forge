use aurora_core::{AppError, AppResult, ErrorSeverity, ResourceKey};
use aurora_mdl::{
    GLB_CACHE_SCHEMA_VERSION, GlbArtifact, MdlDiagnostic, MdlModel, export_glb, parse_mdl,
};
use aurora_resource::{ResourceCatalog, ResourceManager};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Debug)]
pub struct ModelCacheEntry {
    pub artifact: GlbArtifact,
    pub cache_path: PathBuf,
    pub cache_hit: bool,
}

#[derive(Debug)]
pub struct ResolvedModelExport {
    pub model: MdlModel,
    /// Normalized MDL ResRefs whose selected bytes participated in resolution, including the
    /// requested model, supermodels, and recursively expanded reference models.
    pub resource_resrefs: Vec<String>,
}

#[derive(Debug)]
pub struct PreparedModelPreview {
    pub resref: String,
    pub cache_path: Option<PathBuf>,
    pub cache_hit: bool,
    pub byte_length: usize,
    pub error: Option<AppError>,
}

pub fn prepare_model_previews(
    catalog: &ResourceCatalog,
    resrefs: &[String],
    cache_root: &Path,
    max_workers: usize,
    cancelled: &AtomicBool,
) -> Vec<PreparedModelPreview> {
    if resrefs.is_empty() {
        return Vec::new();
    }
    let next = AtomicUsize::new(0);
    let results = Mutex::new(
        (0..resrefs.len())
            .map(|_| None)
            .collect::<Vec<Option<PreparedModelPreview>>>(),
    );
    let workers = max_workers.clamp(1, 8).min(resrefs.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    if cancelled.load(Ordering::Relaxed) {
                        break;
                    }
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(resref) = resrefs.get(index) else {
                        break;
                    };
                    let prepared =
                        match cached_model_preview(catalog, resref, cache_root, cancelled) {
                            Ok(entry) => PreparedModelPreview {
                                resref: resref.clone(),
                                cache_path: Some(entry.cache_path),
                                cache_hit: entry.cache_hit,
                                byte_length: entry.artifact.byte_length,
                                error: None,
                            },
                            Err(error) => PreparedModelPreview {
                                resref: resref.clone(),
                                cache_path: None,
                                cache_hit: false,
                                byte_length: 0,
                                error: Some(*error),
                            },
                        };
                    results.lock().expect("model preparation poisoned")[index] = Some(prepared);
                }
            });
        }
    });
    results
        .into_inner()
        .expect("model preparation poisoned")
        .into_iter()
        .flatten()
        .collect()
}

pub fn build_model_preview(
    catalog: &ResourceCatalog,
    resref: &str,
    cancelled: &AtomicBool,
) -> AppResult<GlbArtifact> {
    let model = resolve_model(catalog, resref, cancelled)?;
    export_model(&model, resref)
}

/// Resolves a model together with its supermodels and referenced models without writing a cache.
/// Migration/export callers can then choose their own deterministic GLB texture URI mapping.
pub fn resolve_model_for_export(
    catalog: &ResourceCatalog,
    resref: &str,
    cancelled: &AtomicBool,
) -> AppResult<MdlModel> {
    resolve_model(catalog, resref, cancelled)
}

pub fn resolve_model_for_export_with_dependencies(
    catalog: &ResourceCatalog,
    resref: &str,
    cancelled: &AtomicBool,
) -> AppResult<ResolvedModelExport> {
    resolve_model_with_dependencies(catalog, resref, cancelled)
}

pub fn cached_model_preview(
    catalog: &ResourceCatalog,
    resref: &str,
    cache_root: &Path,
    cancelled: &AtomicBool,
) -> AppResult<ModelCacheEntry> {
    let key = ResourceKey::new(resref, 2002);
    let model = resolve_model(catalog, resref, cancelled)?;
    let source_sha256 = model.source_sha256.clone();
    let version_root = cache_root.join(format!("glb-v{GLB_CACHE_SCHEMA_VERSION}"));
    fs::create_dir_all(&version_root).map_err(|error| {
        Box::new(AppError::io(
            "create GLB cache",
            version_root.display().to_string(),
            &error,
        ))
    })?;
    let cache_path = version_root.join(format!("{source_sha256}.glb"));
    let metadata_path = version_root.join(format!("{source_sha256}.json"));
    if let Ok(metadata) = fs::read(&metadata_path)
        && let Ok(mut artifact) = serde_json::from_slice::<GlbArtifact>(&metadata)
        && artifact.schema_version == GLB_CACHE_SCHEMA_VERSION
        && artifact.source_sha256 == source_sha256
        && let Ok(bytes) = fs::read(&cache_path)
        && format!("{:x}", Sha256::digest(&bytes)) == artifact.glb_sha256
    {
        artifact.byte_length = bytes.len();
        artifact.bytes = bytes;
        return Ok(ModelCacheEntry {
            artifact,
            cache_path,
            cache_hit: true,
        });
    }

    let artifact = export_model(&model, resref)?;
    atomic_write(&cache_path, &artifact.bytes)?;
    let metadata = serde_json::to_vec_pretty(&artifact).map_err(|error| {
        Box::new(
            AppError::new(
                "GLB_CACHE_METADATA_FAILED",
                "Les métadonnées du cache 3D n'ont pas pu être enregistrées.",
                error.to_string(),
                ErrorSeverity::Error,
            )
            .with_resource(key.to_string()),
        )
    })?;
    atomic_write(&metadata_path, &metadata)?;
    Ok(ModelCacheEntry {
        artifact,
        cache_path,
        cache_hit: false,
    })
}

fn resolve_model(
    catalog: &ResourceCatalog,
    resref: &str,
    cancelled: &AtomicBool,
) -> AppResult<MdlModel> {
    Ok(resolve_model_with_dependencies(catalog, resref, cancelled)?.model)
}

fn resolve_model_with_dependencies(
    catalog: &ResourceCatalog,
    resref: &str,
    cancelled: &AtomicBool,
) -> AppResult<ResolvedModelExport> {
    let mut current = resref.to_ascii_lowercase();
    let mut visited = BTreeSet::new();
    let mut hashed_dependencies = BTreeSet::new();
    let mut dependency_hash = Sha256::new();
    let mut base = None::<MdlModel>;
    let mut inherited_animations = Vec::new();
    for depth in 0..16 {
        if !visited.insert(current.clone()) {
            if let Some(model) = &mut base {
                model.diagnostics.push(MdlDiagnostic {
                    code: "MDL_SUPERMODEL_CYCLE".to_owned(),
                    message: format!("supermodel cycle stops at {current}"),
                    offset: None,
                    node: None,
                });
            }
            break;
        }
        let key = ResourceKey::new(&current, 2002);
        let Some(resource) = catalog.get(&key) else {
            if depth == 0 {
                return Err(Box::new(
                    AppError::new(
                        "MDL_RESOURCE_NOT_FOUND",
                        "Le modèle demandé n'est pas disponible.",
                        format!("Resource Manager did not resolve {key}"),
                        ErrorSeverity::Warning,
                    )
                    .with_resource(key.to_string()),
                ));
            }
            if let Some(model) = &mut base {
                model.diagnostics.push(MdlDiagnostic {
                    code: "MDL_SUPERMODEL_MISSING".to_owned(),
                    message: format!("supermodel {current} is not resolved"),
                    offset: None,
                    node: None,
                });
            }
            break;
        };
        let bytes = ResourceManager::read(&resource.selected, cancelled)?;
        hash_dependency(
            &mut dependency_hash,
            &mut hashed_dependencies,
            &current,
            &bytes,
        );
        let parsed = parse_mdl(&bytes).map_err(|error| {
            Box::new(
                AppError::new(
                    error.code,
                    "Le modèle NWN ne peut pas être converti en aperçu 3D.",
                    error.message,
                    ErrorSeverity::Error,
                )
                .with_resource(key.to_string())
                .with_source(resource.selected.source_path.clone())
                .with_import_stage("mdl_parse"),
            )
        })?;
        let next = parsed.supermodel.clone();
        if let Some(model) = &mut base {
            inherited_animations.extend(parsed.animations);
            model.diagnostics.extend(parsed.diagnostics);
        } else {
            base = Some(parsed);
        }
        let Some(next) = next.filter(|value| !value.eq_ignore_ascii_case("null")) else {
            break;
        };
        current = next;
    }
    let mut model = base.ok_or_else(|| {
        Box::new(
            AppError::new(
                "MDL_RESOLUTION_EMPTY",
                "Le modèle demandé n'a produit aucune donnée.",
                format!("No model data resolved for {resref}"),
                ErrorSeverity::Error,
            )
            .with_resource(ResourceKey::new(resref, 2002).to_string()),
        )
    })?;
    let mut animation_names = model
        .animations
        .iter()
        .map(|animation| animation.name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    for animation in inherited_animations {
        if animation_names.insert(animation.name.to_ascii_lowercase()) {
            model.animations.push(animation);
        }
    }
    model
        .animations
        .sort_by(|left, right| left.name.cmp(&right.name));
    expand_reference_models(
        catalog,
        &mut model,
        resref,
        cancelled,
        &mut dependency_hash,
        &mut hashed_dependencies,
    )?;
    model.source_sha256 = format!("{:x}", dependency_hash.finalize());
    Ok(ResolvedModelExport {
        model,
        resource_resrefs: hashed_dependencies.into_iter().collect(),
    })
}

fn expand_reference_models(
    catalog: &ResourceCatalog,
    model: &mut MdlModel,
    base_resref: &str,
    cancelled: &AtomicBool,
    dependency_hash: &mut Sha256,
    hashed_dependencies: &mut BTreeSet<String>,
) -> AppResult<()> {
    let mut queue = VecDeque::new();
    for (index, node) in model.nodes.iter().enumerate() {
        if let Some(reference) = &node.reference_model {
            queue.push_back((
                index,
                reference.clone(),
                0_usize,
                BTreeSet::from([base_resref.to_ascii_lowercase()]),
            ));
        }
    }
    while let Some((parent, reference, depth, mut ancestry)) = queue.pop_front() {
        if depth >= 16 {
            model.diagnostics.push(MdlDiagnostic {
                code: "MDL_REFERENCE_DEPTH_EXCEEDED".to_owned(),
                message: format!("reference expansion stops before {reference}"),
                offset: None,
                node: model.nodes.get(parent).map(|node| node.name.clone()),
            });
            continue;
        }
        let reference = reference.to_ascii_lowercase();
        if !ancestry.insert(reference.clone()) {
            model.diagnostics.push(MdlDiagnostic {
                code: "MDL_REFERENCE_CYCLE".to_owned(),
                message: format!("reference cycle stops at {reference}"),
                offset: None,
                node: model.nodes.get(parent).map(|node| node.name.clone()),
            });
            continue;
        }
        let key = ResourceKey::new(&reference, 2002);
        let Some(resource) = catalog.get(&key) else {
            model.diagnostics.push(MdlDiagnostic {
                code: "MDL_REFERENCE_MISSING".to_owned(),
                message: format!("referenced model {reference} is not resolved"),
                offset: None,
                node: model.nodes.get(parent).map(|node| node.name.clone()),
            });
            continue;
        };
        let bytes = ResourceManager::read(&resource.selected, cancelled)?;
        hash_dependency(dependency_hash, hashed_dependencies, &reference, &bytes);
        let referenced = match parse_mdl(&bytes) {
            Ok(value) => value,
            Err(error) => {
                model.diagnostics.push(MdlDiagnostic {
                    code: error.code,
                    message: format!("referenced model {reference}: {}", error.message),
                    offset: error.offset,
                    node: model.nodes.get(parent).map(|node| node.name.clone()),
                });
                continue;
            }
        };
        if model.nodes.len().saturating_add(referenced.nodes.len()) > 65_536 {
            model.diagnostics.push(MdlDiagnostic {
                code: "MDL_EXPANDED_NODE_LIMIT_EXCEEDED".to_owned(),
                message: format!("referenced model {reference} would exceed 65536 nodes"),
                offset: None,
                node: model.nodes.get(parent).map(|node| node.name.clone()),
            });
            continue;
        }
        let base = model.nodes.len();
        let referenced_diagnostics = referenced.diagnostics;
        for mut node in referenced.nodes {
            node.parent = node.parent.map(|value| base + value).or(Some(parent));
            for child in &mut node.children {
                *child += base;
            }
            model.nodes.push(node);
        }
        let roots = (base..model.nodes.len())
            .filter(|index| model.nodes[*index].parent == Some(parent))
            .collect::<Vec<_>>();
        if let Some(parent_node) = model.nodes.get_mut(parent) {
            parent_node.children.extend(roots);
            parent_node.children.sort_unstable();
            parent_node.children.dedup();
        }
        for index in base..model.nodes.len() {
            if let Some(child_reference) = model.nodes[index].reference_model.clone() {
                queue.push_back((index, child_reference, depth + 1, ancestry.clone()));
            }
        }
        model.diagnostics.extend(referenced_diagnostics);
    }
    Ok(())
}

fn hash_dependency(hasher: &mut Sha256, hashed: &mut BTreeSet<String>, resref: &str, bytes: &[u8]) {
    if !hashed.insert(resref.to_ascii_lowercase()) {
        return;
    }
    hasher.update((resref.len() as u64).to_le_bytes());
    hasher.update(resref.as_bytes());
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn export_model(model: &MdlModel, resref: &str) -> AppResult<GlbArtifact> {
    export_glb(model).map_err(|error| {
        Box::new(
            AppError::new(
                error.code,
                "L'aperçu GLB du modèle n'a pas pu être produit.",
                error.message,
                ErrorSeverity::Error,
            )
            .with_resource(ResourceKey::new(resref, 2002).to_string())
            .with_import_stage("glb_export"),
        )
    })
}

fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        Box::new(AppError::invalid_path(
            path.display().to_string(),
            "cache target has no parent directory",
        ))
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        Box::new(AppError::io(
            "create temporary cache file",
            parent.display().to_string(),
            &error,
        ))
    })?;
    temporary.write_all(bytes).map_err(|error| {
        Box::new(AppError::io(
            "write temporary cache file",
            path.display().to_string(),
            &error,
        ))
    })?;
    temporary.as_file().sync_all().map_err(|error| {
        Box::new(AppError::io(
            "flush temporary cache file",
            path.display().to_string(),
            &error,
        ))
    })?;
    match temporary.persist(path) {
        Ok(_) => Ok(()),
        Err(_error) if path.is_file() => Ok(()),
        Err(error) => Err(Box::new(AppError::io(
            "persist cache file",
            path.display().to_string(),
            &error.error,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_resource::{
        ResolvedResource, ResourceLocation, ResourceSourceKind, ResourceVersion,
    };

    #[test]
    fn preview_reads_loose_model_without_modifying_it() {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("triangle.mdl");
        let source = b"newmodel triangle\nnode trimesh body\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n";
        fs::write(&path, source).expect("fixture");
        let catalog = ResourceCatalog {
            entries: vec![ResolvedResource {
                key: ResourceKey::new("triangle", 2002),
                selected: ResourceVersion {
                    key: ResourceKey::new("triangle", 2002),
                    source_kind: ResourceSourceKind::Development,
                    source_name: "fixture".to_owned(),
                    source_path: path.display().to_string(),
                    priority: 0,
                    offset: 0,
                    size: source.len() as u64,
                    sha256: None,
                    location: ResourceLocation::File {
                        path: path.display().to_string(),
                    },
                },
                shadowed: Vec::new(),
            }],
            version_count: 1,
            shadowed_count: 0,
            diagnostics: Vec::new(),
        };
        let before = fs::read(&path).expect("before");
        let artifact =
            build_model_preview(&catalog, "triangle", &AtomicBool::new(false)).expect("preview");
        let after = fs::read(&path).expect("after");
        assert_eq!(before, after);
        assert_eq!(&artifact.bytes[..4], b"glTF");

        let first = cached_model_preview(
            &catalog,
            "triangle",
            &root.path().join("cache"),
            &AtomicBool::new(false),
        )
        .expect("first cached preview");
        let second = cached_model_preview(
            &catalog,
            "triangle",
            &root.path().join("cache"),
            &AtomicBool::new(false),
        )
        .expect("second cached preview");
        assert!(!first.cache_hit);
        assert!(second.cache_hit);
        assert_eq!(first.artifact.bytes, second.artifact.bytes);
        assert!(first.cache_path.is_file());
        assert_eq!(before, fs::read(&path).expect("source remains immutable"));
    }

    #[test]
    fn cache_tracks_supermodel_and_reference_dependencies() {
        let root = tempfile::tempdir().expect("tempdir");
        let base = root.path().join("base.mdl");
        let supermodel = root.path().join("super.mdl");
        let part = root.path().join("part.mdl");
        fs::write(
            &base,
            b"newmodel base\nsetsupermodel base super\nnode reference attachment\nrefmodel part\nendnode\n",
        )
        .expect("base");
        fs::write(
            &supermodel,
            b"newmodel super\nnewanim idle\ndoneanim idle\n",
        )
        .expect("supermodel");
        fs::write(
            &part,
            b"newmodel part\nnode trimesh body\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n",
        )
        .expect("part");
        let catalog = ResourceCatalog {
            entries: vec![
                loose_model("base", &base),
                loose_model("part", &part),
                loose_model("super", &supermodel),
            ],
            version_count: 3,
            shadowed_count: 0,
            diagnostics: Vec::new(),
        };
        let resolved =
            resolve_model_for_export_with_dependencies(&catalog, "base", &AtomicBool::new(false))
                .expect("resolved export closure");
        assert_eq!(resolved.resource_resrefs, ["base", "part", "super"]);
        assert!(resolved.model.nodes.len() >= 2);
        let cache = root.path().join("cache");
        let first = cached_model_preview(&catalog, "base", &cache, &AtomicBool::new(false))
            .expect("expanded preview");
        assert_eq!(first.artifact.mesh_count, 1);
        assert!(first.artifact.node_count >= 2);
        fs::write(
            &part,
            b"newmodel part\nnode trimesh body\nverts 3\n0 0 0\n2 0 0\n0 2 0\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n",
        )
        .expect("changed part");
        let second = cached_model_preview(&catalog, "base", &cache, &AtomicBool::new(false))
            .expect("invalidated preview");
        assert!(!second.cache_hit);
        assert_ne!(first.artifact.source_sha256, second.artifact.source_sha256);
        assert_ne!(first.cache_path, second.cache_path);
    }

    #[test]
    fn prepares_multiple_model_previews_and_reuses_their_cache() {
        let root = tempfile::tempdir().expect("tempdir");
        let mut entries = Vec::new();
        let mut resrefs = Vec::new();
        for index in 0..6 {
            let resref = format!("tile_{index}");
            let path = root.path().join(format!("{resref}.mdl"));
            fs::write(
                &path,
                format!("newmodel {resref}\nnode trimesh body\nverts 3\n0 0 0\n1 0 0\n0 1 0\nfaces 1\n0 1 2 0 0 1 2 0\nendnode\n"),
            )
            .expect("model");
            entries.push(loose_model(&resref, &path));
            resrefs.push(resref);
        }
        let catalog = ResourceCatalog {
            entries,
            version_count: 6,
            shadowed_count: 0,
            diagnostics: Vec::new(),
        };
        let cache = root.path().join("cache");

        let first = prepare_model_previews(&catalog, &resrefs, &cache, 4, &AtomicBool::new(false));
        let second = prepare_model_previews(&catalog, &resrefs, &cache, 4, &AtomicBool::new(false));

        assert_eq!(first.len(), 6);
        assert!(first.iter().all(|entry| entry.error.is_none()));
        assert!(first.iter().all(|entry| !entry.cache_hit));
        assert!(second.iter().all(|entry| entry.cache_hit));
        assert!(
            second
                .iter()
                .all(|entry| entry.cache_path.as_ref().is_some_and(|path| path.is_file()))
        );
    }

    fn loose_model(resref: &str, path: &Path) -> ResolvedResource {
        let key = ResourceKey::new(resref, 2002);
        ResolvedResource {
            key: key.clone(),
            selected: ResourceVersion {
                key,
                source_kind: ResourceSourceKind::Development,
                source_name: "fixture".to_owned(),
                source_path: path.display().to_string(),
                priority: 0,
                offset: 0,
                size: fs::metadata(path).expect("metadata").len(),
                sha256: None,
                location: ResourceLocation::File {
                    path: path.display().to_string(),
                },
            },
            shadowed: Vec::new(),
        }
    }
}
