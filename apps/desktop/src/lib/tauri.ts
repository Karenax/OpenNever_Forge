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
  dependencyReport: ModuleDependencyReport;
  resourceCatalogSummary: ResourceCatalogSummary;
  structuredSummary: StructuredResourceSummary;
  scriptIndexSummary: ScriptIndexSummary;
  dialogueIndexSummary: DialogueIndexSummary;
  worldSummary?: WorldSummary;
};

export type Confidence = "certain" | "probable" | "possible";
export type Evidence = { resource: string; fieldPath: string };
export type LocalizedText = { stringRef: number | null; text: string | null };
export type WorldDiagnostic = {
  code: string; severity: "info" | "warning" | "error"; message: string;
  resource: string; evidence: Evidence | null;
};
export type JournalEntry = { id: number; text: LocalizedText; finalState: boolean; delay: number };
export type JournalCategory = {
  tag: string; name: LocalizedText; priority: number; xp: number;
  entries: JournalEntry[]; source: string;
};
export type Faction = { id: number; name: string; parentId: number | null; global: boolean };
export type FactionReputation = { sourceId: number; targetId: number; value: number };
export type NarrativeModel = {
  categories: JournalCategory[]; factions: Faction[]; reputations: FactionReputation[];
  relations: Array<{ source: string; target: string; kind: string; confidence: Confidence; evidence: Evidence }>;
  diagnostics: WorldDiagnostic[];
};
export type AreaTile = { x: number; y: number; tileId: number; orientation: number };
export type AreaInstance = {
  id: string; category: string; tag: string | null; templateResref: string | null;
  x: number; y: number; z: number; bearing: number | null;
  appearance: number | null;
  transitionDestination: string | null; sourcePath: string;
};
export type AreaMap = {
  resref: string; name: LocalizedText; width: number; height: number; tileset: string | null;
  tiles: AreaTile[]; instances: AreaInstance[]; diagnostics: WorldDiagnostic[];
  areSource: string; gitSource: string | null; gicSource: string | null;
};
export type AssetRecord = {
  key: ResourceKey; source: string; format: string; support: "preview" | "metadata" | "unsupported";
  width: number | null; height: number | null; modelNodes: string[]; animations: string[];
  textures: string[]; referencedModels: string[]; supermodel: string | null; meshCount: number; triangleCount: number;
  skinCount: number; walkmeshCount: number; glbPreview: boolean;
  sha256: string; diagnostics: WorldDiagnostic[];
};
export type SceneObject = {
  id: string; kind: string; label: string; x: number; y: number; z: number;
  rotation: number; marker: boolean; modelResref: string | null; modelResrefs: string[];
  walkmeshAvailable: boolean; sourcePath: string;
};
export type SceneManifest = {
  area: string; width: number; height: number; tileset: string | null;
  objects: SceneObject[]; overlays: SceneObject[]; resolvedAssets: number;
  uniqueModels: number; walkmeshAssets: number; missingAssets: number;
  memoryBudgetBytes: number; diagnostics: WorldDiagnostic[];
};
export type GraphNode = { id: string; kind: string; label: string; resource: string | null };
export type GraphEdge = {
  id: string; source: string; target: string; kind: string;
  confidence: Confidence; evidence: Evidence;
};
export type WorldSummary = {
  journalCategories: number; journalEntries: number; factions: number; factionRelations: number;
  areas: number; tiles: number; instances: number; transitions: number; assets: number;
  previewableAssets: number; sceneObjects: number; graphNodes: number; graphEdges: number;
  diagnostics: number;
};
export type WorldIndex = {
  narrative: NarrativeModel; areas: AreaMap[]; assets: { assets: AssetRecord[] };
  scenes: SceneManifest[]; graphNodes: GraphNode[]; graphEdges: GraphEdge[];
  diagnostics: WorldDiagnostic[]; summary: WorldSummary;
};
export type DiagnosticReport = {
  schemaVersion: number; moduleSha256: string; summary: WorldSummary;
  nodes: GraphNode[]; edges: GraphEdge[]; diagnostics: WorldDiagnostic[];
};

