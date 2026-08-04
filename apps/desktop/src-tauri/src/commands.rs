use crate::jobs::JobSnapshot;
use crate::state::AppState;
use aurora_2da::{TwoDaEditAction, TwoDaTable, apply_2da_edit, parse_2da, write_2da};
use aurora_core::{AppError, AppResult, ResourceKey, decode_nwn_text};
use aurora_dialogue::adapt_dialogue;
use aurora_edit::{
    AiApplyReport, AiChangeSet, AiChangeSetPreview, AreaStructureAction, AuroraSyncAction,
    AuroraSyncAppliedFile, AuroraSyncDirection, AuroraSyncManifest, AuroraSyncPlan,
    AuroraSyncReport, AuroraSyncState, AuroraSyncWorkspaceFile, BlueprintStructureAction,
    DevelopmentCleanupReport, DevelopmentDeployment, DialogueStructureAction, EditCommand,
    EditWorkspace, FactionStructureAction, GitWorkspaceStatus, InstancePlacement,
    JournalStructureAction, ModuleBuildProfile, ModuleBuildReport, NewModuleDefinition,
    NwnLaunchProfile, NwnLaunchReport, PaletteManifest, ReproducibleBuildVerification,
    ResourceContentDigest, TileState, Transform, WalkmeshDocument, WalkmeshDraft, WalkmeshKind,
    WalkmeshOperation, WalkmeshValidation, WorkspaceExportManifest, WorkspaceSnapshot,
    add_area_instance, ai_change_set_sha256, apply_walkmesh_operation, baseline_from_plan,
    compare_aurora_sync, create_area_resources, create_empty_module, edit_area_instance,
    edit_area_structure, edit_area_tile, edit_blueprint_structure, edit_dialogue_structure,
    edit_faction_structure, edit_gff_field, edit_journal_structure, edit_module_dependencies,
    inspect_git_repository, inspect_walkmesh, read_aurora_workspace_file, remove_area_instance,
    resource_key_from_aurora_path, scan_aurora_workspace, serialize_walkmesh_ascii,
    validate_build_profile, validate_walkmesh_for_kind, verify_sync_action,
    write_aurora_workspace_file,
};
use aurora_gff::{GenericGff, parse_gff};
use aurora_index::{CatalogPersistence, load_dependency_baseline, replace_resource_catalog};
use aurora_nwscript::{CompileResult, CompilerConfig, NssDocument, compile_nss, parse_nss};
use aurora_project::{
    DependencyRoots, DiagnosticReport, DialogueGraph, DialoguePage, ModuleDependencyReport,
    NarrativeModel, ResourceManager, ResourcePage, ResourceSourceKind, SceneManifest,
    ScriptDocument, ScriptPage, WorldIndex, analyze_module_file_with_roots, build_asset_preview,
    cached_model_preview, compare_dependency_reports,
};
use aurora_tlk::{TalkTable, TlkEditAction, apply_tlk_edit, parse_tlk, write_tlk};
use aurora_world::{AreaMap, adapt_area, adapt_narrative};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use tauri::{AppHandle, Emitter, State, ipc::Response};

