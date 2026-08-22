import { invoke } from "@tauri-apps/api/core";
import { requireTauri } from "./errors";
import type {
  BlueprintStructureAction,
  DiagnosticReportBundle,
  DialogueGraph,
  DialoguePage,
  DialogueStructureAction,
  FactionStructureAction,
  GenericGff,
  GenericGffValue,
  JournalStructureAction,
  NarrativeInspection,
  NarrativeModel,
  PrepareSceneModelsReport,
  ResourceInspection,
  ResourceKey,
  ResourcePage,
  ResourceSourceKind,
  SceneManifest,
  ScriptDocument,
  ScriptPage,
  WorldIndex,
  WorkspaceSnapshot,
} from "./types";

export async function queryResources(request: {
  jobId: string; query?: string; resourceTypes?: number[]; source?: ResourceSourceKind | null;
  offset?: number; limit?: number;
}): Promise<ResourcePage> {
  requireTauri();
  return invoke<ResourcePage>("query_resources", { request: { query: "", offset: 0, limit: 100, ...request } });
}

export async function inspectResource(request: { jobId: string; resref: string; resourceType: number; workspaceId?: string }): Promise<ResourceInspection> {
  requireTauri();
  return invoke<ResourceInspection>("inspect_resource", { request });
}

export async function queryScripts(request: { jobId: string; query?: string; offset?: number; limit?: number }): Promise<ScriptPage> {
  requireTauri();
  return invoke<ScriptPage>("query_scripts", { request: { query: "", offset: 0, limit: 100, ...request } });
}

export async function inspectScript(request: { jobId: string; resref: string }): Promise<ScriptDocument> {
  requireTauri();
  return invoke<ScriptDocument>("inspect_script", { request });
}

export async function queryDialogues(request: { jobId: string; query?: string; offset?: number; limit?: number }): Promise<DialoguePage> {
  requireTauri();
  return invoke<DialoguePage>("query_dialogues", { request: { query: "", offset: 0, limit: 50, ...request } });
}

export async function inspectDialogue(request: { jobId: string; resref: string; workspaceId?: string | null }): Promise<DialogueGraph> {
  requireTauri();
  return invoke<DialogueGraph>("inspect_dialogue", { request });
}

export async function editDialogueField(request: {
  jobId: string; workspaceId: string; resref: string; path: string;
  before: GenericGffValue; after: GenericGffValue;
}): Promise<{ workspace: WorkspaceSnapshot; graph: DialogueGraph }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; graph: DialogueGraph }>("edit_dialogue_field", { request });
}

export async function editDialogueStructure(request: {
  jobId: string; workspaceId: string; resref: string; action: DialogueStructureAction;
}): Promise<{ workspace: WorkspaceSnapshot; graph: DialogueGraph }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; graph: DialogueGraph }>("edit_dialogue_structure_command", { request });
}

export async function inspectWorld(request: { jobId: string }): Promise<WorldIndex> {
  requireTauri();
  return invoke<WorldIndex>("inspect_world", { request });
}

export async function inspectNarrativeDocuments(request: { jobId: string; workspaceId?: string }): Promise<NarrativeInspection> {
  requireTauri();
  return invoke<NarrativeInspection>("inspect_narrative_documents", { request });
}

export async function editJournalStructure(request: { jobId: string; workspaceId: string; resource: ResourceKey; action: JournalStructureAction }): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke("edit_journal_structure_command", { request });
}

export async function editFactionStructure(request: { jobId: string; workspaceId: string; resource: ResourceKey; action: FactionStructureAction }): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke("edit_faction_structure_command", { request });
}

export async function editBlueprintStructure(request: { jobId: string; workspaceId: string; resource: ResourceKey; action: BlueprintStructureAction }): Promise<{ workspace: WorkspaceSnapshot; document: GenericGff }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; document: GenericGff }>("edit_blueprint_structure_command", { request });
}

export async function inspectNarrative(request: { jobId: string }): Promise<NarrativeModel> {
  requireTauri();
  return invoke<NarrativeModel>("inspect_narrative", { request });
}

export async function inspectScene(request: { jobId: string; resref: string }): Promise<SceneManifest> {
  requireTauri();
  return invoke<SceneManifest>("inspect_scene", { request });
}

export async function modelPreviewGlb(request: { jobId: string; resref: string }): Promise<ArrayBuffer> {
  requireTauri();
  return invoke<ArrayBuffer>("model_preview_glb", { request });
}

export async function prepareSceneModels(request: { jobId: string; resrefs: string[] }): Promise<PrepareSceneModelsReport> {
  requireTauri();
  return invoke<PrepareSceneModelsReport>("prepare_scene_models", { request });
}

export async function resolveTexture(request: { jobId: string; resref: string }): Promise<ResourceKey | null> {
  requireTauri();
  return invoke<ResourceKey | null>("resolve_texture", { request });
}

export async function assetPreviewBytes(request: { jobId: string; resref: string; resourceType: number }): Promise<ArrayBuffer> {
  requireTauri();
  return invoke<ArrayBuffer>("asset_preview_bytes", { request });
}

export async function diagnosticReport(request: { jobId: string }): Promise<DiagnosticReportBundle> {
  requireTauri();
  return invoke<DiagnosticReportBundle>("diagnostic_report", { request });
}
