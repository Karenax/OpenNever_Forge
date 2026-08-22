import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App, { isTileOccluder, JobProgress, sceneGroundElevation, sceneGroundPosition, WalkmeshWorkbench } from "./App";
import { applyAiChangeSet, applyAuroraWorkspaceSync, applyMapGeneration, buildWorkspaceModule, createEditWorkspace, createWorkspaceArea, deployWorkspaceDevelopment, draftMapWithAi, editAreaStructure, editBlueprintStructure, editDialogueField, editDialogueStructure, editFactionStructure, editWorkspaceModuleDependencies, inspectDialogue, moveAreaInstance, planAuroraWorkspaceSync, previewAiChangeSet, previewMapGeneration, requestAiChangeSet, restoreModuleSession, saveWorkspaceBuildProfile, saveWorkspaceWalkmesh, selectDirectory, selectModuleOutput, startModuleAnalysis, testAgentProvider, transformWalkmeshDraft, undoEditCommand } from "./lib/tauri";
import type { DialogueGraph } from "./lib/tauri";
import { LAST_EXPLORER_ITEM_STORAGE_KEY, PROJECT_PREFERENCES_STORAGE_KEY } from "./lib/projectPreferences";
import { useWorkbenchStore } from "./store/workbenchStore";
import { useUiStore } from "./store/uiStore";

vi.mock("@monaco-editor/react", () => ({ default: ({ value }: { value: string }) => <pre data-testid="monaco-readonly">{value}</pre> }));
vi.mock("@xyflow/react", () => ({ ReactFlow: ({ nodes, edges, children }: { nodes: Array<{id:string}>; edges:Array<{id:string}>; children: React.ReactNode }) => <div data-testid="dialogue-flow">{nodes.length} nodes · {edges.length} edges{children}</div>, Background:()=>null, Controls:()=>null, MiniMap:()=>null }));

