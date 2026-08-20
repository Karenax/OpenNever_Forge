export type ProjectPreferences = {
  modulePath: string;
  gameInstallPath: string;
  userDataPath: string;
};

type StoredProjectPreferences = {
  version: 1;
  project: ProjectPreferences;
};

export const PROJECT_PREFERENCES_STORAGE_KEY = "opennever-forge.project-preferences";
export const LAST_EXPLORER_ITEM_STORAGE_KEY = "opennever-forge.last-explorer-item";

export const EMPTY_PROJECT_PREFERENCES: ProjectPreferences = {
  modulePath: "",
  gameInstallPath: "",
  userDataPath: "",
};

function browserStorage(): Storage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}

function isProjectPreferences(value: unknown): value is ProjectPreferences {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<ProjectPreferences>;
  return (
    typeof candidate.modulePath === "string" &&
    typeof candidate.gameInstallPath === "string" &&
    typeof candidate.userDataPath === "string"
  );
}

export function loadProjectPreferences(storage = browserStorage()): ProjectPreferences {
  if (!storage) return { ...EMPTY_PROJECT_PREFERENCES };

  try {
    const raw = storage.getItem(PROJECT_PREFERENCES_STORAGE_KEY);
    if (!raw) return { ...EMPTY_PROJECT_PREFERENCES };
    const stored = JSON.parse(raw) as Partial<StoredProjectPreferences>;
    return stored.version === 1 && isProjectPreferences(stored.project)
      ? { ...stored.project }
      : { ...EMPTY_PROJECT_PREFERENCES };
  } catch {
    return { ...EMPTY_PROJECT_PREFERENCES };
  }
}

export function saveProjectPreferences(
  project: ProjectPreferences,
  storage = browserStorage(),
): void {
  if (!storage) return;

  try {
    const stored: StoredProjectPreferences = { version: 1, project };
    storage.setItem(PROJECT_PREFERENCES_STORAGE_KEY, JSON.stringify(stored));
  } catch {
    // L'application reste utilisable si le stockage WebView est indisponible.
  }
}

export function loadLastExplorerItem(storage = browserStorage()): string {
  if (!storage) return "module";
  try {
    const value = storage.getItem(LAST_EXPLORER_ITEM_STORAGE_KEY)?.trim();
    return value && value.length <= 64 ? value : "module";
  } catch {
    return "module";
  }
}

export function saveLastExplorerItem(
  explorerItem: string,
  storage = browserStorage(),
): void {
  if (!storage || !explorerItem || explorerItem.length > 64) return;
  try {
    storage.setItem(LAST_EXPLORER_ITEM_STORAGE_KEY, explorerItem);
  } catch {
    // La navigation reste utilisable si le stockage WebView est indisponible.
  }
}
