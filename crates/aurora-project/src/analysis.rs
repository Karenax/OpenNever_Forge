use crate::{
    AnalysisPhase, DependencyRoots, DialogueIndex, DialogueIndexSummary, HashProgress,
    ModuleDependencyReport, ModuleFingerprint, ResourceCatalog, ResourceCatalogCacheSummary,
    ResourceCatalogSummary, ResourceManager, ResourceManagerConfig, ScriptIndex,
    ScriptIndexSummary, StructuredResourceSummary, WorldIndex, WorldSummary, analyze_dialogues,
    analyze_scripts, analyze_structured_resources, analyze_world, compare_dependency_reports,
    fingerprint_module_dependencies, hash_module_file, inspect_module_dependencies,
};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_erf::{
    ContainerInventory, ContainerReader, ContainerResource, ErfReader, ResourceTypeSummary,
};
use aurora_gff::{LocalizedString, LocalizedValue, ModuleInfo, parse_gff, read_module_info};
use aurora_resource::ResourceSourceKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleAnalysis {
    pub fingerprint: ModuleFingerprint,
    pub inventory: ContainerInventory,
    pub module_info: ModuleInfo,
    pub dependency_report: ModuleDependencyReport,
    #[serde(skip, default)]
    pub resource_catalog: ResourceCatalog,
    #[serde(default)]
    pub resource_catalog_summary: ResourceCatalogSummary,
    #[serde(default)]
    pub resource_catalog_cache: ResourceCatalogCacheSummary,
    #[serde(default)]
    pub structured_summary: StructuredResourceSummary,
    #[serde(skip, default)]
    pub script_index: ScriptIndex,
    #[serde(default)]
    pub script_index_summary: ScriptIndexSummary,
    #[serde(skip, default)]
    pub dialogue_index: DialogueIndex,
    #[serde(default)]
    pub dialogue_index_summary: DialogueIndexSummary,
    #[serde(skip, default)]
    pub world_index: WorldIndex,
    #[serde(default)]
    pub world_summary: WorldSummary,
}

pub fn analyze_module_file<F>(
    path: &Path,
    cancelled: &AtomicBool,
    on_progress: F,
) -> AppResult<ModuleAnalysis>
where
    F: FnMut(HashProgress),
{
    analyze_module_file_with_cache(
        path,
        &DependencyRoots::default(),
        None,
        cancelled,
        on_progress,
    )
}

pub fn analyze_module_file_with_roots<F>(
    path: &Path,
    roots: &DependencyRoots,
    cancelled: &AtomicBool,
    on_progress: F,
) -> AppResult<ModuleAnalysis>
where
    F: FnMut(HashProgress),
{
    analyze_module_file_with_cache(path, roots, None, cancelled, on_progress)
}