const JOB_PROGRESS_EVENT: &str = "job-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_version: &'static str,
    pub read_only: bool,
    pub editing_available: bool,
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
    pub workspace_id: Option<String>,
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
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueEditRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
    pub path: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueEditResult {
    pub workspace: WorkspaceSnapshot,
    pub graph: DialogueGraph,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueStructureRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
    pub action: DialogueStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeInspectionRequest {
    pub job_id: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeDocument {
    pub resource: ResourceKey,
    pub raw: GenericGff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeInspection {
    pub model: NarrativeModel,
    pub journal: Option<NarrativeDocument>,
    pub factions: Option<NarrativeDocument>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalStructureRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resource: ResourceKey,
    pub action: JournalStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactionStructureRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resource: ResourceKey,
    pub action: FactionStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueprintStructureRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resource: ResourceKey,
    pub action: BlueprintStructureAction,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEditWorkspaceRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorkspaceAreasRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditWorkspaceRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GffEditRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resource: ResourceKey,
    pub path: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GffEditResult {
    pub workspace: WorkspaceSnapshot,
    pub document: GenericGff,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptEditRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptEditResult {
    pub workspace: WorkspaceSnapshot,
    pub document: NssDocument,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileScriptRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
    pub compiler_path: String,
    pub game_install_path: String,
    #[serde(default)]
    pub include_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileScriptResult {
    pub workspace: WorkspaceSnapshot,
    pub compilation: CompileResult,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveAreaInstanceRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub area: String,
    pub instance_id: String,
    pub before: Transform,
    pub after: Transform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAreaTileRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub area: String,
    pub x: u32,
    pub y: u32,
    pub before: TileState,
    pub after: TileState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditAreaStructureRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub area: String,
    pub action: AreaStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceAreaInspectionRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub area: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildModuleRequest {
    pub workspace_id: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentRequest {
    pub workspace_id: String,
    pub user_data_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceOutputRequest {
    pub workspace_id: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwoDaEditRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resource: ResourceKey,
    pub action: TwoDaEditAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlkEditRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resource: ResourceKey,
    pub action: TlkEditAction,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableEditResult<T> {
    pub workspace: WorkspaceSnapshot,
    pub document: T,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDependencyEditRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub hak_files: Vec<String>,
    pub custom_tlk: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleDependencyEditResult {
    pub workspace: WorkspaceSnapshot,
    pub document: GenericGff,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProfileRequest {
    pub workspace_id: String,
    pub profile: ModuleBuildProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunBuildProfileRequest {
    pub workspace_id: String,
    pub profile: ModuleBuildProfile,
    pub output_directory: String,
    pub user_data_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProfileRunReport {
    pub profile: ModuleBuildProfile,
    pub build: ModuleBuildReport,
    pub deployment: Option<DevelopmentDeployment>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitWorkspaceRequest {
    pub root: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProfileRequest {
    pub workspace_id: String,
    pub profile: NwnLaunchProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncRequest {
    pub root: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncPlanRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub root: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuroraSyncApplyRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub root: String,
    pub actions: Vec<AuroraSyncAction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshValidationRequest {
    pub draft: WalkmeshDraft,
    pub kind: WalkmeshKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshTransformRequest {
    pub draft: WalkmeshDraft,
    pub operation: WalkmeshOperation,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshTransformResult {
    pub draft: WalkmeshDraft,
    pub validation: WalkmeshValidation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshInspectionRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
    pub kind: WalkmeshKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveWalkmeshRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
    pub kind: WalkmeshKind,
    pub draft: WalkmeshDraft,
    #[serde(default)]
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkmeshEditResult {
    pub workspace: WorkspaceSnapshot,
    pub document: WalkmeshDocument,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChangeSetRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub change_set: AiChangeSet,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConsent {
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default)]
    pub include_module_metadata: bool,
    #[serde(default)]
    pub include_resource_contents: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub selected_resources: Vec<ResourceKey>,
    pub consent: AiConsent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderProposal {
    pub endpoint_origin: String,
    pub model: String,
    pub proposal_sha256: String,
    pub change_set: AiChangeSet,
    pub preview: AiChangeSetPreview,
    pub shared_resources: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyAiChangeSetRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub proposal_sha256: String,
    pub change_set: AiChangeSet,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNewModuleRequest {
    pub output_path: String,
    pub name: String,
    pub tag: String,
    pub entry_area: String,
    pub tileset: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAreaRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
    pub name: String,
    pub tileset: String,
    pub width: u32,
    pub height: u32,
    pub tile_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAreaRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub resref: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAreaResult {
    pub workspace: WorkspaceSnapshot,
    pub area: AreaMap,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAreaInstanceRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub area: String,
    pub placement: InstancePlacement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAreaInstanceRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub area: String,
    pub instance_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddAreaInstanceResult {
    pub workspace: WorkspaceSnapshot,
    pub instance_id: String,
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
        editing_available: true,
        database_schema_version: state.database.schema_version,
    }
}

#[tauri::command]
pub fn create_edit_workspace(
    state: State<'_, AppState>,
    request: CreateEditWorkspaceRequest,
) -> AppResult<WorkspaceSnapshot> {
    let job = state.jobs.get(&request.job_id).ok_or_else(|| {
        Box::new(
            AppError::new(
                "JOB_NOT_FOUND",
                "L’analyse demandée n’existe plus.",
                format!("No analysis job exists with id {}", request.job_id),
                aurora_core::ErrorSeverity::Warning,
            )
            .with_import_stage("edit_workspace"),
        )
    })?;
    let analysis = job.result.ok_or_else(|| {
        Box::new(
            AppError::new(
                "ANALYSIS_NOT_COMPLETED",
                "L’analyse doit être terminée avant d’ouvrir l’édition.",
                format!("Job {} has no completed result", request.job_id),
                aurora_core::ErrorSeverity::Warning,
            )
            .with_import_stage("edit_workspace"),
        )
    })?;
    let root = state.edit_workspace_root.join(
        analysis
            .fingerprint
            .sha256
            .chars()
            .take(16)
            .collect::<String>(),
    );
    let workspace = if root.join("workspace.json").is_file() {
        EditWorkspace::open(&root)?
    } else {
        EditWorkspace::create(
            &root,
            PathBuf::from(&job.source_path).as_path(),
            &analysis.fingerprint.sha256,
            analysis.fingerprint.size_bytes,
        )?
    };
    let snapshot = workspace.snapshot()?;
    if !snapshot.source_intact
        || !snapshot
            .source
            .sha256
            .eq_ignore_ascii_case(&analysis.fingerprint.sha256)
    {
        return Err(Box::new(
            AppError::new(
                "EDIT_SOURCE_CHANGED",
                "La source a changé depuis l’analyse.",
                "The edit workspace source fingerprint no longer matches the completed analysis.",
                aurora_core::ErrorSeverity::Error,
            )
            .with_source(job.source_path)
            .with_import_stage("edit_workspace"),
        ));
    }
    let id = workspace.id().to_owned();
    state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned")
        .insert(id, workspace);
    Ok(snapshot)
}

#[tauri::command]
pub fn get_edit_workspace(
    state: State<'_, AppState>,
    request: EditWorkspaceRequest,
) -> AppResult<WorkspaceSnapshot> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.snapshot()
    })
}

#[tauri::command]
pub fn undo_edit_command(
    state: State<'_, AppState>,
    request: EditWorkspaceRequest,
) -> AppResult<WorkspaceSnapshot> {
    with_edit_workspace(&state, &request.workspace_id, EditWorkspace::undo)
}

#[tauri::command]
pub fn redo_edit_command(
    state: State<'_, AppState>,
    request: EditWorkspaceRequest,
) -> AppResult<WorkspaceSnapshot> {
    with_edit_workspace(&state, &request.workspace_id, EditWorkspace::redo)
}

#[tauri::command]
pub fn apply_gff_edit(
    state: State<'_, AppState>,
    request: GffEditRequest,
) -> AppResult<GffEditResult> {
    let source_bytes = state.jobs.with_analysis(&request.job_id, |analysis| {
        let resolved = analysis
            .resource_catalog
            .get(&request.resource)
            .ok_or_else(|| {
                Box::new(
                    AppError::new(
                        "EDIT_RESOURCE_NOT_RESOLVED",
                        "La ressource à modifier n’est pas résolue.",
                        format!(
                            "Resource Manager has no selected version for {}",
                            request.resource
                        ),
                        aurora_core::ErrorSeverity::Error,
                    )
                    .with_resource(request.resource.to_string())
                    .with_import_stage("gff_edit"),
                )
            })?;
        ResourceManager::read(&resolved.selected, &AtomicBool::new(false))
    })?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces.get_mut(&request.workspace_id).ok_or_else(|| {
        Box::new(
            AppError::new(
                "EDIT_WORKSPACE_NOT_OPEN",
                "L’espace d’édition n’est pas ouvert.",
                format!("No open edit workspace has id {}", request.workspace_id),
                aurora_core::ErrorSeverity::Warning,
            )
            .with_import_stage("gff_edit"),
        )
    })?;
    let command = EditCommand::SetField {
        resource: request.resource.clone(),
        path: request.path.clone(),
        before: request.before.clone(),
        after: request.after.clone(),
    };
    let preview = workspace.preview(command.clone());
    if !preview.valid {
        return Err(Box::new(
            AppError::new(
                "EDIT_PRECONDITION_FAILED",
                "La ressource a changé depuis la prévisualisation.",
                preview
                    .diagnostic
                    .unwrap_or_else(|| "command rejected".to_owned()),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(request.resource.to_string())
            .with_import_stage("gff_edit"),
        ));
    }
    let current_bytes = workspace
        .staged_resource_bytes(&request.resource)?
        .unwrap_or_else(|| source_bytes.clone());
    let (output, document) = edit_gff_field(
        &current_bytes,
        &format!("workspace::{}", request.resource.file_name()),
        &request.path,
        &request.before,
        &request.after,
    )?;
    workspace.stage_resource(request.resource, Some(&source_bytes), &output)?;
    let snapshot = workspace.apply(command)?;
    Ok(GffEditResult {
        workspace: snapshot,
        document,
    })
}

#[tauri::command]
pub fn edit_script_source(
    state: State<'_, AppState>,
    request: ScriptEditRequest,
) -> AppResult<ScriptEditResult> {
    let resource = ResourceKey::new(&request.resref, 2009);
    let source_bytes = state.jobs.with_analysis(&request.job_id, |analysis| {
        let resolved = analysis.resource_catalog.get(&resource).ok_or_else(|| {
            Box::new(
                AppError::new(
                    "EDIT_SCRIPT_NOT_RESOLVED",
                    "La source NSS n’est pas résolue.",
                    format!("Resource Manager has no selected version for {resource}"),
                    aurora_core::ErrorSeverity::Error,
                )
                .with_resource(resource.to_string())
                .with_import_stage("nwscript_edit"),
            )
        })?;
        ResourceManager::read(&resolved.selected, &AtomicBool::new(false))
    })?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "nwscript_edit"))?;
    let current_bytes = workspace
        .staged_resource_bytes(&resource)?
        .unwrap_or_else(|| source_bytes.clone());
    let current = decode_nwn_text(&current_bytes);
    if current != request.before {
        return Err(Box::new(
            AppError::new(
                "EDIT_SCRIPT_PRECONDITION_FAILED",
                "Le script a changé depuis la prévisualisation.",
                format!("Current text for {resource} differs from the editor baseline"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(resource.to_string())
            .with_import_stage("nwscript_edit"),
        ));
    }
    let command = EditCommand::ReplaceText {
        resource: resource.clone(),
        before: request.before,
        after: request.after.clone(),
    };
    let preview = workspace.preview(command.clone());
    if !preview.valid {
        return Err(Box::new(
            AppError::new(
                "EDIT_PRECONDITION_FAILED",
                "Le script a changé depuis la prévisualisation.",
                preview
                    .diagnostic
                    .unwrap_or_else(|| "command rejected".to_owned()),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(resource.to_string())
            .with_import_stage("nwscript_edit"),
        ));
    }
    let document = parse_nss(request.after.as_bytes(), &format!("workspace::{resource}"))?;
    workspace.stage_resource(resource, Some(&source_bytes), request.after.as_bytes())?;
    let snapshot = workspace.apply(command)?;
    Ok(ScriptEditResult {
        workspace: snapshot,
        document,
    })
}

#[tauri::command]
pub fn compile_workspace_script(
    state: State<'_, AppState>,
    request: CompileScriptRequest,
) -> AppResult<CompileScriptResult> {
    let nss_key = ResourceKey::new(&request.resref, 2009);
    let ncs_key = ResourceKey::new(&request.resref, 2010);
    let source_ncs = state.jobs.with_analysis(&request.job_id, |analysis| {
        analysis
            .resource_catalog
            .get(&ncs_key)
            .map(|resolved| ResourceManager::read(&resolved.selected, &AtomicBool::new(false)))
            .transpose()
    })?;
    let (source_text, workspace_root, staged_ncs) = {
        let mut workspaces = state
            .edit_workspaces
            .lock()
            .expect("edit workspace registry poisoned");
        let workspace = workspaces
            .get_mut(&request.workspace_id)
            .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "nwscript_compile"))?;
        let nss = workspace.staged_resource_bytes(&nss_key)?.ok_or_else(|| {
            Box::new(
                AppError::new(
                    "NSS_SOURCE_NOT_STAGED",
                    "Enregistrez d’abord le NSS dans l’espace d’édition.",
                    format!("No staged source exists for {nss_key}"),
                    aurora_core::ErrorSeverity::Warning,
                )
                .with_resource(nss_key.to_string())
                .with_import_stage("nwscript_compile"),
            )
        })?;
        let snapshot = workspace.snapshot()?;
        (
            decode_nwn_text(&nss),
            PathBuf::from(snapshot.root),
            workspace.staged_resource_bytes(&ncs_key)?,
        )
    };
    let requested_include_paths = request
        .include_paths
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let (exact_include_path, compilation_inputs) = prepare_exact_compiler_inputs(
        &state,
        &request.job_id,
        &workspace_root,
        &request.resref,
        source_text.as_bytes(),
        &requested_include_paths,
    )?;
    let config = CompilerConfig {
        executable: PathBuf::from(&request.compiler_path),
        game_install_path: PathBuf::from(&request.game_install_path),
        include_paths: vec![exact_include_path],
    };
    let compiler_sha256 = sha256_file_local(&config.executable)?;
    let mut compilation = compile_nss(&config, &request.resref, &source_text)?;
    if sha256_file_local(&config.executable)? != compiler_sha256 {
        return Err(Box::new(
            AppError::new(
                "NSS_COMPILER_CHANGED_DURING_RUN",
                "Le compilateur NWScript a changé pendant son exécution.",
                format!(
                    "{} changed after its pre-run SHA-256 was recorded",
                    config.executable.display()
                ),
                aurora_core::ErrorSeverity::Error,
            )
            .with_source(config.executable.display().to_string())
            .with_import_stage("nwscript_compile"),
        ));
    }
    if !compilation.success {
        let snapshot = get_open_edit_workspace_snapshot(&state, &request.workspace_id)?;
        return Ok(CompileScriptResult {
            workspace: snapshot,
            compilation,
        });
    }
    let before_bytes = staged_ncs.as_ref().or(source_ncs.as_ref());
    let before_sha256 = before_bytes.map(|bytes| hex::encode(Sha256::digest(bytes)));
    let after_sha256 = hex::encode(Sha256::digest(&compilation.ncs_bytes));
    let command = EditCommand::CompileScript {
        resource: ncs_key.clone(),
        inputs: compilation_inputs,
        compiler_sha256,
        before_sha256,
        after_sha256,
    };
    let snapshot = {
        let mut workspaces = state
            .edit_workspaces
            .lock()
            .expect("edit workspace registry poisoned");
        let workspace = workspaces
            .get_mut(&request.workspace_id)
            .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "nwscript_compile"))?;
        workspace.stage_resource(ncs_key, source_ncs.as_deref(), &compilation.ncs_bytes)?;
        workspace.apply(command)?
    };
    compilation.ncs_bytes.clear();
    Ok(CompileScriptResult {
        workspace: snapshot,
        compilation,
    })
}

fn prepare_exact_compiler_inputs(
    state: &AppState,
    job_id: &str,
    workspace_root: &Path,
    root_resref: &str,
    root_bytes: &[u8],
    requested_include_paths: &[PathBuf],
) -> AppResult<(PathBuf, Vec<ResourceContentDigest>)> {
    for path in requested_include_paths {
        if !path.is_dir() {
            return Err(Box::new(
                AppError::new(
                    "NSS_INCLUDE_PATH_INVALID",
                    "Un dossier d’includes NWScript est invalide.",
                    format!("{} is not a directory", path.display()),
                    aurora_core::ErrorSeverity::Error,
                )
                .with_source(path.display().to_string())
                .with_import_stage("nwscript_compile"),
            ));
        }
    }
    let root_key = ResourceKey::new(root_resref, 2009);
    let mut sources = BTreeMap::from([(root_key.clone(), root_bytes.to_vec())]);
    let root_document = parse_nss(root_bytes, &format!("workspace::{root_key}"))?;
    let mut pending = root_document
        .includes
        .into_iter()
        .map(|include| include.resref)
        .collect::<Vec<_>>();
    let mut visited = BTreeSet::from([root_key.resref.clone()]);
    while let Some(resref) = pending.pop() {
        if !visited.insert(resref.clone()) {
            continue;
        }
        if visited.len() > 1_024 {
            return Err(Box::new(
                AppError::new(
                    "NSS_INCLUDE_LIMIT_EXCEEDED",
                    "La compilation référence trop d’includes.",
                    "The transitive include graph exceeds 1024 scripts.",
                    aurora_core::ErrorSeverity::Error,
                )
                .with_resource(root_key.to_string())
                .with_import_stage("nwscript_compile"),
            ));
        }
        if resref.is_empty()
            || resref.len() > 16
            || !resref
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(Box::new(
                AppError::new(
                    "NSS_INCLUDE_RESREF_INVALID",
                    "Un include NWScript possède un ResRef invalide.",
                    format!("Invalid include ResRef {resref:?}"),
                    aurora_core::ErrorSeverity::Error,
                )
                .with_resource(root_key.to_string())
                .with_import_stage("nwscript_compile"),
            ));
        }
        let key = ResourceKey::new(&resref, 2009);
        let bytes = resolve_exact_include_bytes(
            state,
            job_id,
            workspace_root,
            requested_include_paths,
            &key,
        )?;
        let document = parse_nss(&bytes, &format!("compiler-input::{key}"))?;
        pending.extend(document.includes.into_iter().map(|include| include.resref));
        sources.insert(key, bytes);
    }

    let mut aggregate = Sha256::new();
    let mut inputs = Vec::with_capacity(sources.len());
    for (key, bytes) in &sources {
        let content_sha256 = hex::encode(Sha256::digest(bytes));
        aggregate.update(key.to_string().as_bytes());
        aggregate.update([0]);
        aggregate.update(content_sha256.as_bytes());
        aggregate.update([0xff]);
        inputs.push(ResourceContentDigest {
            resource: key.clone(),
            content_sha256,
        });
    }
    let input_set_sha256 = hex::encode(aggregate.finalize());
    let exact_directory = workspace_root
        .join("compiler-inputs")
        .join(input_set_sha256);
    fs::create_dir_all(&exact_directory).map_err(|error| {
        Box::new(AppError::io(
            "create exact compiler include directory",
            exact_directory.display().to_string(),
            &error,
        ))
    })?;
    for (key, bytes) in sources {
        let path = exact_directory.join(key.file_name());
        if path.is_file() {
            let existing = fs::read(&path).map_err(|error| {
                Box::new(AppError::io(
                    "read exact compiler input",
                    path.display().to_string(),
                    &error,
                ))
            })?;
            if existing == bytes {
                continue;
            }
        }
        fs::write(&path, bytes).map_err(|error| {
            Box::new(AppError::io(
                "write exact compiler input",
                path.display().to_string(),
                &error,
            ))
        })?;
    }
    Ok((exact_directory, inputs))
}

fn resolve_exact_include_bytes(
    state: &AppState,
    job_id: &str,
    workspace_root: &Path,
    requested_include_paths: &[PathBuf],
    key: &ResourceKey,
) -> AppResult<Vec<u8>> {
    let staged = workspace_root.join("resources").join(key.file_name());
    if staged.is_file() {
        return fs::read(&staged).map_err(|error| {
            Box::new(AppError::io(
                "read staged NWScript include",
                staged.display().to_string(),
                &error,
            ))
        });
    }
    for root in requested_include_paths {
        let candidate = root.join(key.file_name());
        if candidate.is_file() {
            return fs::read(&candidate).map_err(|error| {
                Box::new(AppError::io(
                    "read requested NWScript include",
                    candidate.display().to_string(),
                    &error,
                ))
            });
        }
    }
    state.jobs.with_analysis(job_id, |analysis| {
        let resolved = analysis.resource_catalog.get(key).ok_or_else(|| {
            Box::new(
                AppError::new(
                    "NSS_INCLUDE_NOT_RESOLVED",
                    "Un include NWScript requis est introuvable.",
                    format!("Resource Manager cannot resolve {key}"),
                    aurora_core::ErrorSeverity::Error,
                )
                .with_resource(key.to_string())
                .with_import_stage("nwscript_compile"),
            )
        })?;
        ResourceManager::read(&resolved.selected, &AtomicBool::new(false))
    })
}

fn sha256_file_local(path: &Path) -> AppResult<String> {
    let file = File::open(path).map_err(|error| {
        Box::new(AppError::io(
            "open file for SHA-256",
            path.display().to_string(),
            &error,
        ))
    })?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| {
            Box::new(AppError::io(
                "read file for SHA-256",
                path.display().to_string(),
                &error,
            ))
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

#[tauri::command]
pub fn move_area_instance(
    state: State<'_, AppState>,
    request: MoveAreaInstanceRequest,
) -> AppResult<WorkspaceSnapshot> {
    let resource = ResourceKey::new(&request.area, 2023);
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let prefix = format!("{}:", request.area);
    let identity = request.instance_id.strip_prefix(&prefix).ok_or_else(|| {
        Box::new(
            AppError::new(
                "EDIT_AREA_INSTANCE_ID_INVALID",
                "L’identifiant de l’instance ne correspond pas à la zone.",
                format!("{} does not start with {prefix}", request.instance_id),
                aurora_core::ErrorSeverity::Error,
            )
            .with_import_stage("area_edit"),
        )
    })?;
    let (list_label, index) = identity.rsplit_once(':').ok_or_else(|| {
        Box::new(
            AppError::new(
                "EDIT_AREA_INSTANCE_ID_INVALID",
                "L’identifiant de l’instance est invalide.",
                format!("{} has no list/index separator", request.instance_id),
                aurora_core::ErrorSeverity::Error,
            )
            .with_import_stage("area_edit"),
        )
    })?;
    let list_label = list_label.to_owned();
    let index = index.parse::<usize>().map_err(|_| {
        Box::new(
            AppError::new(
                "EDIT_AREA_INSTANCE_ID_INVALID",
                "L’index de l’instance est invalide.",
                format!("{index:?} is not an unsigned integer"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_import_stage("area_edit"),
        )
    })?;
    let command = EditCommand::MoveInstance {
        area: request.area.clone(),
        instance_id: request.instance_id,
        before: request.before,
        after: request.after,
    };
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_edit"))?;
    let current = workspace
        .staged_resource_bytes(&resource)?
        .or_else(|| source_bytes.clone())
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_instance_source"))?;
    let output = edit_area_instance(
        &current,
        &format!("workspace::{resource}"),
        &list_label,
        index,
        request.before,
        request.after,
    )?;
    workspace.stage_resource(resource, source_bytes.as_deref(), &output)?;
    workspace.apply(command)
}

#[tauri::command]
pub fn set_area_tile(
    state: State<'_, AppState>,
    request: SetAreaTileRequest,
) -> AppResult<WorkspaceSnapshot> {
    let resource = ResourceKey::new(&request.area, 2012);
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let command = EditCommand::SetTile {
        area: request.area.clone(),
        x: request.x,
        y: request.y,
        before: request.before,
        after: request.after,
    };
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_edit"))?;
    let current = workspace
        .staged_resource_bytes(&resource)?
        .or_else(|| source_bytes.clone())
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_instance_source"))?;
    let are_document = parse_gff(&current, &format!("workspace::{resource}"))?;
    let width = adapt_area(&request.area, &are_document, None, None).width;
    if width == 0 {
        return Err(Box::new(
            AppError::new(
                "EDIT_AREA_WIDTH_INVALID",
                "La zone ne possède pas de largeur exploitable.",
                format!("{} has a zero Width field", request.area),
                aurora_core::ErrorSeverity::Error,
            )
            .with_import_stage("area_edit"),
        ));
    }
    let tile_index = request
        .y
        .checked_mul(width)
        .and_then(|value| value.checked_add(request.x))
        .ok_or_else(|| {
            Box::new(
                AppError::new(
                    "EDIT_AREA_TILE_INDEX_OVERFLOW",
                    "Les coordonnées de tuile sont invalides.",
                    format!("({}, {}) overflows width {width}", request.x, request.y),
                    aurora_core::ErrorSeverity::Error,
                )
                .with_import_stage("area_edit"),
            )
        })?;
    let output = edit_area_tile(
        &current,
        &format!("workspace::{resource}"),
        tile_index as usize,
        request.before,
        request.after,
    )?;
    workspace.stage_resource(resource, source_bytes.as_deref(), &output)?;
    workspace.apply(command)
}

#[tauri::command]
pub fn edit_area_structure_command(
    state: State<'_, AppState>,
    request: EditAreaStructureRequest,
) -> AppResult<WorkspaceSnapshot> {
    let resource = ResourceKey::new(&request.area, 2023);
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let item_template = match &request.action {
        AreaStructureAction::AddInventoryItem { resref, .. } => {
            let item_resource = ResourceKey::new(resref, 2025);
            let bytes = workspace_or_resolved_resource_bytes(
                &state,
                &request.job_id,
                &request.workspace_id,
                &item_resource,
            )?
            .ok_or_else(|| {
                Box::new(
                    AppError::new(
                        "EDIT_AREA_INVENTORY_TEMPLATE_NOT_RESOLVED",
                        "Le blueprint UTI de l’objet est introuvable.",
                        format!("Resource Manager cannot resolve {item_resource}"),
                        aurora_core::ErrorSeverity::Error,
                    )
                    .with_resource(item_resource.to_string())
                    .with_import_stage("area_inventory"),
                )
            })?;
            Some(parse_gff(
                &bytes,
                &format!("workspace::{}", item_resource.file_name()),
            )?)
        }
        _ => None,
    };
    let operation = serde_json::to_string(&request.action).map_err(|error| {
        Box::new(
            AppError::new(
                "EDIT_AREA_ACTION_INVALID",
                "L’opération de zone n’a pas pu être préparée.",
                format!("cannot serialize area structure action: {error}"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(resource.to_string())
            .with_import_stage("area_structure"),
        )
    })?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_structure"))?;
    let current = workspace
        .staged_resource_bytes(&resource)?
        .or_else(|| source_bytes.clone())
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_structure_source"))?;
    let (output, _) = edit_area_structure(
        &current,
        &format!("workspace::{}", resource.file_name()),
        &request.area,
        &request.action,
        item_template.as_ref(),
    )?;
    let before_sha256 = hex::encode(Sha256::digest(&current));
    let after_sha256 = hex::encode(Sha256::digest(&output));
    workspace.stage_resource(resource.clone(), source_bytes.as_deref(), &output)?;
    workspace.apply(EditCommand::TransformResource {
        resource,
        operation,
        before_sha256,
        after_sha256,
    })
}

#[tauri::command]
pub fn inspect_workspace_area(
    state: State<'_, AppState>,
    request: WorkspaceAreaInspectionRequest,
) -> AppResult<AreaMap> {
    let are_key = ResourceKey::new(&request.area, 2012);
    let git_key = ResourceKey::new(&request.area, 2023);
    let gic_key = ResourceKey::new(&request.area, 2046);
    let are_bytes = workspace_or_resolved_resource_bytes(
        &state,
        &request.job_id,
        &request.workspace_id,
        &are_key,
    )?
    .ok_or_else(|| {
        Box::new(
            AppError::new(
                "AREA_ARE_NOT_RESOLVED",
                "La ressource ARE de la zone est introuvable.",
                format!("Resource Manager cannot resolve {are_key}"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(are_key.to_string())
            .with_import_stage("area_inspection"),
        )
    })?;
    let git_bytes = workspace_or_resolved_resource_bytes(
        &state,
        &request.job_id,
        &request.workspace_id,
        &git_key,
    )?;
    let gic_bytes = workspace_or_resolved_resource_bytes(
        &state,
        &request.job_id,
        &request.workspace_id,
        &gic_key,
    )?;
    let are = parse_gff(&are_bytes, &format!("workspace::{}", are_key.file_name()))?;
    let git = git_bytes
        .as_deref()
        .map(|bytes| parse_gff(bytes, &format!("workspace::{}", git_key.file_name())))
        .transpose()?;
    let gic = gic_bytes
        .as_deref()
        .map(|bytes| parse_gff(bytes, &format!("workspace::{}", gic_key.file_name())))
        .transpose()?;
    Ok(adapt_area(&request.area, &are, git.as_ref(), gic.as_ref()))
}

#[tauri::command]
pub fn build_workspace_module(
    state: State<'_, AppState>,
    request: BuildModuleRequest,
) -> AppResult<ModuleBuildReport> {
    let output_path = PathBuf::from(request.output_path);
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.build_module(&output_path)
    })
}

#[tauri::command]
pub fn deploy_workspace_development(
    state: State<'_, AppState>,
    request: DevelopmentRequest,
) -> AppResult<DevelopmentDeployment> {
    let user_data_path = PathBuf::from(request.user_data_path);
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.deploy_development(&user_data_path)
    })
}

#[tauri::command]
pub fn clean_workspace_development(
    state: State<'_, AppState>,
    request: DevelopmentRequest,
) -> AppResult<DevelopmentCleanupReport> {
    let user_data_path = PathBuf::from(request.user_data_path);
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.clean_development(&user_data_path)
    })
}

#[tauri::command]
pub fn build_workspace_hak(
    state: State<'_, AppState>,
    request: WorkspaceOutputRequest,
) -> AppResult<ModuleBuildReport> {
    let output_path = PathBuf::from(request.output_path);
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.build_hak(&output_path)
    })
}

#[tauri::command]
pub fn export_workspace_sources(
    state: State<'_, AppState>,
    request: WorkspaceOutputRequest,
) -> AppResult<WorkspaceExportManifest> {
    let output_path = PathBuf::from(request.output_path);
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.export_reproducible_sources(&output_path)
    })
}

#[tauri::command]
pub fn edit_workspace_2da(
    state: State<'_, AppState>,
    request: TwoDaEditRequest,
) -> AppResult<TableEditResult<TwoDaTable>> {
    if request.resource.resource_type != 2017 {
        return Err(AppError::invalid_path(
            request.resource.to_string(),
            "2DA edit requires resource type 2017",
        )
        .into());
    }
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &request.resource)?;
    let resource = request.resource.clone();
    let (workspace, document) = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        let current = workspace
            .staged_resource_bytes(&resource)?
            .or_else(|| source_bytes.clone())
            .ok_or_else(|| {
                AppError::invalid_path(resource.to_string(), "2DA resource is not resolved")
            })?;
        let mut document = parse_2da(&current, &format!("workspace::{}", resource.file_name()))?;
        apply_2da_edit(&mut document, &request.action)?;
        let output = write_2da(&document)?;
        let reopened = parse_2da(&output, &format!("workspace::{}", resource.file_name()))?;
        let before_sha256 = hex::encode(Sha256::digest(&current));
        let after_sha256 = hex::encode(Sha256::digest(&output));
        workspace.stage_resource(resource.clone(), source_bytes.as_deref(), &output)?;
        let snapshot = workspace.apply(EditCommand::TransformResource {
            resource: resource.clone(),
            operation: "edit_2da".into(),
            before_sha256,
            after_sha256,
        })?;
        Ok((snapshot, reopened))
    })?;
    Ok(TableEditResult {
        workspace,
        document,
    })
}

#[tauri::command]
pub fn edit_workspace_tlk(
    state: State<'_, AppState>,
    request: TlkEditRequest,
) -> AppResult<TableEditResult<TalkTable>> {
    if request.resource.resource_type != 2018 {
        return Err(AppError::invalid_path(
            request.resource.to_string(),
            "TLK edit requires resource type 2018",
        )
        .into());
    }
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &request.resource)?;
    let resource = request.resource.clone();
    let (workspace, document) = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        let current = workspace
            .staged_resource_bytes(&resource)?
            .or_else(|| source_bytes.clone())
            .ok_or_else(|| {
                AppError::invalid_path(resource.to_string(), "TLK resource is not resolved")
            })?;
        let mut document = parse_tlk(&current, &format!("workspace::{}", resource.file_name()))?;
        apply_tlk_edit(&mut document, &request.action)?;
        let output = write_tlk(&document)?;
        let reopened = parse_tlk(&output, &format!("workspace::{}", resource.file_name()))?;
        let before_sha256 = hex::encode(Sha256::digest(&current));
        let after_sha256 = hex::encode(Sha256::digest(&output));
        workspace.stage_resource(resource.clone(), source_bytes.as_deref(), &output)?;
        let snapshot = workspace.apply(EditCommand::TransformResource {
            resource: resource.clone(),
            operation: "edit_tlk".into(),
            before_sha256,
            after_sha256,
        })?;
        Ok((snapshot, reopened))
    })?;
    Ok(TableEditResult {
        workspace,
        document,
    })
}

#[tauri::command]
pub fn edit_workspace_module_dependencies(
    state: State<'_, AppState>,
    request: ModuleDependencyEditRequest,
) -> AppResult<ModuleDependencyEditResult> {
    let resource = ResourceKey::new("module", 2014);
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let (workspace, document) = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        let current = workspace
            .staged_resource_bytes(&resource)?
            .or_else(|| source_bytes.clone())
            .ok_or_else(|| {
                AppError::invalid_path(resource.to_string(), "module.ifo is not resolved")
            })?;
        let (output, document) = edit_module_dependencies(
            &current,
            "workspace::module.ifo",
            &request.hak_files,
            request.custom_tlk.as_deref(),
        )?;
        let before_sha256 = hex::encode(Sha256::digest(&current));
        let after_sha256 = hex::encode(Sha256::digest(&output));
        workspace.stage_resource(resource.clone(), source_bytes.as_deref(), &output)?;
        let snapshot = workspace.apply(EditCommand::TransformResource {
            resource: resource.clone(),
            operation: "edit_module_dependencies".into(),
            before_sha256,
            after_sha256,
        })?;
        Ok((snapshot, document))
    })?;
    Ok(ModuleDependencyEditResult {
        workspace,
        document,
    })
}

#[tauri::command]
pub fn list_workspace_build_profiles(
    state: State<'_, AppState>,
    request: EditWorkspaceRequest,
) -> AppResult<Vec<ModuleBuildProfile>> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.list_build_profiles()
    })
}

#[tauri::command]
pub fn save_workspace_build_profile(
    state: State<'_, AppState>,
    request: BuildProfileRequest,
) -> AppResult<Vec<ModuleBuildProfile>> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.save_build_profile(request.profile)
    })
}

#[tauri::command]
pub fn verify_workspace_reproducible_build(
    state: State<'_, AppState>,
    request: BuildProfileRequest,
) -> AppResult<ReproducibleBuildVerification> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.verify_reproducible_build(&request.profile)
    })
}

