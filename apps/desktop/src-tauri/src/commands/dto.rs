use crate::jobs::JobSnapshot;
use aurora_2da::TwoDaEditAction;
use aurora_agent::{
    AgentPolicy, AgentRun, CapabilityRegistry, EffectiveCapability, ModuleBlueprint,
    ProviderProfile,
};
use aurora_core::ResourceKey;
use aurora_edit::{
    AiChangeSet, AiChangeSetPreview, AreaAudioPatch, AreaEnvironmentPatch, AreaStructureAction,
    AuroraSyncAction, BlueprintStructureAction, DevelopmentDeployment, DialogueStructureAction,
    FactionStructureAction, InstancePlacement, JournalStructureAction, MapGenerationPlan,
    MapGenerationSpec, ModuleBuildProfile, ModuleBuildReport, NwnLaunchProfile, TileState,
    Transform, WalkmeshDocument, WalkmeshDraft, WalkmeshKind, WalkmeshOperation,
    WalkmeshValidation, WorkspaceSnapshot,
};
use aurora_gff::GenericGff;
use aurora_nwscript::{CompileResult, NssDocument};
use aurora_project::{DialogueGraph, NarrativeModel, ResourceSourceKind};
use aurora_tlk::TlkEditAction;
use aurora_world::AreaMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
pub struct PrepareSceneModelsRequest {
    pub job_id: String,
    pub resrefs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareSceneModelsReport {
    pub requested: usize,
    pub prepared: usize,
    pub cache_hits: usize,
    pub failed: usize,
    pub duration_ms: u128,
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
pub struct AgentResourceSearchArguments {
    pub query: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSetFieldArguments {
    pub resource: ResourceKey,
    pub path: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReplaceScriptArguments {
    pub resource: ResourceKey,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAreaCreateArguments {
    pub resref: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub tileset: String,
    #[serde(default)]
    pub tile_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentScriptCreateArguments {
    pub resref: String,
    pub event: String,
    pub purpose: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentScriptCompileArguments {
    pub resref: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUndoBatchArguments {
    pub checkpoint_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModuleBuildArguments {
    pub output_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDialogueEditArguments {
    pub resref: String,
    pub action: DialogueStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentJournalEditArguments {
    pub resource: ResourceKey,
    pub action: JournalStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentFactionEditArguments {
    pub resource: ResourceKey,
    pub action: FactionStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBlueprintEditArguments {
    pub resource: ResourceKey,
    pub action: BlueprintStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolsetSyncArguments {
    pub actions: Vec<AuroraSyncAction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModuleDependenciesArguments {
    pub hak_files: Vec<String>,
    pub custom_tlk: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModuleCreateArguments {
    pub output_path: String,
    pub name: String,
    pub tag: String,
    pub entry_area: String,
    pub tileset: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTwoDaEditArguments {
    pub resource: ResourceKey,
    pub action: TwoDaEditAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTlkEditArguments {
    pub resource: ResourceKey,
    pub action: TlkEditAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAreaInstanceArguments {
    pub area: String,
    pub placement: InstancePlacement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapContextArguments {
    pub tileset: Option<String>,
    pub query: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapAreaArguments {
    pub area: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapPreviewArguments {
    pub spec: MapGenerationSpec,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapApplyArguments {
    pub spec: MapGenerationSpec,
    pub expected_plan_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapEnvironmentArguments {
    pub area: String,
    pub expected_sha256: String,
    pub patch: AreaEnvironmentPatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapAudioArguments {
    pub area: String,
    pub expected_sha256: String,
    pub patch: AreaAudioPatch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapTileArguments {
    pub area: String,
    pub x: u32,
    pub y: u32,
    pub expected_sha256: String,
    pub before: TileState,
    pub after: TileState,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapInstanceAddArguments {
    pub area: String,
    pub expected_sha256: String,
    pub placement: InstancePlacement,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapInstanceMoveArguments {
    pub area: String,
    pub instance_id: String,
    pub expected_sha256: String,
    pub before: Transform,
    pub after: Transform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapInstanceRemoveArguments {
    pub area: String,
    pub instance_id: String,
    pub expected_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMapStructureArguments {
    pub area: String,
    pub expected_sha256: String,
    pub action: AreaStructureAction,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWalkmeshEditArguments {
    pub resref: String,
    pub kind: WalkmeshKind,
    pub operation: WalkmeshOperation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentArchitectureQueryArguments {
    pub query: String,
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