vi.mock("./lib/tauri", () => ({
  getAgentStudioState: vi.fn().mockResolvedValue({
    policy: {
      schemaVersion: 2,
      name: "Conseiller",
      level: "advisor",
      context: { allowNetwork: false, includeModuleMetadata: false, includeResourceContents: false, includeDiagnostics: true, includeArchitectureGraph: false, includeLocalPaths: false, retainConversation: true, retentionDays: 30, allowInsecureLocalHttp: true, allowedProviderHosts: ["api.openai.com", "localhost", "127.0.0.1", "::1"] },
      limits: { maxTurns: 12, maxToolCalls: 48, maxParallelCalls: 4, maxRetries: 2, maxPromptBytes: 32768, maxContextResources: 16, maxContextResourceBytes: 262144, maxResponseBytes: 2097152, maxOutputTokens: 8192, maxDurationSeconds: 900, maxCostMicroUsd: 5000000 },
      toolRuntime: { compilerPath: "", gameInstallPath: "", userDataPath: "", includePaths: [], developmentPath: "", toolsetTempPath: "", allowedOutputRoots: [], nwnExecutablePath: "", nwnWorkingDirectory: "", nwnArguments: [] },
      capabilityOverrides: { "*": { access: "preview", approval: "always", scope: "workspace", maxCalls: 48 } },
      scopeGrants: { selectedResources: [], areas: [] },
      allowDevelopmentDeploy: false,
      allowToolsetSync: false,
      allowProcessLaunch: false,
      stopOnValidationError: true,
      checkpointBeforeWrite: true,
    },
    presets: [
      {
        schemaVersion: 2,
        name: "Agent supervisé",
        level: "supervised",
        context: { allowNetwork: false, includeModuleMetadata: true, includeResourceContents: true, includeDiagnostics: true, includeArchitectureGraph: false, includeLocalPaths: false, retainConversation: true, retentionDays: 30, allowInsecureLocalHttp: true, allowedProviderHosts: ["api.openai.com", "localhost", "127.0.0.1", "::1"] },
        limits: { maxTurns: 24, maxToolCalls: 128, maxParallelCalls: 4, maxRetries: 2, maxPromptBytes: 32768, maxContextResources: 16, maxContextResourceBytes: 262144, maxResponseBytes: 2097152, maxOutputTokens: 8192, maxDurationSeconds: 900, maxCostMicroUsd: 5000000 },
        toolRuntime: { compilerPath: "", gameInstallPath: "", userDataPath: "", includePaths: [], developmentPath: "", toolsetTempPath: "", allowedOutputRoots: [], nwnExecutablePath: "", nwnWorkingDirectory: "", nwnArguments: [] },
        capabilityOverrides: { "*": { access: "execute", approval: "above_risk", scope: "workspace", maxCalls: 128 } },
        scopeGrants: { selectedResources: [], areas: [] },
        allowDevelopmentDeploy: false,
        allowToolsetSync: false,
        allowProcessLaunch: false,
        stopOnValidationError: true,
        checkpointBeforeWrite: true,
      },
    ],
    registry: { schemaVersion: 2, capabilities: [] },
    effectiveCapabilities: [],
    runs: [],
  }),
  saveAgentPolicy: vi.fn(),
  createAgentRun: vi.fn(),
  advanceAgentRun: vi.fn(),
  testAgentProvider: vi.fn(),
  resolveAgentApproval: vi.fn(),
  cancelAgentRun: vi.fn(),
  getAppStatus: vi.fn().mockResolvedValue({
    appVersion: "0.1.0-test",
    readOnly: true,
    editingAvailable: true,
    databaseSchemaVersion: 6,
  }),
  restoreModuleSession: vi.fn().mockResolvedValue(null),
  getStandardPalette: vi.fn().mockResolvedValue({
    schemaVersion: 1,
    categories: [
      { id: "creatures", label: "Créatures", resourceTypes: [2027] },
      { id: "doors", label: "Portes", resourceTypes: [2042] },
      { id: "encounters", label: "Rencontres", resourceTypes: [2040] },
      { id: "items", label: "Objets", resourceTypes: [2025] },
      { id: "placeables", label: "Plaçables", resourceTypes: [2044] },
      { id: "sounds", label: "Sons", resourceTypes: [2035] },
      { id: "stores", label: "Marchands", resourceTypes: [2051] },
      { id: "triggers", label: "Déclencheurs", resourceTypes: [2032] },
      { id: "waypoints", label: "Points de passage", resourceTypes: [2058] },
    ],
  }),
  getBlueprintFieldOptions: vi.fn().mockResolvedValue({
    fields: {
      Gender: [
        { value: 0, label: "Masculin", source: "règle Aurora Gender" },
        { value: 1, label: "Féminin", source: "règle Aurora Gender" },
      ],
      FactionID: [{ value: 0, label: "PC", source: "factions du module" }],
    },
  }),
  getJob: vi.fn().mockResolvedValue({
    id: "job-1",
    kind: "module_analysis",
    state: "completed",
    sourcePath: "C:/module.mod",
    progress: { bytesRead: 512, totalBytes: 512, percent: 100 },
    result: {
      fingerprint: { sha256: "ABC123", sizeBytes: 512 },
      moduleInfo: {
        name: { stringRef: null, values: [{ languageId: 0, text: "Forge Test" }] },
        description: {
          stringRef: null,
          values: [{ languageId: 0, text: "Synthetic module" }],
        },
        tag: "MODULE",
        minimumGameVersion: "1.69",
        customTlk: null,
        entryArea: "startarea",
        hakFiles: ["shared", "missing"],
      },
      dependencyReport: {
        resolvedCount: 1,
        missingCount: 1,
        uncheckedCount: 0,
        invalidCount: 0,
        changedCount: 1,
        dependencies: [
          {
            kind: "hak",
            logicalName: "shared",
            state: "resolved",
            selectedPath: "C:/user/hak/shared.hak",
            shadowedPaths: ["C:/game/data/hk/shared.hak"],
            searchedPaths: ["C:/user/hak/shared.hak", "C:/game/data/hk/shared.hak"],
            fingerprint: { sha256: "1234567890ABCDEF1234567890ABCDEF", sizeBytes: 1024 },
            change: "content_changed",
          },
          {
            kind: "hak",
            logicalName: "missing",
            state: "missing",
            selectedPath: null,
            shadowedPaths: [],
            searchedPaths: ["C:/user/hak/missing.hak", "C:/game/data/hk/missing.hak"],
            fingerprint: null,
            change: "first_seen",
          },
        ],
      },
      inventory: {
        fileType: "MOD ",
        fileVersion: "V1.0",
        buildYear: 2026,
        buildDay: 213,
        resourceCount: 3,
        resources: [
          {
            key: { resref: "module", resourceType: 2014 },
            resourceId: 0,
            extension: "ifo",
            offset: 224,
            size: 128,
          },
          {
            key: { resref: "start", resourceType: 2009 },
            resourceId: 1,
            extension: "nss",
            offset: 352,
            size: 160,
          },
          {
            key: { resref: "ambience", resourceType: 2035 },
            resourceId: 2,
            extension: "uts",
            offset: 512,
            size: 96,
          },
        ],
        typeSummaries: [
          { resourceType: 2009, extension: "nss", count: 1, totalSize: 160 },
          { resourceType: 2014, extension: "ifo", count: 1, totalSize: 128 },
          { resourceType: 2035, extension: "uts", count: 1, totalSize: 96 },
        ],
      },
      resourceCatalog: {
        versionCount: 2,
        shadowedCount: 0,
        diagnostics: [],
        entries: [
          {
            key: { resref: "module", resourceType: 2014 },
            selected: {
              key: { resref: "module", resourceType: 2014 },
              sourceKind: "module",
              sourceName: "module",
              sourcePath: "C:/module.mod",
              priority: 20,
              offset: 224,
              size: 128,
              sha256: null,
              location: { kind: "erf", path: "C:/module.mod", offset: 224, size: 128 },
            },
            shadowed: [],
          },
          {
            key: { resref: "start", resourceType: 2009 },
            selected: {
              key: { resref: "start", resourceType: 2009 },
              sourceKind: "module",
              sourceName: "module",
              sourcePath: "C:/module.mod",
              priority: 20,
              offset: 352,
              size: 160,
              sha256: null,
              location: { kind: "erf", path: "C:/module.mod", offset: 352, size: 160 },
            },
            shadowed: [],
          },
          {
            key: { resref: "ambience", resourceType: 2035 },
            selected: { key: { resref: "ambience", resourceType: 2035 }, sourceKind: "module", sourceName: "module", sourcePath: "C:/module.mod", priority: 20, offset: 512, size: 96, sha256: null, location: { kind: "erf", path: "C:/module.mod", offset: 512, size: 96 } },
            shadowed: [],
          },
        ],
      },
      resourceCatalogSummary: {
        resourceCount: 3,
        versionCount: 3,
        shadowedCount: 0,
        diagnosticCount: 0,
        typeCounts: [
          { resourceType: 2009, count: 1 },
          { resourceType: 2014, count: 1 },
          { resourceType: 2035, count: 1 },
        ],
        sourceCounts: [{ source: "module", count: 2 }],
      },
      structuredSummary: {
        gff: { discovered: 1, parsed: 1, failed: 0, structCount: 2, fieldCount: 8 },
        twoDaTables: [],
        talkTables: [],
        resolvedModuleName: {
          text: "Forge Test",
          origin: "embedded",
          state: "resolved",
          stringRef: null,
          tableIndex: null,
          source: null,
        },
        areas: [],
        blueprints: [],
        diagnostics: [],
      },
      scriptIndexSummary: { scripts: 1, nss: 1, ncs: 1, paired: 1, missingSource: 0, includes: 0, symbols: 1, calls: 1, inboundReferences: 1, diagnostics: 0 },
      dialogueIndexSummary: { dialogues: 1, nodes: 2, links: 3, sharedNodes: 1, cycles: 1, unreachableNodes: 0, brokenLinks: 0, scriptLinks: 3, references: 1, diagnostics: 1 },
      worldSummary: { journalCategories: 1, journalEntries: 1, factions: 1, factionRelations: 1, areas: 1, tiles: 1, instances: 1, transitions: 0, assets: 1, previewableAssets: 1, sceneObjects: 2, graphNodes: 3, graphEdges: 2, diagnostics: 1 },
    },
  }),
  startModuleAnalysis: vi.fn().mockResolvedValue({
    id: "job-1",
    kind: "module_analysis",
    state: "queued",
    sourcePath: "C:/module.mod",
    progress: { bytesRead: 0, totalBytes: 0, percent: 0 },
  }),
  prepareSceneModels: vi.fn().mockResolvedValue({ requested: 0, prepared: 0, cacheHits: 0, failed: 0, durationMs: 0 }),
  cancelJob: vi.fn(),
  inspectResource: vi.fn().mockResolvedValue({ kind: "gff", value: { fileType: "UTS ", fileVersion: "V3.2", source: "ambience.uts", structCount: 1, fieldCount: 3, root: { index: 0, structType: 4294967295, fields: [{ label: "Tag", fieldType: 10, value: { kind: "string", value: "Ambience" } }, { label: "TemplateResRef", fieldType: 11, value: { kind: "res_ref", value: "ambience" } }, { label: "Sounds", fieldType: 15, value: { kind: "list", value: [] } }] } } }),
  queryResources: vi.fn((request: { query?: string; resourceTypes?: number[]; offset?: number; limit?: number }) => {
    const entries = [
      {
        key: { resref: "module", resourceType: 2014 },
        selected: { key: { resref: "module", resourceType: 2014 }, sourceKind: "module", sourceName: "module", sourcePath: "C:/module.mod", priority: 20, offset: 224, size: 128, sha256: null, location: { kind: "erf", path: "C:/module.mod", offset: 224, size: 128 } },
        shadowed: [],
      },
      {
        key: { resref: "start", resourceType: 2009 },
        selected: { key: { resref: "start", resourceType: 2009 }, sourceKind: "module", sourceName: "module", sourcePath: "C:/module.mod", priority: 20, offset: 352, size: 160, sha256: null, location: { kind: "erf", path: "C:/module.mod", offset: 352, size: 160 } },
        shadowed: [],
      },
      {
        key: { resref: "ambience", resourceType: 2035 },
        selected: { key: { resref: "ambience", resourceType: 2035 }, sourceKind: "module", sourceName: "module", sourcePath: "C:/module.mod", priority: 20, offset: 512, size: 96, sha256: null, location: { kind: "erf", path: "C:/module.mod", offset: 512, size: 96 } },
        shadowed: [],
      },
    ];
    const query = request.query?.toLowerCase() ?? "";
    const matching = entries.filter((entry) =>
      (!query || entry.key.resref.includes(query)) &&
      (!request.resourceTypes?.length || request.resourceTypes.includes(entry.key.resourceType)),
    );
    const offset = request.offset ?? 0;
    const limit = request.limit ?? 100;
    return Promise.resolve({ items: matching.slice(offset, offset + limit), offset, limit, total: matching.length });
  }),
  queryScripts: vi.fn().mockResolvedValue({ items: [{ resref: "start", hasNss: true, hasNcs: true, symbolCount: 1, inboundReferenceCount: 1, diagnosticCount: 0, matches: [] }], offset: 0, limit: 80, total: 1 }),
  inspectScript: vi.fn().mockResolvedValue({
    resref: "start",
    nss: { source: "C:/module.mod::start.nss", text: "void main() { StartingConditional(); }", lineCount: 1, includes: [], symbols: [{ name: "main", kind: "function", line: 1, declaration: "void main()" }], calls: [{ name: "StartingConditional", line: 1 }], diagnostics: [] },
    ncs: { source: "C:/module.mod::start.ncs", size: 12, sha256: "ABC", header: "NCS V1.0", bytecodeSize: 4, hexPreview: "4E43532056312E30", validHeader: true },
    inboundReferences: [{ script: "start", resource: { resref: "module", resourceType: 2014 }, fieldPath: "root.OnModuleLoad", source: "C:/module.mod::module.ifo" }],
    diagnostics: [],
  }),
  queryDialogues: vi.fn().mockResolvedValue({ items: [{ resref: "forge_dialogue", nodeCount: 2, linkCount: 3, cycleCount: 1, diagnosticCount: 1, preview: "Bonjour" }], offset: 0, limit: 50, total: 1 }),
  inspectDialogue: vi.fn().mockResolvedValue({
    key: { resref: "forge_dialogue", resourceType: 2029 }, source: "C:/module.mod::forge_dialogue.dlg",
    nodes: [{ id: "entry:0", kind: "entry", index: 0, text: null, displayText: "Bonjour", speaker: "NPC", comment: "Accueil", animation: 1, animationLoop: true, sound: "hello", quest: null, actionScript: "start" }, { id: "reply:0", kind: "reply", index: 0, text: null, displayText: "Au revoir", speaker: null, comment: null, animation: null, animationLoop: null, sound: null, quest: null, actionScript: null }],
    links: [{ id: "start:entry:0:0", source: null, target: "entry:0", conditionScript: "can_start", actionScript: "begin_scene", comment: null, isChild: false, broken: false }, { id: "entry:0:reply:0:0", source: "entry:0", target: "reply:0", conditionScript: "check", actionScript: null, comment: null, isChild: false, broken: false }, { id: "reply:0:entry:0:0", source: "reply:0", target: "entry:0", conditionScript: null, actionScript: null, comment: null, isChild: true, broken: false }],
    roots: ["entry:0"], sharedNodes: ["entry:0"], unreachableNodes: [], cycles: [["entry:0", "reply:0", "entry:0"]], diagnostics: [{ code: "DLG_CYCLE_DETECTED", message: "Cycle", nodeId: "entry:0", linkId: null }], references: [{ resource: { resref: "creature", resourceType: 2027 }, fieldPath: "root.Conversation", source: "C:/module.mod" }], tree: [{ nodeId: "entry:0", kind: "entry", displayText: "Bonjour", repeated: false, cycle: false, children: [{ nodeId: "reply:0", kind: "reply", displayText: "Au revoir", repeated: false, cycle: false, children: [{ nodeId: "entry:0", kind: "entry", displayText: "Bonjour", repeated: false, cycle: true, children: [] }] }] }], raw: { fileType: "DLG ", fileVersion: "V3.2", source: "forge_dialogue.dlg", structCount: 7, fieldCount: 12, root: { index: 0, structType: 4294967295, fields: [
      { label: "EntryList", fieldType: 15, value: { kind: "list", value: [{ index: 1, structType: 0, fields: [
        { label: "Text", fieldType: 12, value: { kind: "localized_string", value: { stringRef: null, values: [{ languageId: 0, text: "Bonjour" }] } } },
        { label: "Speaker", fieldType: 10, value: { kind: "string", value: "NPC" } },
        { label: "RepliesList", fieldType: 15, value: { kind: "list", value: [{ index: 2, structType: 0, fields: [{ label: "Index", fieldType: 4, value: { kind: "dword", value: 0 } }, { label: "Active", fieldType: 11, value: { kind: "res_ref", value: "check" } }] }] } },
      ] }] } },
      { label: "ReplyList", fieldType: 15, value: { kind: "list", value: [{ index: 3, structType: 0, fields: [
        { label: "Text", fieldType: 12, value: { kind: "localized_string", value: { stringRef: null, values: [{ languageId: 0, text: "Au revoir" }] } } },
        { label: "EntriesList", fieldType: 15, value: { kind: "list", value: [{ index: 4, structType: 0, fields: [{ label: "Index", fieldType: 4, value: { kind: "dword", value: 0 } }] }] } },
      ] }] } },
      { label: "StartingList", fieldType: 15, value: { kind: "list", value: [{ index: 5, structType: 0, fields: [{ label: "Index", fieldType: 4, value: { kind: "dword", value: 0 } }, { label: "Active", fieldType: 11, value: { kind: "res_ref", value: "can_start" } }, { label: "Script", fieldType: 11, value: { kind: "res_ref", value: "begin_scene" } }] }] } },
    ] } },
  }),
  editDialogueField: vi.fn(),
  editDialogueStructure: vi.fn(),
  editJournalStructure: vi.fn(),
  editBlueprintStructure: vi.fn().mockResolvedValue({
    workspace: { schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: "ambience", resourceType: 2035 }, sourceSha256: "OLD", outputSha256: "NEW", sizeBytes: 128, relativePath: "resources/ambience.uts" }], deletedResources: [], journalEvents: 4, values: {} },
    document: { fileType: "UTS ", fileVersion: "V3.2", source: "workspace::ambience.uts", structCount: 2, fieldCount: 4, root: { index: 0, structType: 4294967295, fields: [
      { label: "Tag", fieldType: 10, value: { kind: "string", value: "Ambience" } },
      { label: "TemplateResRef", fieldType: 11, value: { kind: "res_ref", value: "ambience" } },
      { label: "Sounds", fieldType: 15, value: { kind: "list", value: [
        { index: 1, structType: 0, fields: [{ label: "Sound", fieldType: 11, value: { kind: "res_ref", value: "as_test" } }] },
      ] } },
    ] } },
  }),
  editFactionStructure: vi.fn().mockResolvedValue({ schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: "repute", resourceType: 2038 }, sourceSha256: "OLD", outputSha256: "NEW", sizeBytes: 256, relativePath: "resources/repute.fac" }], deletedResources: [], journalEvents: 4, values: {} }),
  inspectNarrativeDocuments: vi.fn().mockResolvedValue({
    model: { categories: [], factions: [{ id: 0, name: "PC", parentId: null, global: true }, { id: 1, name: "Hostile", parentId: null, global: true }], reputations: [{ sourceId: 0, targetId: 1, value: 0 }, { sourceId: 1, targetId: 1, value: 100 }], relations: [], diagnostics: [] },
    journal: null,
    factions: {
      resource: { resref: "repute", resourceType: 2038 },
      raw: { fileType: "FAC ", fileVersion: "V3.2", source: "repute.fac", structCount: 5, fieldCount: 12, root: { index: 0, structType: 4294967295, fields: [
        { label: "FactionList", fieldType: 15, value: { kind: "list", value: [
          { index: 1, structType: 0, fields: [{ label: "FactionParentID", fieldType: 4, value: { kind: "dword", value: 4294967295 } }, { label: "FactionName", fieldType: 10, value: { kind: "string", value: "PC" } }, { label: "FactionGlobal", fieldType: 2, value: { kind: "word", value: 1 } }] },
          { index: 2, structType: 1, fields: [{ label: "FactionParentID", fieldType: 4, value: { kind: "dword", value: 4294967295 } }, { label: "FactionName", fieldType: 10, value: { kind: "string", value: "Hostile" } }, { label: "FactionGlobal", fieldType: 2, value: { kind: "word", value: 1 } }] },
        ] } },
        { label: "RepList", fieldType: 15, value: { kind: "list", value: [] } },
      ] } },
    },
  }),
  inspectWorld: vi.fn().mockResolvedValue({
    narrative: {
      categories: [{ tag: "main_quest", name: { stringRef: null, text: "Quête principale" }, priority: 1, xp: 100, entries: [{ id: 1, text: { stringRef: null, text: "Trouver le trésor" }, finalState: true, delay: 0 }], source: "module.jrl" }],
      factions: [{ id: 0, name: "Commoner", parentId: null, global: true }],
      reputations: [{ sourceId: 0, targetId: 0, value: 100 }],
      relations: [], diagnostics: [],
    },
    areas: [{ resref: "startarea", name: { stringRef: null, text: "Zone de départ" }, width: 1, height: 1, tileset: "tno01", tiles: [{ x: 0, y: 0, tileId: 12, orientation: 1, height: 0 }], instances: [{ id: "startarea:Creature List:0", category: "creature", tag: "guard", templateResref: "guard", x: 4, y: 5, z: 0, bearing: 0, appearance: null, transitionDestination: null, transitionFlags: null, loadScreenId: null, geometry: [], spawnPoints: [], inventory: [], sourcePath: "startarea.git::Creature List[0]" }, { id: "startarea:TriggerList:0", category: "trigger", tag: "exit", templateResref: "newtransition", x: 2, y: 2, z: 0, bearing: 0, appearance: null, transitionDestination: "wp_exit", transitionFlags: 2, loadScreenId: 7, geometry: [{ x: 0, y: 0, z: 0 }, { x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }], spawnPoints: [], inventory: [], sourcePath: "startarea.git::TriggerList[0]" }], diagnostics: [], areSource: "startarea.are", gitSource: "startarea.git", gicSource: null }],
    assets: { assets: [{ key: { resref: "guard", resourceType: 2002 }, source: "guard.mdl", format: "mdl_ascii", support: "preview", width: null, height: null, modelNodes: ["trimesh"], animations: ["walk"], textures: ["guard_diff"], referencedModels: [], supermodel: null, meshCount: 1, triangleCount: 12, skinCount: 0, walkmeshCount: 0, glbPreview: true, sha256: "ABC", diagnostics: [] }] },
    scenes: [{ area: "startarea", width: 1, height: 1, tileset: "tno01", objects: [{ id: "tile:0:0", kind: "tile", label: "Tuile 12", x: 5, y: 0, z: 5, rotation: 0, marker: true, sourcePath: "startarea.are" }, { id: "guard", kind: "creature", label: "guard", x: 4, y: 0, z: 5, rotation: 0, marker: false, sourcePath: "startarea.git" }], overlays: [], missingAssets: 0, memoryBudgetBytes: 268435456, diagnostics: [] }],
    graphNodes: [{ id: "area:startarea", kind: "area", label: "Zone de départ", resource: "startarea.are" }, { id: "instance:guard", kind: "creature", label: "guard", resource: "guard" }, { id: "journal:main_quest", kind: "journal", label: "Quête principale", resource: null }],
    graphEdges: [{ id: "contains", source: "area:startarea", target: "instance:guard", kind: "contains", confidence: "certain", evidence: { resource: "startarea.git", fieldPath: "Creature List[0]" } }, { id: "quest", source: "dialogue:test", target: "journal:main_quest", kind: "journal_reference", confidence: "probable", evidence: { resource: "test.dlg", fieldPath: "entry:0" } }],
    diagnostics: [{ code: "RESOURCE_SHADOWED", severity: "info", message: "Version masquée", resource: "guard.mdl", evidence: null }],
    summary: { journalCategories: 1, journalEntries: 1, factions: 1, factionRelations: 1, areas: 1, tiles: 1, instances: 1, transitions: 0, assets: 1, previewableAssets: 1, sceneObjects: 2, graphNodes: 3, graphEdges: 2, diagnostics: 1 },
  }),
  diagnosticReport: vi.fn().mockResolvedValue({ report: {}, json: "{}", html: "<!doctype html>" }),
  createEditWorkspace: vi.fn().mockResolvedValue({ schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 1, cursor: 1, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: "module", resourceType: 2014 }, sourceSha256: "OLD", outputSha256: "NEW", sizeBytes: 128, relativePath: "resources/module.ifo" }], deletedResources: [], journalEvents: 3, values: {} }),
  undoEditCommand: vi.fn().mockResolvedValue({ schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 1, cursor: 0, canUndo: false, canRedo: true, modifiedResources: [], deletedResources: [], journalEvents: 4, values: {} }),
  redoEditCommand: vi.fn(),
  selectModuleOutput: vi.fn().mockResolvedValue("C:/output.mod"),
  buildWorkspaceModule: vi.fn().mockResolvedValue({ outputPath: "C:/output.mod", sha256: "BUILT", sizeBytes: 1024, resourceCount: 3, modifiedResources: 1, deletedResources: 0, sourceIntact: true }),
  buildWorkspaceHak: vi.fn(),
  exportWorkspaceSources: vi.fn(),
  editWorkspaceTwoDa: vi.fn(),
  editWorkspaceTlk: vi.fn(),
  editWorkspaceModuleDependencies: vi.fn().mockResolvedValue({ workspace: { schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [], deletedResources: [], journalEvents: 4, values: {} }, document: {} }),
  listWorkspaceBuildProfiles: vi.fn().mockResolvedValue([]),
  saveWorkspaceBuildProfile: vi.fn().mockResolvedValue([]),
  verifyWorkspaceReproducibleBuild: vi.fn().mockResolvedValue({ profile: {}, firstSha256: "SAME", secondSha256: "SAME", identical: true, resourceCount: 3 }),
  runWorkspaceBuildProfile: vi.fn(),
  inspectGitWorkspace: vi.fn(),
  listWorkspaceLaunchProfiles: vi.fn().mockResolvedValue([]),
  saveWorkspaceLaunchProfile: vi.fn().mockResolvedValue([]),
  launchWorkspaceTestProfile: vi.fn(),
  selectNwnExecutable: vi.fn(),
  planAuroraWorkspaceSync: vi.fn().mockResolvedValue({ schemaVersion: 2, root: "C:/toolset", baselineFound: false, identicalCount: 0, incomingCount: 1, outgoingCount: 0, conflictCount: 0, entries: [{ resource: { resref: "start", resourceType: 2009 }, relativePath: "start.nss", toolsetSha256: "TOOLSET", workspaceSha256: null, baselineToolsetSha256: null, baselineWorkspaceSha256: null, state: "toolset_only" }] }),
  applyAuroraWorkspaceSync: vi.fn().mockResolvedValue({ schemaVersion: 2, root: "C:/toolset", applied: [{ resource: { resref: "start", resourceType: 2009 }, direction: "pull_from_toolset", sha256: "TOOLSET" }], backups: [], plan: { schemaVersion: 2, root: "C:/toolset", baselineFound: true, identicalCount: 1, incomingCount: 0, outgoingCount: 0, conflictCount: 0, entries: [] }, workspace: { schemaVersion: 3, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [], deletedResources: [], journalEvents: 4, values: {}, migrationHistory: [] } }),
  previewAiChangeSet: vi.fn().mockImplementation(async ({ changeSet }) => ({ summary: changeSet.summary, proposalSha256: "A".repeat(64), allValid: true, previews: changeSet.commands.map((command: { kind: string }, index: number) => ({ command, target: `ai:${index}`, current: null, resulting: null, valid: true, diagnostic: null })) })),
  requestAiChangeSet: vi.fn(),
  applyAiChangeSet: vi.fn().mockResolvedValue({ proposalSha256: "A".repeat(64), appliedCommands: 1, workspace: { schemaVersion: 3, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: "start", resourceType: 2009 }, sourceSha256: "OLD", outputSha256: "NEW", sizeBytes: 24, relativePath: "resources/start.nss" }], deletedResources: [], journalEvents: 5, values: {}, migrationHistory: [] } }),
  deployWorkspaceDevelopment: vi.fn().mockResolvedValue({ workspaceId: "workspace-1", developmentPath: "C:/NWN/development", files: [{ name: "module.ifo", sha256: "NEW", sizeBytes: 128 }] }),
  cleanWorkspaceDevelopment: vi.fn(),
  createWorkspaceArea: vi.fn().mockResolvedValue({ workspace: { schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: "newarea", resourceType: 2012 }, sourceSha256: null, outputSha256: "ARE", sizeBytes: 128, relativePath: "resources/newarea.are" }, { resource: { resref: "newarea", resourceType: 2023 }, sourceSha256: null, outputSha256: "GIT", sizeBytes: 128, relativePath: "resources/newarea.git" }, { resource: { resref: "newarea", resourceType: 2046 }, sourceSha256: null, outputSha256: "GIC", sizeBytes: 128, relativePath: "resources/newarea.gic" }], deletedResources: [], journalEvents: 5, values: {} }, area: { resref: "newarea", name: { stringRef: null, text: "Nouvelle zone" }, width: 1, height: 1, tileset: "tno01", tiles: [{ x: 0, y: 0, tileId: 0, orientation: 0, height: 0 }], instances: [], diagnostics: [], areSource: "workspace::newarea.are", gitSource: "workspace::newarea.git", gicSource: "workspace::newarea.gic" } }),
  getMapAuthoringContext: vi.fn().mockResolvedValue({ limits: { maxWidth: 32, maxHeight: 32, maxTiles: 1024, maxResrefLength: 16, maxDensityRules: 16, maxBlueprintsPerRule: 128, maxPlacements: 2048 }, availableTilesets: ["tno01"], selectedTileset: { resref: "tno01", sha256: "C".repeat(64), tileCount: 128, tileIds: Array.from({length:128},(_,index)=>index) }, blueprintCounts: { placeable: 2, creature: 1 } }),
  previewMapGeneration: vi.fn().mockImplementation(async ({ spec }) => ({ planSha256: "B".repeat(64), spec, tiles: Array.from({length:spec.width*spec.height},(_,index)=>({x:index%spec.width,y:Math.floor(index/spec.width),tileId:spec.baseTileId,orientation:index%4})), placements: [{category:"placeable",templateResref:"plc_table",tag:"plc_1",x:15,y:15,z:0,bearing:0,linkedTo:null}], metrics: {totalTiles:spec.width*spec.height,buildableTiles:80,reservedTiles:20,placementCount:1,occupiedPercent:1}, compatibility: { tilesetResolved: true, tilesetSha256: "C".repeat(64), resolvedTileCount: 128, selectedTileIds: [spec.baseTileId], tileIdsVerified: true, edgeCompatibilityVerified: false }, warnings: [] })),
  draftMapWithAi: vi.fn().mockImplementation(async ({ currentSpec }) => ({ endpointOrigin: "https://api.openai.com", model: "remote-map-model", sharedBlueprintCount: 3, plan: { planSha256: "D".repeat(64), spec: { ...currentSpec, name: "Carte proposée par IA" }, tiles: Array.from({length:currentSpec.width*currentSpec.height},(_,index)=>({x:index%currentSpec.width,y:Math.floor(index/currentSpec.width),tileId:currentSpec.baseTileId,orientation:0})), placements: [], metrics: {totalTiles:currentSpec.width*currentSpec.height,buildableTiles:80,reservedTiles:20,placementCount:0,occupiedPercent:0}, compatibility: { tilesetResolved: true, tilesetSha256: "C".repeat(64), resolvedTileCount: 128, selectedTileIds: [currentSpec.baseTileId], tileIdsVerified: true, edgeCompatibilityVerified: false }, warnings: [] } })),
  applyMapGeneration: vi.fn().mockImplementation(async ({ spec }) => ({ workspace: { schemaVersion: 3, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: spec.resref, resourceType: 2012 }, sourceSha256: null, outputSha256: "MAP", sizeBytes: 512, relativePath: `resources/${spec.resref}.are` }], deletedResources: [], journalEvents: 5, values: {} }, plan: { planSha256: "B".repeat(64), spec, tiles: [], placements: [], metrics: {totalTiles:120,buildableTiles:80,reservedTiles:20,placementCount:1,occupiedPercent:1}, warnings: [] }, area: { resref: spec.resref, name: { stringRef: null, text: spec.name }, width: spec.width, height: spec.height, tileset: spec.tileset, tiles: Array.from({length:spec.width*spec.height},(_,index)=>({x:index%spec.width,y:Math.floor(index/spec.width),tileId:spec.baseTileId,orientation:index%4})), instances: [{id:`${spec.resref}:Placeable List:0`,category:"placeable",tag:"plc_1",templateResref:"plc_table",x:15,y:15,z:0,bearing:0,appearance:null,transitionDestination:null,transitionFlags:null,loadScreenId:null,geometry:[],spawnPoints:[],inventory:[],sourcePath:`workspace::${spec.resref}.git`}], diagnostics: [], areSource:`workspace::${spec.resref}.are`,gitSource:`workspace::${spec.resref}.git`,gicSource:`workspace::${spec.resref}.gic`} })),
  listWorkspaceCreatedAreas: vi.fn().mockResolvedValue([]),
  deleteWorkspaceArea: vi.fn(),
  addWorkspaceAreaInstance: vi.fn(),
  removeWorkspaceAreaInstance: vi.fn(),
  moveAreaInstance: vi.fn(),
  setAreaTile: vi.fn(),
  editAreaStructure: vi.fn().mockResolvedValue({ schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [], deletedResources: [], journalEvents: 4, values: {} }),
  inspectWorkspaceArea: vi.fn().mockRejectedValue(new Error("fixture uses indexed area")),
  validateWalkmeshDraft: vi.fn().mockResolvedValue({ valid: true, diagnostics: [] }),
  transformWalkmeshDraft: vi.fn().mockImplementation((draft: { vertices: Array<[number, number, number]>; faces: Array<[number, number, number]>; surfaceIds: number[]; variants: unknown[]; hooks: unknown[] }, operation: { kind: string; faceIndex?: number }) => {
    const next = structuredClone(draft);
    if (operation.kind === "split_face" && operation.faceIndex !== undefined) {
      const face = next.faces[operation.faceIndex];
      const [a, b, c] = face.map((index) => next.vertices[index]);
      const center = next.vertices.length;
      next.vertices.push([(a[0] + b[0] + c[0]) / 3, (a[1] + b[1] + c[1]) / 3, (a[2] + b[2] + c[2]) / 3]);
      next.faces[operation.faceIndex] = [face[0], face[1], center];
      next.faces.push([face[1], face[2], center], [face[2], face[0], center]);
      next.surfaceIds.push(next.surfaceIds[operation.faceIndex] ?? 0, next.surfaceIds[operation.faceIndex] ?? 0);
    }
    return Promise.resolve({ draft: next, validation: { valid: true, diagnostics: [] } });
  }),
  inspectWorkspaceWalkmesh: vi.fn(),
  saveWorkspaceWalkmesh: vi.fn().mockImplementation((request: { resref: string; kind: "wok" | "pwk" | "dwk"; draft: unknown }) => Promise.resolve({
    workspace: { schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: request.resref, resourceType: 2016 }, sourceSha256: null, outputSha256: "WALK", sizeBytes: 256, relativePath: `resources/${request.resref}.${request.kind}` }], deletedResources: [], journalEvents: 4, values: {} },
    document: { resref: request.resref, kind: request.kind, sourceFormat: "ascii", draft: request.draft, sourceSha256: "WALK" },
  })),
  listAreaMigrationCandidates: vi.fn().mockResolvedValue([{ resref: "startarea", name: "Zone de depart", width: 1, height: 1, tileCount: 1, instanceCount: 2, sourceDiagnosticCount: 0 }]),
  previewAreaMigration: vi.fn().mockResolvedValue({ schemaVersion: "area-migration-bundle@1.0.0", areaResref: "startarea", areaName: "Zone de depart", suggestedDirectoryName: "startarea.area-migration-v1", ready: true, complete: true, counts: { tiles: 1, instances: 2, uniqueModels: 2, textures: 3, preservedNavigation: 1, missingItems: 0, fallbacks: 1, diagnostics: 1, warnings: 1, errors: 0, byStatus: {} }, diagnostics: [], classification: "local_only_proprietary", redistribution: "not_redistributable_without_separate_rights", navigationStatus: "preserved-not-converted" }),
  getAreaMigrationJob: vi.fn().mockResolvedValue(null),
  startAreaMigrationExport: vi.fn(),
  listAssetExportCandidates: vi.fn().mockResolvedValue([{ resref: "hero", format: "mdl_binary", source: "module.mod", exportable: true, declaredAnimationCount: 1, declaredAnimations: ["walk"], meshCount: 2, triangleCount: 12, skinCount: 1, textureCount: 1, diagnosticCount: 0 }]),
  previewAssetExport: vi.fn().mockResolvedValue({ schemaVersion: "opennever-asset-export@1.0.0", resref: "hero", mode: "animated", ready: true, suggestedDirectoryName: "hero.asset-export-v1", nodeCount: 4, meshCount: 2, primitiveCount: 2, skinCount: 1, animationCount: 1, animations: [{ name: "walk", lengthSeconds: 1, transitionSeconds: 0.2, rootNode: "root", trackCount: 2, eventCount: 0, exported: true }], textures: [{ resref: "hero_diff", resourceType: 2033, outputPath: null, status: "planned", diagnostic: null }], warnings: [], classification: "local_only_proprietary", redistribution: "not_redistributable_without_separate_rights" }),
  exportAssetBundle: vi.fn(),
  listDialogueExportCandidates: vi.fn().mockResolvedValue([{ resref: "guard", nodeCount: 2, linkCount: 1, cycleCount: 0, diagnosticCount: 0, preview: "Bienvenue" }]),
  previewDialogueExport: vi.fn().mockResolvedValue({ schemaVersion: "opennever-dialogue-export@1.0.0", resref: "guard", revision: "analysis", ready: true, suggestedDirectoryName: "guard.dialogue-export-v1", sourceResourceSha256: "d".repeat(64), nodeCount: 2, entryCount: 1, replyCount: 1, linkCount: 1, rootCount: 1, sharedNodeCount: 0, unreachableNodeCount: 0, cycleCount: 0, brokenLinkCount: 0, diagnosticCount: 0, referenceCount: 1, scripts: ["open_gate"], transcriptPreview: ["- **Gardien** : Bienvenue `entry:0`"], warnings: [], classification: "local_only_proprietary", redistribution: "not_redistributable_without_separate_rights" }),
  exportDialogueBundle: vi.fn(),
  selectDirectory: vi.fn(),
  selectModule: vi.fn(),
  normalizeAppError: vi.fn((error) => error),
}));

