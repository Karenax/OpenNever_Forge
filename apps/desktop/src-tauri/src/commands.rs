use crate::jobs::JobSnapshot;
use crate::state::AppState;
use aurora_2da::{TwoDaTable, parse_2da};
use aurora_core::{AppError, AppResult, ResourceKey};
use aurora_gff::{GenericGff, parse_gff};
use aurora_index::{CatalogPersistence, load_dependency_baseline, replace_resource_catalog};
use aurora_project::{
    DependencyRoots, DiagnosticReport, DialogueGraph, DialoguePage, ModuleDependencyReport,
    NarrativeModel, ResourceManager, ResourcePage, ResourceSourceKind, SceneManifest,
    ScriptDocument, ScriptPage, WorldIndex, analyze_module_file_with_roots, build_asset_preview,
    cached_model_preview, compare_dependency_reports,
};
use aurora_tlk::{TalkTable, parse_tlk};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use tauri::{AppHandle, Emitter, State, ipc::Response};

const JOB_PROGRESS_EVENT: &str = "job-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_version: &'static str,
    pub read_only: bool,
    pub database_schema_version: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleAnalysisRequest {
    pub module_path: String,
    pub game_install_path: Option<String>,
    pub user_data_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceQueryRequest {
    pub job_id: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub resource_types: Vec<u16>,
    pub source: Option<ResourceSourceKind>,
    #[serde(default)]
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInspectionRequest {
    pub job_id: String,
    pub resref: String,
    pub resource_type: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptQueryRequest {
    pub job_id: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInspectionRequest {
    pub job_id: String,
    pub resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueQueryRequest {
    pub job_id: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub offset: usize,
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueInspectionRequest {
    pub job_id: String,
    pub resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaInspectionRequest {
    pub job_id: String,
    pub resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreviewRequest {
    pub job_id: String,
    pub resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPreviewRequest {
    pub job_id: String,
    pub resref: String,
    pub resource_type: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ResourceInspection {
    Gff(GenericGff),
    TwoDa(TwoDaTable),
    Tlk(TalkTable),
    Binary(BinaryInspection),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryInspection {
    pub size: usize,
    pub sha256: String,
    pub hex_preview: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReportBundle {
    pub report: DiagnosticReport,
    pub json: String,
    pub html: String,
}

#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> AppStatus {
    AppStatus {
        app_version: env!("CARGO_PKG_VERSION"),
        read_only: true,
        database_schema_version: state.database.schema_version,
    }
}

#[tauri::command]
pub fn start_module_analysis(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ModuleAnalysisRequest,
) -> AppResult<JobSnapshot> {
    let module_path = PathBuf::from(&request.module_path);
    if !module_path.is_file() {
        return Err(AppError::module_not_found(request.module_path).into());
    }
    if !module_path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("mod"))
    {
        return Err(AppError::invalid_path(
            module_path.display().to_string(),
            "Module analysis accepts only .mod files",
        )
        .into());
    }
    let roots = DependencyRoots {
        game_install_path: optional_directory(request.game_install_path, "game_install_path")?,
        user_data_path: optional_directory(request.user_data_path, "user_data_path")?,
    };
    let database_path = PathBuf::from(&state.database.path);
    let module_source_path = module_path.display().to_string();
    let previous_dependency_report = load_dependency_baseline(&database_path, &module_source_path)?
        .map(|json| {
            serde_json::from_str::<ModuleDependencyReport>(&json).map_err(|error| {
                Box::new(AppError::database(
                    database_path.display().to_string(),
                    format!("cannot decode dependency baseline: {error}"),
                ))
            })
        })
        .transpose()?;

    let registry = state.jobs.clone();
    let (job, cancellation) = registry.create_analysis_job(module_path.display().to_string());
    let job_id = job.id.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
        if let Some(snapshot) = registry.set_running(&job_id) {
            emit_snapshot(&app_handle, &snapshot);
        }

        let result =
            analyze_module_file_with_roots(&module_path, &roots, &cancellation, |progress| {
                if let Some(snapshot) = registry.set_progress(&job_id, progress) {
                    emit_snapshot(&app_handle, &snapshot);
                }
            })
            .and_then(|mut analysis| {
                compare_dependency_reports(
                    &mut analysis.dependency_report,
                    previous_dependency_report.as_ref(),
                );
                let summary =
                    serde_json::to_string(&analysis.structured_summary).map_err(|error| {
                        AppError::database(
                            database_path.display().to_string(),
                            format!("cannot serialize structured summary: {error}"),
                        )
                    })?;
                let dependencies =
                    serde_json::to_string(&analysis.dependency_report).map_err(|error| {
                        AppError::database(
                            database_path.display().to_string(),
                            format!("cannot serialize dependency baseline: {error}"),
                        )
                    })?;
                replace_resource_catalog(
                    &database_path,
                    CatalogPersistence {
                        project_id: &analysis.fingerprint.sha256,
                        source_digest: &analysis.fingerprint.sha256,
                        catalog: &analysis.resource_catalog,
                        structured_summary_json: &summary,
                        source_path: &module_source_path,
                        dependency_report_json: &dependencies,
                        script_index: &analysis.script_index,
                        dialogue_index: &analysis.dialogue_index,
                        world_index: &analysis.world_index,
                    },
                )?;
                Ok(analysis)
            });

        let snapshot = match result {
            Ok(fingerprint) => registry.complete(&job_id, fingerprint),
            Err(error) => registry.fail(&job_id, *error),
        };
        if let Some(snapshot) = snapshot {
            emit_snapshot(&app_handle, &snapshot);
        }
    });

    Ok(job)
}

fn optional_directory(path: Option<String>, field: &str) -> AppResult<Option<PathBuf>> {
    let Some(path) = path
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let directory = PathBuf::from(&path);
    if directory.is_dir() {
        return Ok(Some(directory));
    }
    Err(AppError::invalid_path(path, format!("{field} is not an existing directory")).into())
}

#[tauri::command]
pub fn get_job(state: State<'_, AppState>, id: String) -> Option<JobSnapshot> {
    state.jobs.get(&id)
}

#[tauri::command]
pub fn cancel_job(state: State<'_, AppState>, id: String) -> AppResult<JobSnapshot> {
    state.jobs.cancel(&id)
}

#[tauri::command]
pub fn query_resources(
    state: State<'_, AppState>,
    request: ResourceQueryRequest,
) -> AppResult<ResourcePage> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        Ok(analysis.resource_catalog.search_many(
            &request.query,
            &request.resource_types,
            request.source,
            request.offset,
            request.limit,
        ))
    })
}

#[tauri::command]
pub fn inspect_resource(
    state: State<'_, AppState>,
    request: ResourceInspectionRequest,
) -> AppResult<ResourceInspection> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        let key = aurora_core::ResourceKey::new(&request.resref, request.resource_type);
        let resource = analysis.resource_catalog.get(&key).ok_or_else(|| {
            AppError::invalid_path(key.to_string(), "resource not found in the indexed catalog")
        })?;
        let bytes = ResourceManager::read(&resource.selected, &AtomicBool::new(false))?;
        let source = format!("{}::{}", resource.selected.source_path, key);
        match key.resource_type {
            2017 => Ok(ResourceInspection::TwoDa(parse_2da(&bytes, &source)?)),
            2018 => Ok(ResourceInspection::Tlk(parse_tlk(&bytes, &source)?)),
            resource_type if is_gff(resource_type) => {
                Ok(ResourceInspection::Gff(parse_gff(&bytes, &source)?))
            }
            _ => {
                let preview_size = bytes.len().min(256);
                Ok(ResourceInspection::Binary(BinaryInspection {
                    size: bytes.len(),
                    sha256: hex::encode(Sha256::digest(&bytes)),
                    hex_preview: hex::encode_upper(&bytes[..preview_size]),
                    truncated: preview_size < bytes.len(),
                }))
            }
        }
    })
}

#[tauri::command]
pub fn query_scripts(
    state: State<'_, AppState>,
    request: ScriptQueryRequest,
) -> AppResult<ScriptPage> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        Ok(analysis
            .script_index
            .search(&request.query, request.offset, request.limit))
    })
}