export type DialogueIndexSummary = { dialogues: number; nodes: number; links: number; sharedNodes: number; cycles: number; unreachableNodes: number; brokenLinks: number; scriptLinks: number; references: number; diagnostics: number };
export type DialogueNode = { id: string; kind: "entry" | "reply"; index: number; text: LocalizedString | null; displayText: string | null; speaker: string | null; comment: string | null; animation: number | null; animationLoop: boolean | null; sound: string | null; quest: string | null; actionScript: string | null };
export type DialogueLink = { id: string; source: string | null; target: string; conditionScript: string | null; actionScript: string | null; comment: string | null; isChild: boolean; broken: boolean };
export type DialogueTreeNode = { nodeId: string; kind: "entry" | "reply"; displayText: string | null; repeated: boolean; cycle: boolean; children: DialogueTreeNode[] };
export type DialogueGraph = { key: ResourceKey; source: string; nodes: DialogueNode[]; links: DialogueLink[]; roots: string[]; sharedNodes: string[]; unreachableNodes: string[]; cycles: string[][]; diagnostics: Array<{ code: string; message: string; nodeId: string | null; linkId: string | null }>; references: Array<{ resource: ResourceKey; fieldPath: string; source: string }>; tree: DialogueTreeNode[]; raw: unknown };
export type DialogueSearchHit = { resref: string; nodeCount: number; linkCount: number; cycleCount: number; diagnosticCount: number; preview: string | null };
export type DialoguePage = { items: DialogueSearchHit[]; offset: number; limit: number; total: number };

export type ScriptIndexSummary = {
  scripts: number; nss: number; ncs: number; paired: number; missingSource: number;
  includes: number; symbols: number; calls: number; inboundReferences: number; diagnostics: number;
};
export type ScriptDiagnostic = { code: string; message: string; line: number | null; resource: string };
export type NssDocument = {
  source: string; text: string; lineCount: number;
  includes: Array<{ resref: string; line: number; resolved: boolean }>;
  symbols: Array<{ name: string; kind: "function" | "constant"; line: number; declaration: string }>;
  calls: Array<{ name: string; line: number }>;
  diagnostics: ScriptDiagnostic[];
};
export type NcsDocument = {
  source: string; size: number; sha256: string; header: string; bytecodeSize: number;
  hexPreview: string; validHeader: boolean;
};
export type ScriptDocument = {
  resref: string; nss: NssDocument | null; ncs: NcsDocument | null;
  inboundReferences: Array<{ script: string; resource: ResourceKey; fieldPath: string; source: string }>;
  diagnostics: ScriptDiagnostic[];
};
export type ScriptSearchHit = {
  resref: string; hasNss: boolean; hasNcs: boolean; symbolCount: number;
  inboundReferenceCount: number; diagnosticCount: number;
  matches: Array<{ line: number; excerpt: string }>;
};
export type ScriptPage = { items: ScriptSearchHit[]; offset: number; limit: number; total: number };

export type ResourceSourceKind = "development" | "override" | "module" | "hak" | "patch" | "key_bif";
export type ResourceLocation =
  | { kind: "file"; path: string }
  | { kind: "erf"; path: string; offset: number; size: number }
  | { kind: "bif"; path: string; offset: number; size: number };
export type ResourceVersion = {
  key: ResourceKey; sourceKind: ResourceSourceKind; sourceName: string; sourcePath: string;
  priority: number; offset: number; size: number; sha256: string | null; location: ResourceLocation;
};
export type ResolvedResource = { key: ResourceKey; selected: ResourceVersion; shadowed: ResourceVersion[] };
export type ResourceCatalog = {
  entries: ResolvedResource[]; versionCount: number; shadowedCount: number;
  diagnostics: Array<{ code: string; message: string; source: string }>;
};
export type ResourceCatalogSummary = {
  resourceCount: number; versionCount: number; shadowedCount: number; diagnosticCount: number;
  typeCounts: Array<{ resourceType: number; count: number }>;
  sourceCounts: Array<{ source: ResourceSourceKind; count: number }>;
};
export type ResourcePage = { items: ResolvedResource[]; offset: number; limit: number; total: number };

