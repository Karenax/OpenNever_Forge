use crate::{
    DependencyRoots, DialogueIndex, DialogueIndexSummary, HashProgress, ModuleDependencyReport,
    ModuleFingerprint, ResourceCatalog, ResourceCatalogSummary, ResourceManager,
    ResourceManagerConfig, ScriptIndex, ScriptIndexSummary, StructuredResourceSummary, WorldIndex,
    WorldSummary, analyze_dialogues, analyze_scripts, analyze_structured_resources, analyze_world,
    compare_dependency_reports, fingerprint_module_dependencies, hash_module_file,
    inspect_module_dependencies,
};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use aurora_erf::{ContainerInventory, ContainerReader, ErfReader};
use aurora_gff::{ModuleInfo, read_module_info};
use serde::{Deserialize, Serialize};
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
    analyze_module_file_with_roots(path, &DependencyRoots::default(), cancelled, on_progress)
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
    let fingerprint = hash_module_file(path, cancelled, on_progress)?;
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
    let resource_catalog = ResourceManager::build(
        &ResourceManagerConfig {
            module_path: path.to_path_buf(),
            hak_paths,
            game_install_path: roots.game_install_path.clone(),
            user_data_path: roots.user_data_path.clone(),
        },
        cancelled,
    )?;
    let resource_catalog_summary = resource_catalog.summary();
    let structured_summary = analyze_structured_resources(
        &resource_catalog,
        &module_info,
        &dependency_report,
        roots,
        cancelled,
    );
    let script_index = analyze_scripts(&resource_catalog, cancelled);
    let script_index_summary = script_index.summary.clone();
    let dialogue_index = analyze_dialogues(&resource_catalog, &dependency_report, roots, cancelled);
    let dialogue_index_summary = dialogue_index.summary.clone();
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
        structured_summary,
        script_index,
        script_index_summary,
        dialogue_index,
        dialogue_index_summary,
        world_index,
        world_summary,
    })
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
}
