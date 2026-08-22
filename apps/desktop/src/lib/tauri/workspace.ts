import { invoke } from "@tauri-apps/api/core";
import { requireTauri } from "./errors";
import type {
  AreaMap,
  AreaStructureAction,
  AuroraSyncAction,
  AuroraSyncManifest,
  AuroraSyncPlan,
  AuroraSyncReport,
  BuildProfileRunReport,
  CompileResult,
  DevelopmentCleanupReport,
  DevelopmentDeployment,
  EditTransform,
  GenericGff,
  GenericGffValue,
  GitWorkspaceStatus,
  ModuleBuildProfile,
  ModuleBuildReport,
  NssDocument,
  NwnLaunchProfile,
  NwnLaunchReport,
  ReproducibleBuildVerification,
  ResourceKey,
  TalkTable,
  TileState,
  TlkEditAction,
  TwoDaEditAction,
  TwoDaTable,
  WorkspaceExportManifest,
  WorkspaceSnapshot,
} from "./types";

export async function createEditWorkspace(request: { jobId: string }): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("create_edit_workspace", { request });
}

export async function getEditWorkspace(request: { workspaceId: string }): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("get_edit_workspace", { request });
}

export async function undoEditCommand(request: { workspaceId: string }): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("undo_edit_command", { request });
}

export async function redoEditCommand(request: { workspaceId: string }): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("redo_edit_command", { request });
}

export async function applyGffEdit(request: {
  jobId: string; workspaceId: string; resource: ResourceKey;
  path: string; before: GenericGffValue; after: GenericGffValue;
}): Promise<{ workspace: WorkspaceSnapshot; document: GenericGff }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; document: GenericGff }>("apply_gff_edit", { request });
}

export async function editScriptSource(request: {
  jobId: string; workspaceId: string; resref: string; before: string; after: string;
}): Promise<{ workspace: WorkspaceSnapshot; document: NssDocument }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; document: NssDocument }>("edit_script_source", { request });
}

export async function compileWorkspaceScript(request: {
  jobId: string; workspaceId: string; resref: string; compilerPath: string;
  gameInstallPath: string; includePaths?: string[];
}): Promise<{ workspace: WorkspaceSnapshot; compilation: CompileResult }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; compilation: CompileResult }>("compile_workspace_script", {
    request: { includePaths: [], ...request },
  });
}

export async function moveAreaInstance(request: {
  jobId: string; workspaceId: string; area: string; instanceId: string;
  before: EditTransform; after: EditTransform;
}): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("move_area_instance", { request });
}

export async function setAreaTile(request: {
  jobId: string; workspaceId: string; area: string; x: number; y: number;
  before: TileState; after: TileState;
}): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("set_area_tile", { request });
}

export async function editAreaStructure(request: {
  jobId: string; workspaceId: string; area: string; action: AreaStructureAction;
}): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("edit_area_structure_command", { request });
}

export async function inspectWorkspaceArea(request: {
  jobId: string; workspaceId: string; area: string;
}): Promise<AreaMap> {
  requireTauri();
  return invoke<AreaMap>("inspect_workspace_area", { request });
}

export async function buildWorkspaceModule(request: {
  workspaceId: string; outputPath: string;
}): Promise<ModuleBuildReport> {
  requireTauri();
  return invoke<ModuleBuildReport>("build_workspace_module", { request });
}

export async function deployWorkspaceDevelopment(request: {
  workspaceId: string; userDataPath: string;
}): Promise<DevelopmentDeployment> {
  requireTauri();
  return invoke<DevelopmentDeployment>("deploy_workspace_development", { request });
}

export async function cleanWorkspaceDevelopment(request: {
  workspaceId: string; userDataPath: string;
}): Promise<DevelopmentCleanupReport> {
  requireTauri();
  return invoke<DevelopmentCleanupReport>("clean_workspace_development", { request });
}