#[tauri::command]
pub fn run_workspace_build_profile(
    state: State<'_, AppState>,
    request: RunBuildProfileRequest,
) -> AppResult<BuildProfileRunReport> {
    validate_build_profile(&request.profile)?;
    let output_root = PathBuf::from(&request.output_directory);
    if !output_root.is_dir() {
        return Err(AppError::invalid_path(
            request.output_directory,
            "build output directory does not exist",
        )
        .into());
    }
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        let warnings = workspace.validate_build_profile_context(&request.profile)?;
        let build = workspace.build_module(&output_root.join(&request.profile.output_name))?;
        let deployment = if request.profile.deploy_development {
            let user_data = request.user_data_path.as_deref().ok_or_else(|| {
                AppError::invalid_path("userDataPath", "profile requires a NWN user data directory")
            })?;
            Some(workspace.deploy_development(Path::new(user_data))?)
        } else {
            None
        };
        Ok(BuildProfileRunReport {
            profile: request.profile,
            build,
            deployment,
            warnings,
        })
    })
}

#[tauri::command]
pub fn inspect_git_workspace(request: GitWorkspaceRequest) -> AppResult<GitWorkspaceStatus> {
    inspect_git_repository(Path::new(&request.root))
}

#[tauri::command]
pub fn list_workspace_launch_profiles(
    state: State<'_, AppState>,
    request: EditWorkspaceRequest,
) -> AppResult<Vec<NwnLaunchProfile>> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.list_launch_profiles()
    })
}