#[tauri::command]
pub fn inspect_script(
    state: State<'_, AppState>,
    request: ScriptInspectionRequest,
) -> AppResult<ScriptDocument> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        analysis
            .script_index
            .get(&request.resref)
            .cloned()
            .ok_or_else(|| {
                AppError::invalid_path(&request.resref, "script not found in the indexed catalog")
                    .into()
            })
    })
}

#[tauri::command]
pub fn query_dialogues(
    state: State<'_, AppState>,
    request: DialogueQueryRequest,
) -> AppResult<DialoguePage> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        Ok(analysis
            .dialogue_index
            .search(&request.query, request.offset, request.limit))
    })
}

#[tauri::command]
pub fn inspect_dialogue(
    state: State<'_, AppState>,
    request: DialogueInspectionRequest,
) -> AppResult<DialogueGraph> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        analysis
            .dialogue_index
            .get(&request.resref)
            .cloned()
            .ok_or_else(|| {
                AppError::invalid_path(&request.resref, "dialogue not found in the indexed catalog")
                    .into()
            })
    })
}

#[tauri::command]
pub fn inspect_world(state: State<'_, AppState>, request: WorldRequest) -> AppResult<WorldIndex> {
    state
        .jobs
        .with_analysis(&request.job_id, |analysis| Ok(analysis.world_index.clone()))
}