export type StructuredResourceSummary = {
  gff: { discovered: number; parsed: number; failed: number; structCount: number; fieldCount: number };
  twoDaTables: Array<{ key: ResourceKey; source: string; columns: number; rows: number; shadowedVersions: number }>;
  talkTables: Array<{ kind: string; source: string; languageId: number; entries: number }>;
  resolvedModuleName: { text: string | null; origin: string; state: string; stringRef: number | null; tableIndex: number | null; source: string | null } | null;
  areas: Array<{ resref: string; name: LocalizedString | null; tag: string | null; width: number | null; height: number | null; tileset: string | null; source: string }>;
  blueprints: Array<{ key: ResourceKey; tag: string | null; name: LocalizedString | null; source: string }>;
  diagnostics: Array<{ code: string; resource: string; source: string; message: string }>;
};

export type ResourceInspection =
  | { kind: "gff"; value: unknown }
  | { kind: "two_da"; value: unknown }
  | { kind: "tlk"; value: unknown }
  | { kind: "binary"; value: { size: number; sha256: string; hexPreview: string; truncated: boolean } };

export type ModuleDependencyKind = "hak" | "custom_tlk";
export type ModuleDependencyState = "resolved" | "missing" | "unchecked" | "invalid";
export type ModuleDependencyChange =
  | "first_seen"
  | "unchanged"
  | "content_changed"
  | "location_changed"
  | "became_available"
  | "became_missing";

export type ModuleDependency = {
  kind: ModuleDependencyKind;
  logicalName: string;
  state: ModuleDependencyState;
  selectedPath: string | null;
  shadowedPaths: string[];
  searchedPaths: string[];
  fingerprint: ModuleFingerprint | null;
  change: ModuleDependencyChange;
};

export type ModuleDependencyReport = {
  dependencies: ModuleDependency[];
  resolvedCount: number;
  missingCount: number;
  uncheckedCount: number;
  invalidCount: number;
  changedCount: number;
};

export type ModuleAnalysisRequest = {
  modulePath: string;
  gameInstallPath?: string | null;
  userDataPath?: string | null;
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
  minimumGameVersion: string | null;
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

export async function queryResources(request: {
  jobId: string; query?: string; resourceTypes?: number[]; source?: ResourceSourceKind | null;
  offset?: number; limit?: number;
}): Promise<ResourcePage> {
  requireTauri();
  return invoke<ResourcePage>("query_resources", { request: { query: "", offset: 0, limit: 100, ...request } });
}

export async function inspectResource(request: { jobId: string; resref: string; resourceType: number }): Promise<ResourceInspection> {
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

export async function inspectDialogue(request: { jobId: string; resref: string }): Promise<DialogueGraph> {
  requireTauri();
  return invoke<DialogueGraph>("inspect_dialogue", { request });
}

export async function inspectWorld(request: { jobId: string }): Promise<WorldIndex> {
  requireTauri();
  return invoke<WorldIndex>("inspect_world", { request });
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

export async function resolveTexture(request: { jobId: string; resref: string }): Promise<ResourceKey | null> {
  requireTauri();
  return invoke<ResourceKey | null>("resolve_texture", { request });
}

export async function assetPreviewBytes(request: { jobId: string; resref: string; resourceType: number }): Promise<ArrayBuffer> {
  requireTauri();
  return invoke<ArrayBuffer>("asset_preview_bytes", { request });
}

export type DiagnosticReportBundle = { report: DiagnosticReport; json: string; html: string };

export async function diagnosticReport(request: { jobId: string }): Promise<DiagnosticReportBundle> {
  requireTauri();
  return invoke<DiagnosticReportBundle>("diagnostic_report", { request });
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