#[tauri::command]
pub fn save_workspace_launch_profile(
    state: State<'_, AppState>,
    request: LaunchProfileRequest,
) -> AppResult<Vec<NwnLaunchProfile>> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.save_launch_profile(request.profile)
    })
}

#[tauri::command]
pub fn launch_workspace_test_profile(
    state: State<'_, AppState>,
    request: LaunchProfileRequest,
) -> AppResult<NwnLaunchReport> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.launch_nwn_profile(&request.profile)
    })
}

#[tauri::command]
pub fn inspect_aurora_workspace(request: AuroraSyncRequest) -> AppResult<AuroraSyncManifest> {
    scan_aurora_workspace(&PathBuf::from(request.root))
}

#[tauri::command]
pub fn plan_aurora_workspace_sync(
    state: State<'_, AppState>,
    request: AuroraSyncPlanRequest,
) -> AppResult<AuroraSyncPlan> {
    build_aurora_sync_plan(
        &state,
        &request.job_id,
        &request.workspace_id,
        &PathBuf::from(request.root),
    )
}

#[tauri::command]
pub fn apply_aurora_workspace_sync(
    state: State<'_, AppState>,
    request: AuroraSyncApplyRequest,
) -> AppResult<AuroraSyncReport> {
    if request.actions.is_empty() || request.actions.len() > 1_000 {
        return Err(Box::new(AppError::new(
            "EDIT_AURORA_SYNC_ACTIONS_INVALID",
            "Sélectionnez entre 1 et 1000 opérations de synchronisation.",
            format!("received {} synchronization actions", request.actions.len()),
            aurora_core::ErrorSeverity::Warning,
        )));
    }
    let root = PathBuf::from(&request.root);
    let preview = build_aurora_sync_plan(&state, &request.job_id, &request.workspace_id, &root)?;
    let by_resource = preview
        .entries
        .iter()
        .map(|entry| (entry.resource.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut selected = BTreeSet::new();
    for action in &request.actions {
        if !selected.insert(action.resource.clone()) {
            return Err(Box::new(AppError::new(
                "EDIT_AURORA_SYNC_ACTION_DUPLICATE",
                "Une ressource ne peut être synchronisée qu’une fois par opération.",
                action.resource.to_string(),
                aurora_core::ErrorSeverity::Warning,
            )));
        }
        let entry = by_resource.get(&action.resource).ok_or_else(|| {
            Box::new(AppError::new(
                "EDIT_AURORA_SYNC_RESOURCE_UNKNOWN",
                "La ressource sélectionnée n’est plus présente dans le plan.",
                action.resource.to_string(),
                aurora_core::ErrorSeverity::Error,
            ))
        })?;
        verify_sync_action(entry, action)?;
        if entry.state == AuroraSyncState::Identical {
            return Err(Box::new(AppError::new(
                "EDIT_AURORA_SYNC_ACTION_REDUNDANT",
                "La ressource est déjà identique des deux côtés.",
                action.resource.to_string(),
                aurora_core::ErrorSeverity::Warning,
            )));
        }
    }
    if request.actions.iter().any(|action| {
        action.direction == AuroraSyncDirection::PushToToolset
            && action.resource.resource_type == 2009
    }) {
        with_edit_workspace(&state, &request.workspace_id, |workspace| {
            workspace.validate_compiled_scripts()
        })?;
    }

    let mut applied = Vec::with_capacity(request.actions.len());
    let mut backups = Vec::new();
    for action in &request.actions {
        let entry = by_resource
            .get(&action.resource)
            .expect("all synchronization actions were validated");
        match action.direction {
            AuroraSyncDirection::PullFromToolset => {
                let incoming = read_aurora_workspace_file(&root, &entry.relative_path)?;
                let current = workspace_sync_resource_bytes(
                    &state,
                    &request.job_id,
                    &request.workspace_id,
                    &action.resource,
                )?;
                if current
                    .as_deref()
                    .map(|bytes| hex::encode(Sha256::digest(bytes)))
                    != action.expected_workspace_sha256
                    || incoming
                        .as_deref()
                        .map(|bytes| hex::encode(Sha256::digest(bytes)))
                        != action.expected_toolset_sha256
                {
                    return Err(Box::new(AppError::new(
                        "EDIT_AURORA_SYNC_PRECONDITION_FAILED",
                        "La ressource a changé depuis la prévisualisation.",
                        action.resource.to_string(),
                        aurora_core::ErrorSeverity::Error,
                    )));
                }
                match (current.as_deref(), incoming.as_deref()) {
                    (Some(current), Some(incoming)) => {
                        let before_sha256 = hex::encode(Sha256::digest(current));
                        let after_sha256 = hex::encode(Sha256::digest(incoming));
                        with_edit_workspace(&state, &request.workspace_id, |workspace| {
                            workspace.stage_resource(
                                action.resource.clone(),
                                Some(current),
                                incoming,
                            )?;
                            workspace.apply(EditCommand::TransformResource {
                                resource: action.resource.clone(),
                                operation: "aurora_sync_pull".to_owned(),
                                before_sha256,
                                after_sha256,
                            })
                        })?;
                    }
                    (None, Some(incoming)) => {
                        with_edit_workspace(&state, &request.workspace_id, |workspace| {
                            workspace.create_resource(action.resource.clone(), incoming)
                        })?;
                    }
                    (Some(current), None) => {
                        with_edit_workspace(&state, &request.workspace_id, |workspace| {
                            workspace.delete_resource(action.resource.clone(), Some(current))
                        })?;
                    }
                    (None, None) => unreachable!("identical missing resources are rejected"),
                }
                applied.push(AuroraSyncAppliedFile {
                    resource: action.resource.clone(),
                    direction: action.direction,
                    sha256: incoming
                        .as_deref()
                        .map(|bytes| hex::encode(Sha256::digest(bytes))),
                });
            }
            AuroraSyncDirection::PushToToolset => {
                let current = workspace_sync_resource_bytes(
                    &state,
                    &request.job_id,
                    &request.workspace_id,
                    &action.resource,
                )?;
                if current
                    .as_deref()
                    .map(|bytes| hex::encode(Sha256::digest(bytes)))
                    != action.expected_workspace_sha256
                {
                    return Err(Box::new(AppError::new(
                        "EDIT_AURORA_SYNC_PRECONDITION_FAILED",
                        "La ressource OpenNever a changé depuis la prévisualisation.",
                        action.resource.to_string(),
                        aurora_core::ErrorSeverity::Error,
                    )));
                }
                if let Some(backup) =
                    write_aurora_workspace_file(&root, &entry.relative_path, current.as_deref())?
                {
                    backups.push(backup);
                }
                applied.push(AuroraSyncAppliedFile {
                    resource: action.resource.clone(),
                    direction: action.direction,
                    sha256: current
                        .as_deref()
                        .map(|bytes| hex::encode(Sha256::digest(bytes))),
                });
            }
        }
    }

    let current_plan =
        build_aurora_sync_plan(&state, &request.job_id, &request.workspace_id, &root)?;
    let old_baseline = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.load_aurora_sync_baseline(&root)
    })?;
    let mut next_baseline = baseline_from_plan(&current_plan);
    let old_entries = old_baseline
        .map(|baseline| {
            baseline
                .entries
                .into_iter()
                .map(|entry| (entry.resource.clone(), entry))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    next_baseline.entries.retain(|entry| {
        selected.contains(&entry.resource)
            || current_plan
                .entries
                .iter()
                .find(|current| current.resource == entry.resource)
                .is_some_and(|current| current.state == AuroraSyncState::Identical)
            || old_entries.contains_key(&entry.resource)
    });
    for entry in &mut next_baseline.entries {
        let resolved = selected.contains(&entry.resource)
            || current_plan
                .entries
                .iter()
                .find(|current| current.resource == entry.resource)
                .is_some_and(|current| current.state == AuroraSyncState::Identical);
        if !resolved && let Some(old) = old_entries.get(&entry.resource) {
            *entry = old.clone();
        }
    }
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace
            .save_aurora_sync_baseline(&root, &next_baseline)
            .map(|_| ())
    })?;
    backups.sort();
    backups.dedup();
    let plan = build_aurora_sync_plan(&state, &request.job_id, &request.workspace_id, &root)?;
    Ok(AuroraSyncReport {
        schema_version: 2,
        root: plan.root.clone(),
        applied,
        backups,
        plan,
        workspace: get_open_edit_workspace_snapshot(&state, &request.workspace_id)?,
    })
}

