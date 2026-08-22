import { invoke, isTauri } from "@tauri-apps/api/core";
import { requireTauri } from "./errors";
import type { AppStatus, JobSnapshot, ModuleAnalysisRequest, RestoredModuleSession } from "./types";

export async function getAppStatus(): Promise<AppStatus> {
  if (!isTauri()) {
    return { appVersion: "browser-preview", readOnly: true, editingAvailable: true, databaseSchemaVersion: 1 };
  }
  return invoke<AppStatus>("get_app_status");
}

export async function restoreModuleSession(
  request: ModuleAnalysisRequest,
): Promise<RestoredModuleSession | null> {
  if (!isTauri()) return null;
  return invoke<RestoredModuleSession | null>("restore_module_session", { request });
}

export async function startModuleAnalysis(request: ModuleAnalysisRequest): Promise<JobSnapshot> {
  requireTauri();
  return invoke<JobSnapshot>("start_module_analysis", { request });
}

export async function getJob(id: string): Promise<JobSnapshot | null> {
  requireTauri();
  return invoke<JobSnapshot | null>("get_job", { id });
}

export async function cancelJob(id: string): Promise<JobSnapshot> {
  requireTauri();
  return invoke<JobSnapshot>("cancel_job", { id });
}
