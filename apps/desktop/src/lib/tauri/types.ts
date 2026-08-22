import type { AppError } from "./errors";

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
export type MapDensityRule = {
  category: string; perHundredTiles: number; minSpacingTiles: number; templateResrefs: string[];
};
export type MapGenerationSpec = {
  schemaVersion: number; brief: string; resref: string; name: string; tileset: string;
  width: number; height: number; seed: number; baseTileId: number; variantTileIds: number[];
  borderMargin: number; reservedPercent: number; densities: MapDensityRule[];
};
export type MapTilePlan = { x: number; y: number; tileId: number; orientation: number; height: number };
export type MapGenerationPlan = {
  planSha256: string; spec: MapGenerationSpec; tiles: MapTilePlan[]; placements: InstancePlacement[];
  metrics: { totalTiles: number; buildableTiles: number; reservedTiles: number; placementCount: number; occupiedPercent: number };
  compatibility: {
    tilesetResolved: boolean; tilesetSha256: string | null; resolvedTileCount: number;
    selectedTileIds: number[]; tileIdsVerified: boolean; edgeCompatibilityVerified: boolean;
  };
  warnings: string[];
};
export type MapAuthoringContext = {
  limits: {
    maxWidth: number; maxHeight: number; maxTiles: number; maxResrefLength: number;
    maxDensityRules: number; maxBlueprintsPerRule: number; maxPlacements: number;
  };
  availableTilesets: string[];
  selectedTileset: { resref: string; sha256: string; tileCount: number; tileIds: number[] } | null;
  blueprintCounts: Record<string, number>;
};
export type AiMapDraftResult = {
  endpointOrigin: string; model: string; plan: MapGenerationPlan; sharedBlueprintCount: number;
};
export type ApplyMapGenerationResult = { workspace: WorkspaceSnapshot; area: AreaMap; plan: MapGenerationPlan };
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
  compilerPath: string; gameInstallPath: string; userDataPath: string; includePaths: string[]; developmentPath: string;
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
export type AreaTile = { x: number; y: number; tileId: number; orientation: number; height: number };
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
  | { kind: "set_link_scripts"; source: DialogueNodeRef | null; position: number; conditionScript: string | null; actionScript: string | null }
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

export type ResourceSourceKind = "standalone" | "development" | "override" | "module" | "hak" | "patch" | "key_bif";
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
  migrationProgress?: MigrationProgress | null;
  migrationResult?: AreaMigrationExportResult | null;
  migrationAnalysisJobId?: string | null;
  migrationAreaResref?: string | null;
  migrationDestination?: string | null;
  error?: AppError | null;
};

export type MigrationStatus = "exact" | "converted" | "approximated" | "placeholder" | "manual" | "unsupported" | "missing" | "license-blocked";
export type MigrationPhase = "preparing" | "audit" | "models" | "textures" | "navigation" | "bundle" | "verifying";
export type MigrationProgress = { phase: MigrationPhase; percent: number; current?: string | null };
export type MigrationDiagnostic = {
  sequence: number; severity: "info" | "warning" | "error"; status: MigrationStatus;
  phase: MigrationPhase; code: string; message: string; resource?: string | null; identity?: string | null;
};
export type MigrationCounts = {
  tiles: number; instances: number; uniqueModels: number; textures: number;
  preservedNavigation: number; missingItems: number; fallbacks: number;
  diagnostics: number; warnings: number; errors: number; byStatus: Record<string, number>;
};
export type AreaMigrationCandidate = {
  resref: string; name: string; width: number; height: number;
  tileCount: number; instanceCount: number; sourceDiagnosticCount: number;
};
export type AreaMigrationPreview = {
  schemaVersion: string; areaResref: string; areaName: string; suggestedDirectoryName: string;
  ready: boolean; complete: boolean; counts: MigrationCounts; diagnostics: MigrationDiagnostic[];
  classification: string; redistribution: string; navigationStatus: string;
};
export type BundleFileRecord = { path: string; sizeBytes: number; sha256: string; role: string };
export type MigrationReport = {
  schemaVersion: string; areaResref: string; complete: boolean; counts: MigrationCounts;
  navigationConverted: boolean; navigationStatus: string; diagnosticsFile: string;
  bundleIsLocalOnly: boolean; sourceModuleImmutable: boolean;
  payloadFileCount: number; payloadSizeBytes: number;
};
export type AreaMigrationExportResult = {
  bundlePath: string; manifestFile: BundleFileRecord;
  report: MigrationReport; diagnostics: MigrationDiagnostic[];
};

