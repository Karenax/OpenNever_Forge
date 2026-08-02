import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type ErrorSeverity = "info" | "warning" | "error" | "fatal";

export type AppError = {
  code: string;
  userMessage: string;
  technicalMessage: string;
  source?: string;
  resource?: string;
  importStage?: string;
  cause?: string;
  severity: ErrorSeverity;
  suggestion?: string;
};

export type AppStatus = {
  appVersion: string;
  readOnly: boolean;
  databaseSchemaVersion: number;
};

export type HashProgress = {
  bytesRead: number;
  totalBytes: number;
  percent: number;
};

export type ModuleFingerprint = {
  sha256: string;
  sizeBytes: number;
};

export type ResourceKey = {
  resref: string;
  resourceType: number;
};

export type ContainerResource = {
  key: ResourceKey;
  resourceId: number;
  extension: string | null;
  offset: number;
  size: number;
};

export type ResourceTypeSummary = {
  resourceType: number;
  extension: string | null;
  count: number;
  totalSize: number;
};

export type ContainerInventory = {
  fileType: string;
  fileVersion: string;
  buildYear: number;
  buildDay: number;
  resourceCount: number;
  resources: ContainerResource[];
  typeSummaries: ResourceTypeSummary[];
};

export type ModuleAnalysis = {
  fingerprint: ModuleFingerprint;
  inventory: ContainerInventory;
  moduleInfo: ModuleInfo;
};

export type LocalizedValue = {
  languageId: number;
  text: string;
};

export type LocalizedString = {
  stringRef: number | null;
  values: LocalizedValue[];
};

export type ModuleInfo = {
  name: LocalizedString;
  description: LocalizedString;
  tag: string;
  minimumGameVersion: string;
  customTlk: string | null;
  entryArea: string;
  hakFiles: string[];
};

export type JobSnapshot = {
  id: string;
  kind: string;
  state: "queued" | "running" | "cancelling" | "cancelled" | "completed" | "failed";
  sourcePath: string;
  progress: HashProgress;
  result?: ModuleAnalysis | null;
  error?: AppError | null;
};

export async function getAppStatus(): Promise<AppStatus> {
  if (!isTauri()) {
    return { appVersion: "browser-preview", readOnly: true, databaseSchemaVersion: 1 };
  }
  return invoke<AppStatus>("get_app_status");
}

export async function startModuleAnalysis(path: string): Promise<JobSnapshot> {
  requireTauri();
  return invoke<JobSnapshot>("start_module_analysis", { path });
}

export async function getJob(id: string): Promise<JobSnapshot | null> {
  requireTauri();
  return invoke<JobSnapshot | null>("get_job", { id });
}

export async function cancelJob(id: string): Promise<JobSnapshot> {
  requireTauri();
  return invoke<JobSnapshot>("cancel_job", { id });
}

export async function selectModule(): Promise<string | null> {
  requireTauri();
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Neverwinter Nights module", extensions: ["mod"] }],
  });
  return typeof selected === "string" ? selected : null;
}

export async function selectDirectory(): Promise<string | null> {
  requireTauri();
  const selected = await open({ multiple: false, directory: true });
  return typeof selected === "string" ? selected : null;
}

export function normalizeAppError(error: unknown): AppError {
  if (isAppError(error)) return error;
  const technicalMessage = error instanceof Error ? error.message : String(error);
  return {
    code: "UNEXPECTED_FRONTEND_ERROR",
    userMessage: "Une erreur inattendue s'est produite.",
    technicalMessage,
    severity: "error",
    suggestion: "Consultez les diagnostics techniques puis réessayez.",
  };
}

function requireTauri() {
  if (!isTauri()) {
    throw {
      code: "TAURI_RUNTIME_REQUIRED",
      userMessage: "Cette action nécessite l'application desktop.",
      technicalMessage: "Tauri IPC is unavailable in browser preview mode.",
      severity: "warning",
    } satisfies AppError;
  }
}

function isAppError(value: unknown): value is AppError {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<AppError>;
  return (
    typeof candidate.code === "string" &&
    typeof candidate.userMessage === "string" &&
    typeof candidate.technicalMessage === "string" &&
    typeof candidate.severity === "string"
  );
}
