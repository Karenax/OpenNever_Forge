import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { startModuleAnalysis } from "./lib/tauri";

vi.mock("@monaco-editor/react", () => ({ default: ({ value }: { value: string }) => <pre data-testid="monaco-readonly">{value}</pre> }));
vi.mock("@xyflow/react", () => ({ ReactFlow: ({ nodes, edges, children }: { nodes: Array<{id:string}>; edges:Array<{id:string}>; children: React.ReactNode }) => <div data-testid="dialogue-flow">{nodes.length} nodes · {edges.length} edges{children}</div>, Background:()=>null, Controls:()=>null, MiniMap:()=>null }));

vi.mock("./lib/tauri", () => ({
  getAppStatus: vi.fn().mockResolvedValue({
    appVersion: "0.1.0-test",
    readOnly: true,
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
        resourceCount: 2,
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
        ],
        typeSummaries: [
          { resourceType: 2009, extension: "nss", count: 1, totalSize: 160 },
          { resourceType: 2014, extension: "ifo", count: 1, totalSize: 128 },
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
        ],
      },
      resourceCatalogSummary: {
        resourceCount: 2,
        versionCount: 2,
        shadowedCount: 0,
        diagnosticCount: 0,
        typeCounts: [
          { resourceType: 2009, count: 1 },
          { resourceType: 2014, count: 1 },
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
  inspectResource: vi.fn(),
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
  inspectWorld: vi.fn().mockResolvedValue({
    narrative: {
      categories: [{ tag: "main_quest", name: { stringRef: null, text: "Quête principale" }, priority: 1, xp: 100, entries: [{ id: 1, text: { stringRef: null, text: "Trouver le trésor" }, finalState: true, delay: 0 }], source: "module.jrl" }],
      factions: [{ id: 0, name: "Commoner", parentId: null, global: true }],
      reputations: [{ sourceId: 0, targetId: 0, value: 100 }],
      relations: [], diagnostics: [],
    },
    areas: [{ resref: "startarea", name: { stringRef: null, text: "Zone de départ" }, width: 1, height: 1, tileset: "tno01", tiles: [{ x: 0, y: 0, tileId: 12, orientation: 1 }], instances: [{ id: "startarea:Creature List:0", category: "creature", tag: "guard", templateResref: "guard", x: 4, y: 5, z: 0, bearing: 0, transitionDestination: null, sourcePath: "startarea.git::Creature List[0]" }], diagnostics: [], areSource: "startarea.are", gitSource: "startarea.git", gicSource: null }],
    assets: { assets: [{ key: { resref: "guard", resourceType: 2002 }, source: "guard.mdl", format: "mdl_ascii", support: "preview", width: null, height: null, modelNodes: ["trimesh"], animations: ["walk"], textures: ["guard_diff"], referencedModels: [], supermodel: null, meshCount: 1, triangleCount: 12, skinCount: 0, walkmeshCount: 0, glbPreview: true, sha256: "ABC", diagnostics: [] }] },
    scenes: [{ area: "startarea", width: 1, height: 1, tileset: "tno01", objects: [{ id: "tile:0:0", kind: "tile", label: "Tuile 12", x: 5, y: 0, z: 5, rotation: 0, marker: true, sourcePath: "startarea.are" }, { id: "guard", kind: "creature", label: "guard", x: 4, y: 0, z: 5, rotation: 0, marker: false, sourcePath: "startarea.git" }], overlays: [], missingAssets: 0, memoryBudgetBytes: 268435456, diagnostics: [] }],
    graphNodes: [{ id: "area:startarea", kind: "area", label: "Zone de départ", resource: "startarea.are" }, { id: "instance:guard", kind: "creature", label: "guard", resource: "guard" }, { id: "journal:main_quest", kind: "journal", label: "Quête principale", resource: null }],
    graphEdges: [{ id: "contains", source: "area:startarea", target: "instance:guard", kind: "contains", confidence: "certain", evidence: { resource: "startarea.git", fieldPath: "Creature List[0]" } }, { id: "quest", source: "dialogue:test", target: "journal:main_quest", kind: "journal_reference", confidence: "probable", evidence: { resource: "test.dlg", fieldPath: "entry:0" } }],
    diagnostics: [{ code: "RESOURCE_SHADOWED", severity: "info", message: "Version masquée", resource: "guard.mdl", evidence: null }],
    summary: { journalCategories: 1, journalEntries: 1, factions: 1, factionRelations: 1, areas: 1, tiles: 1, instances: 1, transitions: 0, assets: 1, previewableAssets: 1, sceneObjects: 2, graphNodes: 3, graphEdges: 2, diagnostics: 1 },
  }),
  diagnosticReport: vi.fn().mockResolvedValue({ report: {}, json: "{}", html: "<!doctype html>" }),
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
    fireEvent.click(screen.getByRole("button", { name: "Ressources (2)" }));
    expect(screen.getByText("2 ressource(s) dans cette catégorie.")).toBeInTheDocument();
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
});