export type AssetExportMode = "static" | "animated";
export type AssetExportCandidate = {
  resref: string; format: string; source: string; exportable: boolean;
  declaredAnimationCount: number; declaredAnimations: string[];
  meshCount: number; triangleCount: number; skinCount: number;
  textureCount: number; diagnosticCount: number;
};
export type AssetAnimationSummary = {
  name: string; lengthSeconds: number; transitionSeconds: number; rootNode: string;
  trackCount: number; eventCount: number; exported: boolean;
};
export type AssetTextureSummary = {
  resref: string; resourceType: number | null; outputPath: string | null;
  status: string; diagnostic: string | null;
};
export type AssetExportPreview = {
  schemaVersion: string; resref: string; mode: AssetExportMode; ready: boolean;
  suggestedDirectoryName: string; nodeCount: number; meshCount: number;
  primitiveCount: number; skinCount: number; animationCount: number;
  animations: AssetAnimationSummary[]; textures: AssetTextureSummary[];
  warnings: string[]; classification: string; redistribution: string;
};
export type AssetExportFile = { path: string; role: string; sizeBytes: number; sha256: string };
export type AssetExportManifest = {
  schemaVersion: string; generator: string; classification: string; redistribution: string;
  sourceModuleSha256: string; sourceModel: string; sourceModelSha256: string;
  sourceDependencies: Record<string, string>; mode: AssetExportMode;
  animations: AssetAnimationSummary[]; textures: AssetTextureSummary[];
  warnings: string[]; files: AssetExportFile[]; sourceModuleImmutable: boolean;
};
export type AssetExportResult = {
  schemaVersion: string; destination: string; resref: string; mode: AssetExportMode;
  glbPath: string; glbSha256: string; glbSizeBytes: number;
  animationCount: number; textureCount: number; warnings: string[];
  manifest: AssetExportManifest;
};

export type DialogueExportRevision = "analysis" | "workspace";
export type DialogueExportPreview = {
  schemaVersion: string; resref: string; revision: DialogueExportRevision; ready: boolean;
  suggestedDirectoryName: string; sourceResourceSha256: string;
  nodeCount: number; entryCount: number; replyCount: number; linkCount: number;
  rootCount: number; sharedNodeCount: number; unreachableNodeCount: number;
  cycleCount: number; brokenLinkCount: number; diagnosticCount: number;
  referenceCount: number; scripts: string[]; transcriptPreview: string[];
  warnings: string[]; classification: string; redistribution: string;
};
export type DialogueExportFile = { path: string; role: string; sizeBytes: number; sha256: string };
export type DialogueExportManifest = {
  schemaVersion: string; generator: string; classification: string; redistribution: string;
  resref: string; revision: DialogueExportRevision; sourceResourceSha256: string;
  nodeCount: number; linkCount: number; rootCount: number; brokenLinkCount: number;
  cycleCount: number; scripts: string[]; warnings: string[];
  files: DialogueExportFile[]; sourceNwnImmutable: boolean;
};
export type DialogueExportResult = {
  schemaVersion: string; destination: string; resref: string; revision: DialogueExportRevision;
  sourceResourceSha256: string; nodeCount: number; linkCount: number;
  fileCount: number; totalSizeBytes: number; warnings: string[];
  manifest: DialogueExportManifest;
};

export type RestoredModuleSession = {
  job: JobSnapshot;
  workspace: WorkspaceSnapshot | null;
};

export type PrepareSceneModelsReport = {
  requested: number;
  prepared: number;
  cacheHits: number;
  failed: number;
  durationMs: number;
};

export type DiagnosticReportBundle = { report: DiagnosticReport; json: string; html: string };
