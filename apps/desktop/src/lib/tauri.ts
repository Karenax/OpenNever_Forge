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
  migrationHistory?: Array<{ fromVersion: number; toVersion: number; backupPath: string; steps: string[] }>;
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
export type BlueprintFieldOption = { value: number; label: string; source: string };
export type BlueprintFieldOptions = { fields: Record<string, BlueprintFieldOption[]> };
export type WorkspaceExportManifest = { schemaVersion: number; workspaceId: string; sourceSha256: string; files: DevelopmentFile[]; deletedResources: ResourceKey[] };
export type AuroraSyncManifest = { schemaVersion: number; root: string; files: DevelopmentFile[] };
export type AuroraSyncState = "identical" | "toolset_only" | "workspace_only" | "toolset_changed" | "workspace_changed" | "conflict";
export type AuroraSyncDirection = "pull_from_toolset" | "push_to_toolset";
export type AuroraSyncEntry = {
  resource: ResourceKey; relativePath: string; toolsetSha256: string | null; workspaceSha256: string | null;
  baselineToolsetSha256: string | null; baselineWorkspaceSha256: string | null; state: AuroraSyncState;
};
export type AuroraSyncPlan = {
  schemaVersion: number; root: string; baselineFound: boolean; entries: AuroraSyncEntry[];
  identicalCount: number; incomingCount: number; outgoingCount: number; conflictCount: number;
};
export type AuroraSyncAction = {
  resource: ResourceKey; direction: AuroraSyncDirection;
  expectedToolsetSha256: string | null; expectedWorkspaceSha256: string | null;
};
export type AuroraSyncReport = {
  schemaVersion: number; root: string;
  applied: Array<{ resource: ResourceKey; direction: AuroraSyncDirection; sha256: string | null }>;
  backups: string[]; plan: AuroraSyncPlan; workspace: WorkspaceSnapshot;
};
export type ModuleBuildProfile = {
  name: string; outputName: string; blockOnWarnings: boolean; deployDevelopment: boolean;
  hakFiles: string[]; customTlk: string | null;
};
export type ReproducibleBuildVerification = {
  profile: ModuleBuildProfile; firstSha256: string; secondSha256: string;
  identical: boolean; resourceCount: number; warnings: string[];
};
export type GitFileStatus = { path: string; indexStatus: string; worktreeStatus: string };
export type GitWorkspaceStatus = {
  root: string; branch: string; head: string | null; clean: boolean; files: GitFileStatus[];
};
export type NwnLaunchProfile = {
  name: string; mode: "client" | "server"; executablePath: string;
  workingDirectory: string; arguments: string[];
};
export type NwnLaunchReport = { profile: NwnLaunchProfile; processId: number; logPath: string };
export type BuildProfileRunReport = {
  profile: ModuleBuildProfile; build: ModuleBuildReport; deployment: DevelopmentDeployment | null; warnings: string[];
};
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
export type AiCommandPreview = { command: EditCommand; target: string; current: unknown; resulting: unknown; valid: boolean; diagnostic: string | null };
export type AiChangeSetPreview = { summary: string; proposalSha256: string; allValid: boolean; previews: AiCommandPreview[] };
export type AiConsent = { includeModuleMetadata: boolean; includeResourceContents: boolean };
export type AiProviderProposal = {
  endpointOrigin: string; model: string; proposalSha256: string; changeSet: AiChangeSet;
  preview: AiChangeSetPreview; sharedResources: number; warnings: string[];
};
export type AiApplyReport = { proposalSha256: string; appliedCommands: number; workspace: WorkspaceSnapshot };
export type SecurityLevel = "observer" | "advisor" | "assisted" | "supervised" | "autonomous" | "operator";
export type CapabilityAccess = "deny" | "read" | "preview" | "execute";
export type ApprovalMode = "always" | "per_batch" | "above_risk" | "never";
export type ToolScope = "selected_resource" | "area" | "module" | "workspace";
export type CapabilityRisk = "low" | "moderate" | "high" | "critical";
export type CapabilitySideEffect = "none" | "reversible_workspace" | "build_output" | "external";
export type AgentLimits = {
  maxTurns: number; maxToolCalls: number; maxParallelCalls: number; maxRetries: number;
  maxPromptBytes: number; maxContextResources: number; maxContextResourceBytes: number;
  maxResponseBytes: number; maxOutputTokens: number; maxDurationSeconds: number; maxCostMicroUsd: number;
};
export type ContextPolicy = {
  allowNetwork: boolean; includeModuleMetadata: boolean; includeResourceContents: boolean;
  includeDiagnostics: boolean; includeArchitectureGraph: boolean; includeLocalPaths: boolean; retainConversation: boolean;
  retentionDays: number; allowInsecureLocalHttp: boolean; allowedProviderHosts: string[];
};
export type ToolRuntimePolicy = {
  compilerPath: string; gameInstallPath: string; includePaths: string[]; developmentPath: string;
  toolsetTempPath: string; allowedOutputRoots: string[];
  nwnExecutablePath: string; nwnWorkingDirectory: string; nwnArguments: string[];
};
export type CapabilityOverride = { access: CapabilityAccess; approval: ApprovalMode; scope: ToolScope; maxCalls: number };
export type ScopeGrants = { selectedResources: ResourceKey[]; areas: string[] };
export type AgentPolicy = {
  schemaVersion: number; name: string; level: SecurityLevel; context: ContextPolicy; limits: AgentLimits;
  toolRuntime: ToolRuntimePolicy; capabilityOverrides: Record<string, CapabilityOverride>; scopeGrants: ScopeGrants; allowDevelopmentDeploy: boolean;
  allowToolsetSync: boolean; allowProcessLaunch: boolean; stopOnValidationError: boolean;
  checkpointBeforeWrite: boolean;
};
export type CapabilityDescriptor = {
  id: string; title: string; description: string; category: string; risk: CapabilityRisk;
  sideEffect: CapabilitySideEffect; reversible: boolean; inputSchema: unknown; outputSchema: unknown;
};
export type CapabilityRegistry = { schemaVersion: number; capabilities: CapabilityDescriptor[] };
export type EffectiveCapability = { id: string; access: CapabilityAccess; approval: ApprovalMode; scope: ToolScope; maxCalls: number; reason: string };
export type ProviderKind = "open_ai_responses" | "open_ai_chat_completions" | "ollama" | "compatible" | "manual";
export type ProviderProfile = {
  id: string; name: string; kind: ProviderKind; endpoint: string; model: string;
  reasoningEffort?: string; temperatureMilli?: number; supportsTools: boolean;
  supportsParallelTools: boolean; supportsStructuredOutput: boolean;
  storeResponses: boolean;
  inputCostMicroUsdPerMillionTokens: number; outputCostMicroUsdPerMillionTokens: number;
};
export type AgentToolCall = {
  id: string; capabilityId: string; arguments: unknown; argumentsSha256: string;
  status: "proposed" | "waiting_approval" | "running" | "completed" | "rejected" | "failed";
  result?: unknown; error?: string; startedUnixMs?: number; completedUnixMs?: number;
};
export type AgentApproval = {
  id: string; toolCallId: string; capabilityId: string; summary: string;
  toolCallIds: string[];
  status: "pending" | "approved" | "rejected"; createdUnixMs: number; resolvedUnixMs?: number;
};
export type AgentEvent = { sequence: number; unixMs: number; kind: string; message: string; data?: unknown };
export type AgentRun = {
  schemaVersion: number; id: string; jobId: string; workspaceId: string; objective: string;
  status: "planned" | "running" | "waiting_approval" | "completed" | "failed" | "cancelled";
  provider: ProviderProfile; policy: AgentPolicy; blueprint?: unknown; currentTurn: number; estimatedCostMicroUsd: number;
  toolCalls: AgentToolCall[]; approvals: AgentApproval[]; events: AgentEvent[]; createdUnixMs: number; updatedUnixMs: number;
};
export type AgentStudioState = {
  policy: AgentPolicy; presets: AgentPolicy[]; registry: CapabilityRegistry; effectiveCapabilities: EffectiveCapability[]; runs: AgentRun[];
};
export type AgentProviderTestReport = {
  endpointOrigin: string; model: string; latencyMs: number; reply: string;
};