#[tauri::command]
pub fn validate_walkmesh_draft(request: WalkmeshValidationRequest) -> WalkmeshValidation {
    validate_walkmesh_for_kind(&request.draft, request.kind)
}

#[tauri::command]
pub fn transform_walkmesh_draft(
    request: WalkmeshTransformRequest,
) -> AppResult<WalkmeshTransformResult> {
    let mut draft = request.draft;
    let validation = apply_walkmesh_operation(&mut draft, &request.operation)?;
    Ok(WalkmeshTransformResult { draft, validation })
}

#[tauri::command]
pub fn inspect_workspace_walkmesh(
    state: State<'_, AppState>,
    request: WalkmeshInspectionRequest,
) -> AppResult<WalkmeshDocument> {
    let resource = ResourceKey::new(&request.resref, request.kind.resource_type());
    let bytes = workspace_or_resolved_resource_bytes(
        &state,
        &request.job_id,
        &request.workspace_id,
        &resource,
    )?
    .ok_or_else(|| {
        Box::new(
            AppError::new(
                "EDIT_WALKMESH_NOT_RESOLVED",
                "Le walkmesh demande est introuvable.",
                format!("Resource Manager cannot resolve {resource}"),
                aurora_core::ErrorSeverity::Warning,
            )
            .with_resource(resource.to_string())
            .with_import_stage("walkmesh"),
        )
    })?;
    inspect_walkmesh(&request.resref, request.kind, &bytes)
}

#[tauri::command]
pub fn save_workspace_walkmesh(
    state: State<'_, AppState>,
    request: SaveWalkmeshRequest,
) -> AppResult<WalkmeshEditResult> {
    let resource = ResourceKey::new(&request.resref, request.kind.resource_type());
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let output = serialize_walkmesh_ascii(&request.resref, request.kind, &request.draft)?;
    let document = inspect_walkmesh(&request.resref, request.kind, &output)?;
    let operation = format!(
        "walkmesh_full_replace:{}:{}",
        request.resref,
        match request.kind {
            WalkmeshKind::Wok => "wok",
            WalkmeshKind::Pwk => "pwk",
            WalkmeshKind::Dwk => "dwk",
        }
    );
    let workspace = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        let staged = workspace.staged_resource_bytes(&resource)?;
        if staged.is_none() && source_bytes.is_some() && !request.replace_existing {
            return Err(Box::new(
                AppError::new(
                    "EDIT_WALKMESH_REPLACEMENT_REQUIRES_CONFIRMATION",
                    "Cette ressource existe deja. Confirmez son remplacement complet.",
                    format!("Refusing implicit full replacement of existing walkmesh {resource}"),
                    aurora_core::ErrorSeverity::Warning,
                )
                .with_resource(resource.to_string())
                .with_import_stage("walkmesh"),
            ));
        }
        match staged {
            None if source_bytes.is_none() => workspace.create_resource(resource.clone(), &output),
            current => {
                let before = current.or_else(|| source_bytes.clone()).ok_or_else(|| {
                    Box::new(
                        AppError::new(
                            "EDIT_WALKMESH_SOURCE_MISSING",
                            "Le walkmesh courant est introuvable.",
                            format!("No current bytes are available for {resource}"),
                            aurora_core::ErrorSeverity::Error,
                        )
                        .with_resource(resource.to_string())
                        .with_import_stage("walkmesh"),
                    )
                })?;
                let before_sha256 = hex::encode(Sha256::digest(&before));
                let after_sha256 = hex::encode(Sha256::digest(&output));
                workspace.stage_resource(resource.clone(), source_bytes.as_deref(), &output)?;
                workspace.apply(EditCommand::TransformResource {
                    resource: resource.clone(),
                    operation: operation.clone(),
                    before_sha256,
                    after_sha256,
                })
            }
        }
    })?;
    Ok(WalkmeshEditResult {
        workspace,
        document,
    })
}

#[tauri::command]
pub fn preview_ai_change_set(
    state: State<'_, AppState>,
    request: AiChangeSetRequest,
) -> AppResult<AiChangeSetPreview> {
    let source_resources = ai_source_resources(&state, &request.job_id, &request.change_set)?;
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.preview_controlled_ai_change_set(&request.change_set, &source_resources)
    })
}

#[tauri::command]
pub async fn request_ai_change_set(
    state: State<'_, AppState>,
    request: AiProviderRequest,
) -> AppResult<AiProviderProposal> {
    if !request.consent.allow_network {
        return Err(ai_error(
            "EDIT_AI_NETWORK_CONSENT_REQUIRED",
            "Activez explicitement l’accès réseau avant de contacter le fournisseur IA.",
            "AI network access is disabled by default",
        ));
    }
    if request.prompt.trim().is_empty() || request.prompt.len() > 16 * 1024 {
        return Err(ai_error(
            "EDIT_AI_PROMPT_INVALID",
            "La demande doit contenir entre 1 et 16 Kio.",
            "AI prompt is empty or exceeds 16 KiB",
        ));
    }
    if request.model.trim().is_empty() || request.model.len() > 128 {
        return Err(ai_error(
            "EDIT_AI_MODEL_INVALID",
            "Choisissez explicitement un modèle valide.",
            "AI model is empty or exceeds 128 bytes",
        ));
    }
    if request.selected_resources.len() > 8 {
        return Err(ai_error(
            "EDIT_AI_CONTEXT_TOO_LARGE",
            "Sélectionnez au maximum huit ressources pour une demande.",
            "AI context contains more than eight selected resources",
        ));
    }
    let endpoint = validated_ai_endpoint(&request.endpoint)?;
    let endpoint_origin = endpoint.origin().ascii_serialization();
    let mut warnings = Vec::new();
    let mut shared_resources = Vec::new();
    if request.consent.include_resource_contents {
        for resource in unique_resource_keys(&request.selected_resources) {
            let bytes = workspace_or_resolved_resource_bytes(
                &state,
                &request.job_id,
                &request.workspace_id,
                &resource,
            )?
            .ok_or_else(|| {
                ai_error(
                    "EDIT_AI_CONTEXT_RESOURCE_MISSING",
                    "Une ressource sélectionnée n’est plus disponible.",
                    format!("selected AI context resource is missing: {resource}"),
                )
            })?;
            let content = ai_resource_context(&resource, &bytes)?;
            let encoded = serde_json::to_vec(&content).map_err(|error| {
                ai_error(
                    "EDIT_AI_CONTEXT_SERIALIZE_FAILED",
                    "Le contexte IA n’a pas pu être préparé.",
                    error.to_string(),
                )
            })?;
            if encoded.len() > 64 * 1024 {
                warnings.push(format!(
                    "{} dépasse 64 Kio et n’a pas été transmis.",
                    resource.file_name()
                ));
                continue;
            }
            shared_resources.push(serde_json::json!({
                "resource": resource,
                "sha256": hex::encode(Sha256::digest(&bytes)),
                "content": content,
            }));
        }
    } else if !request.selected_resources.is_empty() {
        warnings.push(
            "Les ressources sélectionnées n’ont pas été transmises faute de consentement."
                .to_owned(),
        );
    }
    let module_metadata = if request.consent.include_module_metadata {
        let snapshot = get_open_edit_workspace_snapshot(&state, &request.workspace_id)?;
        Some(serde_json::json!({
            "sourceSha256": snapshot.source.sha256,
            "sourceSizeBytes": snapshot.source.size_bytes,
            "workspaceCursor": snapshot.cursor,
            "modifiedResourceCount": snapshot.modified_resources.len(),
        }))
    } else {
        None
    };

    let system_prompt = concat!(
        "You are a controlled Neverwinter Nights editing assistant. Return one JSON object only ",
        "with {\"summary\":string,\"commands\":array}. The only allowed commands are ",
        "{\"kind\":\"set_field\",\"resource\":{\"resref\":string,\"resourceType\":number},",
        "\"path\":string,\"before\":typedGffValue,\"after\":typedGffValue} and ",
        "{\"kind\":\"replace_text\",\"resource\":{\"resref\":string,\"resourceType\":2009},",
        "\"before\":string,\"after\":string}. Never invent a before value: copy it exactly from context. ",
        "Do not output markdown, prose outside the JSON, binary data, paths, secrets, or commands for tools. ",
        "Limit the proposal to 32 operations. A human will preview and explicitly confirm every proposal."
    );
    let user_payload = serde_json::json!({
        "request": request.prompt,
        "moduleMetadata": module_metadata,
        "resources": shared_resources,
    });
    let body = serde_json::json!({
        "model": request.model,
        "temperature": 0,
        "response_format": { "type": "json_object" },
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_payload.to_string() }
        ]
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|error| {
            ai_error(
                "EDIT_AI_CLIENT_FAILED",
                "Le client réseau IA n’a pas pu être initialisé.",
                error.to_string(),
            )
        })?;
    let mut builder = client.post(endpoint).json(&body);
    if let Some(api_key) = request.api_key.as_deref().filter(|value| !value.is_empty()) {
        builder = builder.bearer_auth(api_key);
    }
    let response = builder.send().await.map_err(|error| {
        ai_error(
            "EDIT_AI_PROVIDER_UNREACHABLE",
            "Le fournisseur IA n’a pas répondu.",
            error.without_url().to_string(),
        )
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(ai_error(
            "EDIT_AI_PROVIDER_REJECTED",
            "Le fournisseur IA a refusé la demande.",
            format!("AI provider returned HTTP {status}"),
        ));
    }
    if response
        .content_length()
        .is_some_and(|length| length > 1024 * 1024)
    {
        return Err(ai_error(
            "EDIT_AI_RESPONSE_TOO_LARGE",
            "La réponse IA dépasse la limite de sécurité.",
            "AI provider declared a response larger than 1 MiB",
        ));
    }
    let bytes = response.bytes().await.map_err(|error| {
        ai_error(
            "EDIT_AI_RESPONSE_READ_FAILED",
            "La réponse IA n’a pas pu être lue.",
            error.to_string(),
        )
    })?;
    if bytes.len() > 1024 * 1024 {
        return Err(ai_error(
            "EDIT_AI_RESPONSE_TOO_LARGE",
            "La réponse IA dépasse la limite de sécurité.",
            format!("AI provider returned {} bytes", bytes.len()),
        ));
    }
    let envelope: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        ai_error(
            "EDIT_AI_RESPONSE_INVALID",
            "Le fournisseur IA n’a pas renvoyé un JSON compatible.",
            error.to_string(),
        )
    })?;
    let content = envelope
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            ai_error(
                "EDIT_AI_RESPONSE_INVALID",
                "La réponse du fournisseur ne contient aucune proposition textuelle.",
                "missing choices[0].message.content in OpenAI-compatible response",
            )
        })?;
    let change_set = decode_ai_change_set(content)?;
    let proposal_sha256 = ai_change_set_sha256(&change_set)?;
    let source_resources = ai_source_resources(&state, &request.job_id, &change_set)?;
    let preview = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.preview_controlled_ai_change_set(&change_set, &source_resources)
    })?;
    Ok(AiProviderProposal {
        endpoint_origin,
        model: request.model,
        proposal_sha256,
        change_set,
        preview,
        shared_resources: shared_resources.len(),
        warnings,
    })
}