pub fn analyze_module_file_with_cache<F>(
    path: &Path,
    roots: &DependencyRoots,
    catalog_cache_path: Option<&Path>,
    cancelled: &AtomicBool,
    mut on_progress: F,
) -> AppResult<ModuleAnalysis>
where
    F: FnMut(HashProgress),
{
    let fingerprint = hash_module_file(path, cancelled, |progress| {
        on_progress(progress.scaled(0.0, 8.0))
    })?;
    on_progress(HashProgress::stage(AnalysisPhase::Inventory, 10.0));
    let reader = ErfReader::default();
    let inventory = reader.read_inventory(path, cancelled)?;
    let module_resources = inventory
        .resources
        .iter()
        .filter(|resource| {
            resource.key.resource_type == 2014 && resource.key.resref.eq_ignore_ascii_case("module")
        })
        .collect::<Vec<_>>();
    let module_resource = match module_resources.as_slice() {
        [resource] => *resource,
        [] => {
            return Err(module_info_error(
                path,
                "MODULE_IFO_NOT_FOUND",
                "No module.ifo resource exists in the selected container".to_owned(),
            ));
        }
        resources => {
            return Err(module_info_error(
                path,
                "MODULE_IFO_AMBIGUOUS",
                format!(
                    "Container has {} resources matching module.ifo",
                    resources.len()
                ),
            ));
        }
    };
    let module_bytes = reader.read_resource(path, module_resource, cancelled)?;
    let module_info = read_module_info(&module_bytes, &format!("{}::module.ifo", path.display()))?;
    on_progress(HashProgress::stage(AnalysisPhase::Dependencies, 18.0));
    let mut dependency_report = inspect_module_dependencies(&module_info, roots);
    fingerprint_module_dependencies(&mut dependency_report, cancelled)?;
    compare_dependency_reports(&mut dependency_report, None);
    let hak_paths = dependency_report
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind == crate::ModuleDependencyKind::Hak)
        .filter_map(|dependency| {
            dependency
                .selected_path
                .as_deref()
                .map(std::path::PathBuf::from)
        })
        .collect();
    on_progress(HashProgress::stage(AnalysisPhase::ResourceCatalog, 25.0));
    let resource_build = ResourceManager::build_with_cache(
        &ResourceManagerConfig {
            module_path: path.to_path_buf(),
            loose_source_directory: None,
            hak_paths,
            game_install_path: roots.game_install_path.clone(),
            user_data_path: roots.user_data_path.clone(),
        },
        catalog_cache_path,
        cancelled,
    )?;
    let resource_catalog = resource_build.catalog;
    let resource_catalog_cache = resource_build.cache;
    let resource_catalog_summary = resource_catalog.summary();
    on_progress(HashProgress::stage(
        AnalysisPhase::StructuredResources,
        57.0,
    ));
    let structured_summary = analyze_structured_resources(
        &resource_catalog,
        &module_info,
        &dependency_report,
        roots,
        cancelled,
    );
    on_progress(HashProgress::stage(AnalysisPhase::Scripts, 69.0));
    let script_index = analyze_scripts(&resource_catalog, cancelled);
    let script_index_summary = script_index.summary.clone();
    on_progress(HashProgress::stage(AnalysisPhase::Dialogues, 77.0));
    let dialogue_index = analyze_dialogues(&resource_catalog, &dependency_report, roots, cancelled);
    let dialogue_index_summary = dialogue_index.summary.clone();
    on_progress(HashProgress::stage(AnalysisPhase::World, 86.0));
    let world_index = analyze_world(
        &resource_catalog,
        &script_index,
        &dialogue_index,
        &dependency_report,
        roots,
        cancelled,
    );
    let world_summary = world_index.summary.clone();

    Ok(ModuleAnalysis {
        fingerprint,
        inventory,
        module_info,
        dependency_report,
        resource_catalog,
        resource_catalog_summary,
        resource_catalog_cache,
        structured_summary,
        script_index,
        script_index_summary,
        dialogue_index,
        dialogue_index_summary,
        world_index,
        world_summary,
    })
}

