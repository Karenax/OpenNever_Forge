import { isTauri } from "@tauri-apps/api/core";

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

export function requireTauri() {
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
