import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";

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
  editingAvailable: boolean;
  databaseSchemaVersion: number;
};

export type EditCommand =
  | { kind: "set_field"; resource: ResourceKey; path: string; before: unknown; after: unknown }
  | { kind: "transform_resource"; resource: ResourceKey; operation: string; beforeSha256: string; afterSha256: string }
  | { kind: "replace_text"; resource: ResourceKey; before: string; after: string }
  | { kind: "compile_script"; resource: ResourceKey; inputs: ResourceContentDigest[]; compilerSha256: string; beforeSha256: string | null; afterSha256: string }
  | { kind: "move_instance"; area: string; instanceId: string; before: EditTransform; after: EditTransform }
  | { kind: "set_tile"; area: string; x: number; y: number; before: TileState; after: TileState }
  | { kind: "add_instance"; area: string; instanceId: string; placement: InstancePlacement }
  | { kind: "remove_instance"; area: string; instanceId: string }
  | { kind: "create_resource"; resource: ResourceKey; contentSha256: string }
  | { kind: "delete_resource"; resource: ResourceKey; contentSha256: string }
  | { kind: "create_resource_set"; resources: ResourceContentDigest[] }
  | { kind: "delete_resource_set"; resources: ResourceContentDigest[] };

export type ResourceContentDigest = { resource: ResourceKey; contentSha256: string };

export type EditTransform = { x: number; y: number; z: number; bearing: number };
export type TileState = { tileId: number; orientation: number; height: number };
export type InstancePlacement = {
  category: string; templateResref: string; tag: string;
  x: number; y: number; z: number; bearing: number; linkedTo: string | null;
};
export type ModifiedResource = {
  resource: ResourceKey; sourceSha256: string | null; outputSha256: string;
  sizeBytes: number; relativePath: string;
};
export type WorkspaceSnapshot = {
  schemaVersion: number; workspaceId: string; root: string;
  source: { path: string; sha256: string; sizeBytes: number };
  sourceIntact: boolean; commandCount: number; cursor: number;
  canUndo: boolean; canRedo: boolean; modifiedResources: ModifiedResource[];
  deletedResources: ResourceKey[]; journalEvents: number; values: Record<string, unknown>;
};
export type GenericGffValue = { kind: string; value: unknown };
export type GenericGffField = { label: string; fieldType: number; value: GenericGffValue };
export type GenericGffStruct = { index: number; structType: number; fields: GenericGffField[] };
export type GenericGff = {
  fileType: string; fileVersion: string; source: string;
  structCount: number; fieldCount: number; root: GenericGffStruct;
};
export type CompileResult = {
  success: boolean; compiler: string; stdout: string; stderr: string;
  diagnostics: ScriptDiagnostic[]; ncs: NcsDocument | null;
};
export type ModuleBuildReport = {
  outputPath: string; sha256: string; sizeBytes: number; resourceCount: number;
  modifiedResources: number; deletedResources: number; sourceIntact: boolean;
};
export type DevelopmentFile = { name: string; sha256: string; sizeBytes: number };
export type DevelopmentDeployment = {
  workspaceId: string; developmentPath: string; files: DevelopmentFile[];
};
export type DevelopmentCleanupReport = { removed: string[]; preservedChanged: string[] };
export type PaletteManifest = { schemaVersion: number; categories: Array<{ id: string; label: string; resourceTypes: number[] }> };
export type WorkspaceExportManifest = { schemaVersion: number; workspaceId: string; sourceSha256: string; files: DevelopmentFile[]; deletedResources: ResourceKey[] };
export type AuroraSyncManifest = { schemaVersion: number; root: string; files: DevelopmentFile[] };
export type WalkmeshKind = "wok" | "pwk" | "dwk";
export type WalkmeshDraft = {
  vertices: Array<[number, number, number]>;
  faces: Array<[number, number, number]>;
  surfaceIds: number[];
  variants: WalkmeshVariantDraft[];
  hooks: WalkmeshHookDraft[];
};
export type WalkmeshVariantDraft = {
  name: string; position: [number, number, number]; rotation: [number, number, number, number];
  vertices: Array<[number, number, number]>; faces: Array<[number, number, number]>; surfaceIds: number[];
};
export type WalkmeshHookDraft = { name: string; position: [number, number, number]; rotation: [number, number, number, number] };
export type WalkmeshOperation =
  | { kind: "split_face"; faceIndex: number }
  | { kind: "remove_face"; faceIndex: number }
  | { kind: "weld_vertices"; tolerance: number }
  | { kind: "extrude_face"; faceIndex: number; distance: number }
  | { kind: "move_vertex"; vertexIndex: number; position: [number, number, number] }
  | { kind: "set_surface"; faceIndex: number; surfaceId: number };
export type WalkmeshValidation = { valid: boolean; diagnostics: string[] };
export type WalkmeshDocument = {
  resref: string; kind: WalkmeshKind; sourceFormat: "ascii" | "binary";
  draft: WalkmeshDraft; sourceSha256: string;
};
export type WalkmeshEditResult = { workspace: WorkspaceSnapshot; document: WalkmeshDocument };
export type AiChangeSet = { summary: string; commands: EditCommand[] };
export type AiChangeSetPreview = { summary: string; allValid: boolean; previews: Array<{ command: EditCommand; target: string; current: unknown; resulting: unknown; valid: boolean; diagnostic: string | null }> };

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
export type NarrativeDocument = { resource: ResourceKey; raw: GenericGff };
export type NarrativeInspection = {
  model: NarrativeModel;
  journal: NarrativeDocument | null;
  factions: NarrativeDocument | null;
};
export type JournalStructureAction =
  | { kind: "add_category"; tag: string }
  | { kind: "remove_category"; categoryIndex: number }
  | { kind: "add_entry"; categoryIndex: number }
  | { kind: "remove_entry"; categoryIndex: number; entryIndex: number };