function renderApp() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  );
}

function largeDialogueGraph(): DialogueGraph {
  const nodes = Array.from({ length: 500 }, (_, index) => [
    { id: `entry:${index}`, kind: "entry" as const, index, text: null, displayText: `Réplique PNJ ${index}`, speaker: "Narrateur", comment: null, animation: null, animationLoop: null, sound: null, quest: null, actionScript: null },
    { id: `reply:${index}`, kind: "reply" as const, index, text: null, displayText: `Réponse joueur ${index}`, speaker: null, comment: null, animation: null, animationLoop: null, sound: null, quest: null, actionScript: null },
  ]).flat();
  const links = Array.from({ length: 500 }, (_, index) => [
    { id: `entry:${index}:reply:${index}:0`, source: `entry:${index}`, target: `reply:${index}`, conditionScript: null, actionScript: null, comment: null, isChild: false, broken: false },
    { id: `reply:${index}:entry:${(index + 1) % 500}:0`, source: `reply:${index}`, target: `entry:${(index + 1) % 500}`, conditionScript: null, actionScript: null, comment: null, isChild: false, broken: false },
  ]).flat();
  return {
    key: { resref: "forge_dialogue", resourceType: 2029 }, source: "synthetic::large.dlg", nodes, links,
    roots: ["entry:0"], sharedNodes: [], unreachableNodes: [], cycles: [], diagnostics: [], references: [], tree: [],
    raw: { fileType: "DLG ", fileVersion: "V3.2", source: "synthetic::large.dlg", structCount: 1, fieldCount: 3, root: { index: 0, structType: 4294967295, fields: [
      { label: "EntryList", fieldType: 15, value: { kind: "list", value: [] } },
      { label: "ReplyList", fieldType: 15, value: { kind: "list", value: [] } },
      { label: "StartingList", fieldType: 15, value: { kind: "list", value: [] } },
    ] } },
  };
}

