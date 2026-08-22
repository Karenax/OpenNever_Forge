import { invoke } from "@tauri-apps/api/core";
import { requireTauri } from "./errors";
import type {
  AreaMap,
  ApplyMapGenerationResult,
  AiMapDraftResult,
  BlueprintFieldOptions,
  InstancePlacement,
  MapAuthoringContext,
  MapGenerationPlan,
  MapGenerationSpec,
  ModuleBuildReport,
  PaletteManifest,
  ProviderProfile,
  WorkspaceSnapshot,
} from "./types";

export async function createNewModule(request: {
  outputPath: string; name: string; tag: string; entryArea: string; tileset: string;
}): Promise<ModuleBuildReport> {
  requireTauri();
  return invoke<ModuleBuildReport>("create_new_module", { request });
}

export async function getStandardPalette(): Promise<PaletteManifest> {
  requireTauri();
  return invoke<PaletteManifest>("get_standard_palette");
}

export async function getBlueprintFieldOptions(request: { jobId: string; fileType: string }): Promise<BlueprintFieldOptions> {
  requireTauri();
  return invoke<BlueprintFieldOptions>("get_blueprint_field_options", { request });
}

export async function createWorkspaceArea(request: {
  jobId: string; workspaceId: string; resref: string; name: string; tileset: string;
  width: number; height: number; tileId: number;
}): Promise<{ workspace: WorkspaceSnapshot; area: AreaMap }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; area: AreaMap }>("create_workspace_area", { request });
}

export async function getMapAuthoringContext(request: { jobId: string; tileset: string }): Promise<MapAuthoringContext> {
  requireTauri();
  return invoke<MapAuthoringContext>("get_map_authoring_context", { request });
}

export async function previewMapGeneration(request: { jobId: string; spec: MapGenerationSpec }): Promise<MapGenerationPlan> {
  requireTauri();
  return invoke<MapGenerationPlan>("preview_map_generation", { request });
}

export async function draftMapWithAi(request: {
  jobId: string; currentSpec: MapGenerationSpec; provider: ProviderProfile; apiKey?: string;
  includeBlueprintResrefs: boolean;
}): Promise<AiMapDraftResult> {
  requireTauri();
  return invoke<AiMapDraftResult>("draft_map_with_ai", { request });
}

export async function applyMapGeneration(request: {
  jobId: string; workspaceId: string; spec: MapGenerationSpec; expectedPlanSha256: string;
}): Promise<ApplyMapGenerationResult> {
  requireTauri();
  return invoke<ApplyMapGenerationResult>("apply_map_generation", { request });
}

export async function listWorkspaceCreatedAreas(request: { workspaceId: string }): Promise<AreaMap[]> {
  requireTauri();
  return invoke<AreaMap[]>("list_workspace_created_areas", { request });
}

export async function deleteWorkspaceArea(request: {
  jobId: string; workspaceId: string; resref: string;
}): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("delete_workspace_area", { request });
}

export async function addWorkspaceAreaInstance(request: {
  jobId: string; workspaceId: string; area: string; placement: InstancePlacement;
}): Promise<{ workspace: WorkspaceSnapshot; instanceId: string }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; instanceId: string }>("add_workspace_area_instance", { request });
}

export async function removeWorkspaceAreaInstance(request: {
  jobId: string; workspaceId: string; area: string; instanceId: string;
}): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("remove_workspace_area_instance", { request });
}
