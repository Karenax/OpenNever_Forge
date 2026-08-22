import { invoke } from "@tauri-apps/api/core";
import { requireTauri } from "./errors";
import type {
  AreaMigrationCandidate,
  AreaMigrationPreview,
  AssetExportCandidate,
  AssetExportPreview,
  AssetExportResult,
  DialogueExportPreview,
  DialogueExportResult,
  DialogueSearchHit,
  JobSnapshot,
} from "./types";

export async function listAreaMigrationCandidates(analysisJobId: string): Promise<AreaMigrationCandidate[]> {
  requireTauri();
  return invoke<AreaMigrationCandidate[]>("list_area_migration_candidates", { request: { analysisJobId } });
}

export async function previewAreaMigration(analysisJobId: string, areaResref: string): Promise<AreaMigrationPreview> {
  requireTauri();
  return invoke<AreaMigrationPreview>("preview_area_migration", { request: { analysisJobId, areaResref } });
}

export async function getAreaMigrationJob(
  analysisJobId: string,
  areaResref: string,
): Promise<JobSnapshot | null> {
  requireTauri();
  return invoke<JobSnapshot | null>("get_area_migration_job", {
    request: { analysisJobId, areaResref },
  });
}

export async function startAreaMigrationExport(request: {
  analysisJobId: string; areaResref: string; destination: string;
}): Promise<JobSnapshot> {
  requireTauri();
  return invoke<JobSnapshot>("start_area_migration_export", { request });
}

export async function listAssetExportCandidates(analysisJobId: string): Promise<AssetExportCandidate[]> {
  requireTauri();
  return invoke<AssetExportCandidate[]>("list_asset_export_candidates", { request: { analysisJobId } });
}

export async function previewAssetExport(analysisJobId: string, resref: string): Promise<AssetExportPreview> {
  requireTauri();
  return invoke<AssetExportPreview>("preview_asset_export", { request: { analysisJobId, resref } });
}

export async function exportAssetBundle(request: {
  analysisJobId: string; resref: string; destination: string; localOnlyAcknowledged: boolean;
}): Promise<AssetExportResult> {
  requireTauri();
  return invoke<AssetExportResult>("export_asset_bundle", { request });
}

export async function listDialogueExportCandidates(analysisJobId: string, workspaceId?: string | null): Promise<DialogueSearchHit[]> {
  requireTauri();
  return invoke<DialogueSearchHit[]>("list_dialogue_export_candidates", { request: { analysisJobId, workspaceId } });
}

export async function previewDialogueExport(request: {
  analysisJobId: string; workspaceId?: string | null; resref: string;
}): Promise<DialogueExportPreview> {
  requireTauri();
  return invoke<DialogueExportPreview>("preview_dialogue_export", { request });
}

export async function exportDialogueBundle(request: {
  analysisJobId: string; workspaceId?: string | null; resref: string;
  destination: string; expectedSourceResourceSha256: string; localOnlyAcknowledged: boolean;
}): Promise<DialogueExportResult> {
  requireTauri();
  return invoke<DialogueExportResult>("export_dialogue_bundle", { request });
}