#[tauri::command]
pub fn apply_ai_change_set(
    state: State<'_, AppState>,
    request: ApplyAiChangeSetRequest,
) -> AppResult<AiApplyReport> {
    if !request.confirmed {
        return Err(ai_error(
            "EDIT_AI_CONFIRMATION_REQUIRED",
            "Confirmez explicitement la proposition prévisualisée avant application.",
            "AI proposal application requires confirmed=true",
        ));
    }
    let source_resources = ai_source_resources(&state, &request.job_id, &request.change_set)?;
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.apply_controlled_ai_change_set(
            &request.change_set,
            &request.proposal_sha256,
            &source_resources,
        )
    })
}

#[tauri::command]
pub fn create_new_module(request: CreateNewModuleRequest) -> AppResult<ModuleBuildReport> {
    create_empty_module(
        &PathBuf::from(request.output_path),
        &NewModuleDefinition {
            name: request.name,
            tag: request.tag,
            entry_area: request.entry_area,
            tileset: request.tileset,
        },
    )
}

#[tauri::command]
pub fn get_standard_palette() -> PaletteManifest {
    PaletteManifest::standard()
}

#[tauri::command]
pub fn create_workspace_area(
    state: State<'_, AppState>,
    request: CreateAreaRequest,
) -> AppResult<CreateAreaResult> {
    let resources = create_area_resources(
        &request.resref,
        &request.name,
        &request.tileset,
        request.width,
        request.height,
        request.tile_id,
    )?;
    state.jobs.with_analysis(&request.job_id, |analysis| {
        for resource in &resources {
            if analysis.resource_catalog.get(&resource.key).is_some() {
                return Err(Box::new(
                    AppError::new(
                        "EDIT_AREA_ALREADY_EXISTS",
                        "Une ressource de cette zone existe déjà.",
                        format!(
                            "{} is already resolved by the Resource Manager",
                            resource.key
                        ),
                        aurora_core::ErrorSeverity::Error,
                    )
                    .with_resource(resource.key.to_string())
                    .with_import_stage("area_create"),
                ));
            }
        }
        Ok(())
    })?;
    let are = resources
        .iter()
        .find(|resource| resource.key.resource_type == 2012)
        .ok_or_else(|| generated_area_resource_missing("ARE"))?;
    let git = resources
        .iter()
        .find(|resource| resource.key.resource_type == 2023)
        .ok_or_else(|| generated_area_resource_missing("GIT"))?;
    let gic = resources
        .iter()
        .find(|resource| resource.key.resource_type == 2046)
        .ok_or_else(|| generated_area_resource_missing("GIC"))?;
    let are_document = parse_gff(&are.bytes, &format!("workspace::{}", are.key.file_name()))?;
    let git_document = parse_gff(&git.bytes, &format!("workspace::{}", git.key.file_name()))?;
    let gic_document = parse_gff(&gic.bytes, &format!("workspace::{}", gic.key.file_name()))?;
    let area = adapt_area(
        &request.resref,
        &are_document,
        Some(&git_document),
        Some(&gic_document),
    );
    let workspace = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.create_resources_atomic(&resources)
    })?;
    Ok(CreateAreaResult { workspace, area })
}

#[tauri::command]
pub fn list_workspace_created_areas(
    state: State<'_, AppState>,
    request: ListWorkspaceAreasRequest,
) -> AppResult<Vec<AreaMap>> {
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        let snapshot = workspace.snapshot()?;
        let mut created_resrefs = snapshot
            .modified_resources
            .iter()
            .filter(|resource| {
                resource.resource.resource_type == 2012 && resource.source_sha256.is_none()
            })
            .map(|resource| resource.resource.resref.clone())
            .collect::<Vec<_>>();
        created_resrefs.sort();
        created_resrefs.dedup();
        let mut areas = Vec::with_capacity(created_resrefs.len());
        for resref in created_resrefs {
            let are_key = ResourceKey::new(&resref, 2012);
            let git_key = ResourceKey::new(&resref, 2023);
            let gic_key = ResourceKey::new(&resref, 2046);
            let are_bytes = workspace
                .staged_resource_bytes(&are_key)?
                .ok_or_else(|| generated_area_resource_missing("staged ARE"))?;
            let git_bytes = workspace.staged_resource_bytes(&git_key)?;
            let gic_bytes = workspace.staged_resource_bytes(&gic_key)?;
            let are = parse_gff(&are_bytes, &format!("workspace::{}", are_key.file_name()))?;
            let git = git_bytes
                .as_deref()
                .map(|bytes| parse_gff(bytes, &format!("workspace::{}", git_key.file_name())))
                .transpose()?;
            let gic = gic_bytes
                .as_deref()
                .map(|bytes| parse_gff(bytes, &format!("workspace::{}", gic_key.file_name())))
                .transpose()?;
            areas.push(adapt_area(&resref, &are, git.as_ref(), gic.as_ref()));
        }
        Ok(areas)
    })
}

fn generated_area_resource_missing(kind: &str) -> Box<AppError> {
    Box::new(
        AppError::new(
            "EDIT_AREA_RESOURCE_MISSING",
            "La création de zone n’a pas produit toutes les ressources attendues.",
            format!("Generated area is missing its {kind} resource"),
            aurora_core::ErrorSeverity::Error,
        )
        .with_import_stage("area_create"),
    )
}

#[tauri::command]
pub fn delete_workspace_area(
    state: State<'_, AppState>,
    request: DeleteAreaRequest,
) -> AppResult<WorkspaceSnapshot> {
    let mut resources = Vec::with_capacity(3);
    for resource_type in [2012, 2023, 2046] {
        let key = ResourceKey::new(&request.resref, resource_type);
        let staged = with_edit_workspace(&state, &request.workspace_id, |workspace| {
            workspace.staged_resource_bytes(&key)
        })?;
        let bytes = match staged {
            Some(bytes) => bytes,
            None => resolved_resource_bytes(&state, &request.job_id, &key)?,
        };
        resources.push(aurora_erf::ErfResourceInput { key, bytes });
    }
    with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.delete_resources_atomic(&resources)
    })
}

#[tauri::command]
pub fn add_workspace_area_instance(
    state: State<'_, AppState>,
    request: AddAreaInstanceRequest,
) -> AppResult<AddAreaInstanceResult> {
    let resource = ResourceKey::new(&request.area, 2023);
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_instance_add"))?;
    let current = workspace
        .staged_resource_bytes(&resource)?
        .or_else(|| source_bytes.clone())
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_instance_source"))?;
    let (output, instance_id) = add_area_instance(
        &current,
        &format!("workspace::{resource}"),
        &request.area,
        &request.placement,
    )?;
    let command = EditCommand::AddInstance {
        area: request.area,
        instance_id: instance_id.clone(),
        placement: request.placement,
    };
    workspace.stage_resource(resource, source_bytes.as_deref(), &output)?;
    let snapshot = workspace.apply(command)?;
    Ok(AddAreaInstanceResult {
        workspace: snapshot,
        instance_id,
    })
}

#[tauri::command]
pub fn remove_workspace_area_instance(
    state: State<'_, AppState>,
    request: RemoveAreaInstanceRequest,
) -> AppResult<WorkspaceSnapshot> {
    let resource = ResourceKey::new(&request.area, 2023);
    let source_bytes = try_resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_instance_remove"))?;
    let current = workspace
        .staged_resource_bytes(&resource)?
        .or_else(|| source_bytes.clone())
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "area_instance_source"))?;
    let output = remove_area_instance(
        &current,
        &format!("workspace::{resource}"),
        &request.area,
        &request.instance_id,
    )?;
    let command = EditCommand::RemoveInstance {
        area: request.area,
        instance_id: request.instance_id,
    };
    workspace.stage_resource(resource, source_bytes.as_deref(), &output)?;
    workspace.apply(command)
}

fn validated_ai_endpoint(value: &str) -> AppResult<reqwest::Url> {
    let endpoint = reqwest::Url::parse(value).map_err(|error| {
        ai_error(
            "EDIT_AI_ENDPOINT_INVALID",
            "L’adresse du fournisseur IA n’est pas valide.",
            error.to_string(),
        )
    })?;
    if !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(ai_error(
            "EDIT_AI_ENDPOINT_INVALID",
            "L’adresse IA ne doit contenir ni identifiants ni fragment.",
            "AI endpoint contains userinfo or a URL fragment",
        ));
    }
    let host = endpoint.host_str().unwrap_or_default();
    let local = host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.ends_with(".localhost");
    if endpoint.scheme() != "https" && !(endpoint.scheme() == "http" && local) {
        return Err(ai_error(
            "EDIT_AI_ENDPOINT_INSECURE",
            "Utilisez HTTPS, ou HTTP uniquement pour un modèle local.",
            format!("unsupported AI endpoint scheme for host {host}"),
        ));
    }
    Ok(endpoint)
}

