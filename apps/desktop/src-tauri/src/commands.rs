use crate::blueprint_options::{
    BlueprintFieldOptions, BlueprintFieldOptionsRequest, build_blueprint_field_options,
};
use crate::jobs::JobSnapshot;
use crate::session::{SessionPaths, restore_analysis_session, store_analysis_session};
use crate::state::AppState;
use aurora_2da::{TwoDaEditAction, TwoDaTable, apply_2da_edit, parse_2da, write_2da};
use aurora_agent::{
    AgentEventKind, AgentPolicy, AgentRun, AgentRunStatus, AgentWorkspaceStore, ApprovalRequest,
    ApprovalStatus, CapabilityRegistry, CapabilitySideEffect, EffectiveCapability, ModuleBlueprint,
    PolicyDecision, ProviderKind, ProviderProfile, ProviderRequestContext, ProviderToolOutput,
    SecurityLevel, ToolCallRecord, ToolCallStatus, build_provider_request, built_in_policy,
    compile_module_blueprint, context_allows_capability, decode_provider_response,
    evaluate_capability, sanitize_context_value, validate_agent_policy, validate_module_blueprint,
    validate_tool_scope,
};
use aurora_core::{AppError, AppResult, ResourceKey, decode_nwn_text};
use aurora_dialogue::adapt_dialogue;
use aurora_edit::{
    AiApplyReport, AiChangeSet, AiChangeSetPreview, AreaAudioPatch, AreaEnvironmentPatch,
    AreaStructureAction, AuroraSyncAction, AuroraSyncAppliedFile, AuroraSyncDirection,
    AuroraSyncManifest, AuroraSyncPlan, AuroraSyncReport, AuroraSyncState, AuroraSyncWorkspaceFile,
    BlueprintStructureAction, DevelopmentCleanupReport, DevelopmentDeployment,
    DialogueStructureAction, EditCommand, EditWorkspace, FactionStructureAction,
    GitWorkspaceStatus, InstancePlacement, JournalStructureAction, MAP_MAX_BLUEPRINTS_PER_RULE,
    MAP_MAX_DENSITY_RULES, MAP_MAX_HEIGHT, MAP_MAX_PLACEMENTS, MAP_MAX_TILES, MAP_MAX_WIDTH,
    MapCompatibilityReport, MapGenerationPlan, MapGenerationSpec, ModuleBuildProfile,
    ModuleBuildReport, ModuleManifestDefinition, NewModuleDefinition, NwnLaunchMode,
    NwnLaunchProfile, NwnLaunchReport, PaletteManifest, ReproducibleBuildVerification,
    ResourceContentDigest, TileState, Transform, WalkmeshDocument, WalkmeshDraft, WalkmeshKind,
    WalkmeshOperation, WalkmeshValidation, WorkspaceExportManifest, WorkspaceSnapshot,
    add_area_instance, ai_change_set_sha256, apply_walkmesh_operation, baseline_from_plan,
    compare_aurora_sync, create_area_resources, create_dialogue_resource, create_empty_module,
    create_generated_map_resources, edit_area_audio, edit_area_environment, edit_area_instance,
    edit_area_instance_by_id, edit_area_structure, edit_area_tile, edit_area_tile_at,
    edit_blueprint_structure, edit_dialogue_structure, edit_faction_structure, edit_gff_field,
    edit_journal_structure, edit_module_dependencies, edit_module_manifest,
    generate_map_plan_with_compatibility, inspect_area_audio, inspect_area_environment,
    inspect_git_repository, inspect_walkmesh, read_aurora_workspace_file, remove_area_instance,
    resource_key_from_aurora_path, scan_aurora_workspace, serialize_walkmesh_ascii,
    validate_build_profile, validate_walkmesh_for_kind, verify_sync_action,
    write_aurora_workspace_file,
};
use aurora_erf::ErfResourceInput;
use aurora_gff::{GenericGff, parse_gff};
use aurora_index::{CatalogPersistence, load_dependency_baseline, replace_resource_catalog};
use aurora_nwscript::{CompileResult, CompilerConfig, NssDocument, compile_nss, parse_nss};
use aurora_project::{
    AnalysisPhase, DependencyRoots, DiagnosticReport, DialogueGraph, DialoguePage, HashProgress,
    ModuleDependencyReport, NarrativeModel, ResourceManager, ResourcePage, ResourceSourceKind,
    SceneManifest, ScriptDocument, ScriptPage, WorldIndex, analyze_module_file_with_cache,
    build_asset_preview, cached_model_preview, compare_dependency_reports,
};
use aurora_tlk::{TalkTable, TlkEditAction, apply_tlk_edit, parse_tlk, write_tlk};
use aurora_world::{
    AreaMap, adapt_area, adapt_narrative, parse_set_tile_models, render_area_atlas_svg,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredModuleSession {
    pub job: JobSnapshot,
    pub workspace: Option<WorkspaceSnapshot>,
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
pub struct AgentWorkspaceRequest {
    pub workspace_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAgentPolicyRequest {
    pub workspace_id: String,
    pub policy: AgentPolicy,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStudioState {
    pub policy: AgentPolicy,
    pub presets: Vec<AgentPolicy>,
    pub registry: CapabilityRegistry,
    pub effective_capabilities: Vec<EffectiveCapability>,
    pub runs: Vec<AgentRun>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentRunRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub objective: String,
    pub provider: ProviderProfile,
    pub policy: Option<AgentPolicy>,
    pub blueprint: Option<ModuleBlueprint>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateBlueprintRequest {
    pub blueprint: ModuleBlueprint,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceAgentRunRequest {
    pub workspace_id: String,
    pub run_id: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestAgentProviderRequest {
    pub provider: ProviderProfile,
    pub policy: AgentPolicy,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProviderTestReport {
    pub endpoint_origin: String,
    pub model: String,
    pub latency_ms: u64,
    pub reply: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveAgentApprovalRequest {
    pub workspace_id: String,
    pub run_id: String,
    pub approval_id: String,
    pub approved: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAgentRunRequest {
    pub workspace_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentResourceSearchArguments {
    query: String,
    limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentSetFieldArguments {
    resource: ResourceKey,
    path: String,
    before: serde_json::Value,
    after: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentReplaceScriptArguments {
    resource: ResourceKey,
    before: String,
    after: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentAreaCreateArguments {
    resref: String,
    name: String,
    width: u32,
    height: u32,
    tileset: String,
    #[serde(default)]
    tile_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentScriptCreateArguments {
    resref: String,
    event: String,
    purpose: String,
    source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentScriptCompileArguments {
    resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentUndoBatchArguments {
    checkpoint_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentModuleBuildArguments {
    output_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentDialogueEditArguments {
    resref: String,
    action: DialogueStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentJournalEditArguments {
    resource: ResourceKey,
    action: JournalStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentFactionEditArguments {
    resource: ResourceKey,
    action: FactionStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentBlueprintEditArguments {
    resource: ResourceKey,
    action: BlueprintStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentToolsetSyncArguments {
    actions: Vec<AuroraSyncAction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentModuleDependenciesArguments {
    hak_files: Vec<String>,
    custom_tlk: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentModuleCreateArguments {
    output_path: String,
    name: String,
    tag: String,
    entry_area: String,
    tileset: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentTwoDaEditArguments {
    resource: ResourceKey,
    action: TwoDaEditAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentTlkEditArguments {
    resource: ResourceKey,
    action: TlkEditAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentAreaInstanceArguments {
    area: String,
    placement: InstancePlacement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapContextArguments {
    tileset: Option<String>,
    query: String,
    limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapAreaArguments {
    area: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapPreviewArguments {
    spec: MapGenerationSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapApplyArguments {
    spec: MapGenerationSpec,
    expected_plan_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapEnvironmentArguments {
    area: String,
    expected_sha256: String,
    patch: AreaEnvironmentPatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapAudioArguments {
    area: String,
    expected_sha256: String,
    patch: AreaAudioPatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapTileArguments {
    area: String,
    x: u32,
    y: u32,
    expected_sha256: String,
    before: TileState,
    after: TileState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapInstanceAddArguments {
    area: String,
    expected_sha256: String,
    placement: InstancePlacement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapInstanceMoveArguments {
    area: String,
    instance_id: String,
    expected_sha256: String,
    before: Transform,
    after: Transform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapInstanceRemoveArguments {
    area: String,
    instance_id: String,
    expected_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentMapStructureArguments {
    area: String,
    expected_sha256: String,
    action: AreaStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentWalkmeshEditArguments {
    resref: String,
    kind: WalkmeshKind,
    operation: WalkmeshOperation,
}

#[derive(Debug, Clone, Deserialize)]
struct AgentArchitectureQueryArguments {
    query: String,
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
pub struct PreviewMapGenerationRequest {
    pub job_id: String,
    pub spec: MapGenerationSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMapAuthoringContextRequest {
    pub job_id: String,
    pub tileset: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapKnownLimits {
    pub max_width: u32,
    pub max_height: u32,
    pub max_tiles: usize,
    pub max_resref_length: usize,
    pub max_density_rules: usize,
    pub max_blueprints_per_rule: usize,
    pub max_placements: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapTilesetContext {
    pub resref: String,
    pub sha256: String,
    pub tile_count: usize,
    pub tile_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapAuthoringContext {
    pub limits: MapKnownLimits,
    pub available_tilesets: Vec<String>,
    pub selected_tileset: Option<MapTilesetContext>,
    pub blueprint_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftMapWithAiRequest {
    pub job_id: String,
    pub current_spec: MapGenerationSpec,
    pub provider: ProviderProfile,
    pub api_key: Option<String>,
    #[serde(default)]
    pub include_blueprint_resrefs: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMapDraftResult {
    pub endpoint_origin: String,
    pub model: String,
    pub plan: MapGenerationPlan,
    pub shared_blueprint_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMapGenerationRequest {
    pub job_id: String,
    pub workspace_id: String,
    pub spec: MapGenerationSpec,
    pub expected_plan_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyMapGenerationResult {
    pub workspace: WorkspaceSnapshot,
    pub area: AreaMap,
    pub plan: MapGenerationPlan,
}

fn map_known_limits() -> MapKnownLimits {
    MapKnownLimits {
        max_width: MAP_MAX_WIDTH,
        max_height: MAP_MAX_HEIGHT,
        max_tiles: MAP_MAX_TILES,
        max_resref_length: 16,
        max_density_rules: MAP_MAX_DENSITY_RULES,
        max_blueprints_per_rule: MAP_MAX_BLUEPRINTS_PER_RULE,
        max_placements: MAP_MAX_PLACEMENTS,
    }
}

fn build_map_authoring_context(
    state: &AppState,
    job_id: &str,
    tileset: &str,
) -> AppResult<MapAuthoringContext> {
    state.jobs.with_analysis(job_id, |analysis| {
        let mut available_tilesets = analysis
            .resource_catalog
            .entries
            .iter()
            .filter(|entry| entry.key.resource_type == 2013)
            .map(|entry| entry.key.resref.clone())
            .collect::<Vec<_>>();
        available_tilesets.sort();
        available_tilesets.dedup();
        let selected_key = ResourceKey::new(tileset, 2013);
        let selected_tileset = if analysis.resource_catalog.get(&selected_key).is_some() {
            Some(load_map_tileset_context(analysis, tileset)?)
        } else {
            None
        };
        let mut blueprint_counts = BTreeMap::new();
        for category in map_blueprint_categories() {
            let resource_type = map_template_resource_type(category).expect("known map category");
            let count = analysis
                .resource_catalog
                .entries
                .iter()
                .filter(|entry| entry.key.resource_type == resource_type)
                .count();
            blueprint_counts.insert(category.to_owned(), count);
        }
        Ok(MapAuthoringContext {
            limits: map_known_limits(),
            available_tilesets,
            selected_tileset,
            blueprint_counts,
        })
    })
}

fn load_map_tileset_context(
    analysis: &aurora_project::ModuleAnalysis,
    tileset: &str,
) -> AppResult<MapTilesetContext> {
    let key = ResourceKey::new(tileset, 2013);
    let resolved = analysis.resource_catalog.get(&key).ok_or_else(|| {
        map_generation_error(
            "EDIT_MAP_TILESET_NOT_RESOLVED",
            "Le SET du tileset choisi est introuvable dans le module, ses HAK ou l’installation NWN.",
            key.to_string(),
        )
    })?;
    let cancelled = AtomicBool::new(false);
    let bytes = ResourceManager::read(&resolved.selected, &cancelled)?;
    let models = parse_set_tile_models(&bytes);
    if models.is_empty() {
        return Err(map_generation_error(
            "EDIT_MAP_TILESET_INVALID",
            "Le SET choisi ne contient aucune tuile lisible.",
            key.to_string(),
        ));
    }
    let tile_ids = models.keys().copied().collect::<Vec<_>>();
    Ok(MapTilesetContext {
        resref: tileset.to_owned(),
        sha256: hex::encode(Sha256::digest(&bytes)),
        tile_count: tile_ids.len(),
        tile_ids,
    })
}

fn generate_verified_map_plan(
    state: &AppState,
    job_id: &str,
    spec: &MapGenerationSpec,
) -> AppResult<MapGenerationPlan> {
    state.jobs.with_analysis(job_id, |analysis| {
        let tileset = load_map_tileset_context(analysis, &spec.tileset)?;
        let mut selected_tile_ids = vec![spec.base_tile_id];
        selected_tile_ids.extend(spec.variant_tile_ids.iter().copied());
        selected_tile_ids.sort_unstable();
        selected_tile_ids.dedup();
        let available = tileset.tile_ids.iter().copied().collect::<BTreeSet<_>>();
        let missing = selected_tile_ids
            .iter()
            .filter(|tile_id| !available.contains(tile_id))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(map_generation_error(
                "EDIT_MAP_TILE_NOT_RESOLVED",
                "Une ou plusieurs tuiles n’existent pas dans le SET choisi.",
                format!("{} missing tile ids: {missing:?}", spec.tileset),
            ));
        }
        generate_map_plan_with_compatibility(
            spec,
            MapCompatibilityReport {
                tileset_resolved: true,
                tileset_sha256: Some(tileset.sha256),
                resolved_tile_count: tileset.tile_count,
                selected_tile_ids,
                tile_ids_verified: true,
                edge_compatibility_verified: false,
            },
        )
    })
}

fn map_blueprint_resrefs(
    state: &AppState,
    job_id: &str,
) -> AppResult<BTreeMap<String, Vec<String>>> {
    state.jobs.with_analysis(job_id, |analysis| {
        let mut result = BTreeMap::new();
        for category in map_blueprint_categories() {
            let resource_type = map_template_resource_type(category).expect("known map category");
            let mut resrefs = analysis
                .resource_catalog
                .entries
                .iter()
                .filter(|entry| entry.key.resource_type == resource_type)
                .map(|entry| entry.key.resref.clone())
                .collect::<Vec<_>>();
            resrefs.sort();
            resrefs.dedup();
            resrefs.truncate(MAP_MAX_BLUEPRINTS_PER_RULE);
            result.insert(category.to_owned(), resrefs);
        }
        Ok(result)
    })
}

fn validate_map_blueprints(
    state: &AppState,
    job_id: &str,
    spec: &MapGenerationSpec,
) -> AppResult<()> {
    state.jobs.with_analysis(job_id, |analysis| {
        for rule in &spec.densities {
            let resource_type = map_template_resource_type(&rule.category).ok_or_else(|| {
                map_generation_error(
                    "EDIT_MAP_CATEGORY_UNSUPPORTED",
                    "Une catégorie de placement n’est pas prise en charge.",
                    rule.category.clone(),
                )
            })?;
            for resref in &rule.template_resrefs {
                let key = ResourceKey::new(resref, resource_type);
                if analysis.resource_catalog.get(&key).is_none() {
                    return Err(map_generation_error(
                        "EDIT_MAP_BLUEPRINT_NOT_RESOLVED",
                        "Un blueprint proposé par l’IA est introuvable.",
                        key.to_string(),
                    ));
                }
            }
        }
        Ok(())
    })
}

fn map_blueprint_categories() -> [&'static str; 9] {
    [
        "creature",
        "door",
        "encounter",
        "item",
        "placeable",
        "sound",
        "store",
        "trigger",
        "waypoint",
    ]
}

fn validate_map_ai_provider(request: &DraftMapWithAiRequest) -> AppResult<()> {
    if request
        .api_key
        .as_ref()
        .is_some_and(|key| key.len() > 16 * 1024)
    {
        return Err(map_generation_error(
            "EDIT_MAP_AI_KEY_TOO_LARGE",
            "La clé temporaire dépasse la limite autorisée.",
            "ephemeral map AI key exceeds 16 KiB",
        ));
    }
    if request.provider.kind == ProviderKind::Manual
        || request.provider.model.trim().is_empty()
        || request.provider.model.len() > 256
        || request.provider.endpoint.len() > 2_048
        || !request.provider.supports_tools
    {
        return Err(map_generation_error(
            "EDIT_MAP_AI_PROVIDER_INVALID",
            "Le fournisseur IA doit avoir un endpoint, un modèle et prendre en charge les outils structurés.",
            "invalid map AI provider profile",
        ));
    }
    Ok(())
}

fn map_ai_system_prompt() -> &'static str {
    "Tu es le planificateur de cartes de OpenNever Forge. Réponds par exactement un appel de l'outil map.generate et aucun autre outil. Respecte strictement le schéma. La carte doit rester compatible avec Neverwinter Nights : dimensions maximales 32x32, ResRef ASCII minuscules de 16 caractères maximum, graine entière 32 bits et uniquement les identifiants de tuiles fournis par selectedTileset. Conserve exactement le tileset courant. N'invente jamais un blueprint : utilise seulement les ResRef explicitement fournis, ou laisse templateResrefs vide. N'ajoute des variantTileIds que si le brief les demande clairement ; leurs raccords visuels ne sont pas validés, donc préfère une liste vide. Utilise les densités et espacements pour traduire l'intention du brief. Ne demande et ne reproduis aucun chemin local, script, dialogue, texture, GFF ou donnée binaire."
}

fn map_generation_error(
    code: impl Into<String>,
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
        .with_import_stage("map_generation"),
    )
}

fn verify_map_resource_sha256(bytes: &[u8], expected: &str) -> AppResult<()> {
    let actual = hex::encode(Sha256::digest(bytes));
    if expected.len() != 64 || !expected.eq_ignore_ascii_case(&actual) {
        return Err(map_generation_error(
            "EDIT_MAP_RESOURCE_CHANGED",
            "La carte a changÃ© depuis son inspection. Inspectez-la de nouveau avant de poursuivre.",
            format!("expected {expected:?}, current {actual}"),
        ));
    }
    Ok(())
}

fn validate_agent_map_placement(
    state: &AppState,
    run: &AgentRun,
    placement: &InstancePlacement,
) -> AppResult<()> {
    let resource_type = map_template_resource_type(&placement.category).ok_or_else(|| {
        map_generation_error(
            "EDIT_MAP_CATEGORY_UNSUPPORTED",
            "La catÃ©gorie de placement n'est pas prise en charge.",
            placement.category.clone(),
        )
    })?;
    if placement.tag.is_empty()
        || placement.tag.len() > 64
        || placement
            .tag
            .chars()
            .any(|character| character.is_control())
        || !placement.x.is_finite()
        || !placement.y.is_finite()
        || !placement.z.is_finite()
        || !placement.bearing.is_finite()
    {
        return Err(map_generation_error(
            "EDIT_MAP_PLACEMENT_INVALID",
            "Le placement contient un tag ou des coordonnÃ©es invalides.",
            placement.tag.clone(),
        ));
    }
    let key = ResourceKey::new(&placement.template_resref, resource_type);
    if workspace_or_resolved_resource_bytes(state, &run.job_id, &run.workspace_id, &key)?.is_none()
    {
        return Err(map_generation_error(
            "EDIT_MAP_BLUEPRINT_NOT_RESOLVED",
            "Le blueprint demandÃ© par le placement est introuvable.",
            key.to_string(),
        ));
    }
    Ok(())
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
    compile_workspace_script_inner(&state, request)
}

fn compile_workspace_script_inner(
    state: &AppState,
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
        state,
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
        let snapshot = get_open_edit_workspace_snapshot(state, &request.workspace_id)?;
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
    apply_aurora_workspace_sync_inner(&state, request)
}

fn apply_aurora_workspace_sync_inner(
    state: &AppState,
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
    let preview = build_aurora_sync_plan(state, &request.job_id, &request.workspace_id, &root)?;
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
        with_edit_workspace(state, &request.workspace_id, |workspace| {
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
                    state,
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
                        with_edit_workspace(state, &request.workspace_id, |workspace| {
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
                        with_edit_workspace(state, &request.workspace_id, |workspace| {
                            workspace.create_resource(action.resource.clone(), incoming)
                        })?;
                    }
                    (Some(current), None) => {
                        with_edit_workspace(state, &request.workspace_id, |workspace| {
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
                    state,
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
        build_aurora_sync_plan(state, &request.job_id, &request.workspace_id, &root)?;
    let old_baseline = with_edit_workspace(state, &request.workspace_id, |workspace| {
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
    with_edit_workspace(state, &request.workspace_id, |workspace| {
        workspace
            .save_aurora_sync_baseline(&root, &next_baseline)
            .map(|_| ())
    })?;
    backups.sort();
    backups.dedup();
    let plan = build_aurora_sync_plan(state, &request.job_id, &request.workspace_id, &root)?;
    Ok(AuroraSyncReport {
        schema_version: 2,
        root: plan.root.clone(),
        applied,
        backups,
        plan,
        workspace: get_open_edit_workspace_snapshot(state, &request.workspace_id)?,
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
pub fn get_agent_studio_state(
    state: State<'_, AppState>,
    request: AgentWorkspaceRequest,
) -> AppResult<AgentStudioState> {
    let snapshot = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.snapshot()
    })?;
    let store = AgentWorkspaceStore::new(&snapshot.root);
    let policy = store
        .load_policy()?
        .unwrap_or_else(|| built_in_policy(SecurityLevel::Advisor));
    let registry = CapabilityRegistry::standard();
    let effective_capabilities = registry
        .capabilities
        .iter()
        .map(|descriptor| evaluate_capability(&policy, descriptor, 0).0)
        .collect();
    Ok(AgentStudioState {
        policy,
        presets: [
            SecurityLevel::Observer,
            SecurityLevel::Advisor,
            SecurityLevel::Assisted,
            SecurityLevel::Supervised,
            SecurityLevel::Autonomous,
            SecurityLevel::Operator,
        ]
        .into_iter()
        .map(built_in_policy)
        .collect(),
        registry,
        effective_capabilities,
        runs: store.list_runs()?,
    })
}

#[tauri::command]
pub fn save_agent_policy(
    state: State<'_, AppState>,
    request: SaveAgentPolicyRequest,
) -> AppResult<AgentStudioState> {
    validate_agent_policy(&request.policy).map_err(|error| {
        agent_error(
            "AGENT_POLICY_INVALID",
            "Le profil de sécurité IA n’est pas valide.",
            error,
        )
    })?;
    let snapshot = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.snapshot()
    })?;
    AgentWorkspaceStore::new(&snapshot.root).save_policy(&request.policy)?;
    get_agent_studio_state(
        state,
        AgentWorkspaceRequest {
            workspace_id: request.workspace_id,
        },
    )
}

#[tauri::command]
pub fn create_agent_run(
    state: State<'_, AppState>,
    request: CreateAgentRunRequest,
) -> AppResult<AgentRun> {
    let snapshot = with_edit_workspace(&state, &request.workspace_id, |workspace| {
        workspace.snapshot()
    })?;
    let store = AgentWorkspaceStore::new(&snapshot.root);
    let policy = request
        .policy
        .or(store.load_policy()?)
        .unwrap_or_else(|| built_in_policy(SecurityLevel::Advisor));
    validate_agent_policy(&policy).map_err(|error| {
        agent_error(
            "AGENT_POLICY_INVALID",
            "Le profil de sécurité IA n’est pas valide.",
            error,
        )
    })?;
    if request.objective.trim().is_empty()
        || request.objective.len() > policy.limits.max_prompt_bytes
    {
        return Err(agent_error(
            "AGENT_OBJECTIVE_INVALID",
            "L’objectif IA est vide ou dépasse la limite du profil.",
            format!(
                "objective contains {} bytes with a {} byte limit",
                request.objective.len(),
                policy.limits.max_prompt_bytes
            ),
        ));
    }
    if request.provider.id.trim().is_empty()
        || request.provider.id.len() > 128
        || request.provider.name.trim().is_empty()
        || request.provider.name.len() > 128
        || request.provider.endpoint.len() > 2_048
        || request.provider.model.len() > 256
        || request
            .provider
            .temperature_milli
            .is_some_and(|value| value > 2_000)
        || request
            .provider
            .reasoning_effort
            .as_ref()
            .is_some_and(|value| value.len() > 32)
    {
        return Err(agent_error(
            "AGENT_PROVIDER_PROFILE_INVALID",
            "La configuration du fournisseur est hors limites.",
            "provider identity, endpoint, model, reasoning or temperature is invalid",
        ));
    }
    if request.provider.kind != aurora_agent::ProviderKind::Manual {
        validated_agent_endpoint(&request.provider.endpoint, &policy.context)?;
        if request.provider.model.trim().is_empty() || !request.provider.supports_tools {
            return Err(agent_error(
                "AGENT_PROVIDER_MODEL_INVALID",
                "La configuration du fournisseur est incomplète ou hors limites.",
                "provider identity, model, tool support, reasoning or temperature is invalid",
            ));
        }
    }
    if let Some(blueprint) = &request.blueprint {
        let validation = validate_module_blueprint(blueprint);
        if !validation.valid {
            return Err(agent_error(
                "AGENT_BLUEPRINT_INVALID",
                "Le plan de module contient des erreurs.",
                serde_json::to_string(&validation.diagnostics).unwrap_or_default(),
            ));
        }
    }
    let mut run = AgentRun::new(
        request.job_id,
        request.workspace_id,
        request.objective,
        request.provider,
        policy,
        unix_time_ms(),
    );
    run.blueprint = request.blueprint;
    store.save_run(&run)?;
    Ok(run)
}

#[tauri::command]
pub fn validate_agent_blueprint(
    request: ValidateBlueprintRequest,
) -> aurora_agent::BlueprintValidation {
    validate_module_blueprint(&request.blueprint)
}

struct AgentCancellationGuard<'a> {
    registry: &'a Mutex<HashMap<String, Arc<AtomicBool>>>,
    run_id: String,
}

impl Drop for AgentCancellationGuard<'_> {
    fn drop(&mut self) {
        self.registry
            .lock()
            .expect("agent cancellation registry poisoned")
            .remove(&self.run_id);
    }
}

fn register_agent_cancellation<'a>(
    state: &'a AppState,
    run_id: &str,
) -> AppResult<(Arc<AtomicBool>, AgentCancellationGuard<'a>)> {
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut registry = state
        .agent_cancellations
        .lock()
        .expect("agent cancellation registry poisoned");
    if registry.contains_key(run_id) {
        return Err(agent_error(
            "AGENT_RUN_ALREADY_ACTIVE",
            "Cette exécution IA est déjà active.",
            run_id.to_owned(),
        ));
    }
    registry.insert(run_id.to_owned(), Arc::clone(&cancellation));
    drop(registry);
    Ok((
        cancellation,
        AgentCancellationGuard {
            registry: &state.agent_cancellations,
            run_id: run_id.to_owned(),
        },
    ))
}

fn stop_cancelled_agent_run(
    cancellation: &AtomicBool,
    run: &mut AgentRun,
    store: &AgentWorkspaceStore,
) -> AppResult<bool> {
    if !cancellation.load(Ordering::Acquire) {
        return Ok(false);
    }
    run.status = AgentRunStatus::Cancelled;
    run.push_event(
        unix_time_ms(),
        AgentEventKind::Cancelled,
        "Exécution interrompue à la demande de l’utilisateur.",
        None,
    );
    store.save_run(run)?;
    Ok(true)
}

#[tauri::command]
pub async fn advance_agent_run(
    state: State<'_, AppState>,
    request: AdvanceAgentRunRequest,
) -> AppResult<AgentRun> {
    if request
        .api_key
        .as_ref()
        .is_some_and(|key| key.len() > 16 * 1024)
    {
        return Err(agent_error(
            "AGENT_API_KEY_TOO_LARGE",
            "La clé temporaire dépasse la limite autorisée.",
            "ephemeral API key exceeds 16 KiB",
        ));
    }
    let store = agent_store_for_workspace(&state, &request.workspace_id)?;
    let mut run = store.load_run(&request.run_id)?.ok_or_else(|| {
        agent_error(
            "AGENT_RUN_NOT_FOUND",
            "L’exécution IA demandée est introuvable.",
            format!("no persisted agent run has id {}", request.run_id),
        )
    })?;
    if run.workspace_id != request.workspace_id {
        return Err(agent_error(
            "AGENT_RUN_WORKSPACE_MISMATCH",
            "L’exécution IA n’appartient pas à ce workspace.",
            format!("run {} belongs to workspace {}", run.id, run.workspace_id),
        ));
    }
    if matches!(
        run.status,
        AgentRunStatus::Completed | AgentRunStatus::Failed | AgentRunStatus::Cancelled
    ) {
        return Ok(run);
    }
    if run.status == AgentRunStatus::WaitingApproval {
        return Err(agent_error(
            "AGENT_APPROVAL_PENDING",
            "Une décision est requise avant de poursuivre l’exécution.",
            format!("run {} has a pending approval", run.id),
        ));
    }
    if run
        .tool_calls
        .iter()
        .any(|call| call.status == ToolCallStatus::Running)
    {
        run.status = AgentRunStatus::Failed;
        run.push_event(
            unix_time_ms(),
            AgentEventKind::Failed,
            "Une opération était active lors de l’interruption précédente ; vérifiez le workspace et restaurez son checkpoint si nécessaire.",
            None,
        );
        store.save_run(&run)?;
        return Ok(run);
    }
    if run.provider.kind == ProviderKind::Manual {
        return Err(agent_error(
            "AGENT_PROVIDER_MANUAL",
            "Le fournisseur manuel ne peut pas exécuter une boucle automatique.",
            "manual provider requires imported tool calls or blueprint",
        ));
    }
    validated_agent_endpoint(&run.provider.endpoint, &run.policy.context)?;
    let (cancellation, _cancellation_guard) = register_agent_cancellation(&state, &run.id)?;
    run.status = AgentRunStatus::Running;
    run.push_event(
        unix_time_ms(),
        AgentEventKind::Started,
        "Boucle agentique démarrée.",
        None,
    );
    store.save_run(&run)?;

    while run.current_turn < run.policy.limits.max_turns
        && run.tool_calls.len() < run.policy.limits.max_tool_calls as usize
    {
        if stop_cancelled_agent_run(&cancellation, &mut run, &store)? {
            return Ok(run);
        }
        if unix_time_ms().saturating_sub(run.created_unix_ms)
            > run.policy.limits.max_duration_seconds.saturating_mul(1_000)
        {
            run.status = AgentRunStatus::Failed;
            run.push_event(
                unix_time_ms(),
                AgentEventKind::Failed,
                "Le budget de durée total de l’exécution est épuisé.",
                None,
            );
            store.save_run(&run)?;
            return Ok(run);
        }
        let registry = CapabilityRegistry::standard();
        let available_tools = registry
            .capabilities
            .iter()
            .filter(|descriptor| agent_tool_is_implemented(&descriptor.id))
            .filter(|descriptor| context_allows_capability(&run.policy, &descriptor.id))
            .filter(|descriptor| {
                !matches!(
                    evaluate_capability(
                        &run.policy,
                        descriptor,
                        tool_call_count(&run, &descriptor.id),
                    )
                    .1,
                    PolicyDecision::Denied { .. }
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        if available_tools.is_empty() {
            run.status = AgentRunStatus::Failed;
            run.push_event(
                unix_time_ms(),
                AgentEventKind::Failed,
                "Aucun outil exécutable n’est autorisé par la politique.",
                None,
            );
            store.save_run(&run)?;
            return Ok(run);
        }
        let mut context_budget = run.policy.limits.max_prompt_bytes;
        let (body, request_task_context) = loop {
            let task_context = agent_task_context(&run, context_budget);
            let candidate = build_provider_request(
                &run.provider,
                ProviderRequestContext {
                    system_prompt: agent_system_prompt(),
                    task_context: &task_context,
                    tools: &available_tools,
                    allow_parallel: run.provider.supports_parallel_tools
                        && run.policy.limits.max_parallel_calls > 1,
                    max_output_tokens: run.policy.limits.max_output_tokens,
                    previous_response_id: run.provider_conversation.previous_response_id.as_deref(),
                    tool_outputs: &run.provider_conversation.pending_tool_outputs,
                    replay_items: &run.provider_conversation.replay_items,
                },
            );
            let encoded_size =
                serde_json::to_vec(&candidate).map_or(usize::MAX, |bytes| bytes.len());
            if encoded_size <= run.policy.limits.max_prompt_bytes {
                break (candidate, task_context);
            }
            if context_budget <= 1_024 {
                run.status = AgentRunStatus::Failed;
                run.push_event(
                    unix_time_ms(),
                    AgentEventKind::Failed,
                    "Les schémas des outils autorisés dépassent le budget de prompt.",
                    Some(serde_json::json!({
                        "requestBytes": encoded_size,
                        "budgetBytes": run.policy.limits.max_prompt_bytes,
                    })),
                );
                store.save_run(&run)?;
                return Ok(run);
            }
            let overflow = encoded_size.saturating_sub(run.policy.limits.max_prompt_bytes);
            context_budget = context_budget
                .saturating_sub(overflow.max(1_024))
                .max(1_024);
        };
        run.current_turn += 1;
        run.push_event(
            unix_time_ms(),
            AgentEventKind::ModelRequest,
            format!("Appel fournisseur du tour {}.", run.current_turn),
            Some(serde_json::json!({
                "provider": run.provider.kind,
                "model": run.provider.model,
                "availableTools": available_tools.len(),
            })),
        );
        store.save_run(&run)?;

        let elapsed_ms = unix_time_ms().saturating_sub(run.created_unix_ms);
        let remaining_ms = run
            .policy
            .limits
            .max_duration_seconds
            .saturating_mul(1_000)
            .saturating_sub(elapsed_ms)
            .max(1);
        let timeout = Duration::from_millis(remaining_ms.min(300_000));
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| {
                agent_error(
                    "AGENT_PROVIDER_CLIENT_FAILED",
                    "Le client du fournisseur IA n’a pas pu être initialisé.",
                    error.to_string(),
                )
            })?;
        let mut attempt = 0;
        let mut response = loop {
            let mut builder = client.post(&run.provider.endpoint).json(&body);
            if let Some(api_key) = request.api_key.as_deref().filter(|value| !value.is_empty()) {
                builder = builder.bearer_auth(api_key);
            }
            match builder.send().await {
                Ok(response) => break response,
                Err(error) if attempt < run.policy.limits.max_retries => {
                    attempt += 1;
                    run.push_event(
                        unix_time_ms(),
                        AgentEventKind::Validation,
                        format!(
                            "Fournisseur indisponible ; nouvelle tentative {attempt}/{}.",
                            run.policy.limits.max_retries
                        ),
                        None,
                    );
                    store.save_run(&run)?;
                    if stop_cancelled_agent_run(&cancellation, &mut run, &store)? {
                        return Ok(run);
                    }
                    let _ = error;
                }
                Err(error) => {
                    return Err(agent_error(
                        "AGENT_PROVIDER_UNREACHABLE",
                        "Le fournisseur IA n’a pas répondu.",
                        error.without_url().to_string(),
                    ));
                }
            }
        };
        if stop_cancelled_agent_run(&cancellation, &mut run, &store)? {
            return Ok(run);
        }
        let status = response.status();
        if !status.is_success() {
            run.status = AgentRunStatus::Failed;
            run.push_event(
                unix_time_ms(),
                AgentEventKind::Failed,
                format!("Le fournisseur a renvoyé HTTP {status}."),
                None,
            );
            store.save_run(&run)?;
            return Ok(run);
        }
        if response
            .content_length()
            .is_some_and(|length| length > run.policy.limits.max_response_bytes as u64)
        {
            return Err(agent_error(
                "AGENT_PROVIDER_RESPONSE_TOO_LARGE",
                "La réponse du fournisseur dépasse la limite du profil.",
                "provider content length exceeds policy limit",
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            agent_error(
                "AGENT_PROVIDER_RESPONSE_READ_FAILED",
                "La réponse du fournisseur n’a pas pu être lue.",
                error.without_url().to_string(),
            )
        })? {
            if bytes.len().saturating_add(chunk.len()) > run.policy.limits.max_response_bytes {
                return Err(agent_error(
                    "AGENT_PROVIDER_RESPONSE_TOO_LARGE",
                    "La réponse du fournisseur dépasse la limite du profil.",
                    "chunked provider response exceeds policy limit",
                ));
            }
            bytes.extend_from_slice(&chunk);
            if stop_cancelled_agent_run(&cancellation, &mut run, &store)? {
                return Ok(run);
            }
        }
        let step = decode_provider_response(run.provider.kind, &bytes)?;
        if run.provider.kind == ProviderKind::OpenAiResponses
            && run.provider.store_responses
            && !step.tool_calls.is_empty()
            && step.response_id.is_none()
        {
            run.status = AgentRunStatus::Failed;
            run.push_event(
                unix_time_ms(),
                AgentEventKind::Failed,
                "La rÃ©ponse Responses ne contient pas dâ€™identifiant de continuation.",
                None,
            );
            store.save_run(&run)?;
            return Ok(run);
        }
        if (run.provider.input_cost_micro_usd_per_million_tokens > 0 && step.input_tokens.is_none())
            || (run.provider.output_cost_micro_usd_per_million_tokens > 0
                && step.output_tokens.is_none())
        {
            run.status = AgentRunStatus::Failed;
            run.push_event(
                unix_time_ms(),
                AgentEventKind::Failed,
                "Le fournisseur n’indique pas l’usage token nécessaire au budget de coût.",
                None,
            );
            store.save_run(&run)?;
            return Ok(run);
        }
        let turn_cost = token_cost_micro_usd(
            step.input_tokens.unwrap_or(0),
            run.provider.input_cost_micro_usd_per_million_tokens,
        )
        .saturating_add(token_cost_micro_usd(
            step.output_tokens.unwrap_or(0),
            run.provider.output_cost_micro_usd_per_million_tokens,
        ));
        run.estimated_cost_micro_usd = run.estimated_cost_micro_usd.saturating_add(turn_cost);
        if run.provider.kind == ProviderKind::OpenAiResponses {
            if run.provider.store_responses {
                run.provider_conversation.previous_response_id = step.response_id.clone();
            } else {
                if run.provider_conversation.replay_items.is_empty() {
                    run.provider_conversation
                        .replay_items
                        .push(serde_json::json!({"role":"user","content":request_task_context}));
                }
                run.provider_conversation.replay_items.extend(
                    run.provider_conversation
                        .pending_tool_outputs
                        .iter()
                        .map(|result| {
                            serde_json::json!({
                                "type": "function_call_output",
                                "call_id": result.call_id,
                                "output": serde_json::to_string(&result.output).unwrap_or_else(|_| "null".to_owned()),
                            })
                        }),
                );
                run.provider_conversation
                    .replay_items
                    .extend(step.output_items.iter().cloned());
                run.provider_conversation.previous_response_id = None;
            }
            run.provider_conversation.pending_tool_outputs.clear();
        }
        run.push_event(
            unix_time_ms(),
            AgentEventKind::ModelResponse,
            step.assistant_text
                .as_deref()
                .unwrap_or("Le fournisseur a proposé des appels d’outils."),
            Some(serde_json::json!({
                "responseId": step.response_id,
                "toolCalls": step.tool_calls.len(),
                "inputTokens": step.input_tokens,
                "outputTokens": step.output_tokens,
            })),
        );
        if run.estimated_cost_micro_usd > run.policy.limits.max_cost_micro_usd {
            run.status = AgentRunStatus::Failed;
            run.push_event(
                unix_time_ms(),
                AgentEventKind::Failed,
                "Le budget de coût de l’exécution est dépassé.",
                Some(serde_json::json!({
                    "estimatedCostMicroUsd": run.estimated_cost_micro_usd,
                    "budgetMicroUsd": run.policy.limits.max_cost_micro_usd,
                })),
            );
            store.save_run(&run)?;
            return Ok(run);
        }
        if step.tool_calls.is_empty() {
            if step
                .assistant_text
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty())
            {
                run.status = AgentRunStatus::Completed;
                run.push_event(
                    unix_time_ms(),
                    AgentEventKind::Completed,
                    "Le fournisseur a terminé l’exécution sans nouvel appel d’outil.",
                    None,
                );
            } else {
                run.status = AgentRunStatus::Failed;
                run.push_event(
                    unix_time_ms(),
                    AgentEventKind::Failed,
                    "Le fournisseur n’a renvoyé ni résultat ni appel d’outil.",
                    None,
                );
            }
            store.save_run(&run)?;
            return Ok(run);
        }

        let remaining_tool_calls =
            (run.policy.limits.max_tool_calls as usize).saturating_sub(run.tool_calls.len());
        let turn_parallel_limit = if run.provider.supports_parallel_tools {
            run.policy.limits.max_parallel_calls as usize
        } else {
            1
        };
        let allowed_tool_calls = remaining_tool_calls.min(turn_parallel_limit);
        if step.tool_calls.len() > allowed_tool_calls {
            run.status = AgentRunStatus::Failed;
            run.push_event(
                unix_time_ms(),
                AgentEventKind::Failed,
                "Le fournisseur a proposé plus d’outils que le budget de ce tour ne l’autorise.",
                Some(serde_json::json!({
                    "proposed": step.tool_calls.len(),
                    "allowed": allowed_tool_calls,
                })),
            );
            store.save_run(&run)?;
            return Ok(run);
        }
        let mut waiting_for_approval = false;
        let mut batch_approval_calls = Vec::<String>::new();
        let mut seen_tool_call_ids = run
            .tool_calls
            .iter()
            .map(|call| call.id.clone())
            .collect::<HashSet<_>>();
        for proposed in step.tool_calls {
            if !seen_tool_call_ids.insert(proposed.id.clone()) {
                run.status = AgentRunStatus::Failed;
                run.push_event(
                    unix_time_ms(),
                    AgentEventKind::Failed,
                    "Le fournisseur a réutilisé un identifiant d’appel d’outil.",
                    Some(serde_json::json!({ "toolCallId": proposed.id })),
                );
                break;
            }
            let arguments_sha256 = hex::encode(Sha256::digest(
                serde_json::to_vec(&proposed.arguments).unwrap_or_default(),
            ));
            let descriptor = registry.get(&proposed.capability_id);
            let Some(descriptor) = descriptor else {
                let record = ToolCallRecord {
                    id: proposed.id,
                    capability_id: proposed.capability_id,
                    arguments: proposed.arguments,
                    arguments_sha256,
                    status: ToolCallStatus::Rejected,
                    result: None,
                    error: Some("Capacité inconnue.".to_owned()),
                    started_unix_ms: None,
                    completed_unix_ms: Some(unix_time_ms()),
                };
                queue_provider_tool_output(&mut run, &record);
                run.tool_calls.push(record);
                continue;
            };
            if !agent_tool_is_implemented(&descriptor.id) {
                let record = ToolCallRecord {
                    id: proposed.id,
                    capability_id: proposed.capability_id,
                    arguments: proposed.arguments,
                    arguments_sha256,
                    status: ToolCallStatus::Rejected,
                    result: None,
                    error: Some("Capacité enregistrée mais pas encore exécutable.".to_owned()),
                    started_unix_ms: None,
                    completed_unix_ms: Some(unix_time_ms()),
                };
                queue_provider_tool_output(&mut run, &record);
                run.tool_calls.push(record);
                continue;
            }
            let (effective, mut decision) = evaluate_capability(
                &run.policy,
                descriptor,
                tool_call_count(&run, &descriptor.id),
            );
            if !matches!(decision, PolicyDecision::Denied { .. })
                && let Err(reason) =
                    validate_tool_scope(&run.policy, &effective, descriptor, &proposed.arguments)
            {
                decision = PolicyDecision::Denied { reason };
            }
            let mut record = ToolCallRecord {
                id: proposed.id,
                capability_id: proposed.capability_id,
                arguments: proposed.arguments,
                arguments_sha256,
                status: ToolCallStatus::Proposed,
                result: None,
                error: None,
                started_unix_ms: None,
                completed_unix_ms: None,
            };
            run.push_event(
                unix_time_ms(),
                AgentEventKind::ToolProposed,
                format!("Outil proposé : {}.", descriptor.id),
                Some(serde_json::json!({ "toolCallId": record.id, "argumentsSha256": record.arguments_sha256 })),
            );
            match decision {
                PolicyDecision::Denied { reason } => {
                    record.status = ToolCallStatus::Rejected;
                    record.error = Some(reason);
                    record.completed_unix_ms = Some(unix_time_ms());
                    queue_provider_tool_output(&mut run, &record);
                    run.tool_calls.push(record);
                }
                PolicyDecision::ApprovalRequired { reason } => {
                    record.status = ToolCallStatus::WaitingApproval;
                    if effective.approval == aurora_agent::ApprovalMode::PerBatch {
                        batch_approval_calls.push(record.id.clone());
                        run.tool_calls.push(record);
                        waiting_for_approval = true;
                        continue;
                    }
                    let approval = ApprovalRequest {
                        id: uuid::Uuid::new_v4().to_string(),
                        tool_call_id: record.id.clone(),
                        tool_call_ids: vec![record.id.clone()],
                        capability_id: record.capability_id.clone(),
                        summary: reason,
                        status: ApprovalStatus::Pending,
                        created_unix_ms: unix_time_ms(),
                        resolved_unix_ms: None,
                    };
                    run.push_event(
                        unix_time_ms(),
                        AgentEventKind::ApprovalRequested,
                        format!("Approbation requise pour {}.", record.capability_id),
                        Some(serde_json::json!({ "approvalId": approval.id, "toolCallId": record.id })),
                    );
                    run.tool_calls.push(record);
                    run.approvals.push(approval);
                    waiting_for_approval = true;
                }
                PolicyDecision::Allowed => {
                    record.status = ToolCallStatus::Running;
                    record.started_unix_ms = Some(unix_time_ms());
                    record_agent_checkpoint(&state, &mut run, descriptor)?;
                    let record_index = run.tool_calls.len();
                    run.tool_calls.push(record.clone());
                    store.save_run(&run)?;
                    match execute_agent_tool(&state, &run, &record) {
                        Ok(result) => {
                            record.status = ToolCallStatus::Completed;
                            record.result = Some(result);
                            record.completed_unix_ms = Some(unix_time_ms());
                            run.push_event(
                                unix_time_ms(),
                                AgentEventKind::ToolCompleted,
                                format!("Outil terminé : {}.", record.capability_id),
                                Some(serde_json::json!({ "toolCallId": record.id })),
                            );
                        }
                        Err(error) => {
                            record.status = ToolCallStatus::Failed;
                            record.error = Some(error.user_message.clone());
                            record.completed_unix_ms = Some(unix_time_ms());
                            run.push_event(
                                unix_time_ms(),
                                AgentEventKind::Validation,
                                error.user_message.clone(),
                                Some(serde_json::json!({ "toolCallId": record.id, "code": error.code })),
                            );
                            if run.policy.stop_on_validation_error {
                                run.status = AgentRunStatus::Failed;
                            }
                        }
                    }
                    run.tool_calls[record_index] = record;
                    let completed_record = run.tool_calls[record_index].clone();
                    queue_provider_tool_output(&mut run, &completed_record);
                }
            }
            if run.status == AgentRunStatus::Failed {
                break;
            }
        }
        if !batch_approval_calls.is_empty() {
            let approval = ApprovalRequest {
                id: uuid::Uuid::new_v4().to_string(),
                tool_call_id: batch_approval_calls[0].clone(),
                tool_call_ids: batch_approval_calls.clone(),
                capability_id: "batch".to_owned(),
                summary: format!(
                    "La politique exige une confirmation unique pour ce lot de {} appels.",
                    batch_approval_calls.len()
                ),
                status: ApprovalStatus::Pending,
                created_unix_ms: unix_time_ms(),
                resolved_unix_ms: None,
            };
            run.push_event(
                unix_time_ms(),
                AgentEventKind::ApprovalRequested,
                format!(
                    "Approbation requise pour un lot de {} outils.",
                    batch_approval_calls.len()
                ),
                Some(serde_json::json!({
                    "approvalId": approval.id,
                    "toolCallIds": batch_approval_calls,
                })),
            );
            run.approvals.push(approval);
        }
        if waiting_for_approval {
            run.status = AgentRunStatus::WaitingApproval;
            store.save_run(&run)?;
            return Ok(run);
        }
        if run.status == AgentRunStatus::Failed {
            store.save_run(&run)?;
            return Ok(run);
        }
        store.save_run(&run)?;
    }
    if run.status == AgentRunStatus::Running {
        run.status = AgentRunStatus::Failed;
        run.push_event(
            unix_time_ms(),
            AgentEventKind::Failed,
            "La boucle a atteint une limite de tours ou d’appels d’outils.",
            None,
        );
        store.save_run(&run)?;
    }
    Ok(run)
}

#[tauri::command]
pub async fn test_agent_provider(
    request: TestAgentProviderRequest,
) -> AppResult<AgentProviderTestReport> {
    validate_agent_policy(&request.policy).map_err(|error| {
        agent_error(
            "AGENT_POLICY_INVALID",
            "Le profil de sécurité IA n’est pas valide.",
            error,
        )
    })?;
    if request.provider.kind == ProviderKind::Manual {
        return Err(agent_error(
            "AGENT_PROVIDER_MANUAL",
            "Le mode manuel ne possède aucun modèle à contacter.",
            "manual provider cannot be connection-tested",
        ));
    }
    if request.provider.model.trim().is_empty() {
        return Err(agent_error(
            "AGENT_PROVIDER_MODEL_REQUIRED",
            "Indiquez le nom exact du modèle à tester.",
            "provider connection test requires a model",
        ));
    }
    if request
        .api_key
        .as_ref()
        .is_some_and(|key| key.len() > 16 * 1024)
    {
        return Err(agent_error(
            "AGENT_API_KEY_TOO_LARGE",
            "La clé temporaire dépasse la limite autorisée.",
            "ephemeral API key exceeds 16 KiB",
        ));
    }

    let endpoint = validated_agent_endpoint(&request.provider.endpoint, &request.policy.context)?;
    let body = if request.provider.kind == ProviderKind::OpenAiResponses {
        serde_json::json!({
            "model": request.provider.model,
            "instructions": "Vous effectuez un test de communication technique.",
            "input": "Répondez uniquement par OK.",
            "max_output_tokens": 128,
            "store": false,
        })
    } else {
        serde_json::json!({
            "model": request.provider.model,
            "messages": [{"role": "user", "content": "Répondez uniquement par OK."}],
            "max_tokens": 128,
            "stream": false,
        })
    };
    let timeout_seconds = request.policy.limits.max_duration_seconds.clamp(1, 120);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_seconds))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| {
            agent_error(
                "AGENT_PROVIDER_CLIENT_FAILED",
                "Le client du fournisseur IA n’a pas pu être initialisé.",
                error.to_string(),
            )
        })?;
    let started = Instant::now();
    let mut builder = client.post(endpoint.clone()).json(&body);
    if let Some(api_key) = request.api_key.as_deref().filter(|value| !value.is_empty()) {
        builder = builder.bearer_auth(api_key);
    }
    let mut response = builder.send().await.map_err(|error| {
        agent_error(
            "AGENT_PROVIDER_UNREACHABLE",
            "Impossible de joindre le modèle. Vérifiez l’endpoint et que le serveur est démarré.",
            error.without_url().to_string(),
        )
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(agent_error(
            "AGENT_PROVIDER_HTTP_ERROR",
            format!("Le fournisseur a refusé le test avec HTTP {status}."),
            status.to_string(),
        ));
    }
    const MAX_TEST_RESPONSE_BYTES: usize = 64 * 1024;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        agent_error(
            "AGENT_PROVIDER_RESPONSE_READ_FAILED",
            "La réponse de test n’a pas pu être lue.",
            error.without_url().to_string(),
        )
    })? {
        if bytes.len().saturating_add(chunk.len()) > MAX_TEST_RESPONSE_BYTES {
            return Err(agent_error(
                "AGENT_PROVIDER_RESPONSE_TOO_LARGE",
                "La réponse de test dépasse la limite de 64 Kio.",
                "provider test response exceeds 64 KiB",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let step = decode_provider_response(request.provider.kind, &bytes)?;
    let reply = step
        .assistant_text
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Réponse structurée reçue".to_owned());
    Ok(AgentProviderTestReport {
        endpoint_origin: endpoint.origin().ascii_serialization(),
        model: request.provider.model,
        latency_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        reply: reply.chars().take(160).collect(),
    })
}

#[tauri::command]
pub fn resolve_agent_approval(
    state: State<'_, AppState>,
    request: ResolveAgentApprovalRequest,
) -> AppResult<AgentRun> {
    let store = agent_store_for_workspace(&state, &request.workspace_id)?;
    let mut run = store.load_run(&request.run_id)?.ok_or_else(|| {
        agent_error(
            "AGENT_RUN_NOT_FOUND",
            "L’exécution IA demandée est introuvable.",
            format!("no persisted agent run has id {}", request.run_id),
        )
    })?;
    let approval_index = run
        .approvals
        .iter()
        .position(|approval| approval.id == request.approval_id)
        .ok_or_else(|| {
            agent_error(
                "AGENT_APPROVAL_NOT_FOUND",
                "La demande d’approbation est introuvable.",
                format!("no approval has id {}", request.approval_id),
            )
        })?;
    if run.approvals[approval_index].status != ApprovalStatus::Pending {
        return Err(agent_error(
            "AGENT_APPROVAL_ALREADY_RESOLVED",
            "Cette demande d’approbation a déjà été traitée.",
            request.approval_id,
        ));
    }
    let call_ids = if run.approvals[approval_index].tool_call_ids.is_empty() {
        vec![run.approvals[approval_index].tool_call_id.clone()]
    } else {
        run.approvals[approval_index].tool_call_ids.clone()
    };
    let call_indexes = call_ids
        .iter()
        .map(|call_id| {
            run.tool_calls
                .iter()
                .position(|call| call.id == *call_id)
                .ok_or_else(|| {
                    agent_error(
                        "AGENT_TOOL_CALL_NOT_FOUND",
                        "L’appel d’outil associé est introuvable.",
                        call_id.clone(),
                    )
                })
        })
        .collect::<AppResult<Vec<_>>>()?;
    let now = unix_time_ms();
    run.approvals[approval_index].status = if request.approved {
        ApprovalStatus::Approved
    } else {
        ApprovalStatus::Rejected
    };
    run.approvals[approval_index].resolved_unix_ms = Some(now);
    if request.approved {
        let registry = CapabilityRegistry::standard();
        run.status = AgentRunStatus::Planned;
        for (position, call_index) in call_indexes.iter().copied().enumerate() {
            if run.status == AgentRunStatus::Failed {
                run.tool_calls[call_index].status = ToolCallStatus::Rejected;
                run.tool_calls[call_index].error =
                    Some("Lot interrompu après l’échec d’un outil précédent.".to_owned());
                run.tool_calls[call_index].completed_unix_ms = Some(unix_time_ms());
                let rejected = run.tool_calls[call_index].clone();
                queue_provider_tool_output(&mut run, &rejected);
                continue;
            }
            let mut record = run.tool_calls[call_index].clone();
            record.status = ToolCallStatus::Running;
            record.started_unix_ms = Some(unix_time_ms());
            let descriptor = registry.get(&record.capability_id).ok_or_else(|| {
                agent_error(
                    "AGENT_CAPABILITY_UNKNOWN",
                    "La capacité approuvée n’existe plus dans le registre.",
                    record.capability_id.clone(),
                )
            })?;
            record_agent_checkpoint(&state, &mut run, descriptor)?;
            run.tool_calls[call_index] = record.clone();
            store.save_run(&run)?;
            match execute_agent_tool(&state, &run, &record) {
                Ok(result) => {
                    record.status = ToolCallStatus::Completed;
                    record.result = Some(result);
                    record.completed_unix_ms = Some(unix_time_ms());
                    run.push_event(
                        unix_time_ms(),
                        AgentEventKind::ToolCompleted,
                        format!(
                            "Outil {}/{} du lot terminé : {}.",
                            position + 1,
                            call_indexes.len(),
                            record.capability_id
                        ),
                        Some(serde_json::json!({ "toolCallId": record.id })),
                    );
                }
                Err(error) => {
                    record.status = ToolCallStatus::Failed;
                    record.error = Some(error.user_message.clone());
                    record.completed_unix_ms = Some(unix_time_ms());
                    if run.policy.stop_on_validation_error {
                        run.status = AgentRunStatus::Failed;
                    }
                }
            }
            run.tool_calls[call_index] = record;
            let completed_record = run.tool_calls[call_index].clone();
            queue_provider_tool_output(&mut run, &completed_record);
        }
    } else {
        for call_index in call_indexes {
            run.tool_calls[call_index].status = ToolCallStatus::Rejected;
            run.tool_calls[call_index].error = Some("Appel refusé par l’utilisateur.".to_owned());
            run.tool_calls[call_index].completed_unix_ms = Some(now);
            let rejected = run.tool_calls[call_index].clone();
            queue_provider_tool_output(&mut run, &rejected);
        }
        run.status = AgentRunStatus::Planned;
    }
    run.push_event(
        now,
        AgentEventKind::ApprovalResolved,
        if request.approved {
            "Lot approuvé et exécuté."
        } else {
            "Lot refusé par l’utilisateur."
        },
        Some(
            serde_json::json!({ "approvalId": request.approval_id, "approved": request.approved }),
        ),
    );
    store.save_run(&run)?;
    Ok(run)
}

fn queue_provider_tool_output(run: &mut AgentRun, call: &ToolCallRecord) {
    if run.provider.kind != ProviderKind::OpenAiResponses {
        return;
    }
    let output = match call.status {
        ToolCallStatus::Completed => serde_json::json!({
            "ok": true,
            "result": call.result,
        }),
        ToolCallStatus::Rejected | ToolCallStatus::Failed => serde_json::json!({
            "ok": false,
            "status": call.status,
            "error": call.error,
        }),
        _ => return,
    };
    run.provider_conversation
        .pending_tool_outputs
        .push(ProviderToolOutput {
            call_id: call.id.clone(),
            output,
        });
}

#[tauri::command]
pub fn cancel_agent_run(
    state: State<'_, AppState>,
    request: CancelAgentRunRequest,
) -> AppResult<AgentRun> {
    let store = agent_store_for_workspace(&state, &request.workspace_id)?;
    let mut run = store.load_run(&request.run_id)?.ok_or_else(|| {
        agent_error(
            "AGENT_RUN_NOT_FOUND",
            "L’exécution IA demandée est introuvable.",
            format!("no persisted agent run has id {}", request.run_id),
        )
    })?;
    if run.workspace_id != request.workspace_id {
        return Err(agent_error(
            "AGENT_RUN_WORKSPACE_MISMATCH",
            "L’exécution IA n’appartient pas à ce workspace.",
            format!("run {} belongs to workspace {}", run.id, run.workspace_id),
        ));
    }
    if matches!(
        run.status,
        AgentRunStatus::Completed | AgentRunStatus::Failed | AgentRunStatus::Cancelled
    ) {
        return Ok(run);
    }
    let active = state
        .agent_cancellations
        .lock()
        .expect("agent cancellation registry poisoned")
        .get(&request.run_id)
        .cloned();
    if let Some(cancellation) = active {
        cancellation.store(true, Ordering::Release);
        run.status = AgentRunStatus::Cancelled;
        return Ok(run);
    }
    run.status = AgentRunStatus::Cancelled;
    run.push_event(
        unix_time_ms(),
        AgentEventKind::Cancelled,
        "Exécution annulée avant le prochain tour.",
        None,
    );
    store.save_run(&run)?;
    Ok(run)
}

fn agent_store_for_workspace(
    state: &AppState,
    workspace_id: &str,
) -> AppResult<AgentWorkspaceStore> {
    let snapshot = with_edit_workspace(state, workspace_id, |workspace| workspace.snapshot())?;
    Ok(AgentWorkspaceStore::new(snapshot.root))
}

fn record_agent_checkpoint(
    state: &AppState,
    run: &mut AgentRun,
    descriptor: &aurora_agent::CapabilityDescriptor,
) -> AppResult<()> {
    if !run.policy.checkpoint_before_write
        || descriptor.side_effect != CapabilitySideEffect::ReversibleWorkspace
    {
        return Ok(());
    }
    let snapshot = with_edit_workspace(state, &run.workspace_id, |workspace| workspace.snapshot())?;
    run.push_event(
        unix_time_ms(),
        AgentEventKind::Checkpoint,
        format!("Checkpoint créé avant {}.", descriptor.id),
        Some(serde_json::json!({
            "checkpointId": format!("{}:{}", run.id, snapshot.cursor),
            "cursor": snapshot.cursor,
            "sourceSha256": snapshot.source.sha256,
            "capabilityId": descriptor.id,
        })),
    );
    Ok(())
}

fn agent_tool_is_implemented(id: &str) -> bool {
    matches!(
        id,
        "module.inspect"
            | "architecture.query"
            | "resource.search"
            | "resource.read"
            | "diagnostics.run"
            | "resource.set_field"
            | "script.replace"
            | "script.create"
            | "script.compile"
            | "area.create"
            | "area.instance.add"
            | "map.generate"
            | "map.context"
            | "map.inspect"
            | "map.atlas"
            | "map.preview"
            | "map.apply"
            | "map.environment.edit"
            | "map.audio.edit"
            | "map.tile.edit"
            | "map.instance.add"
            | "map.instance.move"
            | "map.instance.remove"
            | "map.structure.edit"
            | "dialogue.create"
            | "dialogue.edit"
            | "journal.edit"
            | "faction.edit"
            | "blueprint.edit"
            | "blueprint.validate"
            | "blueprint.apply"
            | "workspace.checkpoint"
            | "workspace.undo_batch"
            | "module.validate"
            | "module.build"
            | "module.create"
            | "module.dependencies"
            | "2da.edit"
            | "tlk.edit"
            | "walkmesh.edit"
            | "development.deploy"
            | "toolset.compare"
            | "toolset.sync"
            | "nwn.launch"
    )
}

fn tool_call_count(run: &AgentRun, capability_id: &str) -> u32 {
    run.tool_calls
        .iter()
        .filter(|call| call.capability_id == capability_id)
        .count() as u32
}

fn agent_system_prompt() -> &'static str {
    "You are the controlled OpenNever Forge module construction agent. Use only the supplied tools. Every write is applied by the local transactional engine; never claim a change before its tool succeeds. Inspect before editing, preserve unknown NWN data, and use exact before-values. When the objective is satisfied, return a concise completion report without another tool call. Do not request shell access, arbitrary paths, secrets, or direct writes to source modules."
}

fn agent_task_context(run: &AgentRun, max_bytes: usize) -> String {
    let mut history = run
        .tool_calls
        .iter()
        .rev()
        .take(32)
        .rev()
        .map(|call| {
            serde_json::json!({
                "capability": call.capability_id,
                "argumentsSha256": call.arguments_sha256,
                "status": call.status,
                "result": bounded_agent_value(
                    call.result.as_ref(),
                    8 * 1024,
                    run.policy.context.include_local_paths,
                ),
                "error": call.error,
            })
        })
        .collect::<Vec<_>>();
    let mut objective = run.objective.clone();
    let objective_limit = max_bytes / 2;
    if objective.len() > objective_limit {
        objective.truncate(objective.floor_char_boundary(objective_limit));
    }
    loop {
        let encoded = serde_json::json!({
            "objective": objective,
            "workspaceId": run.workspace_id,
            "turn": run.current_turn,
            "limits": run.policy.limits,
            "moduleBlueprint": bounded_agent_value(
                run.blueprint.as_ref().and_then(|value| serde_json::to_value(value).ok()).as_ref(),
                max_bytes / 3,
                run.policy.context.include_local_paths,
            ),
            "previousToolResults": history,
        })
        .to_string();
        if encoded.len() <= max_bytes || history.is_empty() {
            return encoded;
        }
        history.remove(0);
    }
}

fn bounded_agent_value(
    value: Option<&serde_json::Value>,
    max_bytes: usize,
    include_local_paths: bool,
) -> serde_json::Value {
    let Some(value) = value else {
        return serde_json::Value::Null;
    };
    let safe = sanitize_context_value(value, include_local_paths);
    let encoded = serde_json::to_vec(&safe).unwrap_or_default();
    if encoded.len() <= max_bytes {
        return safe;
    }
    serde_json::json!({
        "truncated": true,
        "sizeBytes": encoded.len(),
        "sha256": hex::encode(Sha256::digest(encoded)),
    })
}

fn token_cost_micro_usd(tokens: u64, price_per_million: u64) -> u64 {
    ((u128::from(tokens) * u128::from(price_per_million)) / 1_000_000).min(u128::from(u64::MAX))
        as u64
}

fn execute_agent_tool(
    state: &AppState,
    run: &AgentRun,
    call: &ToolCallRecord,
) -> AppResult<serde_json::Value> {
    let current_arguments_sha256 = hex::encode(Sha256::digest(
        serde_json::to_vec(&call.arguments).unwrap_or_default(),
    ));
    if current_arguments_sha256 != call.arguments_sha256 {
        return Err(agent_error(
            "AGENT_TOOL_ARGUMENTS_CHANGED",
            "Les paramètres de l’outil ont changé depuis leur validation.",
            call.id.clone(),
        ));
    }
    match call.capability_id.as_str() {
        "module.inspect" => {
            let workspace =
                with_edit_workspace(state, &run.workspace_id, |workspace| workspace.snapshot())?;
            state.jobs.with_analysis(&run.job_id, |analysis| {
                Ok(serde_json::json!({
                    "module": analysis.module_info,
                    "fingerprint": analysis.fingerprint,
                    "resourceCount": analysis.inventory.resource_count,
                    "workspace": workspace,
                }))
            })
        }
        "architecture.query" => {
            let arguments: AgentArchitectureQueryArguments =
                decode_agent_arguments(call, "architecture.query")?;
            if arguments.query.trim().is_empty() || arguments.query.len() > 128 {
                return Err(agent_error(
                    "AGENT_ARCHITECTURE_QUERY_INVALID",
                    "La requête d’architecture doit contenir entre 1 et 128 caractères.",
                    arguments.query,
                ));
            }
            let script = std::env::current_dir()
                .map_err(|error| {
                    agent_error(
                        "AGENT_ARCHITECTURE_ROOT_UNAVAILABLE",
                        "La racine du projet ne peut pas être résolue.",
                        error.to_string(),
                    )
                })?
                .join("scripts")
                .join("architecture_graph.py");
            if !script.is_file() {
                return Err(agent_error(
                    "AGENT_ARCHITECTURE_GRAPH_UNAVAILABLE",
                    "Le graphe d’architecture développeur n’est pas installé.",
                    script.display().to_string(),
                ));
            }
            let output = Command::new("python")
                .arg(script)
                .arg("query")
                .arg(&arguments.query)
                .arg("--depth")
                .arg("1")
                .arg("--max-nodes")
                .arg("40")
                .output()
                .map_err(|error| {
                    agent_error(
                        "AGENT_ARCHITECTURE_QUERY_FAILED",
                        "La requête d’architecture n’a pas pu être exécutée.",
                        error.to_string(),
                    )
                })?;
            if !output.status.success() || output.stdout.len() > 64 * 1024 {
                return Err(agent_error(
                    "AGENT_ARCHITECTURE_QUERY_FAILED",
                    "Le générateur d’architecture a refusé la requête.",
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
            Ok(serde_json::json!({
                "query": arguments.query,
                "result": String::from_utf8_lossy(&output.stdout).to_string(),
            }))
        }
        "resource.search" => {
            let arguments: AgentResourceSearchArguments =
                decode_agent_arguments(call, "resource.search")?;
            state.jobs.with_analysis(&run.job_id, |analysis| {
                serde_json::to_value(analysis.resource_catalog.search_many(
                    &arguments.query,
                    &[],
                    None,
                    0,
                    arguments.limit.clamp(1, 200),
                ))
                .map_err(|error| {
                    agent_error(
                        "AGENT_TOOL_RESULT_INVALID",
                        "Le résultat de recherche n’a pas pu être préparé.",
                        error.to_string(),
                    )
                })
            })
        }
        "resource.read" => {
            if tool_call_count(run, "resource.read")
                > run.policy.limits.max_context_resources as u32
            {
                return Err(agent_error(
                    "AGENT_CONTEXT_RESOURCE_LIMIT",
                    "Le nombre maximal de ressources de contexte est atteint.",
                    run.policy.limits.max_context_resources.to_string(),
                ));
            }
            let resource: ResourceKey = decode_agent_arguments(call, "resource.read")?;
            let bytes = workspace_or_resolved_resource_bytes(
                state,
                &run.job_id,
                &run.workspace_id,
                &resource,
            )?
            .ok_or_else(|| {
                agent_error(
                    "AGENT_RESOURCE_MISSING",
                    "La ressource demandée est introuvable.",
                    resource.to_string(),
                )
            })?;
            if bytes.len() > run.policy.limits.max_context_resource_bytes {
                return Err(agent_error(
                    "AGENT_CONTEXT_RESOURCE_TOO_LARGE",
                    "La ressource dépasse la taille maximale de contexte.",
                    format!(
                        "{} bytes exceeds {}",
                        bytes.len(),
                        run.policy.limits.max_context_resource_bytes
                    ),
                ));
            }
            Ok(serde_json::json!({
                "resource": resource,
                "sha256": hex::encode(Sha256::digest(&bytes)),
                "content": ai_resource_context(&resource, &bytes)?,
            }))
        }
        "diagnostics.run" | "module.validate" => {
            let (workspace, compilation_error) =
                with_edit_workspace(state, &run.workspace_id, |workspace| {
                    let compilation_error =
                        workspace.validate_compiled_scripts().err().map(|error| {
                            serde_json::json!({
                                "code": error.code,
                                "message": error.user_message,
                                "technicalMessage": error.technical_message,
                            })
                        });
                    Ok((workspace.snapshot()?, compilation_error))
                })?;
            state.jobs.with_analysis(&run.job_id, |analysis| {
                let report = analysis
                    .world_index
                    .report(analysis.fingerprint.sha256.clone());
                let compiled_scripts_valid = compilation_error.is_none();
                Ok(serde_json::json!({
                    "sourceIntact": workspace.source_intact,
                    "modifiedResources": workspace.modified_resources.len(),
                    "deletedResources": workspace.deleted_resources.len(),
                    "compiledScriptsValid": compiled_scripts_valid,
                    "compilationDiagnostic": compilation_error,
                    "diagnostics": report.diagnostics,
                    "valid": workspace.source_intact && compiled_scripts_valid && report.diagnostics.iter().all(|diagnostic| diagnostic.severity != aurora_world::DiagnosticSeverity::Error),
                }))
            })
        }
        "resource.set_field" => {
            let arguments: AgentSetFieldArguments =
                decode_agent_arguments(call, "resource.set_field")?;
            let change_set = AiChangeSet {
                summary: format!(
                    "Agent : modifier {} dans {}",
                    arguments.path, arguments.resource
                ),
                commands: vec![EditCommand::SetField {
                    resource: arguments.resource,
                    path: arguments.path,
                    before: arguments.before,
                    after: arguments.after,
                }],
            };
            apply_agent_change_set(state, run, change_set)
        }
        "script.replace" => {
            let arguments: AgentReplaceScriptArguments =
                decode_agent_arguments(call, "script.replace")?;
            let change_set = AiChangeSet {
                summary: format!("Agent : remplacer {}", arguments.resource),
                commands: vec![EditCommand::ReplaceText {
                    resource: arguments.resource,
                    before: arguments.before,
                    after: arguments.after,
                }],
            };
            apply_agent_change_set(state, run, change_set)
        }
        "script.create" => {
            let arguments: AgentScriptCreateArguments =
                decode_agent_arguments(call, "script.create")?;
            parse_nss(
                arguments.source.as_bytes(),
                &format!("agent-create::{}.nss", arguments.resref),
            )?;
            let resource = ResourceKey::new(&arguments.resref, 2009);
            state.jobs.with_analysis(&run.job_id, |analysis| {
                if analysis.resource_catalog.get(&resource).is_some() {
                    return Err(agent_error(
                        "AGENT_SCRIPT_ALREADY_EXISTS",
                        "Le script à créer existe déjà.",
                        resource.to_string(),
                    ));
                }
                Ok(())
            })?;
            let workspace = with_edit_workspace(state, &run.workspace_id, |workspace| {
                workspace.create_resources_atomic(&[ErfResourceInput {
                    key: resource.clone(),
                    bytes: arguments.source.into_bytes(),
                }])
            })?;
            Ok(serde_json::json!({
                "resource": resource,
                "event": arguments.event,
                "purpose": arguments.purpose,
                "requiresCompilation": true,
                "workspace": workspace,
            }))
        }
        "script.compile" => {
            let arguments: AgentScriptCompileArguments =
                decode_agent_arguments(call, "script.compile")?;
            let runtime = &run.policy.tool_runtime;
            if runtime.compiler_path.trim().is_empty()
                || runtime.game_install_path.trim().is_empty()
            {
                return Err(agent_error(
                    "AGENT_COMPILER_NOT_CONFIGURED",
                    "Configurez le compilateur NWScript et l’installation du jeu dans Agent Studio.",
                    "toolRuntime.compilerPath or toolRuntime.gameInstallPath is empty",
                ));
            }
            let result = compile_workspace_script_inner(
                state,
                CompileScriptRequest {
                    job_id: run.job_id.clone(),
                    workspace_id: run.workspace_id.clone(),
                    resref: arguments.resref,
                    compiler_path: runtime.compiler_path.clone(),
                    game_install_path: runtime.game_install_path.clone(),
                    include_paths: runtime.include_paths.clone(),
                },
            )?;
            serde_json::to_value(result).map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le résultat de compilation n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        "area.create" => {
            let arguments: AgentAreaCreateArguments = decode_agent_arguments(call, "area.create")?;
            let resources = create_area_resources(
                &arguments.resref,
                &arguments.name,
                &arguments.tileset,
                arguments.width,
                arguments.height,
                arguments.tile_id,
            )?;
            state.jobs.with_analysis(&run.job_id, |analysis| {
                for resource in &resources {
                    if analysis.resource_catalog.get(&resource.key).is_some() {
                        return Err(agent_error(
                            "AGENT_AREA_ALREADY_EXISTS",
                            "Une ressource de cette zone existe déjà.",
                            resource.key.to_string(),
                        ));
                    }
                }
                Ok(())
            })?;
            let workspace = with_edit_workspace(state, &run.workspace_id, |workspace| {
                workspace.create_resources_atomic(&resources)
            })?;
            Ok(serde_json::json!({
                "createdResources": resources.iter().map(|resource| resource.key.to_string()).collect::<Vec<_>>(),
                "workspace": workspace,
            }))
        }
        "area.instance.add" => {
            let arguments: AgentAreaInstanceArguments =
                decode_agent_arguments(call, "area.instance.add")?;
            let resource = ResourceKey::new(&arguments.area, 2023);
            let source_bytes = try_resolved_resource_bytes(state, &run.job_id, &resource)?;
            let placement = arguments.placement;
            let area = arguments.area;
            let (workspace, instance_id) =
                with_edit_workspace(state, &run.workspace_id, |workspace| {
                    let current = workspace
                        .staged_resource_bytes(&resource)?
                        .or_else(|| source_bytes.clone())
                        .ok_or_else(|| {
                            agent_error(
                                "AGENT_AREA_RESOURCE_MISSING",
                                "La ressource GIT de la zone est introuvable.",
                                resource.to_string(),
                            )
                        })?;
                    let (output, instance_id) = add_area_instance(
                        &current,
                        &format!("workspace::{resource}"),
                        &area,
                        &placement,
                    )?;
                    workspace.stage_resource(resource.clone(), source_bytes.as_deref(), &output)?;
                    let snapshot = workspace.apply(EditCommand::AddInstance {
                        area: area.clone(),
                        instance_id: instance_id.clone(),
                        placement: placement.clone(),
                    })?;
                    Ok((snapshot, instance_id))
                })?;
            Ok(serde_json::json!({ "instanceId": instance_id, "workspace": workspace }))
        }
        "map.generate" => {
            let spec: MapGenerationSpec = decode_agent_arguments(call, "map.generate")?;
            let result = apply_map_generation_in_workspace(
                state,
                &run.job_id,
                &run.workspace_id,
                spec,
                None,
            )?;
            Ok(serde_json::json!({
                "planSha256": result.plan.plan_sha256,
                "area": result.area.resref,
                "metrics": result.plan.metrics,
                "warnings": result.plan.warnings,
                "workspaceCursor": result.workspace.cursor,
            }))
        }
        "map.context" => {
            let arguments: AgentMapContextArguments = decode_agent_arguments(call, "map.context")?;
            let selected = state.jobs.with_analysis(&run.job_id, |analysis| {
                let mut tilesets = analysis
                    .resource_catalog
                    .entries
                    .iter()
                    .filter(|entry| entry.key.resource_type == 2013)
                    .map(|entry| entry.key.resref.clone())
                    .collect::<Vec<_>>();
                tilesets.sort();
                tilesets.dedup();
                Ok(arguments
                    .tileset
                    .clone()
                    .or_else(|| tilesets.first().cloned()))
            })?;
            let context = build_map_authoring_context(
                state,
                &run.job_id,
                selected.as_deref().unwrap_or_default(),
            )?;
            let query = arguments.query.trim().to_ascii_lowercase();
            let limit = arguments.limit.clamp(1, 500);
            let mut blueprints = map_blueprint_resrefs(state, &run.job_id)?;
            for values in blueprints.values_mut() {
                values.retain(|value| query.is_empty() || value.contains(&query));
                values.truncate(limit);
            }
            let areas = state.jobs.with_analysis(&run.job_id, |analysis| {
                Ok(analysis
                    .world_index
                    .areas
                    .iter()
                    .map(|area| area.resref.clone())
                    .collect::<Vec<_>>())
            })?;
            Ok(serde_json::json!({
                "limits": context.limits,
                "availableTilesets": context.available_tilesets,
                "selectedTileset": context.selected_tileset,
                "blueprints": blueprints,
                "existingAreas": areas,
                "editing": {
                    "tileCoordinates": "x and y are zero-based",
                    "worldUnitsPerTile": 10,
                    "instanceCategories": map_blueprint_categories(),
                    "visualEdgeCompatibilityVerified": false,
                }
            }))
        }
        "map.inspect" => {
            let arguments: AgentMapAreaArguments = decode_agent_arguments(call, "map.inspect")?;
            let are_key = ResourceKey::new(&arguments.area, 2012);
            let git_key = ResourceKey::new(&arguments.area, 2023);
            let gic_key = ResourceKey::new(&arguments.area, 2046);
            let are = workspace_or_resolved_resource_bytes(
                state,
                &run.job_id,
                &run.workspace_id,
                &are_key,
            )?
            .ok_or_else(|| {
                agent_error(
                    "AGENT_MAP_AREA_MISSING",
                    "La zone est introuvable.",
                    are_key.to_string(),
                )
            })?;
            let git = workspace_or_resolved_resource_bytes(
                state,
                &run.job_id,
                &run.workspace_id,
                &git_key,
            )?;
            let gic = workspace_or_resolved_resource_bytes(
                state,
                &run.job_id,
                &run.workspace_id,
                &gic_key,
            )?;
            let are_document = parse_gff(&are, &format!("agent::{}", are_key.file_name()))?;
            let git_document = git
                .as_deref()
                .map(|bytes| parse_gff(bytes, &format!("agent::{}", git_key.file_name())))
                .transpose()?;
            let gic_document = gic
                .as_deref()
                .map(|bytes| parse_gff(bytes, &format!("agent::{}", gic_key.file_name())))
                .transpose()?;
            let area = adapt_area(
                &arguments.area,
                &are_document,
                git_document.as_ref(),
                gic_document.as_ref(),
            );
            let environment =
                inspect_area_environment(&are, &format!("agent::{}", are_key.file_name()))?;
            let audio = git
                .as_deref()
                .map(|bytes| inspect_area_audio(bytes, &format!("agent::{}", git_key.file_name())))
                .transpose()?;
            Ok(serde_json::json!({
                "area": area,
                "environment": environment,
                "audio": audio,
                "resourceSha256": {
                    "are": hex::encode(Sha256::digest(&are)),
                    "git": git.as_deref().map(|bytes| hex::encode(Sha256::digest(bytes))),
                    "gic": gic.as_deref().map(|bytes| hex::encode(Sha256::digest(bytes))),
                }
            }))
        }
        "map.atlas" => {
            let arguments: AgentMapAreaArguments = decode_agent_arguments(call, "map.atlas")?;
            let are_key = ResourceKey::new(&arguments.area, 2012);
            let git_key = ResourceKey::new(&arguments.area, 2023);
            let gic_key = ResourceKey::new(&arguments.area, 2046);
            let are = workspace_or_resolved_resource_bytes(
                state,
                &run.job_id,
                &run.workspace_id,
                &are_key,
            )?
            .ok_or_else(|| {
                agent_error(
                    "AGENT_MAP_AREA_MISSING",
                    "La zone est introuvable.",
                    are_key.to_string(),
                )
            })?;
            let git = workspace_or_resolved_resource_bytes(
                state,
                &run.job_id,
                &run.workspace_id,
                &git_key,
            )?;
            let gic = workspace_or_resolved_resource_bytes(
                state,
                &run.job_id,
                &run.workspace_id,
                &gic_key,
            )?;
            let are_document = parse_gff(&are, &format!("agent::{}", are_key.file_name()))?;
            let git_document = git
                .as_deref()
                .map(|bytes| parse_gff(bytes, &format!("agent::{}", git_key.file_name())))
                .transpose()?;
            let gic_document = gic
                .as_deref()
                .map(|bytes| parse_gff(bytes, &format!("agent::{}", gic_key.file_name())))
                .transpose()?;
            let area = adapt_area(
                &arguments.area,
                &are_document,
                git_document.as_ref(),
                gic_document.as_ref(),
            );
            let svg = render_area_atlas_svg(&area);
            Ok(serde_json::json!({
                "area": arguments.area,
                "mimeType":"image/svg+xml",
                "sha256":hex::encode(Sha256::digest(svg.as_bytes())),
                "svg":svg,
            }))
        }
        "map.preview" => {
            let arguments: AgentMapPreviewArguments = decode_agent_arguments(call, "map.preview")?;
            validate_map_blueprints(state, &run.job_id, &arguments.spec)?;
            serde_json::to_value(generate_verified_map_plan(
                state,
                &run.job_id,
                &arguments.spec,
            )?)
            .map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le plan de carte ne peut pas Ãªtre retournÃ©.",
                    error.to_string(),
                )
            })
        }
        "map.apply" => {
            let arguments: AgentMapApplyArguments = decode_agent_arguments(call, "map.apply")?;
            let result = apply_map_generation_in_workspace(
                state,
                &run.job_id,
                &run.workspace_id,
                arguments.spec,
                Some(&arguments.expected_plan_sha256),
            )?;
            Ok(serde_json::json!({
                "planSha256": result.plan.plan_sha256,
                "area": result.area,
                "workspace": result.workspace,
            }))
        }
        "map.environment.edit" => {
            let arguments: AgentMapEnvironmentArguments =
                decode_agent_arguments(call, "map.environment.edit")?;
            let expected = arguments.expected_sha256;
            let patch = arguments.patch;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new(&arguments.area, 2012),
                serde_json::to_string(&patch).unwrap_or_else(|_| "map_environment".to_owned()),
                move |bytes, source| {
                    verify_map_resource_sha256(bytes, &expected)?;
                    edit_area_environment(bytes, source, &patch)
                },
            )
        }
        "map.audio.edit" => {
            let arguments: AgentMapAudioArguments = decode_agent_arguments(call, "map.audio.edit")?;
            let expected = arguments.expected_sha256;
            let patch = arguments.patch;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new(&arguments.area, 2023),
                serde_json::to_string(&patch).unwrap_or_else(|_| "map_audio".to_owned()),
                move |bytes, source| {
                    verify_map_resource_sha256(bytes, &expected)?;
                    edit_area_audio(bytes, source, &patch)
                },
            )
        }
        "map.tile.edit" => {
            let arguments: AgentMapTileArguments = decode_agent_arguments(call, "map.tile.edit")?;
            let expected = arguments.expected_sha256;
            let before = arguments.before;
            let after = arguments.after;
            let x = arguments.x;
            let y = arguments.y;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new(&arguments.area, 2012),
                format!("map_tile:{x},{y}"),
                move |bytes, source| {
                    verify_map_resource_sha256(bytes, &expected)?;
                    edit_area_tile_at(bytes, source, x, y, before, after)
                },
            )
        }
        "map.instance.add" => {
            let arguments: AgentMapInstanceAddArguments =
                decode_agent_arguments(call, "map.instance.add")?;
            let resource = ResourceKey::new(&arguments.area, 2023);
            validate_agent_map_placement(state, run, &arguments.placement)?;
            let expected = arguments.expected_sha256;
            let area = arguments.area;
            let placement = arguments.placement;
            apply_agent_resource_transform(
                state,
                run,
                resource,
                format!("map_instance_add:{}", placement.tag),
                move |bytes, source| {
                    verify_map_resource_sha256(bytes, &expected)?;
                    add_area_instance(bytes, source, &area, &placement).map(|(output, _)| output)
                },
            )
        }
        "map.instance.move" => {
            let arguments: AgentMapInstanceMoveArguments =
                decode_agent_arguments(call, "map.instance.move")?;
            let expected = arguments.expected_sha256;
            let area = arguments.area;
            let instance_id = arguments.instance_id;
            let before = arguments.before;
            let after = arguments.after;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new(&area, 2023),
                format!("map_instance_move:{instance_id}"),
                move |bytes, source| {
                    verify_map_resource_sha256(bytes, &expected)?;
                    edit_area_instance_by_id(bytes, source, &area, &instance_id, before, after)
                },
            )
        }
        "map.instance.remove" => {
            let arguments: AgentMapInstanceRemoveArguments =
                decode_agent_arguments(call, "map.instance.remove")?;
            let expected = arguments.expected_sha256;
            let area = arguments.area;
            let instance_id = arguments.instance_id;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new(&area, 2023),
                format!("map_instance_remove:{instance_id}"),
                move |bytes, source| {
                    verify_map_resource_sha256(bytes, &expected)?;
                    remove_area_instance(bytes, source, &area, &instance_id)
                },
            )
        }
        "map.structure.edit" => {
            let arguments: AgentMapStructureArguments =
                decode_agent_arguments(call, "map.structure.edit")?;
            let item_template = if let AreaStructureAction::AddInventoryItem { resref, .. } =
                &arguments.action
            {
                let key = ResourceKey::new(resref, 2025);
                workspace_or_resolved_resource_bytes(state, &run.job_id, &run.workspace_id, &key)?
                    .map(|bytes| parse_gff(&bytes, &format!("agent::{}", key.file_name())))
                    .transpose()?
            } else {
                None
            };
            let expected = arguments.expected_sha256;
            let area = arguments.area;
            let action = arguments.action;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new(&area, 2023),
                serde_json::to_string(&action).unwrap_or_else(|_| "map_structure".to_owned()),
                move |bytes, source| {
                    verify_map_resource_sha256(bytes, &expected)?;
                    edit_area_structure(bytes, source, &area, &action, item_template.as_ref())
                        .map(|(output, _)| output)
                },
            )
        }
        "dialogue.create" => {
            let dialogue: aurora_agent::DialogueBlueprint =
                decode_agent_arguments(call, "dialogue.create")?;
            let resource = create_dialogue_resource(
                &dialogue.resref,
                &dialogue.owner_tag,
                &dialogue.purpose,
                &dialogue.required_nodes,
            )?;
            state.jobs.with_analysis(&run.job_id, |analysis| {
                if analysis.resource_catalog.get(&resource.key).is_some() {
                    return Err(agent_error(
                        "AGENT_DIALOGUE_ALREADY_EXISTS",
                        "Le dialogue à créer existe déjà.",
                        resource.key.to_string(),
                    ));
                }
                Ok(())
            })?;
            let key = resource.key.clone();
            let workspace = with_edit_workspace(state, &run.workspace_id, |workspace| {
                workspace.create_resources_atomic(&[resource])
            })?;
            Ok(serde_json::json!({ "resource": key, "workspace": workspace }))
        }
        "dialogue.edit" => {
            let arguments: AgentDialogueEditArguments =
                decode_agent_arguments(call, "dialogue.edit")?;
            let action = arguments.action;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new(&arguments.resref, 2029),
                serde_json::to_string(&action).unwrap_or_else(|_| "dialogue_edit".to_owned()),
                move |bytes, source| {
                    edit_dialogue_structure(bytes, source, &action).map(|(output, _)| output)
                },
            )
        }
        "journal.edit" => {
            let arguments: AgentJournalEditArguments =
                decode_agent_arguments(call, "journal.edit")?;
            if arguments.resource.resource_type != 2056 {
                return Err(agent_error(
                    "AGENT_JOURNAL_RESOURCE_INVALID",
                    "La ressource ciblée n’est pas un journal JRL.",
                    arguments.resource.to_string(),
                ));
            }
            let action = arguments.action;
            apply_agent_resource_transform(
                state,
                run,
                arguments.resource,
                serde_json::to_string(&action).unwrap_or_else(|_| "journal_edit".to_owned()),
                move |bytes, source| {
                    edit_journal_structure(bytes, source, &action).map(|(output, _)| output)
                },
            )
        }
        "faction.edit" => {
            let arguments: AgentFactionEditArguments =
                decode_agent_arguments(call, "faction.edit")?;
            if arguments.resource.resource_type != 2038 {
                return Err(agent_error(
                    "AGENT_FACTION_RESOURCE_INVALID",
                    "La ressource ciblée n’est pas une matrice FAC.",
                    arguments.resource.to_string(),
                ));
            }
            let action = arguments.action;
            apply_agent_resource_transform(
                state,
                run,
                arguments.resource,
                serde_json::to_string(&action).unwrap_or_else(|_| "faction_edit".to_owned()),
                move |bytes, source| {
                    edit_faction_structure(bytes, source, &action).map(|(output, _)| output)
                },
            )
        }
        "blueprint.edit" => {
            let arguments: AgentBlueprintEditArguments =
                decode_agent_arguments(call, "blueprint.edit")?;
            let action = arguments.action;
            apply_agent_resource_transform(
                state,
                run,
                arguments.resource,
                serde_json::to_string(&action).unwrap_or_else(|_| "blueprint_edit".to_owned()),
                move |bytes, source| {
                    edit_blueprint_structure(bytes, source, &action).map(|(output, _)| output)
                },
            )
        }
        "workspace.checkpoint" => {
            let workspace =
                with_edit_workspace(state, &run.workspace_id, |workspace| workspace.snapshot())?;
            Ok(serde_json::json!({
                "checkpointId": format!("{}:{}", run.id, workspace.cursor),
                "cursor": workspace.cursor,
                "sourceSha256": workspace.source.sha256,
            }))
        }
        "workspace.undo_batch" => {
            let arguments: AgentUndoBatchArguments =
                decode_agent_arguments(call, "workspace.undo_batch")?;
            let (checkpoint_run, cursor_text) =
                arguments.checkpoint_id.rsplit_once(':').ok_or_else(|| {
                    agent_error(
                        "AGENT_CHECKPOINT_INVALID",
                        "L’identifiant de checkpoint n’est pas valide.",
                        arguments.checkpoint_id.clone(),
                    )
                })?;
            if checkpoint_run != run.id {
                return Err(agent_error(
                    "AGENT_CHECKPOINT_RUN_MISMATCH",
                    "Ce checkpoint appartient à une autre exécution.",
                    arguments.checkpoint_id,
                ));
            }
            let target_cursor = cursor_text.parse::<usize>().map_err(|error| {
                agent_error(
                    "AGENT_CHECKPOINT_INVALID",
                    "Le curseur du checkpoint n’est pas valide.",
                    error.to_string(),
                )
            })?;
            let workspace = with_edit_workspace(state, &run.workspace_id, |workspace| {
                let current = workspace.snapshot()?;
                if target_cursor > current.cursor {
                    return Err(agent_error(
                        "AGENT_CHECKPOINT_AHEAD",
                        "Le checkpoint est postérieur à l’état courant.",
                        format!("target {target_cursor}, current {}", current.cursor),
                    ));
                }
                while workspace.snapshot()?.cursor > target_cursor {
                    workspace.undo()?;
                }
                workspace.snapshot()
            })?;
            Ok(serde_json::json!({
                "restoredCheckpointId": format!("{}:{}", run.id, target_cursor),
                "workspace": workspace,
            }))
        }
        "blueprint.validate" => {
            let blueprint: ModuleBlueprint = decode_agent_arguments(call, "blueprint.validate")?;
            serde_json::to_value(compile_module_blueprint(&blueprint)).map_err(|error| {
                agent_error(
                    "AGENT_BLUEPRINT_RESULT_INVALID",
                    "Le plan de module n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        "blueprint.apply" => {
            let blueprint: ModuleBlueprint = decode_agent_arguments(call, "blueprint.apply")?;
            let plan = compile_module_blueprint(&blueprint);
            if !plan.validation.valid {
                return Err(agent_error(
                    "AGENT_BLUEPRINT_INVALID",
                    "Le plan de module contient des erreurs.",
                    serde_json::to_string(&plan.validation.diagnostics).unwrap_or_default(),
                ));
            }
            let mut occupied = state.jobs.with_analysis(&run.job_id, |analysis| {
                Ok(analysis
                    .resource_catalog
                    .entries
                    .iter()
                    .map(|entry| entry.key.clone())
                    .collect::<BTreeSet<_>>())
            })?;
            let staged =
                with_edit_workspace(state, &run.workspace_id, |workspace| workspace.snapshot())?;
            occupied.extend(
                staged
                    .modified_resources
                    .iter()
                    .map(|resource| resource.resource.clone()),
            );
            let mut resources = Vec::<ErfResourceInput>::new();
            let mut reused_resources = Vec::<String>::new();
            for area in &blueprint.areas {
                let area_resources = create_area_resources(
                    &area.resref,
                    &area.name,
                    &area.tileset,
                    u32::from(area.width),
                    u32::from(area.height),
                    0,
                )?;
                let existing = area_resources
                    .iter()
                    .filter(|resource| occupied.contains(&resource.key))
                    .count();
                if existing == area_resources.len() {
                    reused_resources.extend(
                        area_resources
                            .iter()
                            .map(|resource| resource.key.to_string()),
                    );
                } else if existing == 0 {
                    resources.extend(area_resources);
                } else {
                    return Err(agent_error(
                        "AGENT_BLUEPRINT_AREA_PARTIAL",
                        "La zone existe partiellement ; réparez ARE/GIT/GIC avant de poursuivre.",
                        area.resref.clone(),
                    ));
                }
            }
            for script in &blueprint.scripts {
                let source = script.source.as_deref().ok_or_else(|| {
                    agent_error(
                        "AGENT_BLUEPRINT_SCRIPT_SOURCE_REQUIRED",
                        "Chaque script du plan doit contenir sa source NSS avant application.",
                        script.resref.clone(),
                    )
                })?;
                parse_nss(
                    source.as_bytes(),
                    &format!("agent-blueprint::{}.nss", script.resref),
                )?;
                let key = ResourceKey::new(&script.resref, 2009);
                if occupied.contains(&key) {
                    return Err(agent_error(
                        "AGENT_BLUEPRINT_RESOURCE_EXISTS",
                        "Un script planifié existe déjà.",
                        key.to_string(),
                    ));
                }
                resources.push(ErfResourceInput {
                    key,
                    bytes: source.as_bytes().to_vec(),
                });
            }
            for dialogue in &blueprint.dialogues {
                let resource = create_dialogue_resource(
                    &dialogue.resref,
                    &dialogue.owner_tag,
                    &dialogue.purpose,
                    &dialogue.required_nodes,
                )?;
                if occupied.contains(&resource.key) {
                    return Err(agent_error(
                        "AGENT_BLUEPRINT_RESOURCE_EXISTS",
                        "Un dialogue planifié existe déjà.",
                        resource.key.to_string(),
                    ));
                }
                resources.push(resource);
            }
            let module_resource = ResourceKey::new("module", 2014);
            let source_ifo = resolved_resource_bytes(state, &run.job_id, &module_resource)?;
            let current_ifo = with_edit_workspace(state, &run.workspace_id, |workspace| {
                workspace.staged_resource_bytes(&module_resource)
            })?
            .unwrap_or_else(|| source_ifo.clone());
            let manifest = ModuleManifestDefinition {
                name: blueprint.name.clone(),
                tag: blueprint.tag.clone(),
                description: blueprint.synopsis.clone(),
                entry_area: blueprint.entry_area.clone(),
                areas: blueprint
                    .areas
                    .iter()
                    .map(|area| area.resref.clone())
                    .collect(),
                hak_files: blueprint.hak_dependencies.clone(),
                custom_tlk: blueprint.custom_tlk.clone(),
            };
            let (updated_ifo, _) =
                edit_module_manifest(&current_ifo, "agent-blueprint::module.ifo", &manifest)?;
            let workspace = with_edit_workspace(state, &run.workspace_id, |workspace| {
                if !resources.is_empty() {
                    workspace.create_resources_atomic(&resources)?;
                }
                let before_sha256 = hex::encode(Sha256::digest(&current_ifo));
                let after_sha256 = hex::encode(Sha256::digest(&updated_ifo));
                workspace.stage_resource(
                    module_resource.clone(),
                    Some(&source_ifo),
                    &updated_ifo,
                )?;
                workspace.apply(EditCommand::TransformResource {
                    resource: module_resource,
                    operation: format!("apply_module_blueprint:{}", plan.blueprint_sha256),
                    before_sha256,
                    after_sha256,
                })
            })?;
            Ok(serde_json::json!({
                "blueprintSha256": plan.blueprint_sha256,
                "createdResources": resources.iter().map(|resource| resource.key.to_string()).collect::<Vec<_>>(),
                "reusedResources": reused_resources,
                "scriptsRequireCompilation": blueprint.scripts.iter().map(|script| script.resref.clone()).collect::<Vec<_>>(),
                "workspace": workspace,
            }))
        }
        "module.build" => {
            let arguments: AgentModuleBuildArguments =
                decode_agent_arguments(call, "module.build")?;
            let output = PathBuf::from(&arguments.output_path);
            ensure_agent_output_path(&output, &run.policy.tool_runtime.allowed_output_roots)?;
            let report = with_edit_workspace(state, &run.workspace_id, |workspace| {
                workspace.validate_compiled_scripts()?;
                workspace.build_module(&output)
            })?;
            serde_json::to_value(report).map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le rapport de build n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        "module.create" => {
            let arguments: AgentModuleCreateArguments =
                decode_agent_arguments(call, "module.create")?;
            let output = PathBuf::from(&arguments.output_path);
            ensure_agent_output_path(&output, &run.policy.tool_runtime.allowed_output_roots)?;
            let report = create_empty_module(
                &output,
                &NewModuleDefinition {
                    name: arguments.name,
                    tag: arguments.tag,
                    entry_area: arguments.entry_area,
                    tileset: arguments.tileset,
                },
            )?;
            serde_json::to_value(report).map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le rapport de création n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        "module.dependencies" => {
            let arguments: AgentModuleDependenciesArguments =
                decode_agent_arguments(call, "module.dependencies")?;
            apply_agent_resource_transform(
                state,
                run,
                ResourceKey::new("module", 2014),
                "edit_module_dependencies".to_owned(),
                move |bytes, source| {
                    edit_module_dependencies(
                        bytes,
                        source,
                        &arguments.hak_files,
                        arguments.custom_tlk.as_deref(),
                    )
                    .map(|(output, _)| output)
                },
            )
        }
        "2da.edit" => {
            let arguments: AgentTwoDaEditArguments = decode_agent_arguments(call, "2da.edit")?;
            if arguments.resource.resource_type != 2017 {
                return Err(agent_error(
                    "AGENT_2DA_RESOURCE_INVALID",
                    "La ressource ciblée n’est pas une table 2DA.",
                    arguments.resource.to_string(),
                ));
            }
            let action = arguments.action;
            apply_agent_resource_transform(
                state,
                run,
                arguments.resource,
                serde_json::to_string(&action).unwrap_or_else(|_| "2da_edit".to_owned()),
                move |bytes, source| {
                    let mut table = parse_2da(bytes, source)?;
                    apply_2da_edit(&mut table, &action)?;
                    write_2da(&table)
                },
            )
        }
        "tlk.edit" => {
            let arguments: AgentTlkEditArguments = decode_agent_arguments(call, "tlk.edit")?;
            if arguments.resource.resource_type != 2018 {
                return Err(agent_error(
                    "AGENT_TLK_RESOURCE_INVALID",
                    "La ressource ciblée n’est pas une table TLK.",
                    arguments.resource.to_string(),
                ));
            }
            let action = arguments.action;
            apply_agent_resource_transform(
                state,
                run,
                arguments.resource,
                serde_json::to_string(&action).unwrap_or_else(|_| "tlk_edit".to_owned()),
                move |bytes, source| {
                    let mut table = parse_tlk(bytes, source)?;
                    apply_tlk_edit(&mut table, &action)?;
                    write_tlk(&table)
                },
            )
        }
        "walkmesh.edit" => {
            let arguments: AgentWalkmeshEditArguments =
                decode_agent_arguments(call, "walkmesh.edit")?;
            let resource = ResourceKey::new(&arguments.resref, arguments.kind.resource_type());
            let resref = arguments.resref;
            let kind = arguments.kind;
            let operation = arguments.operation;
            apply_agent_resource_transform(
                state,
                run,
                resource,
                serde_json::to_string(&operation).unwrap_or_else(|_| "walkmesh_edit".to_owned()),
                move |bytes, _source| {
                    let mut document = inspect_walkmesh(&resref, kind, bytes)?;
                    let validation = apply_walkmesh_operation(&mut document.draft, &operation)?;
                    if !validation.valid {
                        return Err(agent_error(
                            "AGENT_WALKMESH_INVALID",
                            "L’opération produit un walkmesh invalide.",
                            validation.diagnostics.join("; "),
                        ));
                    }
                    serialize_walkmesh_ascii(&resref, kind, &document.draft)
                },
            )
        }
        "development.deploy" => {
            let configured = run.policy.tool_runtime.development_path.trim();
            if configured.is_empty() {
                return Err(agent_error(
                    "AGENT_DEVELOPMENT_NOT_CONFIGURED",
                    "Configurez le dossier development dans Agent Studio.",
                    "toolRuntime.developmentPath is empty",
                ));
            }
            let configured = PathBuf::from(configured);
            let user_data = if configured
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("development"))
            {
                configured.parent().map(PathBuf::from).ok_or_else(|| {
                    agent_error(
                        "AGENT_DEVELOPMENT_PATH_INVALID",
                        "Le dossier development configuré n’a pas de parent valide.",
                        configured.display().to_string(),
                    )
                })?
            } else {
                configured
            };
            let deployment = with_edit_workspace(state, &run.workspace_id, |workspace| {
                workspace.validate_compiled_scripts()?;
                workspace.deploy_development(&user_data)
            })?;
            serde_json::to_value(deployment).map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le rapport de déploiement n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        "toolset.compare" => {
            let root = run.policy.tool_runtime.toolset_temp_path.trim();
            if root.is_empty() {
                return Err(agent_error(
                    "AGENT_TOOLSET_NOT_CONFIGURED",
                    "Configurez le dossier temporaire Toolset dans Agent Studio.",
                    "toolRuntime.toolsetTempPath is empty",
                ));
            }
            let plan = build_aurora_sync_plan(
                state,
                &run.job_id,
                &run.workspace_id,
                &PathBuf::from(root),
            )?;
            serde_json::to_value(plan).map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le plan de comparaison Toolset n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        "toolset.sync" => {
            let arguments: AgentToolsetSyncArguments =
                decode_agent_arguments(call, "toolset.sync")?;
            let root = run.policy.tool_runtime.toolset_temp_path.trim();
            if root.is_empty() {
                return Err(agent_error(
                    "AGENT_TOOLSET_NOT_CONFIGURED",
                    "Configurez le dossier temporaire Toolset dans Agent Studio.",
                    "toolRuntime.toolsetTempPath is empty",
                ));
            }
            let report = apply_aurora_workspace_sync_inner(
                state,
                AuroraSyncApplyRequest {
                    job_id: run.job_id.clone(),
                    workspace_id: run.workspace_id.clone(),
                    root: root.to_owned(),
                    actions: arguments.actions,
                },
            )?;
            serde_json::to_value(report).map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le rapport Toolset n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        "nwn.launch" => {
            let runtime = &run.policy.tool_runtime;
            if runtime.nwn_executable_path.trim().is_empty()
                || runtime.nwn_working_directory.trim().is_empty()
            {
                return Err(agent_error(
                    "AGENT_NWN_NOT_CONFIGURED",
                    "Configurez l’exécutable et le dossier de travail NWN dans Agent Studio.",
                    "NWN runtime paths are empty",
                ));
            }
            let mode = if Path::new(&runtime.nwn_executable_path)
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("nwserver.exe"))
            {
                NwnLaunchMode::Server
            } else {
                NwnLaunchMode::Client
            };
            let report = with_edit_workspace(state, &run.workspace_id, |workspace| {
                workspace.validate_compiled_scripts()?;
                workspace.launch_nwn_profile(&NwnLaunchProfile {
                    name: format!("Agent {}", &run.id[..8.min(run.id.len())]),
                    mode,
                    executable_path: runtime.nwn_executable_path.clone(),
                    working_directory: runtime.nwn_working_directory.clone(),
                    arguments: runtime.nwn_arguments.clone(),
                })
            })?;
            serde_json::to_value(report).map_err(|error| {
                agent_error(
                    "AGENT_TOOL_RESULT_INVALID",
                    "Le rapport de lancement n’a pas pu être préparé.",
                    error.to_string(),
                )
            })
        }
        _ => Err(agent_error(
            "AGENT_TOOL_NOT_IMPLEMENTED",
            "La capacité demandée n’est pas encore exécutable.",
            call.capability_id.clone(),
        )),
    }
}

fn apply_agent_change_set(
    state: &AppState,
    run: &AgentRun,
    change_set: AiChangeSet,
) -> AppResult<serde_json::Value> {
    let digest = ai_change_set_sha256(&change_set)?;
    let sources = ai_source_resources(state, &run.job_id, &change_set)?;
    let report = with_edit_workspace(state, &run.workspace_id, |workspace| {
        workspace.apply_controlled_ai_change_set(&change_set, &digest, &sources)
    })?;
    serde_json::to_value(report).map_err(|error| {
        agent_error(
            "AGENT_TOOL_RESULT_INVALID",
            "Le résultat d’édition n’a pas pu être préparé.",
            error.to_string(),
        )
    })
}

fn apply_agent_resource_transform(
    state: &AppState,
    run: &AgentRun,
    resource: ResourceKey,
    operation: String,
    transform: impl FnOnce(&[u8], &str) -> AppResult<Vec<u8>>,
) -> AppResult<serde_json::Value> {
    let source_bytes = try_resolved_resource_bytes(state, &run.job_id, &resource)?;
    let (current, source_for_parser) =
        with_edit_workspace(state, &run.workspace_id, |workspace| {
            workspace.staged_resource_bytes(&resource)
        })?
        .map(|bytes| (bytes, format!("workspace::{}", resource.file_name())))
        .or_else(|| {
            source_bytes
                .clone()
                .map(|bytes| (bytes, format!("source::{}", resource.file_name())))
        })
        .ok_or_else(|| {
            agent_error(
                "AGENT_RESOURCE_MISSING",
                "La ressource à transformer est introuvable.",
                resource.to_string(),
            )
        })?;
    let output = transform(&current, &source_for_parser)?;
    let before_sha256 = hex::encode(Sha256::digest(&current));
    let after_sha256 = hex::encode(Sha256::digest(&output));
    let workspace = with_edit_workspace(state, &run.workspace_id, |workspace| {
        workspace.stage_resource(resource.clone(), source_bytes.as_deref(), &output)?;
        workspace.apply(EditCommand::TransformResource {
            resource,
            operation,
            before_sha256,
            after_sha256,
        })
    })?;
    serde_json::to_value(workspace).map_err(|error| {
        agent_error(
            "AGENT_TOOL_RESULT_INVALID",
            "Le résultat de transformation n’a pas pu être préparé.",
            error.to_string(),
        )
    })
}

fn decode_agent_arguments<T: serde::de::DeserializeOwned>(
    call: &ToolCallRecord,
    capability: &str,
) -> AppResult<T> {
    serde_json::from_value(call.arguments.clone()).map_err(|error| {
        agent_error(
            "AGENT_TOOL_ARGUMENTS_INVALID",
            "Les paramètres proposés pour l’outil sont invalides.",
            format!("{capability}: {error}"),
        )
    })
}

fn ensure_agent_output_path(output: &Path, allowed_roots: &[String]) -> AppResult<()> {
    if allowed_roots.is_empty() || !output.is_absolute() {
        return Err(agent_error(
            "AGENT_OUTPUT_PATH_DENIED",
            "La sortie doit être absolue et appartenir à une racine autorisée.",
            output.display().to_string(),
        ));
    }
    let parent = output.parent().ok_or_else(|| {
        agent_error(
            "AGENT_OUTPUT_PATH_DENIED",
            "La sortie ne possède pas de dossier parent valide.",
            output.display().to_string(),
        )
    })?;
    let parent = parent.canonicalize().map_err(|error| {
        agent_error(
            "AGENT_OUTPUT_PARENT_INVALID",
            "Le dossier parent de sortie n’existe pas ou n’est pas accessible.",
            error.to_string(),
        )
    })?;
    let allowed = allowed_roots.iter().any(|root| {
        PathBuf::from(root)
            .canonicalize()
            .is_ok_and(|root| parent.starts_with(root))
    });
    if !allowed {
        return Err(agent_error(
            "AGENT_OUTPUT_PATH_DENIED",
            "La sortie se trouve hors des racines autorisées.",
            output.display().to_string(),
        ));
    }
    if output.exists() {
        let canonical_output = output.canonicalize().map_err(|error| {
            agent_error(
                "AGENT_OUTPUT_PATH_DENIED",
                "La sortie existante ne peut pas être résolue en sécurité.",
                error.to_string(),
            )
        })?;
        let existing_allowed = allowed_roots.iter().any(|root| {
            PathBuf::from(root)
                .canonicalize()
                .is_ok_and(|root| canonical_output.starts_with(root))
        });
        if !existing_allowed {
            return Err(agent_error(
                "AGENT_OUTPUT_PATH_DENIED",
                "La sortie existante redirige hors des racines autorisées.",
                canonical_output.display().to_string(),
            ));
        }
    }
    Ok(())
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
pub fn get_blueprint_field_options(
    state: State<'_, AppState>,
    request: BlueprintFieldOptionsRequest,
) -> AppResult<BlueprintFieldOptions> {
    state.jobs.with_analysis(&request.job_id, |analysis| {
        Ok(build_blueprint_field_options(analysis, &request.file_type))
    })
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
pub fn preview_map_generation(
    state: State<'_, AppState>,
    request: PreviewMapGenerationRequest,
) -> AppResult<MapGenerationPlan> {
    generate_verified_map_plan(&state, &request.job_id, &request.spec)
}

#[tauri::command]
pub fn get_map_authoring_context(
    state: State<'_, AppState>,
    request: GetMapAuthoringContextRequest,
) -> AppResult<MapAuthoringContext> {
    build_map_authoring_context(&state, &request.job_id, &request.tileset)
}

#[tauri::command]
pub async fn draft_map_with_ai(
    state: State<'_, AppState>,
    request: DraftMapWithAiRequest,
) -> AppResult<AiMapDraftResult> {
    validate_map_ai_provider(&request)?;
    let endpoint = validated_ai_endpoint(&request.provider.endpoint)?;
    let context =
        build_map_authoring_context(&state, &request.job_id, &request.current_spec.tileset)?;
    if context.selected_tileset.is_none() {
        return Err(map_generation_error(
            "EDIT_MAP_TILESET_NOT_RESOLVED",
            "Le SET du tileset choisi doit être résolu avant l’appel IA.",
            request.current_spec.tileset.clone(),
        ));
    }
    let blueprints = if request.include_blueprint_resrefs {
        map_blueprint_resrefs(&state, &request.job_id)?
    } else {
        BTreeMap::new()
    };
    let shared_blueprint_count = blueprints.values().map(Vec::len).sum();
    let mut shared_spec = request.current_spec.clone();
    if !request.include_blueprint_resrefs {
        for rule in &mut shared_spec.densities {
            rule.template_resrefs.clear();
        }
    }
    let task_context = serde_json::to_string(&serde_json::json!({
        "objective": "Proposer le contrat complet d'une carte NWN à partir du brief courant.",
        "currentSpec": shared_spec,
        "knownLimits": context.limits,
        "selectedTileset": context.selected_tileset,
        "availableBlueprintResrefs": blueprints,
        "privacy": "Aucun octet NWN, chemin local, script, dialogue, texture ou contenu GFF n'est transmis.",
    }))
    .map_err(|error| {
        map_generation_error(
            "EDIT_MAP_AI_CONTEXT_FAILED",
            "Le contexte minimal de la carte n’a pas pu être préparé.",
            error.to_string(),
        )
    })?;
    let registry = CapabilityRegistry::standard();
    let tool = registry.get("map.generate").cloned().ok_or_else(|| {
        map_generation_error(
            "EDIT_MAP_AI_TOOL_MISSING",
            "Le contrat de génération de carte est indisponible.",
            "map.generate is missing from the capability registry",
        )
    })?;
    let body = build_provider_request(
        &request.provider,
        ProviderRequestContext {
            system_prompt: map_ai_system_prompt(),
            task_context: &task_context,
            tools: &[tool],
            allow_parallel: false,
            max_output_tokens: 16_384,
            previous_response_id: None,
            tool_outputs: &[],
            replay_items: &[],
        },
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| {
            map_generation_error(
                "EDIT_MAP_AI_CLIENT_FAILED",
                "Le client IA de la carte n’a pas pu être initialisé.",
                error.to_string(),
            )
        })?;
    let mut builder = client.post(endpoint.clone()).json(&body);
    if let Some(api_key) = request.api_key.as_deref().filter(|value| !value.is_empty()) {
        builder = builder.bearer_auth(api_key);
    }
    let mut response = builder.send().await.map_err(|error| {
        map_generation_error(
            "EDIT_MAP_AI_UNREACHABLE",
            "Impossible de joindre le fournisseur IA de la carte.",
            error.without_url().to_string(),
        )
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(map_generation_error(
            "EDIT_MAP_AI_HTTP_ERROR",
            format!("Le fournisseur IA a refusé la demande avec HTTP {status}."),
            status.to_string(),
        ));
    }
    const MAX_MAP_AI_RESPONSE_BYTES: usize = 1024 * 1024;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        map_generation_error(
            "EDIT_MAP_AI_RESPONSE_READ_FAILED",
            "La réponse IA de la carte n’a pas pu être lue.",
            error.without_url().to_string(),
        )
    })? {
        if bytes.len().saturating_add(chunk.len()) > MAX_MAP_AI_RESPONSE_BYTES {
            return Err(map_generation_error(
                "EDIT_MAP_AI_RESPONSE_TOO_LARGE",
                "La réponse IA dépasse la limite de 1 Mio.",
                "map AI response exceeds 1 MiB",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let step = decode_provider_response(request.provider.kind, &bytes)?;
    if step.tool_calls.len() != 1 || step.tool_calls[0].capability_id != "map.generate" {
        return Err(map_generation_error(
            "EDIT_MAP_AI_TOOL_CALL_INVALID",
            "L’IA doit proposer exactement un contrat map.generate.",
            format!("received {} tool calls", step.tool_calls.len()),
        ));
    }
    let spec: MapGenerationSpec = serde_json::from_value(step.tool_calls[0].arguments.clone())
        .map_err(|error| {
            map_generation_error(
                "EDIT_MAP_AI_SPEC_INVALID",
                "Le contrat de carte proposé par l’IA est invalide.",
                error.to_string(),
            )
        })?;
    if spec.tileset != request.current_spec.tileset {
        return Err(map_generation_error(
            "EDIT_MAP_AI_TILESET_CHANGED",
            "L’IA a changé de tileset sans disposer de son SET vérifié.",
            format!(
                "expected tileset {}, received {}",
                request.current_spec.tileset, spec.tileset
            ),
        ));
    }
    validate_map_blueprints(&state, &request.job_id, &spec)?;
    let plan = generate_verified_map_plan(&state, &request.job_id, &spec)?;
    Ok(AiMapDraftResult {
        endpoint_origin: endpoint.origin().ascii_serialization(),
        model: request.provider.model,
        plan,
        shared_blueprint_count,
    })
}

#[tauri::command]
pub fn apply_map_generation(
    state: State<'_, AppState>,
    request: ApplyMapGenerationRequest,
) -> AppResult<ApplyMapGenerationResult> {
    apply_map_generation_in_workspace(
        &state,
        &request.job_id,
        &request.workspace_id,
        request.spec,
        Some(&request.expected_plan_sha256),
    )
}

fn apply_map_generation_in_workspace(
    state: &AppState,
    job_id: &str,
    workspace_id: &str,
    spec: MapGenerationSpec,
    expected_plan_sha256: Option<&str>,
) -> AppResult<ApplyMapGenerationResult> {
    let plan = generate_verified_map_plan(state, job_id, &spec)?;
    if expected_plan_sha256.is_some_and(|expected| expected != plan.plan_sha256) {
        return Err(Box::new(
            AppError::new(
                "EDIT_MAP_PLAN_CHANGED",
                "Le plan de carte a changé depuis sa prévisualisation.",
                format!(
                    "expected {:?}, regenerated {}",
                    expected_plan_sha256, plan.plan_sha256
                ),
                aurora_core::ErrorSeverity::Warning,
            )
            .with_import_stage("map_generation"),
        ));
    }
    let resources = create_generated_map_resources(&plan)?;
    let staged_templates = with_edit_workspace(state, workspace_id, |workspace| {
        Ok(workspace
            .snapshot()?
            .modified_resources
            .into_iter()
            .map(|resource| resource.resource)
            .collect::<BTreeSet<_>>())
    })?;
    state.jobs.with_analysis(job_id, |analysis| {
        for resource in &resources {
            if analysis.resource_catalog.get(&resource.key).is_some() {
                return Err(Box::new(
                    AppError::new(
                        "EDIT_AREA_ALREADY_EXISTS",
                        "Une ressource de cette carte existe déjà.",
                        resource.key.to_string(),
                        aurora_core::ErrorSeverity::Error,
                    )
                    .with_resource(resource.key.to_string())
                    .with_import_stage("map_generation"),
                ));
            }
        }
        for placement in &plan.placements {
            let resource_type =
                map_template_resource_type(&placement.category).ok_or_else(|| {
                    Box::new(
                        AppError::new(
                            "EDIT_MAP_CATEGORY_UNSUPPORTED",
                            "Une catégorie de placement n’est pas prise en charge.",
                            placement.category.clone(),
                            aurora_core::ErrorSeverity::Error,
                        )
                        .with_import_stage("map_generation"),
                    )
                })?;
            let key = ResourceKey::new(&placement.template_resref, resource_type);
            if analysis.resource_catalog.get(&key).is_none() && !staged_templates.contains(&key) {
                return Err(Box::new(
                    AppError::new(
                        "EDIT_MAP_BLUEPRINT_NOT_RESOLVED",
                        "Un blueprint demandé par la carte est introuvable.",
                        key.to_string(),
                        aurora_core::ErrorSeverity::Error,
                    )
                    .with_resource(key.to_string())
                    .with_import_stage("map_generation"),
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
        &plan.spec.resref,
        &are_document,
        Some(&git_document),
        Some(&gic_document),
    );
    let workspace = with_edit_workspace(state, workspace_id, |workspace| {
        workspace.create_resources_atomic(&resources)
    })?;
    Ok(ApplyMapGenerationResult {
        workspace,
        area,
        plan,
    })
}

fn map_template_resource_type(category: &str) -> Option<u16> {
    match category {
        "creature" => Some(2027),
        "door" => Some(2042),
        "encounter" => Some(2040),
        "item" => Some(2025),
        "placeable" => Some(2044),
        "sound" => Some(2035),
        "store" => Some(2051),
        "trigger" => Some(2032),
        "waypoint" => Some(2058),
        _ => None,
    }
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
    let local = is_local_ai_host(host);
    if endpoint.scheme() != "https" && !(endpoint.scheme() == "http" && local) {
        return Err(ai_error(
            "EDIT_AI_ENDPOINT_INSECURE",
            "Utilisez HTTPS, ou HTTP uniquement pour un modèle local.",
            format!("unsupported AI endpoint scheme for host {host}"),
        ));
    }
    Ok(endpoint)
}

fn is_local_ai_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.ends_with(".localhost")
}

fn validated_agent_endpoint(
    value: &str,
    context: &aurora_agent::ContextPolicy,
) -> AppResult<reqwest::Url> {
    let endpoint = validated_ai_endpoint(value)?;
    let host = endpoint.host_str().unwrap_or_default();
    if !is_local_ai_host(host)
        && !context
            .allowed_provider_hosts
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(host))
    {
        return Err(agent_error(
            "AGENT_PROVIDER_HOST_DENIED",
            "L’hôte du fournisseur n’est pas autorisé par le profil.",
            host.to_owned(),
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
    if resource.resource_type == 2017 {
        return serde_json::to_value(parse_2da(bytes, &format!("ai-context::{resource}"))?)
            .map_err(|error| {
                ai_error(
                    "EDIT_AI_CONTEXT_SERIALIZE_FAILED",
                    "La table 2DA sélectionnée n’a pas pu être préparée.",
                    error.to_string(),
                )
            });
    }
    if resource.resource_type == 2018 {
        return serde_json::to_value(parse_tlk(bytes, &format!("ai-context::{resource}"))?)
            .map_err(|error| {
                ai_error(
                    "EDIT_AI_CONTEXT_SERIALIZE_FAILED",
                    "La table TLK sélectionnée n’a pas pu être préparée.",
                    error.to_string(),
                )
            });
    }
    let walkmesh_kind = match resource.resource_type {
        2016 => Some(WalkmeshKind::Wok),
        2052 => Some(WalkmeshKind::Dwk),
        2053 => Some(WalkmeshKind::Pwk),
        _ => None,
    };
    if let Some(kind) = walkmesh_kind {
        return serde_json::to_value(inspect_walkmesh(&resource.resref, kind, bytes)?).map_err(
            |error| {
                ai_error(
                    "EDIT_AI_CONTEXT_SERIALIZE_FAILED",
                    "Le walkmesh sélectionné n’a pas pu être préparé.",
                    error.to_string(),
                )
            },
        );
    }
    Err(ai_error(
        "EDIT_AI_CONTEXT_RESOURCE_UNSUPPORTED",
        "Seuls les GFF, NSS, 2DA, TLK et walkmeshes peuvent être transmis au fournisseur IA.",
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

fn agent_error(
    code: impl Into<String>,
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
        .with_import_stage("agent_runtime"),
    )
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
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

fn restore_persisted_edit_workspace(
    state: &AppState,
    analysis: &aurora_project::ModuleAnalysis,
) -> AppResult<Option<WorkspaceSnapshot>> {
    let root = state.edit_workspace_root.join(
        analysis
            .fingerprint
            .sha256
            .chars()
            .take(16)
            .collect::<String>(),
    );
    if !root.join("workspace.json").is_file() {
        return Ok(None);
    }
    let workspace = EditWorkspace::open(&root)?;
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
                "La source a changé depuis la dernière session.",
                "The persisted edit workspace no longer matches the cached analysis.",
                aurora_core::ErrorSeverity::Error,
            )
            .with_source(snapshot.source.path.clone())
            .with_import_stage("restore_session"),
        ));
    }
    state
        .edit_workspaces
        .lock()
        .expect("edit workspace registry poisoned")
        .insert(workspace.id().to_owned(), workspace);
    Ok(Some(snapshot))
}

#[tauri::command]
pub fn restore_module_session(
    state: State<'_, AppState>,
    request: ModuleAnalysisRequest,
) -> AppResult<Option<RestoredModuleSession>> {
    let module_path = PathBuf::from(&request.module_path);
    if !module_path.is_file()
        || !module_path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("mod"))
    {
        return Ok(None);
    }
    let paths = SessionPaths::new(
        module_path.display().to_string(),
        request
            .game_install_path
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from),
        request
            .user_data_path
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from),
    );
    let Some(analysis) = restore_analysis_session(&state.analysis_session_root, &paths)? else {
        return Ok(None);
    };
    let workspace = restore_persisted_edit_workspace(&state, &analysis)?;
    let job = state
        .jobs
        .restore_completed_analysis(module_path.display().to_string(), analysis);
    Ok(Some(RestoredModuleSession { job, workspace }))
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
    let catalog_cache_path = state.asset_cache_root.join("resource-catalog-v1.json");
    let module_source_path = module_path.display().to_string();
    let session_paths = SessionPaths::new(
        module_source_path.clone(),
        roots.game_install_path.clone(),
        roots.user_data_path.clone(),
    );
    let analysis_session_root = state.analysis_session_root.clone();
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

        let result = analyze_module_file_with_cache(
            &module_path,
            &roots,
            Some(&catalog_cache_path),
            &cancellation,
            |progress| {
                if let Some(snapshot) = registry.set_progress(&job_id, progress) {
                    emit_snapshot(&app_handle, &snapshot);
                }
            },
        )
        .and_then(|mut analysis| {
            if let Some(snapshot) = registry.set_progress(
                &job_id,
                HashProgress::stage(AnalysisPhase::Persisting, 96.0),
            ) {
                emit_snapshot(&app_handle, &snapshot);
            }
            compare_dependency_reports(
                &mut analysis.dependency_report,
                previous_dependency_report.as_ref(),
            );
            let summary = serde_json::to_string(&analysis.structured_summary).map_err(|error| {
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
            if let Err(error) =
                store_analysis_session(&analysis_session_root, &session_paths, &analysis)
            {
                tracing::warn!(%error, "analysis completed but its resumable session cache was not saved");
            }
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
        let mut legacy_context = aurora_agent::ContextPolicy {
            allow_insecure_local_http: false,
            ..aurora_agent::ContextPolicy::default()
        };
        legacy_context.allowed_provider_hosts.clear();
        assert!(
            validated_agent_endpoint(
                "http://127.0.0.1:11434/v1/chat/completions",
                &legacy_context,
            )
            .is_ok(),
            "the explicit test or launch action is sufficient for local HTTP"
        );
        assert_eq!(
            validated_agent_endpoint("https://example.com/v1/chat/completions", &legacy_context)
                .expect_err("remote host outside the profile allowlist")
                .code,
            "AGENT_PROVIDER_HOST_DENIED"
        );
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

    #[test]
    fn every_registered_agent_capability_has_a_local_executor() {
        let registry = CapabilityRegistry::standard();
        let missing = registry
            .capabilities
            .iter()
            .filter(|capability| !agent_tool_is_implemented(&capability.id))
            .map(|capability| capability.id.as_str())
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "registered capabilities without executor: {missing:?}"
        );
    }

    #[test]
    fn provider_context_redacts_local_paths_unless_explicitly_allowed() {
        let value = serde_json::json!({
            "root": "C:\\Users\\karen\\workspace",
            "nested": {
                "outputPath": "E:\\build\\module.mod",
                "label": "safe"
            }
        });
        let redacted = sanitize_context_value(&value, false);
        assert_eq!(redacted["root"], "[local path redacted]");
        assert_eq!(redacted["nested"]["outputPath"], "[local path redacted]");
        assert_eq!(redacted["nested"]["label"], "safe");
        assert_eq!(sanitize_context_value(&value, true), value);
    }
}
