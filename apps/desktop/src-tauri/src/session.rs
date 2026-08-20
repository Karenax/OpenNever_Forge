use aurora_core::{AppError, AppResult};
use aurora_dialogue::{DialogueIndex, DialogueIndexSummary};
use aurora_erf::ContainerInventory;
use aurora_gff::ModuleInfo;
use aurora_nwscript::{ScriptIndex, ScriptIndexSummary};
use aurora_project::{
    ModuleAnalysis, ModuleDependencyReport, ModuleFingerprint, ResourceCatalog,
    ResourceCatalogCacheSummary, ResourceCatalogSummary, ResourceManager,
    StructuredResourceSummary, WorldIndex, WorldSummary, hash_module_file,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::UNIX_EPOCH;

const SESSION_CACHE_SCHEMA_VERSION: u32 = 1;
const MAX_SESSION_CACHE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_WATCHED_DIRECTORIES: usize = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionPaths {
    pub module_path: String,
    pub game_install_path: Option<String>,
    pub user_data_path: Option<String>,
}

impl SessionPaths {
    pub fn new(
        module_path: impl Into<String>,
        game_install_path: Option<PathBuf>,
        user_data_path: Option<PathBuf>,
    ) -> Self {
        Self {
            module_path: module_path.into(),
            game_install_path: game_install_path.map(|path| path.display().to_string()),
            user_data_path: user_data_path.map(|path| path.display().to_string()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisSessionWrite<'a> {
    schema_version: u32,
    paths: &'a SessionPaths,
    source_signature: &'a str,
    analysis: PersistedModuleAnalysisRef<'a>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisSessionRead {
    schema_version: u32,
    paths: SessionPaths,
    source_signature: String,
    analysis: PersistedModuleAnalysis,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedModuleAnalysisRef<'a> {
    fingerprint: &'a ModuleFingerprint,
    inventory: &'a ContainerInventory,
    module_info: &'a ModuleInfo,
    dependency_report: &'a ModuleDependencyReport,
    resource_catalog: &'a ResourceCatalog,
    resource_catalog_summary: &'a ResourceCatalogSummary,
    resource_catalog_cache: &'a ResourceCatalogCacheSummary,
    structured_summary: &'a StructuredResourceSummary,
    script_index: &'a ScriptIndex,
    script_index_summary: &'a ScriptIndexSummary,
    dialogue_index: &'a DialogueIndex,
    dialogue_index_summary: &'a DialogueIndexSummary,
    world_index: &'a WorldIndex,
    world_summary: &'a WorldSummary,
}

impl<'a> From<&'a ModuleAnalysis> for PersistedModuleAnalysisRef<'a> {
    fn from(analysis: &'a ModuleAnalysis) -> Self {
        Self {
            fingerprint: &analysis.fingerprint,
            inventory: &analysis.inventory,
            module_info: &analysis.module_info,
            dependency_report: &analysis.dependency_report,
            resource_catalog: &analysis.resource_catalog,
            resource_catalog_summary: &analysis.resource_catalog_summary,
            resource_catalog_cache: &analysis.resource_catalog_cache,
            structured_summary: &analysis.structured_summary,
            script_index: &analysis.script_index,
            script_index_summary: &analysis.script_index_summary,
            dialogue_index: &analysis.dialogue_index,
            dialogue_index_summary: &analysis.dialogue_index_summary,
            world_index: &analysis.world_index,
            world_summary: &analysis.world_summary,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedModuleAnalysis {
    fingerprint: ModuleFingerprint,
    inventory: ContainerInventory,
    module_info: ModuleInfo,
    dependency_report: ModuleDependencyReport,
    resource_catalog: ResourceCatalog,
    resource_catalog_summary: ResourceCatalogSummary,
    resource_catalog_cache: ResourceCatalogCacheSummary,
    structured_summary: StructuredResourceSummary,
    script_index: ScriptIndex,
    script_index_summary: ScriptIndexSummary,
    dialogue_index: DialogueIndex,
    dialogue_index_summary: DialogueIndexSummary,
    world_index: WorldIndex,
    world_summary: WorldSummary,
}

impl From<PersistedModuleAnalysis> for ModuleAnalysis {
    fn from(analysis: PersistedModuleAnalysis) -> Self {
        Self {
            fingerprint: analysis.fingerprint,
            inventory: analysis.inventory,
            module_info: analysis.module_info,
            dependency_report: analysis.dependency_report,
            resource_catalog: analysis.resource_catalog,
            resource_catalog_summary: analysis.resource_catalog_summary,
            resource_catalog_cache: analysis.resource_catalog_cache,
            structured_summary: analysis.structured_summary,
            script_index: analysis.script_index,
            script_index_summary: analysis.script_index_summary,
            dialogue_index: analysis.dialogue_index,
            dialogue_index_summary: analysis.dialogue_index_summary,
            world_index: analysis.world_index,
            world_summary: analysis.world_summary,
        }
    }
}

pub fn store_analysis_session(
    root: &Path,
    paths: &SessionPaths,
    analysis: &ModuleAnalysis,
) -> AppResult<()> {
    fs::create_dir_all(root).map_err(|error| {
        Box::new(AppError::io(
            "create analysis session cache",
            root.display().to_string(),
            &error,
        ))
    })?;
    let source_signature = source_signature(paths, analysis)?;
    let cache_path = cache_path(root, &paths.module_path);
    let temporary_path = cache_path.with_extension("json.tmp");
    let file = File::create(&temporary_path).map_err(|error| {
        Box::new(AppError::io(
            "create analysis session cache",
            temporary_path.display().to_string(),
            &error,
        ))
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(
        &mut writer,
        &AnalysisSessionWrite {
            schema_version: SESSION_CACHE_SCHEMA_VERSION,
            paths,
            source_signature: &source_signature,
            analysis: analysis.into(),
        },
    )
    .map_err(|error| {
        Box::new(AppError::database(
            temporary_path.display().to_string(),
            format!("cannot serialize analysis session: {error}"),
        ))
    })?;
    writer.flush().map_err(|error| {
        Box::new(AppError::io(
            "flush analysis session cache",
            temporary_path.display().to_string(),
            &error,
        ))
    })?;
    writer.get_ref().sync_all().map_err(|error| {
        Box::new(AppError::io(
            "sync analysis session cache",
            temporary_path.display().to_string(),
            &error,
        ))
    })?;
    drop(writer);
    if cache_path.is_file() {
        fs::remove_file(&cache_path).map_err(|error| {
            Box::new(AppError::io(
                "replace analysis session cache",
                cache_path.display().to_string(),
                &error,
            ))
        })?;
    }
    fs::rename(&temporary_path, &cache_path).map_err(|error| {
        Box::new(AppError::io(
            "publish analysis session cache",
            cache_path.display().to_string(),
            &error,
        ))
    })?;
    Ok(())
}

pub fn restore_analysis_session(
    root: &Path,
    paths: &SessionPaths,
) -> AppResult<Option<ModuleAnalysis>> {
    let cache_path = cache_path(root, &paths.module_path);
    let metadata = match fs::metadata(&cache_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Box::new(AppError::io(
                "inspect analysis session cache",
                cache_path.display().to_string(),
                &error,
            )));
        }
    };
    if metadata.len() > MAX_SESSION_CACHE_BYTES {
        return Ok(None);
    }
    let file = File::open(&cache_path).map_err(|error| {
        Box::new(AppError::io(
            "open analysis session cache",
            cache_path.display().to_string(),
            &error,
        ))
    })?;
    let cached = match serde_json::from_reader::<_, AnalysisSessionRead>(BufReader::new(file)) {
        Ok(cached) => cached,
        Err(error) => {
            tracing::warn!(path = %cache_path.display(), %error, "ignoring invalid analysis session cache");
            return Ok(None);
        }
    };
    if cached.schema_version != SESSION_CACHE_SCHEMA_VERSION || !paths_match(paths, &cached.paths) {
        return Ok(None);
    }
    let analysis: ModuleAnalysis = cached.analysis.into();
    let cancelled = AtomicBool::new(false);
    let current_fingerprint =
        match hash_module_file(Path::new(&paths.module_path), &cancelled, |_| {}) {
            Ok(fingerprint) => fingerprint,
            Err(_) => return Ok(None),
        };
    if current_fingerprint != analysis.fingerprint {
        return Ok(None);
    }
    if source_signature(paths, &analysis)? != cached.source_signature {
        return Ok(None);
    }
    Ok(Some(analysis))
}

fn source_signature(paths: &SessionPaths, analysis: &ModuleAnalysis) -> AppResult<String> {
    let mut candidates = BTreeMap::<String, PathBuf>::new();
    add_path(&mut candidates, PathBuf::from(&paths.module_path));
    for dependency in &analysis.dependency_report.dependencies {
        if let Some(path) = &dependency.selected_path {
            add_path(&mut candidates, PathBuf::from(path));
        }
        for path in dependency
            .shadowed_paths
            .iter()
            .chain(dependency.searched_paths.iter())
        {
            add_path(&mut candidates, PathBuf::from(path));
        }
    }
    for resource in &analysis.resource_catalog.entries {
        add_path(
            &mut candidates,
            PathBuf::from(&resource.selected.source_path),
        );
        for version in &resource.shadowed {
            add_path(&mut candidates, PathBuf::from(&version.source_path));
        }
    }

    let mut watched_directories = BTreeMap::new();
    if let Some(user_root) = &paths.user_data_path {
        collect_directory_tree(
            &Path::new(user_root).join("development"),
            &mut watched_directories,
        )?;
        collect_directory_tree(
            &Path::new(user_root).join("override"),
            &mut watched_directories,
        )?;
    }

    let mut digest = Sha256::new();
    digest.update(b"opennever-analysis-session-v1\0");
    digest.update(normalize_path(&paths.module_path));
    digest.update(b"\0");
    digest.update(normalize_optional_path(&paths.game_install_path));
    digest.update(b"\0");
    digest.update(normalize_optional_path(&paths.user_data_path));
    digest.update(b"\0");
    if let Some(game_root) = &paths.game_install_path {
        let cancelled = AtomicBool::new(false);
        let game_signature =
            ResourceManager::game_source_signature(Path::new(game_root), &cancelled)?;
        digest.update(game_signature.as_bytes());
    }
    for (normalized, path) in candidates {
        digest.update(normalized.as_bytes());
        update_metadata_digest(&mut digest, &path)?;
    }
    for (normalized, directory) in watched_directories {
        digest.update(normalized.as_bytes());
        update_metadata_digest(&mut digest, &directory)?;
    }
    Ok(hex::encode_upper(digest.finalize()))
}

fn add_path(candidates: &mut BTreeMap<String, PathBuf>, path: PathBuf) {
    let normalized = normalize_path(&path.display().to_string());
    candidates.entry(normalized).or_insert(path);
}

fn collect_directory_tree(
    directory: &Path,
    output: &mut BTreeMap<String, PathBuf>,
) -> AppResult<()> {
    let normalized = normalize_path(&directory.display().to_string());
    if output.insert(normalized, directory.to_path_buf()).is_some() || !directory.is_dir() {
        return Ok(());
    }
    if output.len() > MAX_WATCHED_DIRECTORIES {
        return Err(Box::new(AppError::database(
            directory.display().to_string(),
            "analysis session directory watch limit exceeded",
        )));
    }
    let entries = fs::read_dir(directory).map_err(|error| {
        Box::new(AppError::io(
            "enumerate analysis session directory",
            directory.display().to_string(),
            &error,
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            Box::new(AppError::io(
                "enumerate analysis session directory",
                directory.display().to_string(),
                &error,
            ))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            Box::new(AppError::io(
                "inspect analysis session directory entry",
                entry.path().display().to_string(),
                &error,
            ))
        })?;
        if file_type.is_dir() && !file_type.is_symlink() {
            collect_directory_tree(&entry.path(), output)?;
        }
    }
    Ok(())
}

fn update_metadata_digest(digest: &mut Sha256, path: &Path) -> AppResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let kind = if metadata.file_type().is_symlink() {
                b's'
            } else if metadata.is_dir() {
                b'd'
            } else {
                b'f'
            };
            digest.update([kind]);
            digest.update(metadata.len().to_le_bytes());
            let modified = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|value| value.as_nanos())
                .unwrap_or_default();
            digest.update(modified.to_le_bytes());
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            digest.update(b"missing");
            Ok(())
        }
        Err(error) => Err(Box::new(AppError::io(
            "inspect analysis session source",
            path.display().to_string(),
            &error,
        ))),
    }
}

fn cache_path(root: &Path, module_path: &str) -> PathBuf {
    let mut digest = Sha256::new();
    digest.update(normalize_path(module_path));
    root.join(format!("{}.json", hex::encode(digest.finalize())))
}

fn paths_match(left: &SessionPaths, right: &SessionPaths) -> bool {
    normalize_path(&left.module_path) == normalize_path(&right.module_path)
        && normalize_optional_path(&left.game_install_path)
            == normalize_optional_path(&right.game_install_path)
        && normalize_optional_path(&left.user_data_path)
            == normalize_optional_path(&right.user_data_path)
}

fn normalize_optional_path(path: &Option<String>) -> String {
    path.as_deref().map(normalize_path).unwrap_or_default()
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn restores_a_complete_analysis_until_the_module_changes() {
        let root = tempdir().expect("temporary root");
        let module = root.path().join("campaign.mod");
        fs::write(&module, b"module-v1").expect("module fixture");
        let cancelled = AtomicBool::new(false);
        let fingerprint = hash_module_file(&module, &cancelled, |_| {}).expect("fingerprint");
        let analysis: ModuleAnalysis = serde_json::from_value(json!({
            "fingerprint": fingerprint,
            "inventory": {
                "fileType": "MOD ", "fileVersion": "V1.0", "buildYear": 2026,
                "buildDay": 223, "resourceCount": 0, "resources": [], "typeSummaries": []
            },
            "moduleInfo": {
                "name": { "stringRef": null, "values": [] },
                "description": { "stringRef": null, "values": [] },
                "tag": "SESSION", "minimumGameVersion": null, "customTlk": null,
                "entryArea": "start", "hakFiles": []
            },
            "dependencyReport": {
                "dependencies": [], "resolvedCount": 0, "missingCount": 0,
                "uncheckedCount": 0, "invalidCount": 0, "changedCount": 0
            }
        }))
        .expect("analysis fixture");
        let paths = SessionPaths::new(module.display().to_string(), None, None);

        store_analysis_session(root.path(), &paths, &analysis).expect("store session");
        let restored = restore_analysis_session(root.path(), &paths)
            .expect("restore session")
            .expect("cached analysis");
        assert_eq!(restored.fingerprint, analysis.fingerprint);

        fs::write(&module, b"module-v2").expect("change module");
        assert!(
            restore_analysis_session(root.path(), &paths)
                .expect("stale cache is ignored")
                .is_none()
        );
    }
}