export async function buildWorkspaceHak(request: { workspaceId: string; outputPath: string }): Promise<ModuleBuildReport> {
  requireTauri();
  return invoke<ModuleBuildReport>("build_workspace_hak", { request });
}

export async function exportWorkspaceSources(request: { workspaceId: string; outputPath: string }): Promise<WorkspaceExportManifest> {
  requireTauri();
  return invoke<WorkspaceExportManifest>("export_workspace_sources", { request });
}

export async function editWorkspaceTwoDa(request: {
  jobId: string; workspaceId: string; resource: ResourceKey; action: TwoDaEditAction;
}): Promise<{ workspace: WorkspaceSnapshot; document: TwoDaTable }> {
  requireTauri();
  return invoke("edit_workspace_2da", { request });
}

export async function editWorkspaceTlk(request: {
  jobId: string; workspaceId: string; resource: ResourceKey; action: TlkEditAction;
}): Promise<{ workspace: WorkspaceSnapshot; document: TalkTable }> {
  requireTauri();
  return invoke("edit_workspace_tlk", { request });
}

export async function editWorkspaceModuleDependencies(request: {
  jobId: string; workspaceId: string; hakFiles: string[]; customTlk: string | null;
}): Promise<{ workspace: WorkspaceSnapshot; document: GenericGff }> {
  requireTauri();
  return invoke("edit_workspace_module_dependencies", { request });
}

export async function listWorkspaceBuildProfiles(request: { workspaceId: string }): Promise<ModuleBuildProfile[]> {
  requireTauri();
  return invoke("list_workspace_build_profiles", { request });
}

export async function saveWorkspaceBuildProfile(request: { workspaceId: string; profile: ModuleBuildProfile }): Promise<ModuleBuildProfile[]> {
  requireTauri();
  return invoke("save_workspace_build_profile", { request });
}

export async function verifyWorkspaceReproducibleBuild(request: { workspaceId: string; profile: ModuleBuildProfile }): Promise<ReproducibleBuildVerification> {
  requireTauri();
  return invoke("verify_workspace_reproducible_build", { request });
}

export async function runWorkspaceBuildProfile(request: {
  workspaceId: string; profile: ModuleBuildProfile; outputDirectory: string; userDataPath: string | null;
}): Promise<BuildProfileRunReport> {
  requireTauri();
  return invoke("run_workspace_build_profile", { request });
}

export async function inspectGitWorkspace(request: { root: string }): Promise<GitWorkspaceStatus> {
  requireTauri();
  return invoke("inspect_git_workspace", { request });
}

export async function listWorkspaceLaunchProfiles(request: { workspaceId: string }): Promise<NwnLaunchProfile[]> {
  requireTauri();
  return invoke("list_workspace_launch_profiles", { request });
}

export async function saveWorkspaceLaunchProfile(request: { workspaceId: string; profile: NwnLaunchProfile }): Promise<NwnLaunchProfile[]> {
  requireTauri();
  return invoke("save_workspace_launch_profile", { request });
}

export async function launchWorkspaceTestProfile(request: { workspaceId: string; profile: NwnLaunchProfile }): Promise<NwnLaunchReport> {
  requireTauri();
  return invoke("launch_workspace_test_profile", { request });
}

export async function inspectAuroraWorkspace(request: { root: string }): Promise<AuroraSyncManifest> {
  requireTauri();
  return invoke<AuroraSyncManifest>("inspect_aurora_workspace", { request });
}

export async function planAuroraWorkspaceSync(request: { jobId: string; workspaceId: string; root: string }): Promise<AuroraSyncPlan> {
  requireTauri();
  return invoke<AuroraSyncPlan>("plan_aurora_workspace_sync", { request });
}

export async function applyAuroraWorkspaceSync(request: {
  jobId: string; workspaceId: string; root: string; actions: AuroraSyncAction[];
}): Promise<AuroraSyncReport> {
  requireTauri();
  return invoke<AuroraSyncReport>("apply_aurora_workspace_sync", { request });
}