pub fn analyze_standalone_area_file_with_cache<F>(
    path: &Path,
    roots: &DependencyRoots,
    catalog_cache_path: Option<&Path>,
    cancelled: &AtomicBool,
    mut on_progress: F,
) -> AppResult<ModuleAnalysis>
where
    F: FnMut(HashProgress),
{
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("are"))
    {
        return Err(AppError::invalid_path(
            path.display().to_string(),
            "Standalone area analysis accepts only .are files",
        )
        .into());
    }
    let fingerprint = hash_module_file(path, cancelled, |progress| {
        on_progress(progress.scaled(0.0, 8.0))
    })?;
    let resref = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::invalid_path(
                path.display().to_string(),
                "Area filename has no valid ResRef",
            )
        })?
        .to_ascii_lowercase();
    let source_directory = path.parent().ok_or_else(|| {
        AppError::invalid_path(
            path.display().to_string(),
            "Area file has no parent directory",
        )
    })?;
    let module_info = standalone_area_info(&resref);
    let dependency_report = ModuleDependencyReport {
        dependencies: Vec::new(),
        resolved_count: 0,
        missing_count: 0,
        unchecked_count: 0,
        invalid_count: 0,
        changed_count: 0,
    };
    on_progress(HashProgress::stage(AnalysisPhase::ResourceCatalog, 25.0));
    let resource_build = ResourceManager::build_with_cache(
        &ResourceManagerConfig {
            module_path: path.to_path_buf(),
            loose_source_directory: Some(source_directory.to_path_buf()),
            hak_paths: Vec::new(),
            game_install_path: roots.game_install_path.clone(),
            user_data_path: roots.user_data_path.clone(),
        },
        catalog_cache_path,
        cancelled,
    )?;
    let resource_catalog = resource_build.catalog;
    let selected_area = resource_catalog
        .get(&aurora_core::ResourceKey::new(&resref, 2012))
        .ok_or_else(|| {
            AppError::new(
                "STANDALONE_AREA_NOT_RESOLVED",
                "La carte sélectionnée n’a pas pu être chargée.",
                format!("Selected ARE {resref}.are is absent from its source directory"),
                ErrorSeverity::Error,
            )
            .with_source(path.display().to_string())
            .with_import_stage("resource_catalog")
        })?;
    let area_document = ResourceManager::read(&selected_area.selected, cancelled)
        .and_then(|bytes| parse_gff(&bytes, &path.display().to_string()))?;
    if area_document.file_type != "ARE " {
        return Err(AppError::new(
            "STANDALONE_AREA_TYPE_INVALID",
            "Le fichier sélectionné n’est pas une carte ARE valide.",
            format!("Expected ARE GFF type, found {:?}", area_document.file_type),
            ErrorSeverity::Error,
        )
        .with_source(path.display().to_string())
        .with_import_stage("structured_resources")
        .into());
    }
    let inventory = standalone_inventory(&resource_catalog);
    let resource_catalog_cache = resource_build.cache;
    let resource_catalog_summary = resource_catalog.summary();
    on_progress(HashProgress::stage(
        AnalysisPhase::StructuredResources,
        57.0,
    ));
    let structured_summary = analyze_structured_resources(
        &resource_catalog,
        &module_info,
        &dependency_report,
        roots,
        cancelled,
    );
    on_progress(HashProgress::stage(AnalysisPhase::Scripts, 69.0));
    let script_index = analyze_scripts(&resource_catalog, cancelled);
    let script_index_summary = script_index.summary.clone();
    on_progress(HashProgress::stage(AnalysisPhase::Dialogues, 77.0));
    let dialogue_index = analyze_dialogues(&resource_catalog, &dependency_report, roots, cancelled);
    let dialogue_index_summary = dialogue_index.summary.clone();
    on_progress(HashProgress::stage(AnalysisPhase::World, 86.0));
    let world_index = analyze_world(
        &resource_catalog,
        &script_index,
        &dialogue_index,
        &dependency_report,
        roots,
        cancelled,
    );
    let world_summary = world_index.summary.clone();

    Ok(ModuleAnalysis {
        fingerprint,
        inventory,
        module_info,
        dependency_report,
        resource_catalog,
        resource_catalog_summary,
        resource_catalog_cache,
        structured_summary,
        script_index,
        script_index_summary,
        dialogue_index,
        dialogue_index_summary,
        world_index,
        world_summary,
    })
}

fn standalone_area_info(resref: &str) -> ModuleInfo {
    ModuleInfo {
        name: LocalizedString {
            string_ref: None,
            values: vec![LocalizedValue {
                language_id: 0,
                text: resref.to_owned(),
            }],
        },
        description: LocalizedString {
            string_ref: None,
            values: Vec::new(),
        },
        tag: resref.to_owned(),
        minimum_game_version: None,
        custom_tlk: None,
        entry_area: resref.to_owned(),
        hak_files: Vec::new(),
    }
}

fn standalone_inventory(catalog: &ResourceCatalog) -> ContainerInventory {
    let mut resources = catalog
        .entries
        .iter()
        .filter(|entry| entry.selected.source_kind == ResourceSourceKind::Standalone)
        .enumerate()
        .map(|(index, entry)| ContainerResource {
            key: entry.key.clone(),
            resource_id: u32::try_from(index).unwrap_or(u32::MAX),
            extension: aurora_core::resource_extension(entry.key.resource_type).map(str::to_owned),
            offset: 0,
            size: entry.selected.size,
        })
        .collect::<Vec<_>>();
    resources.sort_by(|left, right| left.key.cmp(&right.key));
    let mut summaries = BTreeMap::<u16, ResourceTypeSummary>::new();
    for resource in &resources {
        let summary = summaries
            .entry(resource.key.resource_type)
            .or_insert_with(|| ResourceTypeSummary {
                resource_type: resource.key.resource_type,
                extension: resource.extension.clone(),
                count: 0,
                total_size: 0,
            });
        summary.count = summary.count.saturating_add(1);
        summary.total_size = summary.total_size.saturating_add(resource.size);
    }
    ContainerInventory {
        file_type: "ARE ".to_owned(),
        file_version: "V3.2".to_owned(),
        build_year: 0,
        build_day: 0,
        resource_count: u32::try_from(resources.len()).unwrap_or(u32::MAX),
        resources,
        type_summaries: summaries.into_values().collect(),
    }
}