export type FactionStructureAction =
  | { kind: "add_faction"; name: string; parentId: number | null }
  | { kind: "remove_faction"; factionIndex: number }
  | { kind: "add_reputation"; sourceId: number; targetId: number; value: number }
  | { kind: "remove_reputation"; reputationIndex: number };
export type BlueprintListKind = "feat" | "special_ability" | "class" | "equipped_item" | "item_property" | "sound" | "encounter_creature";
export type BlueprintStructureAction =
  | { kind: "add_feat"; featId: number }
  | { kind: "add_special_ability"; spellId: number; casterLevel: number; flags: number }
  | { kind: "add_class"; classId: number; classLevel: number }
  | { kind: "add_equipped_item"; resref: string; slot: number }
  | { kind: "add_item_property"; propertyName: number; subtype: number; costTable: number; costValue: number; param1: number; param1Value: number; chanceAppear: number }
  | { kind: "add_sound"; resref: string }
  | { kind: "add_encounter_creature"; resref: string; appearance: number; challengeRating: number; singleSpawn: boolean }
  | { kind: "remove_entry"; listKind: BlueprintListKind; entryIndex: number };
export type AreaPoint = { x: number; y: number; z: number };
export type AreaSpawnPoint = AreaPoint & { orientation: number };
export type AreaStructureAction =
  | { kind: "set_geometry"; instanceId: string; points: AreaPoint[] }
  | { kind: "set_spawn_points"; instanceId: string; points: AreaSpawnPoint[] }
  | { kind: "set_transition"; instanceId: string; destination: string; flags: number; loadScreenId: number }
  | { kind: "add_inventory_item"; instanceId: string; resref: string; stackSize: number; x: number; y: number; infinite: boolean; categoryIndex: number | null }
  | { kind: "remove_inventory_item"; instanceId: string; itemIndex: number; categoryIndex: number | null };
export type AreaTile = { x: number; y: number; tileId: number; orientation: number };
export type AreaInstance = {
  id: string; category: string; tag: string | null; templateResref: string | null;
  x: number; y: number; z: number; bearing: number | null;
  appearance: number | null;
  transitionDestination: string | null; transitionFlags: number | null; loadScreenId: number | null;
  geometry: AreaPoint[]; spawnPoints: AreaSpawnPoint[];
  inventory: Array<{ resref: string; tag: string | null; stackSize: number; x: number; y: number; infinite: boolean; categoryIndex: number | null; itemIndex: number }>;
  sourcePath: string;
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
export type DialogueNodeRef = { kind: "entry" | "reply"; index: number };
export type DialogueStructureAction =
  | { kind: "add_node"; nodeKind: "entry" | "reply" }
  | { kind: "remove_node"; node: DialogueNodeRef }
  | { kind: "add_link"; source: DialogueNodeRef | null; target: DialogueNodeRef }
  | { kind: "remove_link"; source: DialogueNodeRef | null; position: number };
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
  | { kind: "gff"; value: GenericGff }
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
    return { appVersion: "browser-preview", readOnly: true, editingAvailable: true, databaseSchemaVersion: 1 };
  }
  return invoke<AppStatus>("get_app_status");
}

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

export async function selectModuleOutput(defaultPath = "opennever-build.mod"): Promise<string | null> {
  requireTauri();
  return save({ defaultPath, filters: [{ name: "Module Neverwinter Nights", extensions: ["mod"] }] });
}

export async function selectHakOutput(defaultPath = "opennever-content.hak"): Promise<string | null> {
  requireTauri();
  return save({ defaultPath, filters: [{ name: "Hakpak Neverwinter Nights", extensions: ["hak"] }] });
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

export async function inspectAuroraWorkspace(request: { root: string }): Promise<AuroraSyncManifest> {
  requireTauri();
  return invoke<AuroraSyncManifest>("inspect_aurora_workspace", { request });
}

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

export async function previewAiChangeSet(request: { workspaceId: string; changeSet: AiChangeSet }): Promise<AiChangeSetPreview> {
  requireTauri();
  return invoke<AiChangeSetPreview>("preview_ai_change_set", { request });
}

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

export async function createWorkspaceArea(request: {
  jobId: string; workspaceId: string; resref: string; name: string; tileset: string;
  width: number; height: number; tileId: number;
}): Promise<{ workspace: WorkspaceSnapshot; area: AreaMap }> {
  requireTauri();
  return invoke<{ workspace: WorkspaceSnapshot; area: AreaMap }>("create_workspace_area", { request });
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
  return invoke<WorkspaceSnapshot>("edit_journal_structure_command", { request });
}

export async function editFactionStructure(request: { jobId: string; workspaceId: string; resource: ResourceKey; action: FactionStructureAction }): Promise<WorkspaceSnapshot> {
  requireTauri();
  return invoke<WorkspaceSnapshot>("edit_faction_structure_command", { request });
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

export async function selectCompiler(): Promise<string | null> {
  requireTauri();
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Compilateur NWScript", extensions: ["exe"] }],
  });
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