fn decode_ai_change_set(content: &str) -> AppResult<AiChangeSet> {
    let trimmed = content.trim();
    if trimmed.starts_with("```") {
        return Err(ai_error(
            "EDIT_AI_RESPONSE_INVALID",
            "La réponse IA contient du Markdown au lieu du JSON strict attendu.",
            "AI response is wrapped in a Markdown code fence",
        ));
    }
    serde_json::from_str::<AiChangeSet>(trimmed).map_err(|error| {
        ai_error(
            "EDIT_AI_CHANGE_SET_INVALID",
            "La proposition IA ne respecte pas le contrat d’opérations typées.",
            error.to_string(),
        )
    })
}

fn ai_resource_context(resource: &ResourceKey, bytes: &[u8]) -> AppResult<serde_json::Value> {
    if resource.resource_type == 2009 {
        return Ok(serde_json::json!({
            "kind": "nss",
            "text": decode_nwn_text(bytes),
        }));
    }
    if is_gff(resource.resource_type) {
        let document = parse_gff(bytes, &format!("ai-context::{resource}"))?;
        return serde_json::to_value(document).map_err(|error| {
            ai_error(
                "EDIT_AI_CONTEXT_SERIALIZE_FAILED",
                "La ressource GFF sélectionnée n’a pas pu être préparée.",
                error.to_string(),
            )
        });
    }
    Err(ai_error(
        "EDIT_AI_CONTEXT_RESOURCE_UNSUPPORTED",
        "Seuls les GFF et scripts NSS peuvent être transmis au fournisseur IA.",
        format!("unsupported AI context resource {resource}"),
    ))
}