fn module_info_error(path: &Path, code: &str, detail: String) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            "Le fichier module.ifo est absent ou ambigu.",
            detail,
            ErrorSeverity::Error,
        )
        .with_source(path.display().to_string())
        .with_resource("module.ifo")
        .with_import_stage("module_info")
        .with_suggestion("Vérifiez que la copie sélectionnée est un module NWN complet."),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aurora_gff::{GenericField, GenericGff, GenericStruct, GenericValue, write_gff};
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    #[test]
    fn rejects_a_container_without_module_info() {
        let root = tempdir().expect("temporary directory");
        let module = root.path().join("empty.mod");
        let mut bytes = vec![0_u8; 160];
        bytes[0..4].copy_from_slice(b"MOD ");
        bytes[4..8].copy_from_slice(b"V1.0");
        bytes[20..24].copy_from_slice(&160_u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&160_u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&160_u32.to_le_bytes());
        fs::write(&module, bytes).expect("write synthetic module");

        let error = analyze_module_file(&module, &AtomicBool::new(false), |_| {})
            .expect_err("missing module.ifo");

        assert_eq!(error.code, "MODULE_IFO_NOT_FOUND");
    }

    #[test]
    fn analyzes_a_standalone_area_with_its_sibling_git() {
        let root = tempdir().expect("temporary directory");
        let area = root.path().join("lonely.are");
        let git = root.path().join("lonely.git");
        fs::write(
            &area,
            write_gff(&gff(
                "ARE ",
                vec![
                    field("Width", 4, GenericValue::Dword(1)),
                    field("Height", 4, GenericValue::Dword(1)),
                    field("Tileset", 11, GenericValue::ResRef("tno01".to_owned())),
                    field(
                        "Tile_List",
                        15,
                        GenericValue::List(vec![GenericStruct {
                            index: 1,
                            struct_type: 1,
                            fields: vec![
                                field("Tile_ID", 4, GenericValue::Dword(12)),
                                field("Tile_Orientation", 0, GenericValue::Byte(2)),
                            ],
                        }]),
                    ),
                ],
            ))
            .expect("write ARE"),
        )
        .expect("store ARE");
        fs::write(
            &git,
            write_gff(&gff("GIT ", Vec::new())).expect("write GIT"),
        )
        .expect("store GIT");

        let analysis = analyze_standalone_area_file_with_cache(
            &area,
            &DependencyRoots::default(),
            None,
            &AtomicBool::new(false),
            |_| {},
        )
        .expect("standalone area analysis");

        assert_eq!(analysis.inventory.file_type, "ARE ");
        assert_eq!(analysis.inventory.resource_count, 2);
        assert_eq!(analysis.module_info.entry_area, "lonely");
        assert_eq!(analysis.world_index.areas.len(), 1);
        assert_eq!(analysis.world_index.areas[0].tiles[0].tile_id, 12);
        assert_eq!(
            analysis
                .resource_catalog
                .get(&aurora_core::ResourceKey::new("lonely", 2012))
                .expect("ARE in catalog")
                .selected
                .source_kind,
            ResourceSourceKind::Standalone
        );
    }

    #[test]
    fn rejects_a_non_are_gff_with_an_are_extension() {
        let root = tempdir().expect("temporary directory");
        let area = root.path().join("renamed.are");
        fs::write(
            &area,
            write_gff(&gff("GIT ", Vec::new())).expect("write renamed GIT"),
        )
        .expect("store renamed GIT");

        let error = analyze_standalone_area_file_with_cache(
            &area,
            &DependencyRoots::default(),
            None,
            &AtomicBool::new(false),
            |_| {},
        )
        .expect_err("renamed GFF must be rejected");

        assert_eq!(error.code, "STANDALONE_AREA_TYPE_INVALID");
    }

    fn gff(file_type: &str, fields: Vec<GenericField>) -> GenericGff {
        GenericGff {
            file_type: file_type.to_owned(),
            file_version: "V3.2".to_owned(),
            source: "synthetic standalone area".to_owned(),
            struct_count: 1,
            field_count: fields.len() as u32,
            root: GenericStruct {
                index: 0,
                struct_type: u32::MAX,
                fields,
            },
        }
    }

    fn field(label: &str, field_type: u32, value: GenericValue) -> GenericField {
        GenericField {
            label: label.to_owned(),
            field_type,
            value,
        }
    }
}