describe("OpenNever Forge shell", () => {
  it("hides Aurora tile-fade and black technical occluders without hiding textured floors", () => {
    expect(isTileOccluder("tile", { nwnTileFade: 1, nwnTextures: ["tin01_roof"] })).toBe(true);
    expect(isTileOccluder("tile", { nwnTileFade: 4, nwnTextures: ["tin01_wall"] })).toBe(true);
    expect(isTileOccluder("tile", { nwnTileFade: 0, nwnTextures: ["tin01_black"] })).toBe(true);
    expect(isTileOccluder("tile", { nwnTileFade: 0, nwnTextures: ["tin01_floor"] })).toBe(false);
    expect(isTileOccluder("placeable", { nwnTextures: ["plc_black"] })).toBe(false);
  });

  it("keeps the technical ground below the textured tile floors", () => {
    expect(sceneGroundElevation([{ kind: "tile", y: 0 }])).toBeCloseTo(-0.05);
    expect(sceneGroundElevation([{ kind: "tile", y: 2 }, { kind: "tile", y: 1 }])).toBeCloseTo(0.95);
    expect(sceneGroundElevation([{ kind: "placeable", y: -3 }])).toBeCloseTo(-0.05);
  });

  it("centers the technical ground under the NWN tile grid", () => {
    expect(sceneGroundPosition(8, 6, [{ kind: "tile", y: 0 }])).toEqual({
      x: 40,
      y: -0.05,
      z: 30,
    });
  });

  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    useWorkbenchStore.setState({ activeExplorerItem: "module", lastContentView: "module", agentObjectiveDraft: "" });
    useUiStore.setState({ explorerOpen: true, inspectorOpen: true, diagnosticsOpen: false });
  });

  it("keeps a running analysis visibly active instead of showing 100 percent", () => {
    render(
      <JobProgress
        job={{
          id: "running-job",
          kind: "module_analysis",
          state: "running",
          sourcePath: "C:/module.mod",
          progress: { bytesRead: 512, totalBytes: 512, percent: 100, phase: "persisting" },
        }}
      />,
    );

    expect(screen.getByRole("status")).toHaveAttribute("aria-busy", "true");
    expect(screen.getByText("Finalisation de l’index")).toBeInTheDocument();
    expect(screen.getByText(/le travail continue/)).toBeInTheDocument();
    expect(screen.getByText(/ACTIF · 0 s/)).toBeInTheDocument();
    expect(screen.queryByText("100.0 %")).not.toBeInTheDocument();
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
  });

  it("stops animating the progress bar once analysis is complete", () => {
    const { container } = render(
      <JobProgress
        job={{
          id: "completed-job",
          kind: "module_analysis",
          state: "completed",
          sourcePath: "C:/module.mod",
          progress: { bytesRead: 512, totalBytes: 512, percent: 100, phase: "world" },
        }}
      />,
    );

    expect(screen.getByRole("status")).toHaveAttribute("aria-busy", "false");
    expect(screen.getByText("Analyse terminée")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
    expect(container.querySelector(".progress-fill")).toHaveClass("is-static");
    expect(container.querySelector(".progress-fill")).not.toHaveClass("is-animated");
  });

  it("renders the explorer, workbench, inspector and diagnostics", async () => {
    renderApp();

    expect(screen.getByLabelText("Explorateur du module")).toBeInTheDocument();
    expect(screen.getByLabelText("Zone de travail")).toBeInTheDocument();
    expect(screen.getByLabelText("Inspecteur")).toBeInTheDocument();
    expect(screen.getByLabelText("Diagnostics")).toBeInTheDocument();
    expect(await screen.findByText("Cœur Rust · v0.1.0-test")).toBeInTheDocument();
  });

  it("exposes the area exporter after analysis", async () => {
    renderApp();
    expect(screen.getByRole("button", { name: "Exporter une carte" })).toBeInTheDocument();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(screen.getByRole("button", { name: "Exporter une carte" }));
    expect(await screen.findByRole("region", { name: "Migration de zone" })).toBeInTheDocument();
    expect(await screen.findByText("BUNDLE DE MIGRATION V1")).toBeInTheDocument();
  });

  it("places the asset exporter after the map exporter", async () => {
    renderApp();
    const mapExporter = screen.getByRole("button", { name: "Exporter une carte" });
    const assetExporter = screen.getByRole("button", { name: "Exporter des assets" });
    expect(mapExporter.compareDocumentPosition(assetExporter) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(assetExporter);
    expect(await screen.findByRole("region", { name: "Export d’assets" })).toBeInTheDocument();
    expect(await screen.findByText("EXPORT ASSET V1")).toBeInTheDocument();
    expect(await screen.findByText("Asset animé")).toBeInTheDocument();
  });

  it("places the dialogue exporter after the asset exporter", async () => {
    renderApp();
    const assetExporter = screen.getByRole("button", { name: "Exporter des assets" });
    const dialogueExporter = screen.getByRole("button", { name: "Exporter des dialogues" });
    expect(assetExporter.compareDocumentPosition(dialogueExporter) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(dialogueExporter);
    expect(await screen.findByRole("region", { name: "Export de dialogues" })).toBeInTheDocument();
    expect(await screen.findByText("DIALOGUE EXPORT V1")).toBeInTheDocument();
    expect(await screen.findByText("Version analysée")).toBeInTheDocument();
  });

  it("opens contextual help and embeds the complete manual", async () => {
    renderApp();
    fireEvent.click(screen.getByRole("button", { name: "Guide et manuel" }));
    expect(await screen.findByRole("region", { name: "Aide utilisateur OpenNever Forge" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Aide et manuel" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Analyser un module existant" })).toBeInTheDocument();
    expect(screen.getByText("Interface : Analyser la copie")).toBeInTheDocument();
    expect(screen.getAllByText(/Résultat attendu/).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: /Modifier et compiler un script NWScript/ }));
    expect(screen.getByRole("heading", { name: "Modifier et compiler un script NWScript" })).toBeInTheDocument();
    expect(screen.getByText("Interface : Enregistrer NSS")).toBeInTheDocument();
    expect(screen.getByText("Interface : Compiler NSS → NCS")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("textbox", { name: "Rechercher dans les tutoriels" }), { target: { value: "401" } });
    expect(screen.getByRole("button", { name: /Configurer et suivre l’IA/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Créer une zone et placer une instance/ })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Manuel complet" }));
    const manual = screen.getByTitle("Manuel complet OpenNever Forge");
    expect(manual).toHaveAttribute("srcdoc", expect.stringContaining("Documentation utilisateur + référence technique"));
    expect(manual).toHaveAttribute("srcdoc", expect.stringContaining("0. Tutoriel de bout en bout"));
    expect(manual).toHaveAttribute("srcdoc", expect.stringContaining("Tester la communication avec le modèle"));
  });

  it("lets the creator fold peripheral panels and diagnostics", () => {
    renderApp();
    fireEvent.click(screen.getByRole("button", { name: "Réduire l'explorateur" }));
    expect(screen.getByRole("button", { name: "Afficher l'explorateur" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Réduire l'inspecteur" }));
    expect(screen.getByRole("button", { name: "Afficher l'inspecteur" })).toBeInTheDocument();
    const diagnostics = screen.getByRole("button", { name: "Diagnostics" });
    expect(diagnostics).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(diagnostics);
    expect(diagnostics).toHaveAttribute("aria-expanded", "true");
  });

  it("starts the hash job only after a module path is provided", async () => {
    renderApp();
    const action = screen.getByRole("button", { name: "Analyser la copie" });
    expect(action).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), {
      target: { value: "C:/module.mod" },
    });
    fireEvent.click(action);

    await waitFor(() =>
      expect(startModuleAnalysis).toHaveBeenCalledWith({
        modulePath: "C:/module.mod",
        gameInstallPath: null,
        userDataPath: null,
      }),
    );
  });

  it("opens a standalone ARE directly in the area workspace without enabling editing", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), {
      target: { value: "C:/areas/lonely.are" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));

    await waitFor(() =>
      expect(startModuleAnalysis).toHaveBeenCalledWith({
        modulePath: "C:/areas/lonely.are",
        gameInstallPath: null,
        userDataPath: null,
      }),
    );
    expect(await screen.findByText("ZONES DU MODULE")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Table de campagne/ }));
    expect(await screen.findByText("Carte autonome en lecture seule")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Créer l’espace d’édition" })).not.toBeInTheDocument();
  });

  it("restores and automatically saves the three project paths", async () => {
    localStorage.setItem(
      PROJECT_PREFERENCES_STORAGE_KEY,
      JSON.stringify({
        version: 1,
        project: {
          modulePath: "C:/campaign/remembered.mod",
          gameInstallPath: "C:/Games/Neverwinter Nights",
          userDataPath: "C:/Users/Creator/Documents/Neverwinter Nights",
        },
      }),
    );

    renderApp();

    expect(screen.getByPlaceholderText("Sélectionner un fichier .mod")).toHaveValue(
      "C:/campaign/remembered.mod",
    );
    expect(screen.getByPlaceholderText("Sélectionner le dossier d'installation")).toHaveValue(
      "C:/Games/Neverwinter Nights",
    );
    expect(screen.getByPlaceholderText("Sélectionner le dossier utilisateur")).toHaveValue(
      "C:/Users/Creator/Documents/Neverwinter Nights",
    );

    fireEvent.change(screen.getByPlaceholderText("Sélectionner le dossier utilisateur"), {
      target: { value: "D:/NWN/User" },
    });

    await waitFor(() =>
      expect(JSON.parse(localStorage.getItem(PROJECT_PREFERENCES_STORAGE_KEY) ?? "{}")).toEqual({
        version: 1,
        project: {
          modulePath: "C:/campaign/remembered.mod",
          gameInstallPath: "C:/Games/Neverwinter Nights",
          userDataPath: "D:/NWN/User",
        },
      }),
    );
  });

  it("resumes the cached analysis and workspace without starting a new analysis", async () => {
    localStorage.setItem(
      PROJECT_PREFERENCES_STORAGE_KEY,
      JSON.stringify({
        version: 1,
        project: {
          modulePath: "C:/campaign/remembered.mod",
          gameInstallPath: "C:/Games/Neverwinter Nights",
          userDataPath: "C:/Users/Creator/Documents/Neverwinter Nights",
        },
      }),
    );
    vi.mocked(restoreModuleSession).mockResolvedValueOnce({
      job: {
        id: "restored-job",
        kind: "module_analysis",
        state: "completed",
        sourcePath: "C:/campaign/remembered.mod",
        progress: { bytesRead: 512, totalBytes: 512, percent: 100, phase: "persisting" },
      },
      workspace: {
        schemaVersion: 3,
        workspaceId: "workspace-1",
        root: "C:/cache/workspace-1",
        source: { path: "C:/campaign/remembered.mod", sha256: "ABC123", sizeBytes: 512 },
        sourceIntact: true,
        commandCount: 1,
        cursor: 1,
        canUndo: true,
        canRedo: false,
        modifiedResources: [],
        deletedResources: [],
        journalEvents: 3,
        values: {},
      },
    });

    renderApp();

    await waitFor(() => expect(restoreModuleSession).toHaveBeenCalledWith({
      modulePath: "C:/campaign/remembered.mod",
      gameInstallPath: "C:/Games/Neverwinter Nights",
      userDataPath: "C:/Users/Creator/Documents/Neverwinter Nights",
    }));
    expect(await screen.findByText("Session restaurée · travail repris automatiquement")).toBeInTheDocument();
    expect(screen.getByText("Révision 1/1")).toBeInTheDocument();
    expect(startModuleAnalysis).not.toHaveBeenCalled();
  });

  it("remembers the last open workbench page", async () => {
    const first = renderApp();
    fireEvent.click(screen.getByRole("button", { name: "Guide et manuel" }));
    await waitFor(() => expect(localStorage.getItem(LAST_EXPLORER_ITEM_STORAGE_KEY)).toBe("help"));
    first.unmount();
    useWorkbenchStore.setState({ activeExplorerItem: "module" });

    renderApp();

    expect(await screen.findByRole("heading", { name: "Aide et manuel" })).toBeInTheDocument();
  });

  it("renders and filters the resolved resource catalog returned by Rust", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), {
      target: { value: "C:/module.mod" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));

    expect(await screen.findByRole("heading", { name: "Forge Test" })).toBeInTheDocument();
    expect(screen.getByText("1 ressource(s) dans cette catégorie.")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Dépendances du module" })).toBeInTheDocument();
    expect(screen.getByText("Introuvable")).toBeInTheDocument();
    expect(screen.getByText(/SHA-256 1234567890ABCDEF/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ressources (3)" }));
    expect(await screen.findByRole("region", { name: "Atelier des ressources" })).toBeInTheDocument();
    expect(screen.getByRole("list", { name: "Ressources résolues" })).toBeInTheDocument();
    expect(screen.getByText("3 ressource(s) dans cette catégorie.")).toBeInTheDocument();
    expect((await screen.findAllByText("module")).length).toBeGreaterThan(0);
    expect(await screen.findByText("start")).toBeInTheDocument();
    expect(screen.getByText("RESOURCE_SHADOWED")).toBeInTheDocument();
    expect(screen.getByText("HAK_NOT_FOUND")).toBeInTheDocument();
    expect(screen.getByText("DEPENDENCY_CONTENT_CHANGED")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Rechercher une ressource"), {
      target: { value: "module" },
    });
    expect((await screen.findAllByText("module")).length).toBeGreaterThan(0);
    await waitFor(() => expect(screen.queryByText("start")).not.toBeInTheDocument());
  });

  it("opens a script in the read-only source and NCS technical views", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Scripts (1)" }));
    expect(await screen.findByRole("region", { name: "Explorateur NWScript" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Scripts NWScript" })).toBeInTheDocument();
    expect(screen.getByLabelText("Rechercher un script")).toBeInTheDocument();
    expect(await screen.findByTestId("monaco-readonly")).toHaveTextContent("void main()");
    expect(screen.getByText(/module\.\#2014/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Bytecode NCS" }));
    expect(await screen.findByText("NCS V1.0")).toBeInTheDocument();
  });

  it("shows dialogue lines first and keeps the full graph and raw GFF as advanced views", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Dialogues (1)" }));
    expect(await screen.findByRole("region", { name: "Explorateur de dialogues" })).toBeInTheDocument();
    expect(inspectDialogue).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Choisissez un dialogue" })).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Ouvrir le dialogue forge_dialogue" }));
    expect(await screen.findByRole("button", { name: "Lignes" })).toHaveClass("active");
    expect(screen.getByRole("article", { name: "Ligne entry:0" })).toHaveTextContent("Bonjour");
    expect(screen.getByRole("article", { name: "Ligne entry:0" })).toHaveTextContent("Déclencheur · check");
    expect(screen.getByRole("article", { name: "Ligne entry:0" })).toHaveTextContent("Condition · can_start");
    expect(screen.queryByRole("article", { name: "Ligne reply:0" })).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Rechercher une ligne"), { target: { value: "au revoir" } });
    fireEvent.click(screen.getByRole("button", { name: "Ouvrir la ligne reply:0" }));
    expect(screen.queryByRole("article", { name: "Ligne entry:0" })).not.toBeInTheDocument();
    expect(screen.getByRole("article", { name: "Ligne reply:0" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Graphe (avancé)" }));
    expect(await screen.findByTestId("dialogue-flow")).toHaveTextContent("2 nodes · 2 edges");
    expect(screen.getByText(/creature\.\#2027/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "GFF (avancé)" }));
    expect(await screen.findByText(/fileType/)).toBeInTheDocument();
  });

  it("edits dialogue text and associates trigger scripts directly from a line", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    fireEvent.click(await screen.findByRole("button", { name: "Dialogues (1)" }));
    fireEvent.click(await screen.findByRole("button", { name: "Ouvrir le dialogue forge_dialogue" }));

    const line = await screen.findByRole("article", { name: "Ligne entry:0" });
    const text = within(line).getByRole("textbox", { name: "Langue/genre 0" });
    fireEvent.change(text, { target: { value: "Bienvenue, voyageur." } });
    const textEditor = text.closest(".localized-dialogue-field");
    expect(textEditor).not.toBeNull();
    fireEvent.click(within(textEditor as HTMLElement).getByRole("button", { name: "Appliquer" }));
    await waitFor(() => expect(editDialogueField).toHaveBeenCalledWith(expect.objectContaining({
      jobId: "job-1",
      workspaceId: "workspace-1",
      resref: "forge_dialogue",
      path: "/EntryList/0/Text",
      after: { kind: "localized_string", value: { stringRef: null, values: [{ languageId: 0, text: "Bienvenue, voyageur." }] } },
    })));

    const trigger = within(line).getByRole("textbox", { name: "Déclencheur de entry:0:reply:0:0" });
    fireEvent.change(trigger, { target: { value: "has_key" } });
    const triggerEditor = trigger.closest(".dialogue-trigger-editor");
    expect(triggerEditor).not.toBeNull();
    fireEvent.click(within(triggerEditor as HTMLElement).getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(editDialogueStructure).toHaveBeenCalledWith(expect.objectContaining({
      action: {
        kind: "set_link_scripts",
        source: { kind: "entry", index: 0 },
        position: 0,
        conditionScript: "has_key",
        actionScript: null,
      },
    })));

    expect(within(line).queryByRole("textbox", { name: /Rechercher une cible/ })).not.toBeInTheDocument();
    fireEvent.click(within(line).getByRole("button", { name: /Associer une réponse joueur/ }));
    expect(within(line).getByRole("textbox", { name: "Rechercher une cible pour Associer une réponse joueur" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "+ Réplique PNJ" }));
    await waitFor(() => expect(editDialogueStructure).toHaveBeenCalledWith(expect.objectContaining({ action: { kind: "add_node", nodeKind: "entry" } })));
    fireEvent.click(within(line).getByRole("button", { name: "Supprimer la ligne" }));
    fireEvent.click(within(line).getByRole("button", { name: "Confirmer" }));
    await waitFor(() => expect(editDialogueStructure).toHaveBeenCalledWith(expect.objectContaining({ action: { kind: "remove_node", node: { kind: "entry", index: 0 } } })));
  });

  it("keeps a 1,000-line dialogue bounded to one editor and one navigator page", async () => {
    vi.mocked(inspectDialogue).mockResolvedValueOnce(largeDialogueGraph());
    const { container } = renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Dialogues (1)" }));
    fireEvent.click(await screen.findByRole("button", { name: "Ouvrir le dialogue forge_dialogue" }));

    expect(await screen.findByRole("article", { name: "Ligne entry:0" })).toBeInTheDocument();
    expect(container.querySelectorAll(".dialogue-line-card")).toHaveLength(1);
    expect(container.querySelectorAll(".dialogue-node-list > button")).toHaveLength(60);
    expect(container.querySelectorAll(".dialogue-target-picker")).toHaveLength(0);
    expect(screen.getByText(/1.000 lignes/)).toBeInTheDocument();
  });

  it("navigates the narrative, 2D map, assets and targeted global graph", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    expect(await screen.findByRole("heading", { name: "Forge Test" }, { timeout: 10_000 })).toBeInTheDocument();
    fireEvent.click(await screen.findByRole("button", { name: "Journal et quêtes (1)" }));
    expect(await screen.findByText("Quête principale")).toBeInTheDocument();
    expect(screen.getByText("État final")).toBeInTheDocument();
    expect(screen.getByLabelText("Rechercher dans Journal et quêtes")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Factions (1)" }));
    expect((await screen.findAllByText("PC")).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Zones (1)" }));
    expect((await screen.findAllByText("Zone de départ")).length).toBeGreaterThan(0);
    expect(screen.getByLabelText("creature guard")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Assets (1)" }));
    expect(await screen.findByRole("navigation", { name: "Filtres des assets" })).toBeInTheDocument();
    expect(await screen.findByText("mdl_ascii · preview")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Références (2)" }));
    expect(await screen.findByRole("heading", { name: "Graphe global et validation" })).toBeInTheDocument();
    expect(screen.getByText("Relations directes · 1")).toBeInTheDocument();
    expect(screen.getByText("contains · certain")).toBeInTheDocument();
  });

  it("previews and atomically creates a deterministic map, then prepares its Agent brief", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    expect(await screen.findByRole("heading", { name: "Préparer, valider et tester" })).toBeInTheDocument();
    fireEvent.click(within(screen.getByRole("navigation", { name: "Menu principal" })).getByRole("button", { name: "Construire" }));

    expect(await screen.findByRole("heading", { name: "Vibecoder une carte complète" })).toBeInTheDocument();
    expect(screen.getByLabelText("Densité Plaçables")).toBeInTheDocument();
    expect(screen.getByLabelText("Zone de l’atlas")).toBeInTheDocument();
    expect(await screen.findByText(/SET tno01 résolu/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Modèle IA de carte"), { target: { value: "remote-map-model" } });
    fireEvent.click(screen.getByRole("button", { name: "Générer le plan avec l’IA" }));
    await waitFor(() => expect(draftMapWithAi).toHaveBeenCalledWith(expect.objectContaining({ jobId: "job-1", includeBlueprintResrefs: true })));
    expect(await screen.findByText(/Plan IA reçu de remote-map-model/)).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Blueprints Plaçables"), { target: { value: "plc_table" } });
    fireEvent.click(screen.getByRole("button", { name: "Prévisualiser" }));
    await waitFor(() => expect(previewMapGeneration).toHaveBeenCalledWith({ jobId: "job-1", spec: expect.objectContaining({ resref: "vibe_map", seed: 20260811 }) }));
    expect(await screen.findByText(/Plan BBBBBBBBBBBB prêt/)).toBeInTheDocument();
    expect(screen.getAllByRole("img", { name: /Carte de repérage Carte proposée par IA/ }).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: "Créer ARE/GIT/GIC" }));
    await waitFor(() => expect(applyMapGeneration).toHaveBeenCalledWith(expect.objectContaining({ workspaceId: "workspace-1", expectedPlanSha256: "B".repeat(64) })));
    expect(await screen.findByText(/créée et relue depuis l’overlay/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Confier le brief à l’Agent" }));
    expect(await screen.findByRole("heading", { name: "Agent Studio" })).toBeInTheDocument();
    expect((screen.getByLabelText("Objectif de l’agent") as HTMLTextAreaElement).value).toContain("map.generate");
  });

  it("moves an area instance only after an intentional pointer drag", async () => {
    const movedWorkspace = await createEditWorkspace({ jobId: "fixture" });
    vi.mocked(moveAreaInstance).mockResolvedValue(movedWorkspace);
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    fireEvent.click(await screen.findByRole("button", { name: "Zones (1)" }));
    const marker = await screen.findByLabelText("creature guard");
    Object.assign(marker, { setPointerCapture: vi.fn() });
    vi.spyOn(marker.parentElement as HTMLElement, "getBoundingClientRect").mockReturnValue({ x: 0, y: 0, left: 0, top: 0, right: 100, bottom: 100, width: 100, height: 100, toJSON: () => ({}) });

    fireEvent.pointerDown(marker, { pointerId: 1, clientX: 40, clientY: 50 });
    fireEvent.pointerUp(marker, { pointerId: 1, clientX: 70, clientY: 30 });

    await waitFor(() => expect(moveAreaInstance).toHaveBeenCalledWith({
      jobId: "job-1",
      workspaceId: "workspace-1",
      area: "startarea",
      instanceId: "startarea:Creature List:0",
      before: { x: 4, y: 5, z: 0, bearing: 0 },
      after: { x: 7, y: 7, z: 0, bearing: 0 },
    }));
  });

  it("opens an edit workspace and builds a separate module", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    await waitFor(() => expect(createEditWorkspace).toHaveBeenCalledWith({ jobId: "job-1" }));
    fireEvent.click(await screen.findByRole("button", { name: "Construire un MOD" }));
    await waitFor(() => expect(selectModuleOutput).toHaveBeenCalled());
    expect(buildWorkspaceModule).toHaveBeenCalledWith({ workspaceId: "workspace-1", outputPath: "C:/output.mod" });
    expect(await screen.findByText(/3 ressources sauvegardées/)).toBeInTheDocument();
  });

  it("stages HAK and TLK declarations through the dependency manager", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    const hakEditor = await screen.findByLabelText("HAK, dans l’ordre de priorité");
    fireEvent.change(hakEditor, { target: { value: "base.hak\npatch.hak" } });
    fireEvent.change(screen.getByLabelText("TLK personnalisé", { selector: "input" }), { target: { value: "custom.tlk" } });
    fireEvent.click(screen.getByRole("button", { name: "Appliquer à module.ifo" }));
    await waitFor(() => expect(editWorkspaceModuleDependencies).toHaveBeenCalledWith({
      jobId: "job-1",
      workspaceId: "workspace-1",
      hakFiles: ["base.hak", "patch.hak"],
      customTlk: "custom.tlk",
    }));
  });

  it("persists a reproducible build profile in the workspace", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    expect(await screen.findByRole("heading", { name: "Profils de build et Git" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Sauvegarder" }));
    await waitFor(() => expect(saveWorkspaceBuildProfile).toHaveBeenCalledWith({
      workspaceId: "workspace-1",
      profile: {
        name: "Test local",
        outputName: "opennever-test.mod",
        blockOnWarnings: true,
        deployDevelopment: false,
        hakFiles: [],
        customTlk: null,
      },
    }));
  });

  it("previews and applies a controlled Toolset synchronization", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    vi.mocked(selectDirectory).mockResolvedValueOnce("C:/toolset");
    const panel = await screen.findByRole("region", { name: "Synchronisation avec Aurora Toolset" });
    fireEvent.click(within(panel).getByRole("button", { name: "Parcourir" }));
    await waitFor(() => expect(within(panel).getByDisplayValue("C:/toolset")).toBeInTheDocument());
    fireEvent.click(within(panel).getByRole("button", { name: "Comparer" }));
    expect(await within(panel).findByText("start.nss")).toBeInTheDocument();
    fireEvent.click(within(panel).getByRole("button", { name: "Synchroniser la sélection" }));
    await waitFor(() => expect(applyAuroraWorkspaceSync).toHaveBeenCalledWith(expect.objectContaining({
      jobId: "job-1", workspaceId: "workspace-1", root: "C:/toolset",
      actions: [expect.objectContaining({ direction: "pull_from_toolset" })],
    })));
    expect(planAuroraWorkspaceSync).toHaveBeenCalled();
  });

  it("keeps AI offline by default and applies only a validated local proposal", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    await screen.findByRole("heading", { name: "Profils de build et Git" });
    fireEvent.click(screen.getByRole("button", { name: "Agent Studio" }));
    const panel = await screen.findByRole("region", { name: "Assistant IA contrôlé" });
    expect(within(panel).getByRole("button", { name: "Générer et prévisualiser" })).toBeDisabled();
    fireEvent.click(within(panel).getByText("Prévisualiser une proposition JSON locale, sans réseau"));
    const changeSet = { summary: "Modifier le script", commands: [{ kind: "replace_text", resource: { resref: "start", resourceType: 2009 }, before: "void main() {}", after: "void main() { int n = 1; }" }] };
    fireEvent.change(within(panel).getByLabelText("Proposition JSON locale"), { target: { value: JSON.stringify(changeSet) } });
    fireEvent.click(within(panel).getByRole("button", { name: "Valider localement" }));
    await waitFor(() => expect(previewAiChangeSet).toHaveBeenCalledWith({ jobId: "job-1", workspaceId: "workspace-1", changeSet }));
    expect(await within(panel).findByText("Précondition vérifiée")).toBeInTheDocument();
    vi.spyOn(window, "confirm").mockReturnValueOnce(true);
    fireEvent.click(within(panel).getByRole("button", { name: "Confirmer et appliquer" }));
    await waitFor(() => expect(applyAiChangeSet).toHaveBeenCalledWith(expect.objectContaining({ jobId: "job-1", workspaceId: "workspace-1", proposalSha256: "A".repeat(64), confirmed: true })));
  });

  it("creates and splits a walkmesh in the editable overlay", async () => {
    const workspace = { schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 1, cursor: 1, canUndo: true, canRedo: false, modifiedResources: [], deletedResources: [], journalEvents: 3, values: {} };
    render(<WalkmeshWorkbench jobId="job-1" workspace={workspace} onWorkspace={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: "Ouvrir" }));
    expect(screen.getByRole("button", { name: "Supprimer la face" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Extruder" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Souder les sommets" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Decouper la face" }));
    await waitFor(() => expect(transformWalkmeshDraft).toHaveBeenCalledWith(expect.objectContaining({ faces: [[0, 1, 2], [0, 2, 3]] }), { kind: "split_face", faceIndex: 0 }));
    expect(await screen.findByText(/decoupee au centroide/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer dans l'overlay" }));
    await waitFor(() => expect(saveWorkspaceWalkmesh).toHaveBeenCalledWith(expect.objectContaining({
      jobId: "job-1", workspaceId: "workspace-1", resref: "onf_walkmesh", kind: "wok",
      draft: expect.objectContaining({ faces: expect.arrayContaining([[1, 2, 4], [2, 0, 4]]) }),
    })));
    expect(await screen.findByText(/serialise en ASCII NWN/)).toBeInTheDocument();
  });

  it("creates an atomic area and opens it immediately", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    fireEvent.click(await screen.findByRole("button", { name: "Zones (1)" }));
    fireEvent.click(await screen.findByRole("button", { name: "+ Nouvelle zone" }));
    fireEvent.click(screen.getByRole("button", { name: "Créer ARE/GIT/GIC" }));
    await waitFor(() => expect(createWorkspaceArea).toHaveBeenCalledWith(expect.objectContaining({ workspaceId: "workspace-1", resref: "newarea" })));
    expect(await screen.findByText("Zone créée dans l’overlay et ouverte immédiatement.")).toBeInTheDocument();
    expect((await screen.findAllByText("Nouvelle zone")).length).toBeGreaterThan(0);
  });

  it("edits a trigger polygon through the transactional area command", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    fireEvent.click(await screen.findByRole("button", { name: "Zones (1)" }));
    fireEvent.click(await screen.findByRole("button", { name: "trigger exit" }));
    expect(await screen.findByText("Polygone local")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
    await waitFor(() => expect(editAreaStructure).toHaveBeenCalledWith(expect.objectContaining({
      jobId: "job-1",
      workspaceId: "workspace-1",
      area: "startarea",
      action: expect.objectContaining({ kind: "set_geometry", instanceId: "startarea:TriggerList:0" }),
    })));
  });

  it("creates a faction through the transactional FAC command", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    fireEvent.click(await screen.findByRole("button", { name: "Factions (1)" }));
    fireEvent.change(await screen.findByPlaceholderText("Nouvelle faction"), { target: { value: "Merchants" } });
    fireEvent.click(screen.getByRole("button", { name: "+ Faction" }));
    await waitFor(() => expect(editFactionStructure).toHaveBeenCalledWith({
      jobId: "job-1",
      workspaceId: "workspace-1",
      resource: { resref: "repute", resourceType: 2038 },
      action: { kind: "add_faction", name: "Merchants", parentId: null },
    }));
  });

  it("edits a blueprint list through the transactional structure command", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    fireEvent.click(await screen.findByRole("button", { name: "Blueprints (1)" }));
    const blueprints=await screen.findByRole("region",{name:"Atelier des blueprints"});
    fireEvent.click(await within(blueprints).findByRole("button", { name: /ambience/ }));
    fireEvent.change(await screen.findByLabelText("Son"), { target: { value: "as_test" } });
    fireEvent.click(screen.getByRole("button", { name: "+ Son" }));
    await waitFor(() => expect(editBlueprintStructure).toHaveBeenCalledWith({
      jobId: "job-1",
      workspaceId: "workspace-1",
      resource: { resref: "ambience", resourceType: 2035 },
      action: { kind: "add_sound", resref: "as_test" },
    }));
  });

  it("deploys the overlay separately and keeps undo wired", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.change(screen.getByPlaceholderText("Sélectionner le dossier utilisateur"), { target: { value: "C:/NWN" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    fireEvent.click(await screen.findByRole("button", { name: "Déployer development" }));
    await waitFor(() => expect(deployWorkspaceDevelopment).toHaveBeenCalledWith({ workspaceId: "workspace-1", userDataPath: "C:/NWN" }));
    fireEvent.click(screen.getByRole("button", { name: "Annuler" }));
    await waitFor(() => expect(undoEditCommand).toHaveBeenCalledWith({ workspaceId: "workspace-1" }));
  });

  it("exposes fine-grained provider, safety, scope and runtime controls in Agent Studio", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    await screen.findByRole("heading", { name: "Profils de build et Git" });
    fireEvent.click(screen.getByRole("button", { name: "Agent Studio" }));
    const studio = await screen.findByRole("region", { name: "Agent Studio" });
    const controls = within(studio);
    expect(controls.getByLabelText("Niveau")).toHaveValue("advisor");
    expect(controls.queryByLabelText("Autoriser le réseau")).not.toBeInTheDocument();
    expect(controls.queryByLabelText("Autoriser HTTP local")).not.toBeInTheDocument();
    expect(controls.getByLabelText("Chemins locaux dans le contexte")).not.toBeChecked();
    expect(controls.getByText("Aucun de ces chemins n’est nécessaire pour consulter ou analyser un module.", { exact: false })).toBeInTheDocument();
    expect(controls.getByRole("heading", { name: "Compiler les scripts" })).toBeInTheDocument();
    expect(controls.getByRole("heading", { name: "Produire, tester et synchroniser" })).toBeInTheDocument();
    expect(controls.getByRole("heading", { name: "Lancer le jeu ou le serveur" })).toBeInTheDocument();
    expect(controls.getByLabelText("Compilateur de scripts NWScript")).toHaveAttribute("placeholder", "C:\\…\\nwn_script_comp.exe");
    expect(controls.getByLabelText("Dossiers de sortie autorisés")).toHaveAccessibleDescription("Dossiers · requis pour créer ou construire un .mod · Barrière de sécurité : l’agent ne peut produire un module que dans ces dossiers. Séparez plusieurs dossiers par un point-virgule (;).");
    expect(controls.getByLabelText("Programme à lancer")).toHaveAccessibleDescription("Fichier .exe · requis pour lancer · Choisissez nwmain.exe pour le jeu ou nwserver.exe pour un serveur.");
    expect(controls.getByLabelText("Ressources sélectionnées (`resref:type`)")).toBeInTheDocument();
    expect(controls.getByText("Choisir le moteur IA")).toBeInTheDocument();
    expect(controls.getByText("Contrôler le contexte")).toBeInTheDocument();
    expect(controls.getByRole("button", { name: "Enregistrer le profil" })).toBeInTheDocument();
    expect(controls.getByRole("option", { name: "OpenAI compatible · Chat Completions" })).toBeInTheDocument();
    expect(controls.getByRole("option", { name: "Serveur compatible OpenAI · personnalisé" })).toBeInTheDocument();
    fireEvent.change(controls.getByLabelText("Protocole"), { target: { value: "open_ai_responses" } });
    expect(controls.getByLabelText("Endpoint")).toHaveValue("https://api.openai.com/v1/responses");
    expect(controls.getByLabelText("Stockage de la conversation chez le fournisseur")).not.toBeChecked();
    fireEvent.change(controls.getByLabelText("Niveau"), { target: { value: "supervised" } });
    expect(controls.getByLabelText("Niveau")).toHaveValue("supervised");

    vi.mocked(testAgentProvider).mockResolvedValue({ endpointOrigin: "http://127.0.0.1:11434", model: "gemma:12b", latencyMs: 842, reply: "OK" });
    fireEvent.change(controls.getAllByLabelText("Modèle")[1], { target: { value: "gemma:12b" } });
    const testButton = controls.getByRole("button", { name: "Tester la communication avec le modèle" });
    expect(testButton).toBeEnabled();
    fireEvent.click(testButton);
    expect(await controls.findByText("Connexion réussie · gemma:12b · 842 ms · réponse : OK")).toHaveAttribute("role", "status");
    expect(testAgentProvider).toHaveBeenCalledWith(expect.objectContaining({
      policy: expect.objectContaining({ context: expect.objectContaining({ allowNetwork: false }) }),
    }));
  });

  it("explains the controlled assistant and shows when its model is working", async () => {
    vi.mocked(requestAiChangeSet).mockImplementation(() => new Promise(() => undefined));
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Créer l’espace d’édition" }));
    await screen.findByRole("heading", { name: "Profils de build et Git" });
    fireEvent.click(screen.getByRole("button", { name: "Agent Studio" }));

    const assistant = await screen.findByRole("region", { name: "Assistant IA contrôlé" });
    const controls = within(assistant);
    expect(controls.getByText("À quoi sert ce panneau ?")).toBeInTheDocument();
    expect(controls.queryByLabelText("Autoriser cet appel réseau")).not.toBeInTheDocument();
    fireEvent.change(controls.getByLabelText("Modèle choisi"), { target: { value: "gemma:12b" } });
    fireEvent.change(controls.getByLabelText("Demande"), { target: { value: "Corriger ce script" } });
    const generateButton = controls.getByRole("button", { name: "Générer et prévisualiser" });
    expect(generateButton).toBeEnabled();
    fireEvent.click(generateButton);
    expect(requestAiChangeSet).toHaveBeenCalledWith(expect.objectContaining({
      consent: { includeModuleMetadata: false, includeResourceContents: false },
    }));

    expect(await controls.findByRole("status")).toHaveTextContent("Travail en cours");
    expect(controls.getByRole("button", { name: "Modèle en cours…" })).toBeDisabled();
  });
});
