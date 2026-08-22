import { invoke } from "@tauri-apps/api/core";
import { requireTauri } from "./errors";
import type { WalkmeshDocument, WalkmeshDraft, WalkmeshEditResult, WalkmeshKind, WalkmeshOperation, WalkmeshValidation } from "./types";

export async function validateWalkmeshDraft(draft: WalkmeshDraft, kind: WalkmeshKind): Promise<WalkmeshValidation> {
  requireTauri();
  return invoke<WalkmeshValidation>("validate_walkmesh_draft", { request: { draft, kind } });
}

export async function transformWalkmeshDraft(draft: WalkmeshDraft, operation: WalkmeshOperation): Promise<{ draft: WalkmeshDraft; validation: WalkmeshValidation }> {
  requireTauri();
  return invoke<{ draft: WalkmeshDraft; validation: WalkmeshValidation }>("transform_walkmesh_draft", { request: { draft, operation } });
}

export async function inspectWorkspaceWalkmesh(request: {
  jobId: string; workspaceId: string; resref: string; kind: WalkmeshKind;
}): Promise<WalkmeshDocument> {
  requireTauri();
  return invoke<WalkmeshDocument>("inspect_workspace_walkmesh", { request });
}

export async function saveWorkspaceWalkmesh(request: {
  jobId: string; workspaceId: string; resref: string; kind: WalkmeshKind;
  draft: WalkmeshDraft; replaceExisting?: boolean;
}): Promise<WalkmeshEditResult> {
  requireTauri();
  return invoke<WalkmeshEditResult>("save_workspace_walkmesh", { request });
}