#[tauri::command]
pub fn inspect_narrative(
    state: State<'_, AppState>,
    request: WorldRequest,
) -> AppResult<NarrativeModel> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        Ok(analysis.world_index.narrative.clone())
    })
}

#[tauri::command]
pub fn inspect_scene(
    state: State<'_, AppState>,
    request: AreaInspectionRequest,
) -> AppResult<SceneManifest> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        analysis
            .world_index
            .scenes
            .iter()
            .find(|scene| scene.area.eq_ignore_ascii_case(&request.resref))
            .cloned()
            .ok_or_else(|| {
                AppError::invalid_path(&request.resref, "area scene not found in world index")
                    .into()
            })
    })
}

#[tauri::command]
pub fn model_preview_glb(
    state: State<'_, AppState>,
    request: ModelPreviewRequest,
) -> AppResult<Response> {
    let cancelled = AtomicBool::new(false);
    let entry = state.jobs.with_analysis(&request.job_id, |analysis| {
        cached_model_preview(
            &analysis.resource_catalog,
            &request.resref,
            &state.asset_cache_root,
            &cancelled,
        )
    })?;
    tracing::debug!(
        resref = request.resref,
        cache_hit = entry.cache_hit,
        cache_path = %entry.cache_path.display(),
        "model preview ready"
    );
    Ok(Response::new(entry.artifact.bytes))
}

#[tauri::command]
pub fn resolve_texture(
    state: State<'_, AppState>,
    request: ModelPreviewRequest,
) -> AppResult<Option<ResourceKey>> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        Ok([2033, 3, 2073, 2080, 2081, 2079, 6]
            .into_iter()
            .map(|resource_type| ResourceKey::new(&request.resref, resource_type))
            .find(|key| analysis.resource_catalog.get(key).is_some()))
    })
}

#[tauri::command]
pub fn asset_preview_bytes(
    state: State<'_, AppState>,
    request: AssetPreviewRequest,
) -> AppResult<Response> {
    let cancelled = AtomicBool::new(false);
    let preview = state.jobs.with_analysis(&request.job_id, |analysis| {
        build_asset_preview(
            &analysis.resource_catalog,
            &request.resref,
            request.resource_type,
            &cancelled,
        )
    })?;
    Ok(Response::new(preview.bytes))
}

#[tauri::command]
pub fn diagnostic_report(
    state: State<'_, AppState>,
    request: WorldRequest,
) -> AppResult<DiagnosticReportBundle> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        let report = analysis
            .world_index
            .report(analysis.fingerprint.sha256.clone());
        Ok(DiagnosticReportBundle {
            json: report.stable_json(),
            html: report.standalone_html(),
            report,
        })
    })
}

fn is_gff(resource_type: u16) -> bool {
    matches!(
        aurora_core::resource_extension(resource_type),
        Some(
            "are"
                | "ifo"
                | "bic"
                | "git"
                | "uti"
                | "utc"
                | "dlg"
                | "itp"
                | "utt"
                | "uts"
                | "gff"
                | "fac"
                | "ute"
                | "utd"
                | "utp"
                | "gic"
                | "gui"
                | "utm"
                | "jrl"
                | "utw"
        )
    )
}

fn emit_snapshot(app: &AppHandle, snapshot: &JobSnapshot) {
    if let Err(error) = app.emit(JOB_PROGRESS_EVENT, snapshot) {
        tracing::warn!(job_id = snapshot.id, %error, "cannot emit job progress");
    }
}