fn unique_resource_keys(resources: &[ResourceKey]) -> Vec<ResourceKey> {
    resources
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn ai_source_resources(
    state: &AppState,
    job_id: &str,
    change_set: &AiChangeSet,
) -> AppResult<BTreeMap<String, Vec<u8>>> {
    ai_change_set_sha256(change_set)?;
    let resources = change_set
        .commands
        .iter()
        .filter_map(|command| match command {
            EditCommand::SetField { resource, .. } | EditCommand::ReplaceText { resource, .. } => {
                Some(resource.clone())
            }
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    resources
        .into_iter()
        .map(|resource| {
            resolved_resource_bytes(state, job_id, &resource)
                .map(|bytes| (resource.to_string(), bytes))
        })
        .collect()
}

fn ai_error(
    code: &str,
    user_message: impl Into<String>,
    technical_message: impl Into<String>,
) -> Box<AppError> {
    Box::new(
        AppError::new(
            code,
            user_message,
            technical_message,
            aurora_core::ErrorSeverity::Error,
        )
        .with_import_stage("ai_assistant"),
    )
}

fn resolved_resource_bytes(
    state: &AppState,
    job_id: &str,
    resource: &ResourceKey,
) -> AppResult<Vec<u8>> {
    try_resolved_resource_bytes(state, job_id, resource)?.ok_or_else(|| {
        Box::new(
            AppError::new(
                "EDIT_RESOURCE_NOT_RESOLVED",
                "La ressource à modifier n’est pas résolue.",
                format!("Resource Manager has no selected version for {resource}"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(resource.to_string())
            .with_import_stage("edit_resource"),
        )
    })
}

fn try_resolved_resource_bytes(
    state: &AppState,
    job_id: &str,
    resource: &ResourceKey,
) -> AppResult<Option<Vec<u8>>> {
    state.jobs.with_analysis(job_id, |analysis| {
        let Some(resolved) = analysis.resource_catalog.get(resource) else {
            return Ok(None);
        };
        ResourceManager::read(&resolved.selected, &AtomicBool::new(false)).map(Some)
    })
}

fn workspace_or_resolved_resource_bytes(
    state: &AppState,
    job_id: &str,
    workspace_id: &str,
    resource: &ResourceKey,
) -> AppResult<Option<Vec<u8>>> {
    let staged = with_edit_workspace(state, workspace_id, |workspace| {
        workspace.staged_resource_bytes(resource)
    })?;
    match staged {
        Some(bytes) => Ok(Some(bytes)),
        None => try_resolved_resource_bytes(state, job_id, resource),
    }
}

fn workspace_sync_resource_bytes(
    state: &AppState,
    job_id: &str,
    workspace_id: &str,
    resource: &ResourceKey,
) -> AppResult<Option<Vec<u8>>> {
    let snapshot = get_open_edit_workspace_snapshot(state, workspace_id)?;
    if snapshot
        .deleted_resources
        .iter()
        .any(|deleted| deleted == resource)
    {
        return Ok(None);
    }
    workspace_or_resolved_resource_bytes(state, job_id, workspace_id, resource)
}

fn build_aurora_sync_plan(
    state: &AppState,
    job_id: &str,
    workspace_id: &str,
    root: &Path,
) -> AppResult<AuroraSyncPlan> {
    let toolset = scan_aurora_workspace(root)?;
    let snapshot = get_open_edit_workspace_snapshot(state, workspace_id)?;
    let deleted = snapshot
        .deleted_resources
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let modified = snapshot
        .modified_resources
        .iter()
        .map(|resource| (resource.resource.clone(), resource))
        .collect::<BTreeMap<_, _>>();
    let mut keys = BTreeSet::new();
    for file in &toolset.files {
        keys.insert(resource_key_from_aurora_path(&file.name)?);
    }
    keys.extend(modified.keys().cloned());
    keys.extend(deleted.iter().cloned());
    let mut workspace_files = Vec::with_capacity(keys.len());
    for resource in keys {
        if deleted.contains(&resource) {
            workspace_files.push(AuroraSyncWorkspaceFile {
                resource,
                sha256: None,
                size_bytes: None,
            });
            continue;
        }
        if let Some(modified) = modified.get(&resource) {
            workspace_files.push(AuroraSyncWorkspaceFile {
                resource,
                sha256: Some(modified.output_sha256.clone()),
                size_bytes: Some(modified.size_bytes),
            });
            continue;
        }
        let bytes = workspace_or_resolved_resource_bytes(state, job_id, workspace_id, &resource)?;
        workspace_files.push(AuroraSyncWorkspaceFile {
            resource,
            sha256: bytes
                .as_deref()
                .map(|bytes| hex::encode(Sha256::digest(bytes))),
            size_bytes: bytes.as_ref().map(|bytes| bytes.len() as u64),
        });
    }
    let baseline = with_edit_workspace(state, workspace_id, |workspace| {
        workspace.load_aurora_sync_baseline(root)
    })?;
    compare_aurora_sync(&toolset, &workspace_files, baseline.as_ref())
}

fn edit_workspace_not_open(workspace_id: &str, stage: &str) -> Box<AppError> {
    Box::new(
        AppError::new(
            "EDIT_WORKSPACE_NOT_OPEN",
            "L’espace d’édition n’est pas ouvert.",
            format!("No open edit workspace has id {workspace_id}"),
            aurora_core::ErrorSeverity::Warning,
        )
        .with_import_stage(stage),
    )
}

fn get_open_edit_workspace_snapshot(
    state: &AppState,
    workspace_id: &str,
) -> AppResult<WorkspaceSnapshot> {
    let workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    workspaces
        .get(workspace_id)
        .ok_or_else(|| edit_workspace_not_open(workspace_id, "edit_workspace"))?
        .snapshot()
}

fn with_edit_workspace<T>(
    state: &AppState,
    workspace_id: &str,
    operation: impl FnOnce(&mut EditWorkspace) -> AppResult<T>,
) -> AppResult<T> {
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces.get_mut(workspace_id).ok_or_else(|| {
        Box::new(
            AppError::new(
                "EDIT_WORKSPACE_NOT_OPEN",
                "L’espace d’édition n’est pas ouvert.",
                format!("No open edit workspace has id {workspace_id}"),
                aurora_core::ErrorSeverity::Warning,
            )
            .with_import_stage("edit_workspace"),
        )
    })?;
    operation(workspace)
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
    let key = aurora_core::ResourceKey::new(&request.resref, request.resource_type);
    if let Some(workspace_id) = request.workspace_id.as_deref()
        && let Some(bytes) = with_edit_workspace(&state, workspace_id, |workspace| {
            workspace.staged_resource_bytes(&key)
        })?
    {
        return parse_resource_inspection(&key, &bytes, &format!("workspace::{}", key.file_name()));
    }
    state.jobs.with_analysis(&request.job_id, |analysis| {
        let resource = analysis.resource_catalog.get(&key).ok_or_else(|| {
            AppError::invalid_path(key.to_string(), "resource not found in the indexed catalog")
        })?;
        let bytes = ResourceManager::read(&resource.selected, &AtomicBool::new(false))?;
        let source = format!("{}::{}", resource.selected.source_path, key);
        parse_resource_inspection(&key, &bytes, &source)
    })
}

fn parse_resource_inspection(
    key: &ResourceKey,
    bytes: &[u8],
    source: &str,
) -> AppResult<ResourceInspection> {
    match key.resource_type {
        2017 => Ok(ResourceInspection::TwoDa(parse_2da(bytes, source)?)),
        2018 => Ok(ResourceInspection::Tlk(parse_tlk(bytes, source)?)),
        resource_type if is_gff(resource_type) => {
            Ok(ResourceInspection::Gff(parse_gff(bytes, source)?))
        }
        _ => {
            let preview_size = bytes.len().min(256);
            Ok(ResourceInspection::Binary(BinaryInspection {
                size: bytes.len(),
                sha256: hex::encode(Sha256::digest(bytes)),
                hex_preview: hex::encode_upper(&bytes[..preview_size]),
                truncated: preview_size < bytes.len(),
            }))
        }
    }
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
    if let Some(workspace_id) = request.workspace_id.as_deref() {
        let resource = ResourceKey::new(&request.resref, 2029);
        if let Some(bytes) = with_edit_workspace(&state, workspace_id, |workspace| {
            workspace.staged_resource_bytes(&resource)
        })? {
            let raw = parse_gff(&bytes, &format!("workspace::{}", resource.file_name()))?;
            return dialogue_graph_with_indexed_references(&state, &request.job_id, resource, raw);
        }
    }
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
pub fn edit_dialogue_field(
    state: State<'_, AppState>,
    request: DialogueEditRequest,
) -> AppResult<DialogueEditResult> {
    let resource = ResourceKey::new(&request.resref, 2029);
    let source_bytes = resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let (workspace_snapshot, raw) = {
        let mut workspaces = state
            .edit_workspaces
            .lock()
            .expect("edit workspace registry poisoned");
        let workspace = workspaces
            .get_mut(&request.workspace_id)
            .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "dialogue_edit"))?;
        let current = workspace
            .staged_resource_bytes(&resource)?
            .unwrap_or_else(|| source_bytes.clone());
        let (output, raw) = edit_gff_field(
            &current,
            &format!("workspace::{}", resource.file_name()),
            &request.path,
            &request.before,
            &request.after,
        )?;
        workspace.stage_resource(resource.clone(), Some(&source_bytes), &output)?;
        let snapshot = workspace.apply(EditCommand::SetField {
            resource: resource.clone(),
            path: request.path,
            before: request.before,
            after: request.after,
        })?;
        (snapshot, raw)
    };
    let graph = dialogue_graph_with_indexed_references(&state, &request.job_id, resource, raw)?;
    Ok(DialogueEditResult {
        workspace: workspace_snapshot,
        graph,
    })
}

#[tauri::command]
pub fn edit_dialogue_structure_command(
    state: State<'_, AppState>,
    request: DialogueStructureRequest,
) -> AppResult<DialogueEditResult> {
    let resource = ResourceKey::new(&request.resref, 2029);
    let source_bytes = resolved_resource_bytes(&state, &request.job_id, &resource)?;
    let operation = serde_json::to_string(&request.action).map_err(|error| {
        Box::new(
            AppError::new(
                "EDIT_DIALOGUE_ACTION_INVALID",
                "L’opération de dialogue n’a pas pu être préparée.",
                format!("cannot serialize dialogue structure action: {error}"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(resource.to_string())
            .with_import_stage("dialogue_structure"),
        )
    })?;
    let (workspace_snapshot, raw) = {
        let mut workspaces = state
            .edit_workspaces
            .lock()
            .expect("edit workspace registry poisoned");
        let workspace = workspaces
            .get_mut(&request.workspace_id)
            .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "dialogue_structure"))?;
        let current = workspace
            .staged_resource_bytes(&resource)?
            .unwrap_or_else(|| source_bytes.clone());
        let (output, raw) = edit_dialogue_structure(
            &current,
            &format!("workspace::{}", resource.file_name()),
            &request.action,
        )?;
        let before_sha256 = hex::encode(Sha256::digest(&current));
        let after_sha256 = hex::encode(Sha256::digest(&output));
        workspace.stage_resource(resource.clone(), Some(&source_bytes), &output)?;
        let snapshot = workspace.apply(EditCommand::TransformResource {
            resource: resource.clone(),
            operation,
            before_sha256,
            after_sha256,
        })?;
        (snapshot, raw)
    };
    let graph = dialogue_graph_with_indexed_references(&state, &request.job_id, resource, raw)?;
    Ok(DialogueEditResult {
        workspace: workspace_snapshot,
        graph,
    })
}

fn dialogue_graph_with_indexed_references(
    state: &AppState,
    job_id: &str,
    resource: ResourceKey,
    raw: GenericGff,
) -> AppResult<DialogueGraph> {
    let mut graph = adapt_dialogue(
        resource.clone(),
        format!("workspace::{}", resource.file_name()),
        raw,
    );
    graph.references = state.jobs.with_analysis(job_id, |analysis| {
        Ok(analysis
            .dialogue_index
            .get(&resource.resref)
            .map(|dialogue| dialogue.references.clone())
            .unwrap_or_default())
    })?;
    Ok(graph)
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
pub fn inspect_narrative_documents(
    state: State<'_, AppState>,
    request: NarrativeInspectionRequest,
) -> AppResult<NarrativeInspection> {
    let journal = inspect_narrative_document(
        &state,
        &request.job_id,
        request.workspace_id.as_deref(),
        2056,
    )?;
    let factions = inspect_narrative_document(
        &state,
        &request.job_id,
        request.workspace_id.as_deref(),
        2038,
    )?;
    let model = adapt_narrative(
        journal.as_ref().map(|document| &document.raw),
        factions.as_ref().map(|document| &document.raw),
    );
    Ok(NarrativeInspection {
        model,
        journal,
        factions,
    })
}

fn inspect_narrative_document(
    state: &AppState,
    job_id: &str,
    workspace_id: Option<&str>,
    resource_type: u16,
) -> AppResult<Option<NarrativeDocument>> {
    let resource = state.jobs.with_analysis(job_id, |analysis| {
        Ok(analysis
            .resource_catalog
            .entries
            .iter()
            .find(|entry| entry.key.resource_type == resource_type)
            .map(|entry| entry.key.clone()))
    })?;
    let Some(resource) = resource else {
        return Ok(None);
    };
    let staged = if let Some(workspace_id) = workspace_id {
        with_edit_workspace(state, workspace_id, |workspace| {
            workspace.staged_resource_bytes(&resource)
        })?
    } else {
        None
    };
    let bytes = match staged {
        Some(bytes) => bytes,
        None => resolved_resource_bytes(state, job_id, &resource)?,
    };
    let raw = parse_gff(&bytes, &format!("workspace::{}", resource.file_name()))?;
    Ok(Some(NarrativeDocument { resource, raw }))
}

#[tauri::command]
pub fn edit_journal_structure_command(
    state: State<'_, AppState>,
    request: JournalStructureRequest,
) -> AppResult<WorkspaceSnapshot> {
    if request.resource.resource_type != 2056 {
        return Err(Box::new(
            AppError::new(
                "EDIT_JOURNAL_RESOURCE_INVALID",
                "La ressource sélectionnée n’est pas un journal JRL.",
                format!("{} is not a JRL resource", request.resource),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(request.resource.to_string())
            .with_import_stage("journal_structure"),
        ));
    }
    let source_bytes = resolved_resource_bytes(&state, &request.job_id, &request.resource)?;
    let operation = serde_json::to_string(&request.action).map_err(|error| {
        Box::new(
            AppError::new(
                "EDIT_JOURNAL_ACTION_INVALID",
                "L’opération de journal n’a pas pu être préparée.",
                format!("cannot serialize journal structure action: {error}"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(request.resource.to_string())
            .with_import_stage("journal_structure"),
        )
    })?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "journal_structure"))?;
    let current = workspace
        .staged_resource_bytes(&request.resource)?
        .unwrap_or_else(|| source_bytes.clone());
    let (output, _) = edit_journal_structure(
        &current,
        &format!("workspace::{}", request.resource.file_name()),
        &request.action,
    )?;
    let before_sha256 = hex::encode(Sha256::digest(&current));
    let after_sha256 = hex::encode(Sha256::digest(&output));
    workspace.stage_resource(request.resource.clone(), Some(&source_bytes), &output)?;
    workspace.apply(EditCommand::TransformResource {
        resource: request.resource,
        operation,
        before_sha256,
        after_sha256,
    })
}

#[tauri::command]
pub fn edit_faction_structure_command(
    state: State<'_, AppState>,
    request: FactionStructureRequest,
) -> AppResult<WorkspaceSnapshot> {
    if request.resource.resource_type != 2038 {
        return Err(Box::new(
            AppError::new(
                "EDIT_FACTION_RESOURCE_INVALID",
                "La ressource sélectionnée n’est pas une matrice FAC.",
                format!("{} is not a FAC resource", request.resource),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(request.resource.to_string())
            .with_import_stage("faction_structure"),
        ));
    }
    let source_bytes = resolved_resource_bytes(&state, &request.job_id, &request.resource)?;
    let operation = serde_json::to_string(&request.action).map_err(|error| {
        Box::new(
            AppError::new(
                "EDIT_FACTION_ACTION_INVALID",
                "L’opération de faction n’a pas pu être préparée.",
                format!("cannot serialize faction structure action: {error}"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(request.resource.to_string())
            .with_import_stage("faction_structure"),
        )
    })?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "faction_structure"))?;
    let current = workspace
        .staged_resource_bytes(&request.resource)?
        .unwrap_or_else(|| source_bytes.clone());
    let (output, _) = edit_faction_structure(
        &current,
        &format!("workspace::{}", request.resource.file_name()),
        &request.action,
    )?;
    let before_sha256 = hex::encode(Sha256::digest(&current));
    let after_sha256 = hex::encode(Sha256::digest(&output));
    workspace.stage_resource(request.resource.clone(), Some(&source_bytes), &output)?;
    workspace.apply(EditCommand::TransformResource {
        resource: request.resource,
        operation,
        before_sha256,
        after_sha256,
    })
}

#[tauri::command]
pub fn edit_blueprint_structure_command(
    state: State<'_, AppState>,
    request: BlueprintStructureRequest,
) -> AppResult<GffEditResult> {
    const BLUEPRINT_TYPES: &[u16] = &[2025, 2027, 2032, 2035, 2040, 2042, 2044, 2051, 2055, 2058];
    if !BLUEPRINT_TYPES.contains(&request.resource.resource_type) {
        return Err(Box::new(
            AppError::new(
                "EDIT_BLUEPRINT_RESOURCE_INVALID",
                "La ressource sélectionnée n’est pas un blueprint pris en charge.",
                format!("{} is not a supported blueprint resource", request.resource),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(request.resource.to_string())
            .with_import_stage("blueprint_structure"),
        ));
    }
    let source_bytes = resolved_resource_bytes(&state, &request.job_id, &request.resource)?;
    let operation = serde_json::to_string(&request.action).map_err(|error| {
        Box::new(
            AppError::new(
                "EDIT_BLUEPRINT_ACTION_INVALID",
                "L’opération de blueprint n’a pas pu être préparée.",
                format!("cannot serialize blueprint structure action: {error}"),
                aurora_core::ErrorSeverity::Error,
            )
            .with_resource(request.resource.to_string())
            .with_import_stage("blueprint_structure"),
        )
    })?;
    let mut workspaces = state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned");
    let workspace = workspaces
        .get_mut(&request.workspace_id)
        .ok_or_else(|| edit_workspace_not_open(&request.workspace_id, "blueprint_structure"))?;
    let current = workspace
        .staged_resource_bytes(&request.resource)?
        .unwrap_or_else(|| source_bytes.clone());
    let (output, document) = edit_blueprint_structure(
        &current,
        &format!("workspace::{}", request.resource.file_name()),
        &request.action,
    )?;
    let before_sha256 = hex::encode(Sha256::digest(&current));
    let after_sha256 = hex::encode(Sha256::digest(&output));
    workspace.stage_resource(request.resource.clone(), Some(&source_bytes), &output)?;
    let snapshot = workspace.apply(EditCommand::TransformResource {
        resource: request.resource,
        operation,
        before_sha256,
        after_sha256,
    })?;
    Ok(GffEditResult {
        workspace: snapshot,
        document,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_endpoints_require_https_except_for_local_models() {
        assert!(validated_ai_endpoint("http://127.0.0.1:11434/v1/chat/completions").is_ok());
        assert!(validated_ai_endpoint("http://localhost:1234/v1/chat/completions").is_ok());
        assert_eq!(
            validated_ai_endpoint("http://example.com/v1/chat/completions")
                .expect_err("remote HTTP")
                .code,
            "EDIT_AI_ENDPOINT_INSECURE"
        );
        assert_eq!(
            validated_ai_endpoint("https://secret@example.com/v1/chat/completions")
                .expect_err("embedded credentials")
                .code,
            "EDIT_AI_ENDPOINT_INVALID"
        );
    }

    #[test]
    fn ai_response_decoder_accepts_strict_json_and_rejects_markdown() {
        let decoded = decode_ai_change_set(
            r#"{"summary":"Edit script","commands":[{"kind":"replace_text","resource":{"resref":"start","resourceType":2009},"before":"void main() {}","after":"void main() { int n = 1; }"}]}"#,
        )
        .expect("strict JSON change set");
        assert_eq!(decoded.commands.len(), 1);
        assert_eq!(
            decode_ai_change_set("```json\n{}\n```")
                .expect_err("Markdown response")
                .code,
            "EDIT_AI_RESPONSE_INVALID"
        );
    }
}