export type HashProgress = {
  bytesRead: number;
  totalBytes: number;
  percent: number;
  phase: "hashing" | "inventory" | "dependencies" | "resource_catalog" | "structured_resources" | "scripts" | "dialogues" | "world" | "persisting";
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
  resourceCatalogCache: {
    state: "disabled" | "hit" | "miss";
    signature?: string | null;
    path?: string | null;
    gameResourceCount: number;
  };
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
  | { kind: "two_da"; value: TwoDaTable }
  | { kind: "tlk"; value: TalkTable }
  | { kind: "binary"; value: { size: number; sha256: string; hexPreview: string; truncated: boolean } };

export type TwoDaTable = {
  source: string; defaultValue: string | null; columns: string[];
  rows: Array<{ label: string; cells: Array<string | null> }>;
};
export type TwoDaEditAction =
  | { kind: "set_cell"; rowIndex: number; columnIndex: number; value: string | null }
  | { kind: "add_row"; label: string }
  | { kind: "remove_row"; rowIndex: number }
  | { kind: "set_default"; value: string | null };
export type TlkEntry = {
  index: number; flags: number; text: string | null; soundResref: string | null;
  volumeVariance: number; pitchVariance: number; soundLength: number;
};
export type TalkTable = { languageId: number; entries: TlkEntry[]; source: string };
export type TlkEditAction =
  | { kind: "set_entry"; index: number; text: string | null; soundResref: string | null; soundLength: number }
  | { kind: "append_entry"; text: string | null };

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

export async function editWorkspaceTwoDa(request: {
  jobId: string; workspaceId: string; resource: ResourceKey; action: TwoDaEditAction;
}): Promise<{ workspace: WorkspaceSnapshot; document: TwoDaTable }> {
  requireTauri();
  return invoke("edit_workspace_2da", { request });
}

export async function editWorkspaceTlk(request: {
  jobId: string; workspaceId: string; resource: ResourceKey; action: TlkEditAction;
}): Promise<{ workspace: WorkspaceSnapshot; document: TalkTable }> {
  requireTauri();
  return invoke("edit_workspace_tlk", { request });
}

export async function editWorkspaceModuleDependencies(request: {
  jobId: string; workspaceId: string; hakFiles: string[]; customTlk: string | null;
}): Promise<{ workspace: WorkspaceSnapshot; document: GenericGff }> {
  requireTauri();
  return invoke("edit_workspace_module_dependencies", { request });
}

export async function listWorkspaceBuildProfiles(request: { workspaceId: string }): Promise<ModuleBuildProfile[]> {
  requireTauri();
  return invoke("list_workspace_build_profiles", { request });
}

export async function saveWorkspaceBuildProfile(request: { workspaceId: string; profile: ModuleBuildProfile }): Promise<ModuleBuildProfile[]> {
  requireTauri();
  return invoke("save_workspace_build_profile", { request });
}

export async function verifyWorkspaceReproducibleBuild(request: { workspaceId: string; profile: ModuleBuildProfile }): Promise<ReproducibleBuildVerification> {
  requireTauri();
  return invoke("verify_workspace_reproducible_build", { request });
}

export async function runWorkspaceBuildProfile(request: {
  workspaceId: string; profile: ModuleBuildProfile; outputDirectory: string; userDataPath: string | null;
}): Promise<BuildProfileRunReport> {
  requireTauri();
  return invoke("run_workspace_build_profile", { request });
}

export async function inspectGitWorkspace(request: { root: string }): Promise<GitWorkspaceStatus> {
  requireTauri();
  return invoke("inspect_git_workspace", { request });
}

export async function listWorkspaceLaunchProfiles(request: { workspaceId: string }): Promise<NwnLaunchProfile[]> {
  requireTauri();
  return invoke("list_workspace_launch_profiles", { request });
}

export async function saveWorkspaceLaunchProfile(request: { workspaceId: string; profile: NwnLaunchProfile }): Promise<NwnLaunchProfile[]> {
  requireTauri();
  return invoke("save_workspace_launch_profile", { request });
}

export async function launchWorkspaceTestProfile(request: { workspaceId: string; profile: NwnLaunchProfile }): Promise<NwnLaunchReport> {
  requireTauri();
  return invoke("launch_workspace_test_profile", { request });
}

export async function inspectAuroraWorkspace(request: { root: string }): Promise<AuroraSyncManifest> {
  requireTauri();
  return invoke<AuroraSyncManifest>("inspect_aurora_workspace", { request });
}

export async function planAuroraWorkspaceSync(request: { jobId: string; workspaceId: string; root: string }): Promise<AuroraSyncPlan> {
  requireTauri();
  return invoke<AuroraSyncPlan>("plan_aurora_workspace_sync", { request });
}

export async function applyAuroraWorkspaceSync(request: {
  jobId: string; workspaceId: string; root: string; actions: AuroraSyncAction[];
}): Promise<AuroraSyncReport> {
  requireTauri();
  return invoke<AuroraSyncReport>("apply_aurora_workspace_sync", { request });
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

export async function previewAiChangeSet(request: { jobId: string; workspaceId: string; changeSet: AiChangeSet }): Promise<AiChangeSetPreview> {
  requireTauri();
  return invoke<AiChangeSetPreview>("preview_ai_change_set", { request });
}

export async function requestAiChangeSet(request: {
  jobId: string; workspaceId: string; endpoint: string; model: string; apiKey?: string;
  prompt: string; selectedResources: ResourceKey[]; consent: AiConsent;
}): Promise<AiProviderProposal> {
  requireTauri();
  return invoke<AiProviderProposal>("request_ai_change_set", { request });
}

export async function applyAiChangeSet(request: {
  jobId: string; workspaceId: string; proposalSha256: string; changeSet: AiChangeSet; confirmed: boolean;
}): Promise<AiApplyReport> {
  requireTauri();
  return invoke<AiApplyReport>("apply_ai_change_set", { request });
}

export async function getAgentStudioState(workspaceId: string): Promise<AgentStudioState> {
  requireTauri();
  return invoke<AgentStudioState>("get_agent_studio_state", { request: { workspaceId } });
}

export async function saveAgentPolicy(workspaceId: string, policy: AgentPolicy): Promise<AgentStudioState> {
  requireTauri();
  return invoke<AgentStudioState>("save_agent_policy", { request: { workspaceId, policy } });
}

export async function createAgentRun(request: {
  jobId: string; workspaceId: string; objective: string; provider: ProviderProfile; policy?: AgentPolicy; blueprint?: unknown;
}): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("create_agent_run", { request });
}

export async function advanceAgentRun(request: { workspaceId: string; runId: string; apiKey?: string }): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("advance_agent_run", { request });
}

export async function testAgentProvider(request: {
  provider: ProviderProfile; policy: AgentPolicy; apiKey?: string;
}): Promise<AgentProviderTestReport> {
  requireTauri();
  return invoke<AgentProviderTestReport>("test_agent_provider", { request });
}

export async function resolveAgentApproval(request: { workspaceId: string; runId: string; approvalId: string; approved: boolean }): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("resolve_agent_approval", { request });
}

export async function cancelAgentRun(request: { workspaceId: string; runId: string }): Promise<AgentRun> {
  requireTauri();
  return invoke<AgentRun>("cancel_agent_run", { request });
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

export async function selectNwnExecutable(): Promise<string | null> {
  requireTauri();
  const selected = await open({ multiple: false, directory: false, filters: [{ name: "Neverwinter Nights", extensions: ["exe"] }] });
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
