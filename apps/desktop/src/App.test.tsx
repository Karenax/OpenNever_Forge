import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App, { WalkmeshWorkbench } from "./App";
import { applyAiChangeSet, applyAuroraWorkspaceSync, buildWorkspaceModule, createEditWorkspace, createWorkspaceArea, deployWorkspaceDevelopment, editAreaStructure, editBlueprintStructure, editFactionStructure, editWorkspaceModuleDependencies, planAuroraWorkspaceSync, previewAiChangeSet, saveWorkspaceBuildProfile, saveWorkspaceWalkmesh, selectDirectory, selectModuleOutput, startModuleAnalysis, transformWalkmeshDraft, undoEditCommand } from "./lib/tauri";

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
      toolRuntime: { compilerPath: "", gameInstallPath: "", includePaths: [], developmentPath: "", toolsetTempPath: "", allowedOutputRoots: [], nwnExecutablePath: "", nwnWorkingDirectory: "", nwnArguments: [] },
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
        toolRuntime: { compilerPath: "", gameInstallPath: "", includePaths: [], developmentPath: "", toolsetTempPath: "", allowedOutputRoots: [], nwnExecutablePath: "", nwnWorkingDirectory: "", nwnArguments: [] },
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
  resolveAgentApproval: vi.fn(),
  cancelAgentRun: vi.fn(),
  getAppStatus: vi.fn().mockResolvedValue({
    appVersion: "0.1.0-test",
    readOnly: true,
    editingAvailable: true,
    databaseSchemaVersion: 6,
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
      dialogueIndexSummary: { dialogues: 1, nodes: 2, links: 2, sharedNodes: 1, cycles: 1, unreachableNodes: 0, brokenLinks: 0, scriptLinks: 2, references: 1, diagnostics: 1 },
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
  queryDialogues: vi.fn().mockResolvedValue({ items: [{ resref: "forge_dialogue", nodeCount: 2, linkCount: 2, cycleCount: 1, diagnosticCount: 1, preview: "Bonjour" }], offset: 0, limit: 50, total: 1 }),
  inspectDialogue: vi.fn().mockResolvedValue({
    key: { resref: "forge_dialogue", resourceType: 2029 }, source: "C:/module.mod::forge_dialogue.dlg",
    nodes: [{ id: "entry:0", kind: "entry", index: 0, text: null, displayText: "Bonjour", speaker: "NPC", comment: "Accueil", animation: 1, animationLoop: true, sound: "hello", quest: null, actionScript: "start" }, { id: "reply:0", kind: "reply", index: 0, text: null, displayText: "Au revoir", speaker: null, comment: null, animation: null, animationLoop: null, sound: null, quest: null, actionScript: null }],
    links: [{ id: "entry:0:reply:0:0", source: "entry:0", target: "reply:0", conditionScript: "check", actionScript: null, comment: null, isChild: false, broken: false }, { id: "reply:0:entry:0:0", source: "reply:0", target: "entry:0", conditionScript: null, actionScript: null, comment: null, isChild: true, broken: false }],
    roots: ["entry:0"], sharedNodes: ["entry:0"], unreachableNodes: [], cycles: [["entry:0", "reply:0", "entry:0"]], diagnostics: [{ code: "DLG_CYCLE_DETECTED", message: "Cycle", nodeId: "entry:0", linkId: null }], references: [{ resource: { resref: "creature", resourceType: 2027 }, fieldPath: "root.Conversation", source: "C:/module.mod" }], tree: [{ nodeId: "entry:0", kind: "entry", displayText: "Bonjour", repeated: false, cycle: false, children: [{ nodeId: "reply:0", kind: "reply", displayText: "Au revoir", repeated: false, cycle: false, children: [{ nodeId: "entry:0", kind: "entry", displayText: "Bonjour", repeated: false, cycle: true, children: [] }] }] }], raw: { fileType: "DLG ", fileVersion: "V3.2" },
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
    areas: [{ resref: "startarea", name: { stringRef: null, text: "Zone de départ" }, width: 1, height: 1, tileset: "tno01", tiles: [{ x: 0, y: 0, tileId: 12, orientation: 1 }], instances: [{ id: "startarea:Creature List:0", category: "creature", tag: "guard", templateResref: "guard", x: 4, y: 5, z: 0, bearing: 0, appearance: null, transitionDestination: null, transitionFlags: null, loadScreenId: null, geometry: [], spawnPoints: [], inventory: [], sourcePath: "startarea.git::Creature List[0]" }, { id: "startarea:TriggerList:0", category: "trigger", tag: "exit", templateResref: "newtransition", x: 2, y: 2, z: 0, bearing: 0, appearance: null, transitionDestination: "wp_exit", transitionFlags: 2, loadScreenId: 7, geometry: [{ x: 0, y: 0, z: 0 }, { x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }], spawnPoints: [], inventory: [], sourcePath: "startarea.git::TriggerList[0]" }], diagnostics: [], areSource: "startarea.are", gitSource: "startarea.git", gicSource: null }],
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
  createWorkspaceArea: vi.fn().mockResolvedValue({ workspace: { schemaVersion: 2, workspaceId: "workspace-1", root: "C:/cache/workspace-1", source: { path: "C:/module.mod", sha256: "ABC123", sizeBytes: 512 }, sourceIntact: true, commandCount: 2, cursor: 2, canUndo: true, canRedo: false, modifiedResources: [{ resource: { resref: "newarea", resourceType: 2012 }, sourceSha256: null, outputSha256: "ARE", sizeBytes: 128, relativePath: "resources/newarea.are" }, { resource: { resref: "newarea", resourceType: 2023 }, sourceSha256: null, outputSha256: "GIT", sizeBytes: 128, relativePath: "resources/newarea.git" }, { resource: { resref: "newarea", resourceType: 2046 }, sourceSha256: null, outputSha256: "GIC", sizeBytes: 128, relativePath: "resources/newarea.gic" }], deletedResources: [], journalEvents: 5, values: {} }, area: { resref: "newarea", name: { stringRef: null, text: "Nouvelle zone" }, width: 1, height: 1, tileset: "tno01", tiles: [{ x: 0, y: 0, tileId: 0, orientation: 0 }], instances: [], diagnostics: [], areSource: "workspace::newarea.are", gitSource: "workspace::newarea.git", gicSource: "workspace::newarea.gic" } }),
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

describe("OpenNever Forge shell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the explorer, workbench, inspector and diagnostics", async () => {
    renderApp();

    expect(screen.getByLabelText("Explorateur du module")).toBeInTheDocument();
    expect(screen.getByLabelText("Zone de travail")).toBeInTheDocument();
    expect(screen.getByLabelText("Inspecteur")).toBeInTheDocument();
    expect(screen.getByLabelText("Diagnostics")).toBeInTheDocument();
    expect(await screen.findByText("Cœur Rust · v0.1.0-test")).toBeInTheDocument();
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

  it("renders and filters the resolved resource catalog returned by Rust", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), {
      target: { value: "C:/module.mod" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));

    expect(await screen.findByRole("table", { name: "Ressources résolues" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Forge Test" })).toBeInTheDocument();
    expect(screen.getByText("1 ressource(s) dans cette catégorie.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ressources (3)" }));
    expect(screen.getByText("3 ressource(s) dans cette catégorie.")).toBeInTheDocument();
    expect((await screen.findAllByText("module")).length).toBeGreaterThan(0);
    expect(await screen.findByText("start")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Dépendances du module" })).toBeInTheDocument();
    expect(screen.getByText("Introuvable")).toBeInTheDocument();
    expect(screen.getByText("RESOURCE_SHADOWED")).toBeInTheDocument();
    expect(screen.getByText("HAK_NOT_FOUND")).toBeInTheDocument();
    expect(screen.getByText("DEPENDENCY_CONTENT_CHANGED")).toBeInTheDocument();
    expect(screen.getByText(/SHA-256 1234567890ABCDEF/)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Filtrer les ressources"), {
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
    expect(await screen.findByTestId("monaco-readonly")).toHaveTextContent("void main()");
    expect(screen.getByText(/module\.\#2014/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Bytecode NCS" }));
    expect(await screen.findByText("NCS V1.0")).toBeInTheDocument();
  });

  it("shows a cyclic dialogue as a bounded tree, full graph and raw GFF", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Dialogues (1)" }));
    expect(await screen.findByRole("region", { name: "Explorateur de dialogues" })).toBeInTheDocument();
    expect(await screen.findByText("cycle")).toBeInTheDocument();
    expect(screen.getByText(/creature\.\#2027/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Graphe complet" }));
    expect(await screen.findByTestId("dialogue-flow")).toHaveTextContent("2 nodes · 2 edges");
    fireEvent.click(screen.getByRole("button", { name: "GFF brut" }));
    expect(await screen.findByText(/fileType/)).toBeInTheDocument();
  });

  it("navigates the narrative, 2D map, assets and targeted global graph", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), { target: { value: "C:/module.mod" } });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));
    fireEvent.click(await screen.findByRole("button", { name: "Journal et factions (2)" }));
    expect(await screen.findByText("Quête principale")).toBeInTheDocument();
    expect(screen.getByText("état final")).toBeInTheDocument();
    expect(screen.getByText("Commoner")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Zones (1)" }));
    expect((await screen.findAllByText("Zone de départ")).length).toBeGreaterThan(0);
    expect(screen.getByLabelText("creature guard")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Assets (1)" }));
    expect(await screen.findByText("mdl_ascii · preview")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Références (2)" }));
    expect(await screen.findByText("Rapport JSON stable")).toBeInTheDocument();
    expect(screen.getByText("contains · certain")).toBeInTheDocument();
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
    fireEvent.click(await screen.findByRole("button", { name: "Journal et factions (2)" }));
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
    fireEvent.click(await screen.findByRole("row", { name: /ambience/ }));
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
    const studio = await screen.findByRole("region", { name: "Agent Studio" });
    const controls = within(studio);
    expect(controls.getByLabelText("Niveau")).toHaveValue("advisor");
    expect(controls.getByLabelText("Autoriser le réseau")).not.toBeChecked();
    expect(controls.getByLabelText("Chemins locaux dans le contexte")).not.toBeChecked();
    expect(controls.getByLabelText("Compilateur NWScript")).toBeInTheDocument();
    expect(controls.getByLabelText("Ressources sélectionnées (`resref:type`)")).toBeInTheDocument();
    expect(controls.getByRole("button", { name: "Enregistrer le profil" })).toBeInTheDocument();
    fireEvent.change(controls.getByLabelText("Protocole"), { target: { value: "open_ai_responses" } });
    expect(controls.getByLabelText("Stockage de la conversation chez le fournisseur")).not.toBeChecked();
    fireEvent.change(controls.getByLabelText("Niveau"), { target: { value: "supervised" } });
    fireEvent.click(controls.getByLabelText("Autoriser le réseau"));
    expect(controls.getByLabelText("Niveau")).toHaveValue("supervised");
    expect(controls.getByLabelText("Autoriser le réseau")).toBeChecked();
  });
});
