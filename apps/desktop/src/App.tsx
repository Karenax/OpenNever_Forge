import {
  AlertTriangle,
  Archive,
  Box,
  BookOpen,
  Braces,
  ChevronRight,
  CircleGauge,
  Code2,
  Database,
  Download,
  FileSearch,
  FolderOpen,
  GitBranch,
  Hash,
  History,
  Map,
  MessageSquareText,
  PanelLeftClose,
  Search,
  ShieldCheck,
  SquareStack,
  Orbit,
  PencilLine,
  Redo2,
  Undo2,
  X,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import Editor, { type Monaco } from "@monaco-editor/react";
import { Background, Controls, MiniMap, ReactFlow, type Edge, type Node } from "@xyflow/react";
import { useEffect, useMemo, useState } from "react";
import {
  cancelJob,
  assetPreviewBytes,
  getAppStatus,
  getJob,
  inspectResource,
  inspectDialogue,
  inspectNarrativeDocuments,
  inspectScript,
  inspectWorld,
  inspectWorkspaceArea,
  inspectWorkspaceWalkmesh,
  modelPreviewGlb,
  diagnosticReport,
  normalizeAppError,
  queryResources,
  queryDialogues,
  queryScripts,
  resolveTexture,
  selectCompiler,
  selectDirectory,
  selectModule,
  startModuleAnalysis,
  applyGffEdit,
  addWorkspaceAreaInstance,
  buildWorkspaceModule,
  buildWorkspaceHak,
  cleanWorkspaceDevelopment,
  compileWorkspaceScript,
  createEditWorkspace,
  createNewModule,
  createWorkspaceArea,
  listWorkspaceCreatedAreas,
  deleteWorkspaceArea,
  deployWorkspaceDevelopment,
  editScriptSource,
  editDialogueField,
  editDialogueStructure,
  editBlueprintStructure,
  editAreaStructure,
  editFactionStructure,
  editJournalStructure,
  exportWorkspaceSources,
  editWorkspaceTwoDa,
  editWorkspaceTlk,
  editWorkspaceModuleDependencies,
  inspectGitWorkspace,
  listWorkspaceBuildProfiles,
  listWorkspaceLaunchProfiles,
  launchWorkspaceTestProfile,
  runWorkspaceBuildProfile,
  saveWorkspaceBuildProfile,
  saveWorkspaceLaunchProfile,
  selectNwnExecutable,
  verifyWorkspaceReproducibleBuild,
  moveAreaInstance,
  removeWorkspaceAreaInstance,
  redoEditCommand,
  selectModuleOutput,
  selectHakOutput,
  setAreaTile,
  saveWorkspaceWalkmesh,
  transformWalkmeshDraft,
  undoEditCommand,
  validateWalkmeshDraft,
  type JobSnapshot,
  type ModuleDependency,
  type ModuleDependencyReport,
  type ResolvedResource,
  type ResourceCatalogSummary,
  type ResourceInspection,
  type DialogueGraph,
  type DialogueIndexSummary,
  type DialogueTreeNode,
  type DialogueNodeRef,
  type DialogueStructureAction,
  type AreaStructureAction,
  type BlueprintListKind,
  type BlueprintStructureAction,
  type FactionStructureAction,
  type JournalStructureAction,
  type ScriptDocument,
  type ScriptIndexSummary,
  type StructuredResourceSummary,
  type AreaMap,
  type AssetRecord,
  type SceneManifest,
  type WorldIndex,
  type WorldSummary,
  type NarrativeDocument,
  type WorkspaceSnapshot,
  type GenericGff,
  type GenericGffValue,
  type CompileResult,
  type WalkmeshDraft,
  type WalkmeshKind,
  type WalkmeshOperation,
  type ModuleBuildProfile,
  type GitWorkspaceStatus,
  type TwoDaTable,
  type TalkTable,
  type TwoDaEditAction,
  type TlkEditAction,
  type NwnLaunchProfile,
} from "./lib/tauri";
import { AuroraSyncPanel } from "./components/AuroraSyncPanel";
import { AiAssistantPanel } from "./components/AiAssistantPanel";
import { useUiStore } from "./store/uiStore";
import "./App.css";

type ProjectField = "modulePath" | "gameInstallPath" | "userDataPath";

type Diagnostic = {
  id: string;
  level: "info" | "warning" | "error";
  code: string;
  message: string;
};

const explorerGroupDefinitions = [
  { id: "module", label: "Informations", icon: FileSearch },
  { id: "areas", label: "Zones", icon: Map },
  { id: "narrative", label: "Journal et factions", icon: BookOpen },
  { id: "dialogues", label: "Dialogues", icon: MessageSquareText },
  { id: "scripts", label: "Scripts", icon: Code2 },
  { id: "blueprints", label: "Blueprints", icon: Box },
  { id: "assets", label: "Assets", icon: SquareStack },
  { id: "scene", label: "Vue 3D", icon: Orbit },
  { id: "resources", label: "Ressources", icon: Archive },
  { id: "tables", label: "2DA et TLK", icon: Database },
  { id: "graph", label: "Références", icon: GitBranch },
];

const resourceTypesByGroup: Record<string, Set<number>> = {
  module: new Set([2014, 2038, 2056]),
  areas: new Set([2012, 2023, 2046]),
  dialogues: new Set([2029]),
  scripts: new Set([2009, 2010, 2064]),
  blueprints: new Set([2025, 2027, 2032, 2035, 2040, 2042, 2044, 2051, 2055, 2058]),
  tables: new Set([2017, 2018]),
  graph: new Set(),
};

const terminalStates = new Set(["completed", "failed", "cancelled"]);

function App() {
  const activeExplorerItem = useUiStore((state) => state.activeExplorerItem);
  const setActiveExplorerItem = useUiStore((state) => state.setActiveExplorerItem);
  const [project, setProject] = useState({
    modulePath: "",
    gameInstallPath: "",
    userDataPath: "",
  });
  const [jobId, setJobId] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [resourceFilter, setResourceFilter] = useState("");
  const [selectedResource, setSelectedResource] = useState<ResolvedResource>();
  const [inspection, setInspection] = useState<ResourceInspection>();
  const [inspectionBusy, setInspectionBusy] = useState(false);
  const [editWorkspace, setEditWorkspace] = useState<WorkspaceSnapshot>();
  const [editBusy, setEditBusy] = useState(false);
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([
    {
      id: "source-readonly",
      level: "info",
      code: "SOURCE_READ_ONLY",
      message: "La source NWN reste immuable ; les éditions utilisent un espace de travail séparé.",
    },
  ]);

  const statusQuery = useQuery({
    queryKey: ["app-status"],
    queryFn: getAppStatus,
    retry: false,
  });
  const jobQuery = useQuery({
    queryKey: ["job", jobId],
    queryFn: () => getJob(jobId as string),
    enabled: Boolean(jobId),
    refetchInterval: (query) => {
      const job = query.state.data;
      return job && terminalStates.has(job.state) ? false : 250;
    },
  });

  const job = jobQuery.data;
  const analysis = job?.result;
  const fingerprint = analysis?.fingerprint;
  const inventory = analysis?.inventory;
  const moduleInfo = analysis?.moduleInfo;
  const dependencyReport = analysis?.dependencyReport;
  const resourceCatalogSummary = analysis?.resourceCatalogSummary;
  const structuredSummary = analysis?.structuredSummary;
  const scriptIndexSummary = analysis?.scriptIndexSummary;
  const dialogueIndexSummary = analysis?.dialogueIndexSummary;
  const worldSummary = analysis?.worldSummary;
  const moduleName = localizedPrimary(moduleInfo?.name);
  const dependencyDiagnostics = useMemo(
    () => diagnosticsForDependencies(dependencyReport),
    [dependencyReport],
  );
  const visibleDiagnostics = [...diagnostics, ...dependencyDiagnostics];
  const appReady = statusQuery.data;
  const explorerGroups = useMemo(
    () =>
      explorerGroupDefinitions.map((item) => ({
        ...item,
        count:
          item.id === "dialogues"
            ? dialogueIndexSummary?.dialogues ?? 0
            : item.id === "narrative"
            ? (worldSummary?.journalCategories ?? 0) + (worldSummary?.factions ?? 0)
            : item.id === "areas"
            ? worldSummary?.areas ?? 0
            : item.id === "assets"
            ? worldSummary?.assets ?? 0
            : item.id === "scene"
            ? worldSummary?.sceneObjects ?? 0
            : item.id === "graph"
            ? worldSummary?.graphEdges ?? 0
            : item.id === "scripts"
            ? scriptIndexSummary?.scripts ?? 0
            : item.id === "resources"
            ? resourceCatalogSummary?.resourceCount ?? 0
            : resourceCatalogSummary?.typeCounts
                .filter((entry) => resourceTypesByGroup[item.id]?.has(entry.resourceType))
                .reduce((total, entry) => total + entry.count, 0) ?? 0,
      })),
    [resourceCatalogSummary, scriptIndexSummary, dialogueIndexSummary, worldSummary],
  );
  const currentExplorer = useMemo(
    () => explorerGroups.find((item) => item.id === activeExplorerItem),
    [activeExplorerItem, explorerGroups],
  );
  const CurrentExplorerIcon = currentExplorer?.icon ?? Braces;

  function updateField(field: ProjectField, value: string) {
    setProject((current) => ({ ...current, [field]: value }));
  }

  function pushError(error: unknown) {
    const normalized = normalizeAppError(error);
    setDiagnostics((current) => [
      ...current.filter((item) => item.id !== normalized.code),
      {
        id: normalized.code,
        level: normalized.severity === "info" ? "info" : "error",
        code: normalized.code,
        message: normalized.userMessage,
      },
    ]);
  }

  async function browse(field: ProjectField) {
    try {
      const selected =
        field === "modulePath" ? await selectModule() : await selectDirectory();
      if (selected) updateField(field, selected);
    } catch (error) {
      pushError(error);
    }
  }

  async function analyzeModule() {
    setBusy(true);
    try {
      const created = await startModuleAnalysis({
        modulePath: project.modulePath,
        gameInstallPath: project.gameInstallPath || null,
        userDataPath: project.userDataPath || null,
      });
      setJobId(created.id);
      setDiagnostics((current) => [
        ...current.filter((item) => item.id !== "MODULE_ANALYSIS_STARTED"),
        {
          id: "MODULE_ANALYSIS_STARTED",
          level: "info",
          code: "MODULE_ANALYSIS_STARTED",
          message: "Lecture ERF et empreinte des dépendances lancées en arrière-plan.",
        },
      ]);
    } catch (error) {
      pushError(error);
    } finally {
      setBusy(false);
    }
  }

  async function stopJob() {
    if (!jobId) return;
    try {
      await cancelJob(jobId);
    } catch (error) {
      pushError(error);
    }
  }

  async function openEditWorkspace() {
    if (!jobId) return;
    setEditBusy(true);
    try {
      const snapshot = await createEditWorkspace({ jobId });
      setEditWorkspace(snapshot);
      setDiagnostics((current) => [
        ...current.filter((item) => item.id !== "EDIT_WORKSPACE_READY"),
        {
          id: "EDIT_WORKSPACE_READY",
          level: "info",
          code: "EDIT_WORKSPACE_READY",
          message: "Espace transactionnel prêt ; la source reste protégée.",
        },
      ]);
    } catch (error) {
      pushError(error);
    } finally {
      setEditBusy(false);
    }
  }

  async function moveEditCursor(direction: "undo" | "redo") {
    if (!editWorkspace) return;
    setEditBusy(true);
    try {
      const action = direction === "undo" ? undoEditCommand : redoEditCommand;
      setEditWorkspace(await action({ workspaceId: editWorkspace.workspaceId }));
    } catch (error) {
      pushError(error);
    } finally {
      setEditBusy(false);
    }
  }

  async function buildEditedModule() {
    if (!editWorkspace) return;
    const outputPath = await selectModuleOutput();
    if (!outputPath) return;
    setEditBusy(true);
    try {
      const report = await buildWorkspaceModule({ workspaceId: editWorkspace.workspaceId, outputPath });
      setDiagnostics((current) => [...current, {
        id: `MODULE_BUILD_${report.sha256}`,
        level: "info",
        code: "MODULE_BUILD_READY",
        message: `${report.resourceCount} ressources sauvegardées dans ${report.outputPath}.`,
      }]);
    } catch (error) {
      pushError(error);
    } finally {
      setEditBusy(false);
    }
  }

  async function buildEditedHak() {
    if (!editWorkspace) return;
    const outputPath = await selectHakOutput();
    if (!outputPath) return;
    setEditBusy(true);
    try {
      const report = await buildWorkspaceHak({ workspaceId: editWorkspace.workspaceId, outputPath });
      setDiagnostics((current) => [...current, { id: `HAK_BUILD_${report.sha256}`, level: "info", code: "HAK_BUILD_READY", message: `${report.resourceCount} ressources sauvegardées dans ${report.outputPath}.` }]);
    } catch (error) { pushError(error); } finally { setEditBusy(false); }
  }

  async function exportEditedSources() {
    if (!editWorkspace) return;
    const outputPath = await selectDirectory();
    if (!outputPath) return;
    setEditBusy(true);
    try {
      const manifest = await exportWorkspaceSources({ workspaceId: editWorkspace.workspaceId, outputPath });
      setDiagnostics((current) => [...current, { id: `SOURCE_EXPORT_${editWorkspace.cursor}`, level: "info", code: "SOURCE_EXPORT_READY", message: `${manifest.files.length} ressource(s) exportée(s) avec manifeste reproductible.` }]);
    } catch (error) { pushError(error); } finally { setEditBusy(false); }
  }

  async function deployEditedResources() {
    if (!editWorkspace || !project.userDataPath) return;
    setEditBusy(true);
    try {
      const deployment = await deployWorkspaceDevelopment({ workspaceId: editWorkspace.workspaceId, userDataPath: project.userDataPath });
      setDiagnostics((current) => [...current, {
        id: `DEVELOPMENT_DEPLOY_${editWorkspace.cursor}`,
        level: "info",
        code: "DEVELOPMENT_DEPLOYED",
        message: `${deployment.files.length} ressource(s) déployée(s) dans ${deployment.developmentPath}.`,
      }]);
    } catch (error) {
      pushError(error);
    } finally {
      setEditBusy(false);
    }
  }

  async function cleanEditedResources() {
    if (!editWorkspace || !project.userDataPath) return;
    setEditBusy(true);
    try {
      const cleanup = await cleanWorkspaceDevelopment({ workspaceId: editWorkspace.workspaceId, userDataPath: project.userDataPath });
      setDiagnostics((current) => [...current, {
        id: `DEVELOPMENT_CLEAN_${editWorkspace.cursor}`,
        level: cleanup.preservedChanged.length ? "warning" : "info",
        code: "DEVELOPMENT_CLEANED",
        message: `${cleanup.removed.length} fichier(s) retiré(s), ${cleanup.preservedChanged.length} fichier(s) modifié(s) conservé(s).`,
      }]);
    } catch (error) {
      pushError(error);
    } finally {
      setEditBusy(false);
    }
  }

  async function selectResource(resource: ResolvedResource) {
    if (!jobId) return;
    setSelectedResource(resource);
    setInspection(undefined);
    setInspectionBusy(true);
    try {
      setInspection(
        await inspectResource({
          jobId,
          resref: resource.key.resref,
          resourceType: resource.key.resourceType,
          workspaceId: editWorkspace?.workspaceId,
        }),
      );
    } catch (error) {
      pushError(error);
    } finally {
      setInspectionBusy(false);
    }
  }

  async function editSelectedGff(path: string, before: GenericGffValue, after: GenericGffValue) {
    if (!jobId || !editWorkspace || !selectedResource) return;
    try {
      const result = await applyGffEdit({
        jobId,
        workspaceId: editWorkspace.workspaceId,
        resource: selectedResource.key,
        path,
        before,
        after,
      });
      setEditWorkspace(result.workspace);
      setInspection({ kind: "gff", value: result.document });
    } catch (error) {
      pushError(error);
      throw error;
    }
  }

  async function editSelectedBlueprintStructure(action: BlueprintStructureAction) {
    if (!jobId || !editWorkspace || !selectedResource) return;
    try {
      const result = await editBlueprintStructure({
        jobId,
        workspaceId: editWorkspace.workspaceId,
        resource: selectedResource.key,
        action,
      });
      setEditWorkspace(result.workspace);
      setInspection({ kind: "gff", value: result.document });
    } catch (error) {
      pushError(error);
      throw error;
    }
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="brand-block">
          <div className="brand-mark" aria-hidden="true">
            <SquareStack size={18} strokeWidth={1.8} />
          </div>
          <div>
            <strong>OpenNever Forge</strong>
            <span>Explorateur de modules NWN</span>
          </div>
        </div>
        <nav className="main-menu" aria-label="Menu principal">
          <button type="button">Projet</button>
          <button type="button">Recherche</button>
          <button type="button">Diagnostics</button>
        </nav>
        <div className="runtime-status">
          <span className={appReady ? "status-dot ready" : "status-dot"} />
          {appReady ? `Cœur Rust · v${appReady.appVersion}` : "Connexion au cœur…"}
          <span className={editWorkspace ? "readonly-badge editing" : "readonly-badge"}>
            <ShieldCheck size={13} /> {editWorkspace ? "Source protégée · édition contrôlée" : "Source protégée"}
          </span>
        </div>
      </header>

      <section className="workspace-grid">
        <aside className="explorer panel" aria-label="Explorateur du module">
          <div className="panel-title">
            <span>Explorateur</span>
            <button type="button" className="icon-button" aria-label="Réduire l'explorateur">
              <PanelLeftClose size={15} />
            </button>
          </div>
          <div className="search-box">
            <Search size={14} />
            <input
              aria-label="Filtrer les ressources"
              placeholder="Filtrer les ressources…"
              value={resourceFilter}
              onChange={(event) => setResourceFilter(event.currentTarget.value)}
            />
          </div>
          <div className="tree-root">
            <div className="tree-heading">
              <ChevronRight size={14} className="tree-chevron" />
              <Archive size={15} />
              <span>
                {resourceCatalogSummary ? `${resourceCatalogSummary.resourceCount} ressources` : "Module non indexé"}
              </span>
            </div>
            <div className="tree-items">
              {explorerGroups.map((item) => {
                const Icon = item.icon;
                return (
                  <button
                    type="button"
                    key={item.id}
                    className={activeExplorerItem === item.id ? "tree-item active" : "tree-item"}
                    onClick={() => setActiveExplorerItem(item.id)}
                    aria-label={`${item.label} (${item.count})`}
                  >
                    <Icon size={14} />
                    <span>{item.label}</span>
                    {item.count !== undefined && <small aria-hidden="true">{item.count}</small>}
                  </button>
                );
              })}
            </div>
          </div>
          <div className="explorer-footer">
            <CircleGauge size={14} />
            <span>
              {job && !terminalStates.has(job.state)
                ? "Analyse en cours"
                : resourceCatalogSummary
                  ? "Resource Manager prêt"
                  : "Aucune indexation active"}
            </span>
          </div>
        </aside>

        <section className="workbench panel" aria-label="Zone de travail">
          <div className="tab-strip">
            <div className="tab active">
              <FolderOpen size={14} />
              <span>Accueil</span>
              <X size={13} />
            </div>
          </div>
          <div className="welcome-canvas">
            <div className="welcome-heading">
              <span className="eyebrow">LOTS 6–10 · COMPRÉHENSION COMPLÈTE DU MODULE</span>
              <h1>{moduleName ?? "Ouvrir une copie de travail"}</h1>
              {moduleInfo ? (
                <p>
                  Module <code>{moduleInfo.tag}</code> · NWN {moduleInfo.minimumGameVersion ?? "non spécifiée"} ·
                  zone d'entrée <code>{moduleInfo.entryArea}</code>
                </p>
              ) : (
                <p>
                  Sélectionnez un module et les deux racines NWN. L'analyse lit l'index ERF et vérifie
                  les HAK/TLK déclarés, sans extraire ni modifier aucune ressource.
                </p>
              )}
            </div>

            <div className="project-form">
              <PathField
                label="Module NWN"
                hint="Fichier .mod copié dans un espace de travail local"
                value={project.modulePath}
                placeholder="Sélectionner un fichier .mod"
                onChange={(value) => updateField("modulePath", value)}
                onBrowse={() => browse("modulePath")}
              />
              <PathField
                label="Installation du jeu"
                hint="Racine de Neverwinter Nights: Enhanced Edition"
                value={project.gameInstallPath}
                placeholder="Sélectionner le dossier d'installation"
                onChange={(value) => updateField("gameInstallPath", value)}
                onBrowse={() => browse("gameInstallPath")}
              />
              <PathField
                label="Données utilisateur"
                hint="Dossier Documents/ Neverwinter Nights de l'utilisateur"
                value={project.userDataPath}
                placeholder="Sélectionner le dossier utilisateur"
                onChange={(value) => updateField("userDataPath", value)}
                onBrowse={() => browse("userDataPath")}
              />

              {job && <JobProgress job={job} />}

              <div className="form-actions">
                <span>
                  <ShieldCheck size={14} /> Source protégée, cache séparé
                </span>
                {job && !terminalStates.has(job.state) ? (
                  <button type="button" className="secondary-button" onClick={stopJob}>
                    Annuler
                  </button>
                ) : (
                  <button
                    type="button"
                    className="primary-button"
                    disabled={!project.modulePath || busy}
                    onClick={analyzeModule}
                  >
                    <Hash size={16} />
                    {busy ? "Démarrage…" : "Analyser la copie"}
                  </button>
                )}
              </div>
            </div>

            {!jobId && <NewModuleCreator onCreated={(path) => updateField("modulePath", path)} />}

            {dependencyReport && (
              <DependencyReportView
                report={dependencyReport}
                moduleInfo={moduleInfo}
                jobId={jobId}
                editWorkspace={editWorkspace}
                onWorkspace={setEditWorkspace}
                onError={pushError}
              />
            )}

            {structuredSummary && <StructuredSummaryView summary={structuredSummary} />}

            {scriptIndexSummary && <ScriptSummaryView summary={scriptIndexSummary} />}

            {dialogueIndexSummary && <DialogueSummaryView summary={dialogueIndexSummary} />}

            {worldSummary && <WorldSummaryView summary={worldSummary} />}

            {job?.state === "completed" && (
              <section className="inventory-card edit-workspace-card" aria-label="Édition contrôlée">
                <div className="edit-workspace-heading">
                  <div>
                    <span className="eyebrow">LOT 11 · ESPACE TRANSACTIONNEL</span>
                    <h2>Édition contrôlée</h2>
                  </div>
                  <PencilLine size={22} />
                </div>
                {editWorkspace ? (
                  <>
                    <div className="edit-workspace-stats">
                      <Property label="Source" value={editWorkspace.sourceIntact ? "Intacte" : "Modifiée extérieurement"} />
                      <Property label="Commandes" value={`${editWorkspace.cursor}/${editWorkspace.commandCount}`} />
                      <Property label="Ressources modifiées" value={editWorkspace.modifiedResources.length.toString()} />
                      <Property label="Journal" value={`${editWorkspace.journalEvents} événement(s)`} />
                    </div>
                    <code className="source-path">{editWorkspace.root}</code>
                    <div className="edit-workspace-actions">
                      <button type="button" className="secondary-button" disabled={!editWorkspace.canUndo || editBusy} onClick={() => moveEditCursor("undo")}>
                        <Undo2 size={14} /> Annuler
                      </button>
                      <button type="button" className="secondary-button" disabled={!editWorkspace.canRedo || editBusy} onClick={() => moveEditCursor("redo")}>
                        <Redo2 size={14} /> Rétablir
                      </button>
                      <button type="button" className="secondary-button" disabled={!editWorkspace.modifiedResources.length || editBusy} onClick={() => void buildEditedModule()}>
                        <Download size={14} /> Construire un MOD
                      </button>
                      <button type="button" className="secondary-button" disabled={!editWorkspace.modifiedResources.length || editBusy} onClick={() => void buildEditedHak()}>
                        <Archive size={14} /> Construire un HAK
                      </button>
                      <button type="button" className="secondary-button" disabled={!editWorkspace.modifiedResources.length || editBusy} onClick={() => void exportEditedSources()}>
                        <GitBranch size={14} /> Export reproductible
                      </button>
                      <button type="button" className="secondary-button" disabled={!editWorkspace.modifiedResources.length || !project.userDataPath || editBusy} onClick={() => void deployEditedResources()}>
                        <Orbit size={14} /> Déployer development
                      </button>
                      <button type="button" className="secondary-button" disabled={!project.userDataPath || editBusy} onClick={() => void cleanEditedResources()}>
                        <X size={14} /> Nettoyer development
                      </button>
                      <span><History size={14} /> Historique append-only</span>
                    </div>
                  </>
                ) : (
                  <div className="edit-workspace-empty">
                    <p>Crée un overlay local lié à l’empreinte du module. Aucun octet n’est écrit dans le MOD, les HAK ou l’installation.</p>
                    <button type="button" className="primary-button" disabled={editBusy} onClick={openEditWorkspace}>
                      <PencilLine size={15} /> {editBusy ? "Création…" : "Créer l’espace d’édition"}
                    </button>
                  </div>
                )}
              </section>
            )}

            {editWorkspace && (
              <BuildProfilesPanel
                workspace={editWorkspace}
                userDataPath={project.userDataPath}
                onError={pushError}
              />
            )}

            {editWorkspace && jobId && (
              <AuroraSyncPanel
                jobId={jobId}
                workspace={editWorkspace}
                onWorkspaceChange={setEditWorkspace}
                onError={pushError}
              />
            )}

            {editWorkspace && jobId && (
              <AiAssistantPanel
                jobId={jobId}
                workspace={editWorkspace}
                selectedResource={selectedResource?.key}
                onWorkspaceChange={setEditWorkspace}
                onError={pushError}
              />
            )}

            {scriptIndexSummary && jobId && activeExplorerItem === "scripts" && (
              <ScriptWorkspace jobId={jobId} summary={scriptIndexSummary} filter={resourceFilter} editWorkspace={editWorkspace} gameInstallPath={project.gameInstallPath} onWorkspace={setEditWorkspace} />
            )}

            {dialogueIndexSummary && jobId && activeExplorerItem === "dialogues" && (
              <DialogueWorkspace jobId={jobId} summary={dialogueIndexSummary} filter={resourceFilter} editWorkspace={editWorkspace} onWorkspace={setEditWorkspace} onOpenScript={(script) => { setResourceFilter(script); setActiveExplorerItem("scripts"); }} />
            )}

            {worldSummary && jobId && ["narrative", "areas", "assets", "scene", "graph"].includes(activeExplorerItem) && (
              <PhaseOneWorkspace jobId={jobId} activeView={activeExplorerItem} editWorkspace={editWorkspace} onWorkspace={setEditWorkspace} />
            )}

            {resourceCatalogSummary && jobId && !["scripts", "dialogues", "narrative", "areas", "assets", "scene", "graph"].includes(activeExplorerItem) && (
              <CatalogView
                jobId={jobId}
                summary={resourceCatalogSummary}
                activeGroup={activeExplorerItem}
                filter={resourceFilter}
                selected={selectedResource}
                onSelect={selectResource}
              />
            )}

            <div className="safety-note">
              <ShieldCheck size={18} />
              <div>
                <strong>Garantie d’intégrité</strong>
                <p>Le module source, les HAK et l’installation du jeu ne sont jamais ouverts en écriture ; seules les ressources modifiées sont copiées dans l’overlay.</p>
              </div>
            </div>
          </div>
        </section>

        <aside className="inspector panel" aria-label="Inspecteur">
          <div className="panel-title">Inspecteur</div>
          <div className="inspector-empty">
            <div className="inspector-icon">
              <CurrentExplorerIcon size={22} />
            </div>
            <strong>{currentExplorer?.label ?? "Aucune sélection"}</strong>
            <span>
              {resourceCatalogSummary
                ? `${currentExplorer?.count ?? 0} ressource(s) dans cette catégorie.`
                : "Les propriétés apparaîtront après l'indexation."}
            </span>
          </div>
          <div className="property-section">
            <h2>Projet</h2>
            <Property label="Mode" value={editWorkspace ? "Édition contrôlée" : "Source protégée"} />
            <Property label="Schéma cache" value={appReady ? `v${appReady.databaseSchemaVersion}` : "—"} />
            <Property label="Module" value={project.modulePath ? "Sélectionné" : "Non sélectionné"} />
          </div>
          {fingerprint && (
            <div className="property-section fingerprint-section">
              <h2>Empreinte source</h2>
              <Property label="Taille" value={`${fingerprint.sizeBytes.toLocaleString("fr-FR")} octets`} />
              <code>{fingerprint.sha256}</code>
            </div>
          )}
          {inventory && (
            <div className="property-section">
              <h2>Conteneur</h2>
              <Property label="Format" value={`${inventory.fileType.trim()} ${inventory.fileVersion}`} />
              <Property label="Ressources" value={inventory.resourceCount.toLocaleString("fr-FR")} />
              <Property label="Construction" value={`${inventory.buildYear} · jour ${inventory.buildDay}`} />
            </div>
          )}
          {moduleInfo && (
            <div className="property-section">
              <h2>Module NWN</h2>
              <Property label="Nom" value={moduleName ?? "Sans nom intégré"} />
              <Property label="Tag" value={moduleInfo.tag} />
              <Property label="Version minimale" value={moduleInfo.minimumGameVersion ?? "Non spécifiée"} />
              <Property label="Zone d'entrée" value={moduleInfo.entryArea} />
              <Property label="HAK requis" value={moduleInfo.hakFiles.length.toLocaleString("fr-FR")} />
              <Property label="TLK personnalisé" value={moduleInfo.customTlk ?? "Aucun"} />
            </div>
          )}
          {selectedResource && (
            <div className="property-section">
              <h2>Ressource sélectionnée</h2>
              <Property label="Clé" value={resourceKeyName(selectedResource.key)} />
              <Property label="Source" value={selectedResource.selected.sourceKind} />
              <Property label="Priorité" value={selectedResource.selected.priority.toString()} />
              <Property
                label="Taille"
                value={`${selectedResource.selected.size.toLocaleString("fr-FR")} octets`}
              />
              <Property label="Versions masquées" value={selectedResource.shadowed.length.toString()} />
              <code className="source-path">{selectedResource.selected.sourcePath}</code>
              {![2009, 2010].includes(selectedResource.key.resourceType) && (
                <button type="button" className="resource-script-link" onClick={() => { setResourceFilter(selectedResource.key.resref); setActiveExplorerItem("scripts"); }}>
                  <Code2 size={13} /> Rechercher les scripts liés
                </button>
              )}
              {![2009, 2010, 2029].includes(selectedResource.key.resourceType) && (
                <button type="button" className="resource-script-link" onClick={() => { setResourceFilter(selectedResource.key.resref); setActiveExplorerItem("dialogues"); }}>
                  <MessageSquareText size={13} /> Rechercher les dialogues liés
                </button>
              )}
              {inspectionBusy && <small>Lecture bornée en cours…</small>}
              {inspection && (
                <>
                  {inspection.kind === "gff" && editWorkspace && (
                    <GffFieldEditor document={inspection.value} onCommit={editSelectedGff} onStructure={editSelectedBlueprintStructure} />
                  )}
                  {inspection.kind === "two_da" && editWorkspace && jobId && selectedResource && (
                    <TwoDaEditor
                      table={inspection.value}
                      onCommit={async (action) => {
                        try {
                          const result = await editWorkspaceTwoDa({ jobId, workspaceId: editWorkspace.workspaceId, resource: selectedResource.key, action });
                          setEditWorkspace(result.workspace);
                          setInspection({ kind: "two_da", value: result.document });
                        } catch (error) { pushError(error); throw error; }
                      }}
                    />
                  )}
                  {inspection.kind === "tlk" && editWorkspace && jobId && selectedResource && (
                    <TlkEditor
                      table={inspection.value}
                      onCommit={async (action) => {
                        try {
                          const result = await editWorkspaceTlk({ jobId, workspaceId: editWorkspace.workspaceId, resource: selectedResource.key, action });
                          setEditWorkspace(result.workspace);
                          setInspection({ kind: "tlk", value: result.document });
                        } catch (error) { pushError(error); throw error; }
                      }}
                    />
                  )}
                  <pre className="raw-inspector">{JSON.stringify(inspection.value, null, 2)}</pre>
                </>
              )}
            </div>
          )}
          {dependencyReport && dependencyReport.dependencies.length > 0 && (
            <div className="property-section">
              <h2>Dépendances</h2>
              <Property label="Résolues" value={dependencyReport.resolvedCount.toLocaleString("fr-FR")} />
              <Property label="Introuvables" value={dependencyReport.missingCount.toLocaleString("fr-FR")} />
              <Property label="Non vérifiées" value={dependencyReport.uncheckedCount.toLocaleString("fr-FR")} />
              <Property label="Modifiées" value={dependencyReport.changedCount.toLocaleString("fr-FR")} />
            </div>
          )}
        </aside>
      </section>

      <section className="diagnostics panel" aria-label="Diagnostics">
        <div className="diagnostic-tabs">
          <button type="button" className="active">Diagnostics</button>
          <button type="button">Import</button>
          <button type="button">Journal</button>
          <span>{visibleDiagnostics.length} message{visibleDiagnostics.length > 1 ? "s" : ""}</span>
        </div>
        <div className="diagnostic-list">
          {visibleDiagnostics.map((item) => (
            <div key={item.id} className={`diagnostic-row ${item.level}`}>
              {item.level === "error" ? <AlertTriangle size={14} /> : <ShieldCheck size={14} />}
              <code>{item.code}</code>
              <span>{item.message}</span>
            </div>
          ))}
          {job?.error && (
            <div className="diagnostic-row error">
              <AlertTriangle size={14} />
              <code>{job.error.code}</code>
              <span>{job.error.userMessage}</span>
            </div>
          )}
        </div>
      </section>
    </main>
  );
}

function GffFieldEditor({
  document,
  onCommit,
  onStructure,
}: {
  document: GenericGff;
  onCommit: (path: string, before: GenericGffValue, after: GenericGffValue) => Promise<void>;
  onStructure: (action: BlueprintStructureAction) => Promise<void>;
}) {
  const blueprint=blueprintFileTypes.has(document.fileType);
  const editable = document.root.fields.filter((field) =>
    ["string", "res_ref", "localized_string", "byte", "char", "word", "short", "dword", "int", "float", "double"].includes(field.value.kind)&&(!blueprint||blueprintFieldLabel(field.label)!==undefined),
  );
  if (editable.length === 0) return null;
  const limit=blueprint?32:12;
  return (
    <div className="gff-field-editor">
      <strong>{blueprint?`Blueprint ${document.fileType.trim()} typé`:"Champs éditables"}</strong>
      {editable.slice(0, limit).map((field) => (
        <EditableGffField
          key={`${field.label}-${JSON.stringify(field.value)}`}
          label={blueprintFieldLabel(field.label)??field.label}
          value={field.value}
          boolean={blueprintBooleanFields.has(field.label)}
          onCommit={(after) => onCommit(`/${field.label}`, field.value, after)}
        />
      ))}
      {editable.length > limit && <small>{editable.length - limit} autre(s) champ(s) restent visibles dans la vue brute.</small>}
      {blueprint&&<BlueprintStructureEditor document={document} onCommit={onCommit} onStructure={onStructure}/>}
    </div>
  );
}

const blueprintFileTypes=new Set(["UTC ","UTI ","UTP ","UTD ","UTE ","UTT ","UTS ","UTM ","UTW "]);
const blueprintBooleanFields=new Set(["Plot","Identified","Cursed","Stolen","Static","Useable","Interruptable","AutoRemoveKey","KeyRequired","Lockable","Locked","TrapDetectable","TrapDisarmable","TrapFlag","TrapOneShot","Active","Continuous","Looping","Random","RandomPosition","StartsEnabled","BlackMarket","MapNoteEnabled","HasMapNote"]);
const blueprintLabels:Record<string,string>={Tag:"Tag",TemplateResRef:"ResRef du template",LocalizedName:"Nom localisé",LocName:"Nom localisé",FirstName:"Prénom localisé",LastName:"Nom localisé",Description:"Description localisée",DescIdentified:"Description identifiée",Conversation:"Dialogue",Comment:"Commentaire",Appearance_Type:"Apparence",Appearance:"Apparence",Race:"Race",Gender:"Genre",FactionID:"Faction",ChallengeRating:"Facteur de puissance",BaseItem:"Type d'objet",Cost:"Coût",StackSize:"Taille de pile",Charges:"Charges",Plot:"Intrigue",Identified:"Identifié",Cursed:"Maudit",Stolen:"Volé",Static:"Statique",Useable:"Utilisable",Interruptable:"Interruptible",AnimationState:"Animation",BodyBag:"Sac mortuaire",HP:"Points de vie",CurrentHP:"PV actuels",Hardness:"Solidité",LinkedTo:"Transition liée",LinkedToFlags:"Type de transition",KeyName:"Clé",KeyRequired:"Clé requise",AutoRemoveKey:"Retirer la clé",Lockable:"Verrouillable",Locked:"Verrouillé",OpenLockDC:"DD crochetage",CloseLockDC:"DD verrouillage",TrapDetectable:"Piège détectable",TrapDisarmable:"Piège désamorçable",TrapFlag:"Piégé",TrapOneShot:"Piège unique",TrapType:"Type de piège",DisarmDC:"DD désamorçage",DetectDC:"DD détection",Active:"Actif",Difficulty:"Difficulté",DifficultyIndex:"Indice de difficulté",MaxCreatures:"Créatures max.",SpawnOption:"Option d'apparition",LoadScreenID:"Écran de chargement",PaletteID:"Palette",PortraitId:"Portrait",SoundSetFile:"Voix",Continuous:"Continu",Looping:"Boucle",Random:"Aléatoire",RandomPosition:"Position aléatoire",Volume:"Volume",Hours:"Heure",Times:"Répétitions",StartsEnabled:"Activé au départ",BlackMarket:"Marché noir",MarkDown:"Rachat (%)",MarkUp:"Vente (%)",MapNote:"Note de carte",MapNoteEnabled:"Note visible",HasMapNote:"Note présente"};
function blueprintFieldLabel(label:string){return blueprintLabels[label]}

type BlueprintListDefinition={label:string;title:string;kind?:BlueprintListKind};
const blueprintListDefinitions:Record<string,BlueprintListDefinition[]>={
  "UTC ":[{label:"SkillList",title:"Compétences"},{label:"FeatList",title:"Dons",kind:"feat"},{label:"SpecAbilityList",title:"Capacités spéciales",kind:"special_ability"},{label:"ClassList",title:"Classes",kind:"class"},{label:"Equip_ItemList",title:"Équipement",kind:"equipped_item"}],
  "UTI ":[{label:"PropertiesList",title:"Propriétés d’objet",kind:"item_property"}],
  "UTS ":[{label:"Sounds",title:"Variantes sonores",kind:"sound"}],
  "UTE ":[{label:"CreatureList",title:"Créatures de la rencontre",kind:"encounter_creature"}],
};

function BlueprintStructureEditor({document,onCommit,onStructure}:{document:GenericGff;onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>;onStructure:(action:BlueprintStructureAction)=>Promise<void>}) {
  const definitions=blueprintListDefinitions[document.fileType]??[];
  if(!definitions.length)return null;
  return <div className="blueprint-structure-editor"><strong>Sous-structures {document.fileType.trim()}</strong><BlueprintAddEntryForm fileType={document.fileType} onCommit={onStructure}/>{definitions.map(definition=>{const field=document.root.fields.find(candidate=>candidate.label===definition.label&&candidate.value.kind==="list");const entries=(field?.value.value as GenericGff["root"][]|undefined)??[];if(!field)return null;return <details key={definition.label}><summary>{definition.title} · {entries.length}</summary><div className="blueprint-list-entries">{entries.map((entry,entryIndex)=><div className="blueprint-list-entry" key={`${definition.label}-${entryIndex}`}><code>#{entryIndex}</code>{entry.fields.filter(candidate=>["string","res_ref","byte","char","word","short","dword","int","float","double"].includes(candidate.value.kind)).map(candidate=><EditableGffField key={candidate.label} label={blueprintFieldLabel(candidate.label)??candidate.label} value={candidate.value} boolean={["SingleSpawn"].includes(candidate.label)} onCommit={after=>onCommit(`/${field.label}/${entryIndex}/${candidate.label}`,candidate.value,after)}/>)}{definition.kind&&<BlueprintStructureButton label="Supprimer" action={{kind:"remove_entry",listKind:definition.kind,entryIndex}} onCommit={onStructure}/>}</div>)}</div></details>})}</div>;
}

function BlueprintAddEntryForm({fileType,onCommit}:{fileType:string;onCommit:(action:BlueprintStructureAction)=>Promise<void>}) {
  if(fileType==="UTC ")return <BlueprintUtcAddForm onCommit={onCommit}/>;
  if(fileType==="UTI ")return <BlueprintItemPropertyAddForm onCommit={onCommit}/>;
  if(fileType==="UTS ")return <BlueprintResrefAddForm label="Son" button="+ Son" makeAction={resref=>({kind:"add_sound",resref})} onCommit={onCommit}/>;
  if(fileType==="UTE ")return <BlueprintEncounterAddForm onCommit={onCommit}/>;
  return null;
}

function BlueprintUtcAddForm({onCommit}:{onCommit:(action:BlueprintStructureAction)=>Promise<void>}) {
  const [mode,setMode]=useState<"feat"|"special"|"class"|"equipped">("feat");const [primary,setPrimary]=useState(0);const [secondary,setSecondary]=useState(1);const [tertiary,setTertiary]=useState(1);const [resref,setResref]=useState("");
  const action=():BlueprintStructureAction=>mode==="feat"?{kind:"add_feat",featId:primary}:mode==="special"?{kind:"add_special_ability",spellId:primary,casterLevel:secondary,flags:tertiary}:mode==="class"?{kind:"add_class",classId:primary,classLevel:secondary}:{kind:"add_equipped_item",resref,slot:primary};
  return <div className="blueprint-add-form"><select aria-label="Sous-structure UTC" value={mode} onChange={event=>setMode(event.currentTarget.value as typeof mode)}><option value="feat">Don</option><option value="special">Capacité spéciale</option><option value="class">Classe</option><option value="equipped">Objet équipé</option></select>{mode==="equipped"&&<input aria-label="ResRef équipé" maxLength={16} value={resref} onChange={event=>setResref(event.currentTarget.value.toLocaleLowerCase())}/>}<label>{mode==="equipped"?"Slot":"ID"}<input type="number" min={0} value={primary} onChange={event=>setPrimary(Number(event.currentTarget.value))}/></label>{["special","class"].includes(mode)&&<label>{mode==="class"?"Niveau":"Niveau lanceur"}<input type="number" min={0} max={60} value={secondary} onChange={event=>setSecondary(Number(event.currentTarget.value))}/></label>}{mode==="special"&&<label>Flags<input type="number" min={0} max={255} value={tertiary} onChange={event=>setTertiary(Number(event.currentTarget.value))}/></label>}<BlueprintStructureButton label="+ Ajouter" action={action()} onCommit={onCommit} disabled={mode==="equipped"&&!resref}/></div>;
}

function BlueprintItemPropertyAddForm({onCommit}:{onCommit:(action:BlueprintStructureAction)=>Promise<void>}) {
  const [values,setValues]=useState({propertyName:0,subtype:0,costTable:0,costValue:0,param1:0,param1Value:0,chanceAppear:100});
  const set=(key:keyof typeof values,value:number)=>setValues(current=>({...current,[key]:value}));
  return <div className="blueprint-add-form item-property-add">{([['propertyName','Propriété'],['subtype','Sous-type'],['costTable','Table coût'],['costValue','Valeur coût'],['param1','Paramètre'],['param1Value','Valeur paramètre'],['chanceAppear','Chance %']] as Array<[keyof typeof values,string]>).map(([key,label])=><label key={key}>{label}<input type="number" min={0} max={key==="chanceAppear"?100:65535} value={values[key]} onChange={event=>set(key,Number(event.currentTarget.value))}/></label>)}<BlueprintStructureButton label="+ Propriété" action={{kind:"add_item_property",...values}} onCommit={onCommit}/></div>;
}

function BlueprintResrefAddForm({label,button,makeAction,onCommit}:{label:string;button:string;makeAction:(resref:string)=>BlueprintStructureAction;onCommit:(action:BlueprintStructureAction)=>Promise<void>}) {
  const [resref,setResref]=useState("");return <div className="blueprint-add-form"><label>{label}<input maxLength={16} value={resref} onChange={event=>setResref(event.currentTarget.value.toLocaleLowerCase())}/></label><BlueprintStructureButton label={button} action={makeAction(resref)} onCommit={onCommit} disabled={!resref}/></div>;
}

function BlueprintEncounterAddForm({onCommit}:{onCommit:(action:BlueprintStructureAction)=>Promise<void>}) {
  const [resref,setResref]=useState("");const [appearance,setAppearance]=useState(0);const [challengeRating,setChallengeRating]=useState(0);const [singleSpawn,setSingleSpawn]=useState(false);return <div className="blueprint-add-form"><label>ResRef UTC<input maxLength={16} value={resref} onChange={event=>setResref(event.currentTarget.value.toLocaleLowerCase())}/></label><label>Apparence<input type="number" min={0} value={appearance} onChange={event=>setAppearance(Number(event.currentTarget.value))}/></label><label>FP<input type="number" min={0} step="0.25" value={challengeRating} onChange={event=>setChallengeRating(Number(event.currentTarget.value))}/></label><label className="checkbox-label"><input type="checkbox" checked={singleSpawn} onChange={event=>setSingleSpawn(event.currentTarget.checked)}/> Unique</label><BlueprintStructureButton label="+ Créature" action={{kind:"add_encounter_creature",resref,appearance,challengeRating,singleSpawn}} onCommit={onCommit} disabled={!resref}/></div>;
}

function BlueprintStructureButton({label,action,onCommit,disabled=false}:{label:string;action:BlueprintStructureAction;onCommit:(action:BlueprintStructureAction)=>Promise<void>;disabled?:boolean}) {
  const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const run=async()=>{setBusy(true);setMessage("");try{await onCommit(action)}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};return <span className="structure-action"><button type="button" disabled={disabled||busy} onClick={()=>void run()}>{busy?"…":label}</button>{message&&<small>{message}</small>}</span>;
}

function EditableGffField({
  label,
  value,
  boolean=false,
  onCommit,
}: {
  label: string;
  value: GenericGffValue;
  boolean?:boolean;
  onCommit: (after: GenericGffValue) => Promise<void>;
}) {
  if(value.kind==="localized_string")return <EditableLocalizedDialogueField field={{label,path:label,value}} onCommit={(_,__,after)=>onCommit(after)}/>;
  if(boolean&&isIntegerGffValue(value))return <EditableBooleanGffField field={{label,path:label,value}} onCommit={(_,__,after)=>onCommit(after)}/>;
  return <EditableScalarGffField label={label} value={value} onCommit={onCommit}/>;
}

function EditableScalarGffField({label,value,onCommit}:{label:string;value:GenericGffValue;onCommit:(after:GenericGffValue)=>Promise<void>}) {
  const [draft, setDraft] = useState(String(value.value ?? ""));
  const [busy, setBusy] = useState(false);
  const original = String(value.value ?? "");
  async function commit() {
    setBusy(true);
    try {
      const nextValue = ["string", "res_ref"].includes(value.kind) ? draft : Number(draft);
      await onCommit({ kind: value.kind, value: nextValue });
    } finally {
      setBusy(false);
    }
  }
  return (
    <label className="gff-field-row">
      <span>{label}</span>
      <input value={draft} onChange={(event) => setDraft(event.currentTarget.value)} />
      <button type="button" disabled={busy || draft === original || (!draft && !["string", "res_ref"].includes(value.kind))} onClick={commit}>
        {busy ? "…" : "Appliquer"}
      </button>
    </label>
  );
}

function DependencyReportView({ report, moduleInfo, jobId, editWorkspace, onWorkspace, onError }: {
  report: ModuleDependencyReport;
  moduleInfo: { hakFiles: string[]; customTlk: string | null } | undefined;
  jobId: string | undefined;
  editWorkspace: WorkspaceSnapshot | undefined;
  onWorkspace: (workspace: WorkspaceSnapshot) => void;
  onError: (error: unknown) => void;
}) {
  return (
    <section className="dependency-card" aria-label="Dépendances du module">
      <div className="dependency-heading">
        <div>
          <span className="eyebrow">RÉSOLUTION EN LECTURE SEULE</span>
          <h2>Dépendances du module</h2>
        </div>
        <span className="dependency-summary">
          {report.resolvedCount}/{report.dependencies.length} résolue(s)
          {report.changedCount > 0 && ` · ${report.changedCount} modifiée(s)`}
        </span>
      </div>
      {report.dependencies.length === 0 ? (
        <p className="dependency-empty">Ce module ne déclare aucun HAK ni TLK personnalisé.</p>
      ) : (
        <div className="dependency-list">
          {report.dependencies.map((dependency, index) => (
            <DependencyRow
              key={`${dependency.kind}-${dependency.logicalName}-${index}`}
              dependency={dependency}
            />
          ))}
        </div>
      )}
      {moduleInfo && jobId && editWorkspace && (
        <DependencyEditor
          initialHakFiles={moduleInfo.hakFiles}
          initialCustomTlk={moduleInfo.customTlk}
          onCommit={async (hakFiles, customTlk) => {
            try {
              const result = await editWorkspaceModuleDependencies({
                jobId,
                workspaceId: editWorkspace.workspaceId,
                hakFiles,
                customTlk,
              });
              onWorkspace(result.workspace);
            } catch (error) {
              onError(error);
              throw error;
            }
          }}
        />
      )}
    </section>
  );
}

function DependencyEditor({ initialHakFiles, initialCustomTlk, onCommit }: {
  initialHakFiles: string[];
  initialCustomTlk: string | null;
  onCommit: (hakFiles: string[], customTlk: string | null) => Promise<void>;
}) {
  const [haks, setHaks] = useState(initialHakFiles.join("\n"));
  const [tlk, setTlk] = useState(initialCustomTlk ?? "");
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);
  async function commit() {
    setBusy(true);
    setSaved(false);
    try {
      const hakFiles = haks.split(/\r?\n/).map((value) => value.trim()).filter(Boolean);
      await onCommit(hakFiles, tlk.trim() || null);
      setSaved(true);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="dependency-editor">
      <strong>Gestionnaire HAK/TLK</strong>
      <label>HAK, dans l’ordre de priorité<textarea value={haks} onChange={(event) => setHaks(event.currentTarget.value)} placeholder="contenu.hak" /></label>
      <label>TLK personnalisé<input value={tlk} onChange={(event) => setTlk(event.currentTarget.value)} placeholder="dialog.tlk" /></label>
      <button type="button" disabled={busy} onClick={() => void commit()}>{busy ? "Validation…" : "Appliquer à module.ifo"}</button>
      {saved && <small>Déclarations relues depuis l’overlay et ajoutées à l’historique annulable.</small>}
    </div>
  );
}

function TwoDaEditor({ table, onCommit }: { table: TwoDaTable; onCommit: (action: TwoDaEditAction) => Promise<void> }) {
  const [label, setLabel] = useState("");
  const visibleRows = table.rows.slice(0, 100);
  return (
    <div className="table-editor">
      <div className="table-editor-heading"><strong>Éditeur 2DA</strong><span>{table.rows.length} ligne(s) · {table.columns.length} colonne(s)</span></div>
      <div className="table-editor-grid" style={{ gridTemplateColumns: `72px repeat(${Math.max(1, table.columns.length)}, minmax(110px, 1fr)) 34px` }}>
        <b>Label</b>{table.columns.map((column) => <b key={column}>{column}</b>)}<b />
        {visibleRows.map((row, rowIndex) => (
          <div className="table-editor-row" style={{ display: "contents" }} key={`${row.label}-${rowIndex}`}>
            <code>{row.label}</code>
            {table.columns.map((column, columnIndex) => (
              <TwoDaCellEditor key={column} value={row.cells[columnIndex] ?? null} onCommit={(value) => onCommit({ kind: "set_cell", rowIndex, columnIndex, value })} />
            ))}
            <button type="button" title="Supprimer la ligne" onClick={() => void onCommit({ kind: "remove_row", rowIndex })}>×</button>
          </div>
        ))}
      </div>
      {table.rows.length > visibleRows.length && <small>Affichage borné aux 100 premières lignes ; la ressource complète reste conservée.</small>}
      <div className="table-editor-add"><input value={label} onChange={(event) => setLabel(event.currentTarget.value)} placeholder="Nouveau label" /><button type="button" disabled={!label.trim()} onClick={() => { void onCommit({ kind: "add_row", label: label.trim() }); setLabel(""); }}>+ Ligne</button></div>
    </div>
  );
}

function TwoDaCellEditor({ value, onCommit }: { value: string | null; onCommit: (value: string | null) => Promise<void> }) {
  const [draft, setDraft] = useState(value ?? "");
  const original = value ?? "";
  return <input aria-label="Cellule 2DA" value={draft} placeholder="****" onChange={(event) => setDraft(event.currentTarget.value)} onBlur={() => { if (draft !== original) void onCommit(draft || null); }} />;
}

function TlkEditor({ table, onCommit }: { table: TalkTable; onCommit: (action: TlkEditAction) => Promise<void> }) {
  const [newText, setNewText] = useState("");
  return (
    <div className="table-editor tlk-editor">
      <div className="table-editor-heading"><strong>Éditeur TLK</strong><span>Langue {table.languageId} · {table.entries.length} entrée(s)</span></div>
      {table.entries.slice(0, 100).map((entry) => <TlkEntryEditor key={entry.index} entry={entry} onCommit={onCommit} />)}
      {table.entries.length > 100 && <small>Affichage borné aux 100 premières entrées.</small>}
      <div className="table-editor-add"><input value={newText} onChange={(event) => setNewText(event.currentTarget.value)} placeholder="Nouvelle chaîne" /><button type="button" onClick={() => { void onCommit({ kind: "append_entry", text: newText || null }); setNewText(""); }}>+ Entrée</button></div>
    </div>
  );
}

function TlkEntryEditor({ entry, onCommit }: { entry: TalkTable["entries"][number]; onCommit: (action: TlkEditAction) => Promise<void> }) {
  const [text, setText] = useState(entry.text ?? "");
  const [sound, setSound] = useState(entry.soundResref ?? "");
  return (
    <div className="tlk-entry-row">
      <code>{entry.index}</code>
      <textarea value={text} onChange={(event) => setText(event.currentTarget.value)} />
      <input value={sound} onChange={(event) => setSound(event.currentTarget.value)} placeholder="sound resref" />
      <button type="button" disabled={text === (entry.text ?? "") && sound === (entry.soundResref ?? "")} onClick={() => void onCommit({ kind: "set_entry", index: entry.index, text: text || null, soundResref: sound || null, soundLength: entry.soundLength })}>Appliquer</button>
    </div>
  );
}

function BuildProfilesPanel({ workspace, userDataPath, onError }: { workspace: WorkspaceSnapshot; userDataPath: string; onError: (error: unknown) => void }) {
  const [profile, setProfile] = useState<ModuleBuildProfile>({ name: "Test local", outputName: "opennever-test.mod", blockOnWarnings: true, deployDevelopment: false, hakFiles: [], customTlk: null });
  const [profiles, setProfiles] = useState<ModuleBuildProfile[]>([]);
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState("");
  const [git, setGit] = useState<GitWorkspaceStatus>();
  const [launchProfiles, setLaunchProfiles] = useState<NwnLaunchProfile[]>([]);
  const [launchProfile, setLaunchProfile] = useState<NwnLaunchProfile>({ name: "Client local", mode: "client", executablePath: "", workingDirectory: "", arguments: [] });
  useEffect(() => {
    void listWorkspaceBuildProfiles({ workspaceId: workspace.workspaceId }).then(setProfiles).catch(onError);
    void listWorkspaceLaunchProfiles({ workspaceId: workspace.workspaceId }).then(setLaunchProfiles).catch(onError);
  }, [workspace.workspaceId]);
  async function execute(action: "save" | "verify" | "run" | "git") {
    setBusy(true);
    try {
      if (action === "save") {
        setProfiles(await saveWorkspaceBuildProfile({ workspaceId: workspace.workspaceId, profile }));
        setResult("Profil sauvegardé dans le workspace.");
      } else if (action === "verify") {
        const verification = await verifyWorkspaceReproducibleBuild({ workspaceId: workspace.workspaceId, profile });
        setResult(verification.identical ? `Build reproductible confirmé · ${verification.firstSha256.slice(0, 16)}…${verification.warnings.length ? ` · ${verification.warnings.length} avertissement(s)` : ""}` : "Échec : les deux builds diffèrent.");
      } else if (action === "run") {
        const outputDirectory = await selectDirectory();
        if (!outputDirectory) return;
        const report = await runWorkspaceBuildProfile({ workspaceId: workspace.workspaceId, profile, outputDirectory, userDataPath: userDataPath || null });
        setResult(`Build ${report.build.sha256.slice(0, 16)}… · ${report.build.resourceCount} ressource(s)${report.deployment ? " · development déployé" : ""}${report.warnings.length ? ` · ${report.warnings.length} avertissement(s)` : ""}.`);
      } else {
        const root = await selectDirectory();
        if (!root) return;
        const status = await inspectGitWorkspace({ root });
        setGit(status);
        setResult(status.clean ? "Dépôt Git propre." : `${status.files.length} changement(s) Git visible(s).`);
      }
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }
  async function executeLaunch(action: "save" | "launch") {
    setBusy(true);
    try {
      if (action === "save") {
        setLaunchProfiles(await saveWorkspaceLaunchProfile({ workspaceId: workspace.workspaceId, profile: launchProfile }));
        setResult("Profil de test NWN sauvegardé.");
      } else {
        const report = await launchWorkspaceTestProfile({ workspaceId: workspace.workspaceId, profile: launchProfile });
        setResult(`Processus ${report.processId} lancé · journal ${report.logPath}.`);
      }
    } catch (error) { onError(error); } finally { setBusy(false); }
  }
  return (
    <section className="build-profile-card" aria-label="Profils de build et intégration Git">
      <div className="dependency-heading"><div><span className="eyebrow">LOT 22 · REPRODUCTIBILITÉ</span><h2>Profils de build et Git</h2></div><span>{profiles.length} profil(s)</span></div>
      <div className="build-profile-form">
        <label>Nom<input value={profile.name} onChange={(event) => setProfile({ ...profile, name: event.currentTarget.value })} /></label>
        <label>Fichier MOD<input value={profile.outputName} onChange={(event) => setProfile({ ...profile, outputName: event.currentTarget.value })} /></label>
        <label>HAK attendus<input value={profile.hakFiles.join(", ")} onChange={(event) => setProfile({ ...profile, hakFiles: event.currentTarget.value.split(",").map((value) => value.trim()).filter(Boolean) })} placeholder="contenu.hak, correctifs.hak" /></label>
        <label>TLK attendu<input value={profile.customTlk ?? ""} onChange={(event) => setProfile({ ...profile, customTlk: event.currentTarget.value.trim() || null })} placeholder="dialog.tlk" /></label>
        <label className="check-row"><input type="checkbox" checked={profile.blockOnWarnings} onChange={(event) => setProfile({ ...profile, blockOnWarnings: event.currentTarget.checked })} /> Bloquer sur avertissements</label>
        <label className="check-row"><input type="checkbox" checked={profile.deployDevelopment} onChange={(event) => setProfile({ ...profile, deployDevelopment: event.currentTarget.checked })} /> Déployer dans development</label>
      </div>
      <div className="profile-actions"><button type="button" disabled={busy} onClick={() => void execute("save")}>Sauvegarder</button><button type="button" disabled={busy || !workspace.modifiedResources.length} onClick={() => void execute("verify")}>Vérifier ×2</button><button type="button" disabled={busy || !workspace.modifiedResources.length} onClick={() => void execute("run")}>Construire</button><button type="button" disabled={busy} onClick={() => void execute("git")}><GitBranch size={13} /> Inspecter Git</button></div>
      {result && <p className="profile-result">{result}</p>}
      {git && <div className="git-status"><strong>{git.branch || "HEAD détachée"}</strong><code>{git.head?.slice(0, 12) ?? "aucun commit"}</code><span>{git.clean ? "Propre" : `${git.files.length} fichier(s) modifié(s)`}</span>{git.files.slice(0, 20).map((file) => <small key={`${file.indexStatus}${file.worktreeStatus}${file.path}`}><code>{file.indexStatus}{file.worktreeStatus}</code> {file.path}</small>)}</div>}
      <div className="launch-profile-section">
        <div className="table-editor-heading"><strong>Profil de test NWN</strong><span>{launchProfiles.length} profil(s)</span></div>
        <div className="build-profile-form">
          <label>Nom<input value={launchProfile.name} onChange={(event) => setLaunchProfile({ ...launchProfile, name: event.currentTarget.value })} /></label>
          <label>Mode<select value={launchProfile.mode} onChange={(event) => setLaunchProfile({ ...launchProfile, mode: event.currentTarget.value as "client" | "server" })}><option value="client">Client nwmain</option><option value="server">Serveur nwserver</option></select></label>
          <label>Exécutable<div className="path-picker"><input value={launchProfile.executablePath} readOnly /><button type="button" onClick={async () => { const path = await selectNwnExecutable(); if (path) setLaunchProfile({ ...launchProfile, executablePath: path }); }}>Parcourir</button></div></label>
          <label>Dossier de travail<div className="path-picker"><input value={launchProfile.workingDirectory} readOnly /><button type="button" onClick={async () => { const path = await selectDirectory(); if (path) setLaunchProfile({ ...launchProfile, workingDirectory: path }); }}>Parcourir</button></div></label>
          <label className="launch-arguments">Arguments, un par ligne<textarea value={launchProfile.arguments.join("\n")} onChange={(event) => setLaunchProfile({ ...launchProfile, arguments: event.currentTarget.value.split(/\r?\n/).filter(Boolean) })} placeholder="-module\nopennever-test" /></label>
        </div>
        <div className="profile-actions"><button type="button" disabled={busy || !launchProfile.executablePath || !launchProfile.workingDirectory} onClick={() => void executeLaunch("save")}>Sauvegarder le test</button><button type="button" disabled={busy || !launchProfile.executablePath || !launchProfile.workingDirectory} onClick={() => void executeLaunch("launch")}>Lancer maintenant</button></div>
      </div>
    </section>
  );
}

function DependencyRow({ dependency }: { dependency: ModuleDependency }) {
  const stateLabels: Record<ModuleDependency["state"], string> = {
    resolved: "Résolue",
    missing: "Introuvable",
    unchecked: "Non vérifiée",
    invalid: "Nom invalide",
  };
  const location =
    dependency.selectedPath ??
    dependency.searchedPaths[0] ??
    "Racine correspondante non renseignée";
  const changeLabels: Partial<Record<ModuleDependency["change"], string>> = {
    content_changed: "Contenu modifié depuis la dernière analyse",
    location_changed: "Source prioritaire modifiée",
    became_available: "Dépendance maintenant disponible",
    became_missing: "Dépendance disparue depuis la dernière analyse",
  };
  const changeLabel = changeLabels[dependency.change];

  return (
    <div className="dependency-row">
      <span className="dependency-kind">{dependency.kind === "hak" ? "HAK" : "TLK"}</span>
      <div className="dependency-detail">
        <strong>{dependency.logicalName}</strong>
        <code title={location}>{location}</code>
        {dependency.fingerprint && (
          <small className="dependency-fingerprint">
            SHA-256 {dependency.fingerprint.sha256.slice(0, 16)}… · {dependency.fingerprint.sizeBytes.toLocaleString("fr-FR")} octets
          </small>
        )}
        {dependency.shadowedPaths.length > 0 && (
          <small className="dependency-shadowed">
            {dependency.shadowedPaths.length} copie(s) masquée(s)
          </small>
        )}
        {changeLabel && <small className="dependency-change">{changeLabel}</small>}
      </div>
      <span className={`dependency-state ${dependency.state}`}>{stateLabels[dependency.state]}</span>
    </div>
  );
}

function diagnosticsForDependencies(report: ModuleDependencyReport | undefined): Diagnostic[] {
  if (!report) return [];

  const result: Diagnostic[] = [];
  for (const [index, dependency] of report.dependencies.entries()) {
    const id = `${dependency.kind}-${dependency.logicalName}-${index}`;
    if (dependency.state === "missing") {
      const disappeared = dependency.change === "became_missing";
      result.push({
        id: `missing-${id}`,
        level: "warning",
        code: disappeared
          ? "DEPENDENCY_BECAME_MISSING"
          : dependency.kind === "hak"
            ? "HAK_NOT_FOUND"
            : "CUSTOM_TLK_NOT_FOUND",
        message: disappeared
          ? `${dependency.logicalName} a disparu depuis la dernière analyse réussie.`
          : `${dependency.logicalName} est déclaré mais absent des emplacements vérifiés.`,
      });
    }
    if (dependency.state === "invalid") {
      result.push({
        id: `invalid-${id}`,
        level: "error",
        code: "DEPENDENCY_NAME_INVALID",
        message: `${dependency.logicalName} n'est pas un nom de dépendance NWN sûr.`,
      });
    }
    if (dependency.shadowedPaths.length > 0) {
      result.push({
        id: `shadowed-${id}`,
        level: "info",
        code: "RESOURCE_SHADOWED",
        message: `${dependency.logicalName} utilise la copie prioritaire ; ${dependency.shadowedPaths.length} autre(s) copie(s) restent visibles.`,
      });
    }
    if (dependency.change === "content_changed") {
      result.push({
        id: `content-changed-${id}`,
        level: "warning",
        code: "DEPENDENCY_CONTENT_CHANGED",
        message: `Le contenu de ${dependency.logicalName} a changé depuis la dernière analyse.`,
      });
    }
    if (dependency.change === "location_changed") {
      result.push({
        id: `location-changed-${id}`,
        level: "warning",
        code: "DEPENDENCY_LOCATION_CHANGED",
        message: `La source prioritaire de ${dependency.logicalName} a changé.`,
      });
    }
    if (dependency.change === "became_available") {
      result.push({
        id: `available-${id}`,
        level: "info",
        code: "DEPENDENCY_BECAME_AVAILABLE",
        message: `${dependency.logicalName} est maintenant disponible.`,
      });
    }
  }
  if (report.uncheckedCount > 0) {
    result.push({
      id: "dependency-roots-not-configured",
      level: "warning",
      code: "DEPENDENCY_ROOTS_NOT_CONFIGURED",
      message: "Renseignez les racines du jeu et des données utilisateur pour vérifier toutes les dépendances.",
    });
  }
  return result;
}

type PathFieldProps = {
  label: string;
  hint: string;
  value: string;
  placeholder: string;
  onChange: (value: string) => void;
  onBrowse: () => void;
};

function PathField({ label, hint, value, placeholder, onChange, onBrowse }: PathFieldProps) {
  return (
    <label className="path-field">
      <span className="field-label">{label}</span>
      <small>{hint}</small>
      <div className="path-control">
        <input
          value={value}
          placeholder={placeholder}
          onChange={(event) => onChange(event.currentTarget.value)}
        />
        <button type="button" onClick={onBrowse} aria-label={`Parcourir : ${label}`}>
          <FolderOpen size={15} />
          Parcourir
        </button>
      </div>
    </label>
  );
}

function JobProgress({ job }: { job: JobSnapshot }) {
  return (
    <div className="job-progress" aria-live="polite">
      <div>
        <span>Analyse du module</span>
        <strong>{job.progress.percent.toFixed(1)} %</strong>
      </div>
      <div className="progress-track">
        <div style={{ width: `${Math.min(job.progress.percent, 100)}%` }} />
      </div>
      <small>État : {job.state.replace("_", " ")}</small>
    </div>
  );
}

type CatalogViewProps = {
  jobId: string;
  summary: ResourceCatalogSummary;
  activeGroup: string;
  filter: string;
  selected?: ResolvedResource;
  onSelect: (resource: ResolvedResource) => void;
};

function CatalogView({ jobId, summary, activeGroup, filter, selected, onSelect }: CatalogViewProps) {
  const [page, setPage] = useState(0);
  const pageSize = 100;
  const groupTypes = resourceTypesByGroup[activeGroup];
  const resourceTypes = activeGroup === "resources" ? [] : Array.from(groupTypes ?? []);
  useEffect(() => setPage(0), [activeGroup, filter]);
  const pageQuery = useQuery({
    queryKey: ["resources", jobId, activeGroup, filter, page],
    queryFn: () => queryResources({ jobId, query: filter, resourceTypes, offset: page * pageSize, limit: pageSize }),
  });
  const resources = pageQuery.data?.items ?? [];
  const total = pageQuery.data?.total ?? 0;
  const pageCount = Math.max(1, Math.ceil(total / pageSize));
  const safePage = Math.min(page, pageCount - 1);

  return (
    <section className="inventory-card" aria-label="Catalogue de ressources">
      <div className="inventory-heading">
        <div>
          <span className="eyebrow">RESOURCE MANAGER · PROVENANCE VALIDÉE</span>
          <h2>Catalogue résolu</h2>
        </div>
        <span className="format-badge">{summary.versionCount} versions</span>
      </div>
      <div className="inventory-metrics">
        <Metric label="Ressources" value={summary.resourceCount.toLocaleString("fr-FR")} />
        <Metric label="Versions masquées" value={summary.shadowedCount.toLocaleString("fr-FR")} />
        <Metric label="Résultats" value={total.toLocaleString("fr-FR")} />
      </div>
      <div className="resource-table" role="table" aria-label="Ressources résolues">
        <div className="resource-row resource-header" role="row">
          <span role="columnheader">ResRef</span>
          <span role="columnheader">Type</span>
          <span role="columnheader">Source</span>
          <span role="columnheader">Masquées</span>
        </div>
        {resources.map((resource) => (
          <button
            type="button"
            className={
              selected?.key.resref === resource.key.resref &&
              selected.key.resourceType === resource.key.resourceType
                ? "resource-row selected"
                : "resource-row"
            }
            role="row"
            key={`${resource.key.resref}-${resource.key.resourceType}`}
            onClick={() => onSelect(resource)}
          >
            <code role="cell">{resource.key.resref}</code>
            <span role="cell">#{resource.key.resourceType}</span>
            <span role="cell">{resource.selected.sourceKind}</span>
            <span role="cell">{resource.shadowed.length.toLocaleString("fr-FR")}</span>
          </button>
        ))}
        {resources.length === 0 && (
          <div className="resource-empty">
            {pageQuery.isLoading ? "Chargement de la page…" : "Aucune ressource ne correspond à cette vue."}
          </div>
        )}
      </div>
      <div className="catalog-pagination">
        <button type="button" disabled={safePage === 0} onClick={() => setPage(Math.max(0, safePage - 1))}>Précédent</button>
        <span>Page {safePage + 1}/{pageCount}</span>
        <button type="button" disabled={safePage + 1 >= pageCount} onClick={() => setPage(safePage + 1)}>Suivant</button>
      </div>
    </section>
  );
}

function DialogueSummaryView({ summary }: { summary: DialogueIndexSummary }) {
  return (
    <section className="inventory-card dialogue-summary-card" aria-label="Index des dialogues">
      <div className="inventory-heading"><div><span className="eyebrow">DLG · ARBRE · GRAPHE</span><h2>Dialogues fidèles</h2></div><span className={summary.diagnostics ? "format-badge warning" : "format-badge"}>{summary.dialogues.toLocaleString("fr-FR")} DLG</span></div>
      <div className="inventory-metrics"><Metric label="Nœuds" value={summary.nodes.toLocaleString("fr-FR")} /><Metric label="Liens" value={summary.links.toLocaleString("fr-FR")} /><Metric label="Partagés" value={summary.sharedNodes.toLocaleString("fr-FR")} /><Metric label="Références" value={summary.references.toLocaleString("fr-FR")} /></div>
      <p className="structured-note">{summary.cycles.toLocaleString("fr-FR")} cycles · {summary.unreachableNodes.toLocaleString("fr-FR")} nœuds inaccessibles · {summary.brokenLinks.toLocaleString("fr-FR")} liens cassés · {summary.scriptLinks.toLocaleString("fr-FR")} liens vers des scripts.</p>
    </section>
  );
}

function DialogueWorkspace({ jobId, summary, filter, editWorkspace, onWorkspace, onOpenScript }: { jobId: string; summary: DialogueIndexSummary; filter: string; editWorkspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void; onOpenScript: (script: string) => void }) {
  const [page, setPage] = useState(0); const [selected, setSelected] = useState<string>(); const pageSize=50;
  useEffect(()=>setPage(0),[filter]);
  const pageQuery=useQuery({queryKey:["dialogues",jobId,filter,page],queryFn:()=>queryDialogues({jobId,query:filter,offset:page*pageSize,limit:pageSize})});
  const items=pageQuery.data?.items??[];
  useEffect(()=>{if(!selected&&items[0])setSelected(items[0].resref);if(selected&&items.length&&!items.some(value=>value.resref===selected))setSelected(items[0].resref)},[items,selected]);
  const graphQuery=useQuery({queryKey:["dialogue",jobId,selected,editWorkspace?.workspaceId,editWorkspace?.cursor],queryFn:()=>inspectDialogue({jobId,resref:selected as string,workspaceId:editWorkspace?.workspaceId}),enabled:Boolean(selected)});
  const total=pageQuery.data?.total??0; const pages=Math.max(1,Math.ceil(total/pageSize));
  return <section className="inventory-card dialogue-workspace" aria-label="Explorateur de dialogues">
    <div className="inventory-heading"><div><span className="eyebrow">STRUCTURE COMPLÈTE · PROVENANCE GFF</span><h2>Dialogues</h2></div><span className="format-badge">{total} résultat(s)</span></div>
    <div className="dialogue-layout"><div className="dialogue-list">{items.map(item=><button type="button" key={item.resref} className={selected===item.resref?"dialogue-list-item selected":"dialogue-list-item"} onClick={()=>setSelected(item.resref)}><span><MessageSquareText size={13}/><code>{item.resref}</code></span><small>{item.nodeCount} nœuds · {item.linkCount} liens · {item.cycleCount} cycles</small>{item.preview&&<em>{item.preview}</em>}</button>)}
      {!items.length&&<p className="resource-empty">{pageQuery.isLoading?"Indexation…":"Aucun dialogue ne correspond."}</p>}
      <div className="catalog-pagination compact"><button type="button" disabled={page===0} onClick={()=>setPage(value=>Math.max(0,value-1))}>‹</button><span>{page+1}/{pages}</span><button type="button" disabled={page+1>=pages} onClick={()=>setPage(value=>value+1)}>›</button></div>
    </div><DialogueGraphView jobId={jobId} graph={graphQuery.data} loading={graphQuery.isLoading} editWorkspace={editWorkspace} onWorkspace={onWorkspace} onOpenScript={onOpenScript}/></div>
    <span className="script-total-hidden">{summary.nodes}</span>
  </section>;
}

function DialogueGraphView({ jobId, graph, loading, editWorkspace, onWorkspace, onOpenScript }: { jobId: string; graph?: DialogueGraph; loading: boolean; editWorkspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void; onOpenScript: (script: string)=>void }) {
  const [tab,setTab]=useState<"tree"|"graph"|"raw">("tree"); const [selectedNode,setSelectedNode]=useState<string>();
  const [editedGraph,setEditedGraph]=useState<DialogueGraph>();
  const [structureBusy,setStructureBusy]=useState(false);const [structureMessage,setStructureMessage]=useState("");
  useEffect(()=>{if(graph)setEditedGraph(graph)},[graph]);
  const currentGraph=editedGraph??graph;
  useEffect(()=>{if(currentGraph&&!currentGraph.nodes.some(node=>node.id===selectedNode))setSelectedNode(currentGraph.roots[0]??currentGraph.nodes[0]?.id)},[currentGraph,selectedNode]);
  if(loading&&!currentGraph)return <div className="dialogue-empty">Ouverture du dialogue…</div>; if(!currentGraph)return <div className="dialogue-empty">Sélectionnez un dialogue.</div>;
  const node=currentGraph.nodes.find(value=>value.id===selectedNode);
  const commitField=async(path:string,before:GenericGffValue,after:GenericGffValue)=>{
    if(!editWorkspace)return;
    const result=await editDialogueField({jobId,workspaceId:editWorkspace.workspaceId,resref:currentGraph.key.resref,path,before,after});
    onWorkspace(result.workspace); setEditedGraph(result.graph);
  };
  const commitStructure=async(action:DialogueStructureAction)=>{
    if(!editWorkspace)return;
    setStructureBusy(true);setStructureMessage("Mise à jour de la structure…");
    try{const result=await editDialogueStructure({jobId,workspaceId:editWorkspace.workspaceId,resref:currentGraph.key.resref,action});onWorkspace(result.workspace);setEditedGraph(result.graph);setStructureMessage("Structure enregistrée dans l'overlay.")}catch(error){setStructureMessage(normalizeAppError(error).technicalMessage);throw error}finally{setStructureBusy(false)}
  };
  return <div className="dialogue-document"><div className="script-tabs"><button type="button" className={tab==="tree"?"active":""} onClick={()=>setTab("tree")}>Arbre simplifié</button><button type="button" className={tab==="graph"?"active":""} onClick={()=>setTab("graph")}>Graphe complet</button><button type="button" className={tab==="raw"?"active":""} onClick={()=>setTab("raw")}>GFF brut</button><strong>{currentGraph.key.resref}</strong></div>
    {editWorkspace&&<div className="dialogue-structure-actions"><strong>Structure DLG</strong><button type="button" disabled={structureBusy} onClick={()=>void commitStructure({kind:"add_node",nodeKind:"entry"}).catch(()=>undefined)}>+ Nœud Entry</button><button type="button" disabled={structureBusy} onClick={()=>void commitStructure({kind:"add_node",nodeKind:"reply"}).catch(()=>undefined)}>+ Nœud Reply</button><DialogueAddLinkEditor graph={currentGraph} source={null} label="Ajouter un départ" onAdd={commitStructure}/>{structureMessage&&<small>{structureMessage}</small>}</div>}
    <div className="dialogue-content">{tab==="tree"?<div className="dialogue-tree">{currentGraph.tree.map(value=><DialogueTreeBranch key={value.nodeId} value={value} onSelect={setSelectedNode}/>)}</div>:tab==="graph"?<DialogueFlow graph={currentGraph} onSelect={setSelectedNode}/>:<pre className="dialogue-raw">{JSON.stringify(currentGraph.raw,null,2)}</pre>}</div>
    <DialogueInspector graph={currentGraph} node={node} editWorkspace={editWorkspace} onCommitField={commitField} onCommitStructure={commitStructure} onOpenScript={onOpenScript}/>
  </div>;
}

function DialogueTreeBranch({ value, onSelect }: { value: DialogueTreeNode; onSelect: (id:string)=>void }) {
  return <div className="dialogue-tree-branch"><button type="button" className={`dialogue-tree-node ${value.kind}`} onClick={()=>onSelect(value.nodeId)}><code>{value.nodeId}</code><span>{value.displayText??"Texte non résolu"}</span>{value.cycle&&<em>cycle</em>}{value.repeated&&<em>lien partagé</em>}</button>{value.children.length>0&&<div className="dialogue-tree-children">{value.children.map((child,index)=><DialogueTreeBranch key={`${child.nodeId}-${index}`} value={child} onSelect={onSelect}/>)}</div>}</div>;
}

function DialogueFlow({ graph, onSelect }: { graph: DialogueGraph; onSelect: (id:string)=>void }) {
  const nodes:Node[]=graph.nodes.map(value=>({id:value.id,position:{x:value.kind==="entry"?0:430,y:value.index*92},data:{label:<div className={`flow-dialogue-node ${value.kind}`}><code>{value.id}</code><span>{value.displayText?.slice(0,100)??"Texte non résolu"}</span></div>},className:graph.sharedNodes.includes(value.id)?"shared":""}));
  const edges:Edge[]=graph.links.filter(value=>value.source&&!value.broken).map(value=>({id:value.id,source:value.source as string,target:value.target,animated:graph.cycles.some(cycle=>cycle.includes(value.source as string)&&cycle.includes(value.target)),label:value.conditionScript??undefined,style:{stroke:value.isChild?"#d59b55":"#5f819e"}}));
  return <div className="dialogue-flow"><ReactFlow nodes={nodes} edges={edges} fitView nodesDraggable={false} nodesConnectable={false} elementsSelectable onNodeClick={(_,value)=>onSelect(value.id)}><Background color="#27323c" gap={24}/><MiniMap pannable zoomable nodeColor={node=>node.id.startsWith("entry")?"#567d9d":"#9a7044"}/><Controls showInteractive={false}/></ReactFlow></div>;
}

function DialogueInspector({ graph, node, editWorkspace, onCommitField, onCommitStructure, onOpenScript }: { graph: DialogueGraph; node?: DialogueGraph["nodes"][number]; editWorkspace?: WorkspaceSnapshot; onCommitField:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>; onCommitStructure:(action:DialogueStructureAction)=>Promise<void>; onOpenScript:(script:string)=>void }) {
  const nodeRef=node?{kind:node.kind,index:node.index} satisfies DialogueNodeRef:undefined;
  return <div className="dialogue-inspection"><div><h3>Nœud sélectionné</h3>{node?<><strong>{node.id}</strong><p>{node.displayText??"Texte non résolu"}</p>{node.speaker&&<span>Locuteur · {node.speaker}</span>}{node.comment&&<span>Commentaire · {node.comment}</span>}{node.animation!==null&&<span>Animation · {node.animation}{node.animationLoop?" · boucle":""}</span>}{node.sound&&<span>Son · {node.sound}</span>}{node.quest&&<span>Quête · {node.quest}</span>}{node.actionScript&&<button type="button" onClick={()=>onOpenScript(node.actionScript as string)}><Code2 size={12}/> Action · {node.actionScript}</button>}{editWorkspace&&<DialogueNodeFieldEditor graph={graph} nodeId={node.id} onCommit={onCommitField}/>} {editWorkspace&&nodeRef&&<><DialogueAddLinkEditor graph={graph} source={nodeRef} label="Ajouter un lien sortant" onAdd={onCommitStructure}/><button type="button" className="danger-button" onClick={()=>void onCommitStructure({kind:"remove_node",node:nodeRef}).catch(()=>undefined)}>Supprimer ce nœud</button></>}</>:<span>Aucun nœud.</span>}</div>
    <div><h3>Scripts et cibles des liens</h3>{graph.links.filter(link=>link.source===node?.id).map(link=><div className="dialogue-link-meta" key={link.id}><span>→ {link.target}{link.isChild?" · partagé":""}</span>{link.conditionScript&&<button type="button" onClick={()=>onOpenScript(link.conditionScript as string)}><Code2 size={12}/> Condition · {link.conditionScript}</button>}{link.actionScript&&<button type="button" onClick={()=>onOpenScript(link.actionScript as string)}><Code2 size={12}/> Action · {link.actionScript}</button>}{editWorkspace&&<DialogueLinkFieldEditor graph={graph} link={link} onCommit={onCommitField} onRemove={onCommitStructure}/>}</div>)}{editWorkspace&&graph.links.filter(link=>link.source===null).map(link=><div className="dialogue-link-meta" key={link.id}><span>Départ → {link.target}</span><DialogueLinkFieldEditor graph={graph} link={link} onCommit={onCommitField} onRemove={onCommitStructure}/></div>)}</div>
    <div><h3>Références entrantes</h3>{graph.references.slice(0,100).map(value=><span key={`${value.resource.resref}-${value.fieldPath}`}>{value.resource.resref}.#{value.resource.resourceType} · {value.fieldPath}</span>)}{graph.references.length===0&&<span>Aucune référence GFF détectée.</span>}</div>
    {graph.diagnostics.length>0&&<div><h3>Diagnostics</h3>{graph.diagnostics.slice(0,50).map((value,index)=><span className="missing" key={`${value.code}-${index}`}>{value.code} · {value.message}</span>)}</div>}
  </div>;
}

function DialogueNodeFieldEditor({ graph, nodeId, onCommit }: { graph: DialogueGraph; nodeId: string; onCommit: (path: string, before: GenericGffValue, after: GenericGffValue) => Promise<void> }) {
  const fields = dialogueNodeEditableFields(graph, nodeId);
  if (!fields.length) return <small>Aucun champ texte existant n'est éditable sur ce nœud.</small>;
  return <div className="gff-field-editor dialogue-node-editor"><strong>Édition DLG transactionnelle</strong>{fields.map((field)=><EditableDialogueField key={field.path} field={field} onCommit={onCommit}/>)}</div>;
}

function dialogueNodeEditableFields(graph: DialogueGraph, nodeId: string): Array<{ label: string; path: string; value: GenericGffValue }> {
  const [kind,indexText]=nodeId.split(":");
  const index=Number(indexText);
  if(!Number.isInteger(index)||index<0)return [];
  const raw=graph.raw as GenericGff;
  const listCandidates=kind==="entry"?["EntryList","EntriesList"]:kind==="reply"?["ReplyList","RepliesList"]:[];
  const listField=raw?.root?.fields?.find((field)=>listCandidates.includes(field.label));
  if(!listField||listField.value.kind!=="list"||!Array.isArray(listField.value.value))return [];
  const child=(listField.value.value as GenericGff["root"][])[index];
  if(!child)return [];
  const labels:Record<string,string>={Text:"Texte localisé",Speaker:"Locuteur",Comment:"Commentaire",Script:"Script d'action",ActionScript:"Script d'action"};
  return child.fields.filter((field)=>Object.hasOwn(labels,field.label)&&["string","res_ref","localized_string"].includes(field.value.kind)).map((field)=>({label:labels[field.label],path:`/${listField.label}/${index}/${field.label}`,value:field.value}));
}

function EditableDialogueField({ field, onCommit }: { field: { label: string; path: string; value: GenericGffValue }; onCommit: (path: string, before: GenericGffValue, after: GenericGffValue) => Promise<void> }) {
  if(field.value.kind==="localized_string")return <EditableLocalizedDialogueField field={field} onCommit={onCommit}/>;
  return <EditableScalarDialogueField field={field} onCommit={onCommit}/>;
}

function EditableScalarDialogueField({ field, onCommit }: { field: { label: string; path: string; value: GenericGffValue }; onCommit: (path: string, before: GenericGffValue, after: GenericGffValue) => Promise<void> }) {
  const original=String(field.value.value??""); const [draft,setDraft]=useState(original); const [busy,setBusy]=useState(false); const [message,setMessage]=useState("");
  useEffect(()=>{setDraft(original);setMessage("")},[original,field.path]);
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(field.path,field.value,{kind:field.value.kind,value:draft});setMessage("Enregistré dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <label className="gff-field-row"><span>{field.label}</span><input value={draft} onChange={(event)=>setDraft(event.currentTarget.value)}/><button type="button" disabled={busy||draft===original} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button>{message&&<small>{message}</small>}</label>;
}

type DialogueLocalizedValue={languageId:number;text:string};
type DialogueLocalizedString={stringRef:number|null;values:DialogueLocalizedValue[]};

function asDialogueLocalizedString(value:unknown):DialogueLocalizedString {
  if(!value||typeof value!=="object")return {stringRef:null,values:[]};
  const candidate=value as {stringRef?:unknown;values?:unknown};
  const stringRef=typeof candidate.stringRef==="number"?candidate.stringRef:null;
  const values=Array.isArray(candidate.values)?candidate.values.flatMap((entry)=>{
    if(!entry||typeof entry!=="object")return [];
    const item=entry as {languageId?:unknown;text?:unknown};
    return typeof item.languageId==="number"&&typeof item.text==="string"?[{languageId:item.languageId,text:item.text}]:[];
  }):[];
  return {stringRef,values};
}

function EditableLocalizedDialogueField({field,onCommit}:{field:{label:string;path:string;value:GenericGffValue};onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>}) {
  const original=asDialogueLocalizedString(field.value.value);
  const originalJson=JSON.stringify(original);
  const [draft,setDraft]=useState<DialogueLocalizedString>(original);
  const [busy,setBusy]=useState(false); const [message,setMessage]=useState("");
  useEffect(()=>{setDraft(original);setMessage("")},[originalJson,field.path]);
  const update=(index:number,text:string)=>setDraft(value=>({...value,values:value.values.map((entry,position)=>position===index?{...entry,text}:entry)}));
  const addVariant=()=>setDraft(value=>{const used=new Set(value.values.map(entry=>entry.languageId));let languageId=0;while(used.has(languageId))languageId+=2;return {...value,values:[...value.values,{languageId,text:""}]}});
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(field.path,field.value,{kind:"localized_string",value:draft});setMessage("Variantes localisées enregistrées dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="gff-field-row localized-dialogue-field"><span>{field.label}{draft.stringRef!==null?` · StrRef ${draft.stringRef}`:""}</span>{draft.values.map((entry,index)=><label key={`${entry.languageId}-${index}`}><small>Langue/genre {entry.languageId}</small><textarea value={entry.text} onChange={(event)=>update(index,event.currentTarget.value)}/></label>)}<div><button type="button" disabled={busy} onClick={addVariant}>Ajouter une variante</button><button type="button" disabled={busy||JSON.stringify(draft)===originalJson} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button></div>{message&&<small>{message}</small>}</div>;
}

function DialogueAddLinkEditor({graph,source,label,onAdd}:{graph:DialogueGraph;source:DialogueNodeRef|null;label:string;onAdd:(action:DialogueStructureAction)=>Promise<void>}) {
  const targetKind=source?.kind==="entry"?"reply":"entry";
  const targets=graph.nodes.filter(node=>node.kind===targetKind);
  const [targetIndex,setTargetIndex]=useState(targets[0]?.index??0);const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{if(!targets.some(node=>node.index===targetIndex))setTargetIndex(targets[0]?.index??0)},[targets,targetIndex]);
  const add=async()=>{setBusy(true);setMessage("Ajout…");try{await onAdd({kind:"add_link",source,target:{kind:targetKind,index:targetIndex}});setMessage("Lien ajouté dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="dialogue-add-link"><label><span>{label}</span><select value={targetIndex} disabled={!targets.length||busy} onChange={event=>setTargetIndex(Number(event.currentTarget.value))}>{targets.map(node=><option key={node.id} value={node.index}>{node.id} · {node.displayText??"Texte non résolu"}</option>)}</select></label><button type="button" disabled={!targets.length||busy} onClick={()=>void add()}>{busy?"…":"Ajouter"}</button>{message&&<small>{message}</small>}</div>;
}

function DialogueLinkFieldEditor({graph,link,onCommit,onRemove}:{graph:DialogueGraph;link:DialogueGraph["links"][number];onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>;onRemove:(action:DialogueStructureAction)=>Promise<void>}) {
  const context=dialogueLinkEditContext(graph,link);
  if(!context)return <small>Structure brute du lien introuvable.</small>;
  const indexField=context.structure.fields.find(field=>field.label==="Index"&&["byte","word","dword"].includes(field.value.kind));
  const textLabels:Record<string,string>={Active:"Condition",Conditional:"Condition",Script:"Action",ActionScript:"Action",LinkComment:"Commentaire",Comment:"Commentaire"};
  const textFields=context.structure.fields.filter(field=>Object.hasOwn(textLabels,field.label)&&["string","res_ref"].includes(field.value.kind));
  const source=dialogueNodeRefFromId(link.source);const position=Number(link.id.split(":").at(-1));
  return <div className="gff-field-editor dialogue-link-editor"><strong>Édition du lien</strong>{indexField&&<EditableDialogueTargetField graph={graph} path={`${context.path}/Index`} value={indexField.value} targetKind={context.targetKind} onCommit={onCommit}/>} {textFields.map(field=><EditableDialogueField key={field.label} field={{label:textLabels[field.label],path:`${context.path}/${field.label}`,value:field.value}} onCommit={onCommit}/>)}<button type="button" className="danger-button" onClick={()=>void onRemove({kind:"remove_link",source,position}).catch(()=>undefined)}>Supprimer ce lien</button></div>;
}

function dialogueNodeRefFromId(id:string|null):DialogueNodeRef|null {
  if(id===null)return null;const [kind,indexText]=id.split(":");const index=Number(indexText);return (kind==="entry"||kind==="reply")&&Number.isInteger(index)&&index>=0?{kind,index}:null;
}

function dialogueLinkEditContext(graph:DialogueGraph,link:DialogueGraph["links"][number]):{path:string;structure:GenericGff["root"];targetKind:"entry"|"reply"}|undefined {
  const raw=graph.raw as GenericGff;
  const position=Number(link.id.split(":").at(-1));
  if(!Number.isInteger(position)||position<0)return undefined;
  if(link.source===null){const list=raw.root.fields.find(field=>field.label==="StartingList"&&field.value.kind==="list");const structure=(list?.value.value as GenericGff["root"][]|undefined)?.[position];return list&&structure?{path:`/${list.label}/${position}`,structure,targetKind:"entry"}:undefined;}
  const [sourceKind,indexText]=link.source.split(":");const sourceIndex=Number(indexText);
  if(!Number.isInteger(sourceIndex)||sourceIndex<0||!(["entry","reply"].includes(sourceKind)))return undefined;
  const nodeCandidates=sourceKind==="entry"?["EntryList","EntriesList"]:["ReplyList","RepliesList"];
  const linkCandidates=sourceKind==="entry"?["RepliesList","ReplyList"]:["EntriesList","EntryList"];
  const nodeList=raw.root.fields.find(field=>nodeCandidates.includes(field.label)&&field.value.kind==="list");
  const node=(nodeList?.value.value as GenericGff["root"][]|undefined)?.[sourceIndex];
  const linkList=node?.fields.find(field=>linkCandidates.includes(field.label)&&field.value.kind==="list");
  const structure=(linkList?.value.value as GenericGff["root"][]|undefined)?.[position];
  return nodeList&&linkList&&structure?{path:`/${nodeList.label}/${sourceIndex}/${linkList.label}/${position}`,structure,targetKind:sourceKind==="entry"?"reply":"entry"}:undefined;
}

function EditableDialogueTargetField({graph,path,value,targetKind,onCommit}:{graph:DialogueGraph;path:string;value:GenericGffValue;targetKind:"entry"|"reply";onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>}) {
  const original=Number(value.value);const [draft,setDraft]=useState(original);const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{setDraft(original);setMessage("")},[original,path]);
  const targets=graph.nodes.filter(node=>node.kind===targetKind);
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(path,value,{kind:value.kind,value:draft});setMessage("Cible enregistrée dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <label className="gff-field-row"><span>Cible</span><select value={draft} onChange={(event)=>setDraft(Number(event.currentTarget.value))}>{targets.map(node=><option value={node.index} key={node.id}>{node.id} · {node.displayText??"Texte non résolu"}</option>)}</select><button type="button" disabled={busy||draft===original} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button>{message&&<small>{message}</small>}</label>;
}

function ScriptSummaryView({ summary }: { summary: ScriptIndexSummary }) {
  return (
    <section className="inventory-card script-summary-card" aria-label="Index NWScript">
      <div className="inventory-heading">
        <div>
          <span className="eyebrow">NSS · NCS · RÉFÉRENCES GFF</span>
          <h2>Index NWScript</h2>
        </div>
        <span className={summary.diagnostics ? "format-badge warning" : "format-badge"}>
          {summary.scripts.toLocaleString("fr-FR")} scripts
        </span>
      </div>
      <div className="inventory-metrics">
        <Metric label="Sources NSS" value={summary.nss.toLocaleString("fr-FR")} />
        <Metric label="Bytecodes NCS" value={summary.ncs.toLocaleString("fr-FR")} />
        <Metric label="Symboles" value={summary.symbols.toLocaleString("fr-FR")} />
        <Metric label="Liens entrants" value={summary.inboundReferences.toLocaleString("fr-FR")} />
      </div>
      <p className="structured-note">
        {summary.includes.toLocaleString("fr-FR")} includes · {summary.calls.toLocaleString("fr-FR")} appels détectés · {summary.missingSource.toLocaleString("fr-FR")} NCS sans source NSS.
      </p>
    </section>
  );
}

function ScriptWorkspace({ jobId, summary, filter, editWorkspace, gameInstallPath, onWorkspace }: { jobId: string; summary: ScriptIndexSummary; filter: string; editWorkspace?: WorkspaceSnapshot; gameInstallPath: string; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [page, setPage] = useState(0);
  const [selected, setSelected] = useState<string>();
  const [tab, setTab] = useState<"source" | "bytecode">("source");
  const pageSize = 80;
  useEffect(() => setPage(0), [filter]);
  const pageQuery = useQuery({
    queryKey: ["scripts", jobId, filter, page],
    queryFn: () => queryScripts({ jobId, query: filter, offset: page * pageSize, limit: pageSize }),
  });
  const items = pageQuery.data?.items ?? [];
  useEffect(() => {
    if (!selected && items[0]) setSelected(items[0].resref);
    if (selected && items.length && !items.some((item) => item.resref === selected)) setSelected(items[0].resref);
  }, [items, selected]);
  const inspectionQuery = useQuery({
    queryKey: ["script", jobId, selected],
    queryFn: () => inspectScript({ jobId, resref: selected as string }),
    enabled: Boolean(selected),
  });
  const document = inspectionQuery.data;
  const total = pageQuery.data?.total ?? 0;
  const pages = Math.max(1, Math.ceil(total / pageSize));

  return (
    <section className="inventory-card script-workspace" aria-label="Explorateur NWScript">
      <div className="inventory-heading">
        <div><span className="eyebrow">RECHERCHE PLEIN TEXTE · MONACO</span><h2>Scripts en lecture seule</h2></div>
        <span className="format-badge">{total.toLocaleString("fr-FR")} résultat(s)</span>
      </div>
      <div className="script-layout">
        <div className="script-list">
          {items.map((item) => (
            <button type="button" key={item.resref} className={selected === item.resref ? "script-list-item selected" : "script-list-item"} onClick={() => { setSelected(item.resref); setTab(item.hasNss ? "source" : "bytecode"); }}>
              <span><Code2 size={13} /><code>{item.resref}</code></span>
              <small>{item.hasNss ? "NSS" : "—"} · {item.hasNcs ? "NCS" : "—"} · {item.inboundReferenceCount} lien(s)</small>
              {item.matches[0] && <em>L{item.matches[0].line} · {item.matches[0].excerpt}</em>}
            </button>
          ))}
          {!items.length && <p className="resource-empty">{pageQuery.isLoading ? "Indexation…" : "Aucun script ne correspond à la recherche."}</p>}
          <div className="catalog-pagination compact">
            <button type="button" disabled={page === 0} onClick={() => setPage((value) => Math.max(0, value - 1))}>‹</button>
            <span>{page + 1}/{pages}</span>
            <button type="button" disabled={page + 1 >= pages} onClick={() => setPage((value) => value + 1)}>›</button>
          </div>
        </div>
        <ScriptDocumentView document={document} loading={inspectionQuery.isLoading} tab={tab} onTab={setTab} jobId={jobId} editWorkspace={editWorkspace} gameInstallPath={gameInstallPath} onWorkspace={onWorkspace} />
      </div>
      <p className="inventory-limit">Le compilateur externe n’est jamais embarqué ni exécuté sans sélection explicite. Le NSS doit être enregistré puis compilé en NCS avant tout build du module.</p>
      <span className="script-total-hidden" aria-hidden="true">{summary.paired}</span>
    </section>
  );
}

function ScriptDocumentView({ document, loading, tab, onTab, jobId, editWorkspace, gameInstallPath, onWorkspace }: { document?: ScriptDocument; loading: boolean; tab: "source" | "bytecode"; onTab: (tab: "source" | "bytecode") => void; jobId: string; editWorkspace?: WorkspaceSnapshot; gameInstallPath: string; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [draft, setDraft] = useState("");
  const [savedText, setSavedText] = useState("");
  const [compilerPath, setCompilerPath] = useState("");
  const [compilation, setCompilation] = useState<CompileResult>();
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    const source = document?.nss?.text ?? "";
    setDraft(source);
    setSavedText(source);
    setCompilation(undefined);
  }, [document?.resref, document?.nss?.text]);
  if (loading) return <div className="script-document-empty">Ouverture du script…</div>;
  if (!document) return <div className="script-document-empty">Sélectionnez un script.</div>;
  const resref = document.resref;
  async function saveSource() {
    if (!editWorkspace || !document?.nss) return;
    setBusy(true);
    try {
      const result = await editScriptSource({ jobId, workspaceId: editWorkspace.workspaceId, resref, before: savedText, after: draft });
      setSavedText(result.document.text);
      setDraft(result.document.text);
      onWorkspace(result.workspace);
      setCompilation(undefined);
    } finally {
      setBusy(false);
    }
  }
  async function chooseCompiler() {
    const selected = await selectCompiler();
    if (selected) setCompilerPath(selected);
  }
  async function compileSource() {
    if (!editWorkspace || !compilerPath || draft !== savedText) return;
    setBusy(true);
    try {
      const result = await compileWorkspaceScript({ jobId, workspaceId: editWorkspace.workspaceId, resref, compilerPath, gameInstallPath });
      setCompilation(result.compilation);
      onWorkspace(result.workspace);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="script-document">
      <div className="script-tabs">
        <button type="button" className={tab === "source" ? "active" : ""} onClick={() => onTab("source")}>Source NSS</button>
        <button type="button" className={tab === "bytecode" ? "active" : ""} onClick={() => onTab("bytecode")}>Bytecode NCS</button>
        <strong>{document.resref}</strong>
      </div>
      {tab === "source" ? (
        document.nss ? (
          <>
            <Editor height="430px" theme="opennever-dark" language="nwscript" value={draft} onChange={(value) => setDraft(value ?? "")} beforeMount={configureNwscriptMonaco} options={{ readOnly: !editWorkspace, domReadOnly: !editWorkspace, automaticLayout: true, minimap: { enabled: false }, fontFamily: "Cascadia Code, Consolas, monospace", fontSize: 12, scrollBeyondLastLine: false, renderWhitespace: "selection" }} />
            {editWorkspace && (
              <div className="script-edit-actions">
                <button type="button" className="secondary-button" disabled={busy || draft === savedText} onClick={saveSource}>Enregistrer NSS</button>
                <button type="button" className="secondary-button" disabled={busy} onClick={chooseCompiler}>{compilerPath ? "Changer de compilateur" : "Choisir nwnsc.exe"}</button>
                <button type="button" className="primary-button" disabled={busy || !compilerPath || !gameInstallPath || draft !== savedText || !document.nss} onClick={compileSource}>Compiler NSS → NCS</button>
                {compilerPath && <code>{compilerPath}</code>}
              </div>
            )}
            {compilation && (
              <div className={compilation.success ? "script-compilation success" : "script-compilation failed"}>
                <strong>{compilation.success ? "Compilation réussie" : "Compilation refusée"}</strong>
                {compilation.ncs && <span>{compilation.ncs.size.toLocaleString("fr-FR")} octets · SHA-256 {compilation.ncs.sha256.slice(0, 16)}…</span>}
                {compilation.diagnostics.map((diagnostic, index) => <span key={`${diagnostic.code}-${index}`}>{diagnostic.line ? `L${diagnostic.line} · ` : ""}{diagnostic.message}</span>)}
              </div>
            )}
            <ScriptMetadata document={document} />
          </>
        ) : <div className="script-document-empty warning"><AlertTriangle size={18} /> Source NSS absente. Le NCS reste consultable dans la vue technique.</div>
      ) : (
        document.ncs ? <NcsTechnicalView document={document} /> : <div className="script-document-empty">Aucun bytecode NCS résolu pour cette source.</div>
      )}
    </div>
  );
}

function ScriptMetadata({ document }: { document: ScriptDocument }) {
  const nss = document.nss;
  if (!nss) return null;
  return (
    <div className="script-metadata">
      <div><h3>Symboles</h3>{nss.symbols.slice(0, 80).map((value) => <button type="button" key={`${value.name}-${value.line}`}>{value.kind} · {value.name} · L{value.line}</button>)}</div>
      <div><h3>Includes</h3>{nss.includes.map((value) => <span className={value.resolved ? "resolved" : "missing"} key={`${value.resref}-${value.line}`}>{value.resref}.nss · L{value.line}</span>)}</div>
      <div><h3>Références entrantes</h3>{document.inboundReferences.slice(0, 100).map((value) => <span key={`${value.resource.resref}-${value.fieldPath}`}>{value.resource.resref}.#{value.resource.resourceType} · {value.fieldPath}</span>)}</div>
    </div>
  );
}

function NcsTechnicalView({ document }: { document: ScriptDocument }) {
  const ncs = document.ncs;
  if (!ncs) return null;
  return (
    <div className="ncs-view">
      <div className="inventory-metrics"><Metric label="En-tête" value={ncs.header} /><Metric label="Octets" value={ncs.size.toLocaleString("fr-FR")} /><Metric label="Bytecode" value={ncs.bytecodeSize.toLocaleString("fr-FR")} /></div>
      {!ncs.validHeader && <p className="inventory-limit">En-tête NCS V1.0 non reconnu.</p>}
      <code className="source-path">SHA-256 · {ncs.sha256}</code>
      <pre>{formatHex(ncs.hexPreview)}</pre>
    </div>
  );
}

function configureNwscriptMonaco(monaco: Monaco) {
  if (!monaco.languages.getLanguages().some((language: { id: string }) => language.id === "nwscript")) monaco.languages.register({ id: "nwscript" });
  monaco.languages.setMonarchTokensProvider("nwscript", {
    keywords: ["break", "case", "const", "continue", "default", "do", "else", "for", "if", "return", "struct", "switch", "while"],
    typeKeywords: ["void", "int", "float", "string", "object", "vector", "location", "effect", "event", "itemproperty", "talent", "sqlquery", "json"],
    tokenizer: { root: [[/[a-zA-Z_]\w*/, { cases: { "@keywords": "keyword", "@typeKeywords": "type", "@default": "identifier" } }], [/\/\/.*$/, "comment"], [/\/\*/, "comment", "@comment"], [/"([^"\\]|\\.)*$/, "string.invalid"], [/"/, "string", "@string"], [/#\s*include/, "keyword.directive"], [/[{}()[\]]/, "@brackets"], [/[0-9]+(\.[0-9]+)?/, "number"]], comment: [[/[^/*]+/, "comment"], [/\*\//, "comment", "@pop"], [/[/*]/, "comment"]], string: [[/[^\\"]+/, "string"], [/\\./, "string.escape"], [/"/, "string", "@pop"]] },
  });
  monaco.editor.defineTheme("opennever-dark", { base: "vs-dark", inherit: true, rules: [{ token: "keyword", foreground: "D59B55" }, { token: "type", foreground: "73B4E8" }, { token: "comment", foreground: "66798A" }], colors: { "editor.background": "#0D1217", "editor.lineHighlightBackground": "#151C23" } });
}

function formatHex(hex: string) { return hex.match(/.{1,32}/g)?.join("\n") ?? hex; }

function WorldSummaryView({ summary }: { summary: WorldSummary }) {
  return <section className="inventory-card world-summary-card" aria-label="Synthèse de la Phase 1">
    <div className="inventory-heading"><div><span className="eyebrow">JRL · FAC · CARTE · ASSETS · SCÈNE · GRAPHE</span><h2>Compréhension globale</h2></div><span className={summary.diagnostics ? "format-badge warning" : "format-badge"}>{summary.graphEdges.toLocaleString("fr-FR")} relations</span></div>
    <div className="inventory-metrics world-metrics"><Metric label="Quêtes" value={summary.journalCategories.toLocaleString("fr-FR")} /><Metric label="Factions" value={summary.factions.toLocaleString("fr-FR")} /><Metric label="Zones" value={summary.areas.toLocaleString("fr-FR")} /><Metric label="Instances" value={summary.instances.toLocaleString("fr-FR")} /><Metric label="Assets inspectés" value={summary.assets.toLocaleString("fr-FR")} /><Metric label="Nœuds globaux" value={summary.graphNodes.toLocaleString("fr-FR")} /></div>
  </section>;
}

function NewModuleCreator({ onCreated }: { onCreated: (path: string) => void }) {
  const [openPanel, setOpenPanel] = useState(false);
  const [definition, setDefinition] = useState({ name: "Nouveau module", tag: "NEW_MODULE", entryArea: "startarea", tileset: "tno01" });
  const [status, setStatus] = useState("");
  const create = async () => {
    const outputPath = await selectModuleOutput("nouveau-module.mod");
    if (!outputPath) return;
    setStatus("Construction du module…");
    try {
      const report = await createNewModule({ outputPath, ...definition });
      setStatus(`${report.resourceCount} ressources créées · ${report.sha256.slice(0, 12)}…`);
      onCreated(report.outputPath);
    } catch (error) { setStatus(normalizeAppError(error).technicalMessage); }
  };
  return <section className="inventory-card new-module-card"><div className="inventory-heading"><div><span className="eyebrow">LOT 17 · CRÉATION</span><h2>Créer un module vide</h2></div><button type="button" className="secondary-button" onClick={() => setOpenPanel((value) => !value)}>{openPanel ? "Fermer" : "Configurer"}</button></div>{openPanel && <div className="new-module-form"><label>Nom<input value={definition.name} onChange={(event) => setDefinition((value) => ({ ...value, name: event.currentTarget.value }))} /></label><label>Tag<input value={definition.tag} onChange={(event) => setDefinition((value) => ({ ...value, tag: event.currentTarget.value }))} /></label><label>Zone d’entrée<input value={definition.entryArea} maxLength={16} onChange={(event) => setDefinition((value) => ({ ...value, entryArea: event.currentTarget.value.toLocaleLowerCase() }))} /></label><label>Tileset<input value={definition.tileset} onChange={(event) => setDefinition((value) => ({ ...value, tileset: event.currentTarget.value.toLocaleLowerCase() }))} /></label><button type="button" className="primary-button" onClick={() => void create()}>Créer le nouveau MOD</button>{status && <code>{status}</code>}</div>}</section>;
}

function PhaseOneWorkspace({ jobId, activeView, editWorkspace, onWorkspace }: { jobId: string; activeView: string; editWorkspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [filter, setFilter] = useState("");
  const worldQuery = useQuery({ queryKey: ["world", jobId], queryFn: () => inspectWorld({ jobId }), staleTime: Number.POSITIVE_INFINITY });
  const world = worldQuery.data;
  if (worldQuery.isLoading) return <section className="inventory-card world-workspace">Construction de la vue métier…</section>;
  if (!world) return <section className="inventory-card world-workspace">L’index global n’est pas disponible.</section>;
  return <section className="inventory-card world-workspace" aria-label="Explorateur de la Phase 1">
    <div className="inventory-heading"><div><span className="eyebrow">SOURCE RUST · PROVENANCE CONSERVÉE</span><h2>{worldViewTitle(activeView)}</h2></div><label className="world-filter"><Search size={13} /><input value={filter} onChange={(event) => setFilter(event.currentTarget.value)} placeholder="Filtrer cette vue…" /></label></div>
    {activeView === "narrative" && <NarrativeView jobId={jobId} world={world} filter={filter} editWorkspace={editWorkspace} onWorkspace={onWorkspace} />}
    {activeView === "areas" && <AreaMapView jobId={jobId} world={world} filter={filter} editWorkspace={editWorkspace} onWorkspace={onWorkspace} />}
    {activeView === "assets" && <><WalkmeshWorkbench jobId={jobId} workspace={editWorkspace} onWorkspace={onWorkspace} /><AssetView jobId={jobId} assets={world.assets.assets} filter={filter} /></>}
    {activeView === "scene" && <SceneView jobId={jobId} world={world} filter={filter} />}
    {activeView === "graph" && <GlobalGraphView jobId={jobId} world={world} filter={filter} />}
  </section>;
}

function worldViewTitle(view: string) {
  return ({ narrative: "Journal, quêtes et factions", areas: "Carte 2D des zones", assets: "Modèles, textures et animations", scene: "Vue 3D des zones", graph: "Graphe global et validation" } as Record<string, string>)[view] ?? "Phase 1";
}

function NarrativeView({ jobId, world, filter, editWorkspace, onWorkspace }: { jobId:string; world: WorldIndex; filter: string; editWorkspace?:WorkspaceSnapshot; onWorkspace:(workspace:WorkspaceSnapshot)=>void }) {
  const narrativeQuery=useQuery({queryKey:["narrative-documents",jobId,editWorkspace?.workspaceId,editWorkspace?.cursor],queryFn:()=>inspectNarrativeDocuments({jobId,workspaceId:editWorkspace?.workspaceId})});
  const narrative=narrativeQuery.data?.model??world.narrative;
  const query = filter.trim().toLocaleLowerCase();
  const categories = narrative.categories.map((value,index)=>({value,index})).filter(({value}) => [value.tag, value.name.text ?? "", String(value.name.stringRef ?? "")].some((candidate) => candidate.toLocaleLowerCase().includes(query)));
  const commit=async(document:NarrativeDocument,path:string,before:GenericGffValue,after:GenericGffValue)=>{if(!editWorkspace)return;const result=await applyGffEdit({jobId,workspaceId:editWorkspace.workspaceId,resource:document.resource,path,before,after});onWorkspace(result.workspace)};
  const commitJournalStructure=async(action:JournalStructureAction)=>{if(!editWorkspace||!narrativeQuery.data?.journal)return;onWorkspace(await editJournalStructure({jobId,workspaceId:editWorkspace.workspaceId,resource:narrativeQuery.data.journal.resource,action}))};
  const commitFactionStructure=async(action:FactionStructureAction)=>{if(!editWorkspace||!narrativeQuery.data?.factions)return;onWorkspace(await editFactionStructure({jobId,workspaceId:editWorkspace.workspaceId,resource:narrativeQuery.data.factions.resource,action}))};
  return <div className="narrative-layout"><div className="journal-list"><h3>Journal · {categories.length} catégorie(s)</h3>
    {editWorkspace&&narrativeQuery.data?.journal&&<JournalStructureToolbar onCommit={commitJournalStructure}/>}
    {categories.map(({value:category,index}) => <article key={`${category.tag}-${index}`} className="journal-category"><header><strong>{category.name.text ?? category.tag}</strong><code>{category.tag}</code><span>priorité {category.priority} · {category.xp} XP</span></header>
      {editWorkspace&&narrativeQuery.data?.journal&&<JournalCategoryEditor document={narrativeQuery.data.journal} categoryIndex={index} onCommit={commit} onStructure={commitJournalStructure}/>}
      {category.entries.map((entry,entryIndex) => <div className={entry.finalState ? "journal-entry final" : "journal-entry"} key={`${entry.id}-${entryIndex}`}><b>Étape {entry.id}</b><span>{entry.text.text ?? "StrRef " + String(entry.text.stringRef ?? "absente")}</span>{entry.finalState && <em>état final</em>}{editWorkspace&&narrativeQuery.data?.journal&&<JournalEntryEditor document={narrativeQuery.data.journal} categoryIndex={index} entryIndex={entryIndex} onCommit={commit} onStructure={commitJournalStructure}/>}</div>)}
    </article>)}</div><FactionMatrix narrative={narrative} query={query} document={editWorkspace?narrativeQuery.data?.factions??undefined:undefined} onCommit={commit} onStructure={commitFactionStructure}/><div className="confidence-legend"><b>Confiance</b><span className="certain">certain · champ explicite</span><span className="probable">probable · rapprochement nommé</span><span className="possible">possible · cible non résolue</span></div>
  </div>;
}

type NarrativeCommit=(document:NarrativeDocument,path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>;
type JournalStructureCommit=(action:JournalStructureAction)=>Promise<void>;
type FactionStructureCommit=(action:FactionStructureAction)=>Promise<void>;

function JournalStructureToolbar({onCommit}:{onCommit:JournalStructureCommit}) {
  const [tag,setTag]=useState("");const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  const add=async()=>{setBusy(true);setMessage("Ajout…");try{await onCommit({kind:"add_category",tag});setTag("");setMessage("Catégorie ajoutée dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="journal-structure-toolbar"><input value={tag} maxLength={64} placeholder="tag_nouvelle_quete" onChange={event=>setTag(event.currentTarget.value)}/><button type="button" disabled={busy||!tag} onClick={()=>void add()}>+ Catégorie</button>{message&&<small>{message}</small>}</div>;
}

function JournalCategoryEditor({document,categoryIndex,onCommit,onStructure}:{document:NarrativeDocument;categoryIndex:number;onCommit:NarrativeCommit;onStructure:JournalStructureCommit}) {
  const categories=document.raw.root.fields.find(field=>["Categories","CategoryList"].includes(field.label)&&field.value.kind==="list");
  const category=(categories?.value.value as GenericGff["root"][]|undefined)?.[categoryIndex];
  if(!categories||!category)return <small>Structure JRL brute introuvable.</small>;
  const base=`/${categories.label}/${categoryIndex}`;
  const textLabels:Record<string,string>={Tag:"Tag",Name:"Nom localisé"};
  const numericLabels:Record<string,string>={Priority:"Priorité",XP:"XP",Xp:"XP"};
  return <div className="gff-field-editor journal-field-editor"><strong>Catégorie JRL</strong>{category.fields.filter(field=>Object.hasOwn(textLabels,field.label)&&["string","res_ref","localized_string"].includes(field.value.kind)).map(field=><EditableDialogueField key={field.label} field={{label:textLabels[field.label],path:`${base}/${field.label}`,value:field.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>)}{category.fields.filter(field=>Object.hasOwn(numericLabels,field.label)&&isIntegerGffValue(field.value)).map(field=><EditableIntegerGffField key={field.label} field={{label:numericLabels[field.label],path:`${base}/${field.label}`,value:field.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>)}<div className="structure-row-actions"><JournalStructureButton label="+ Étape" action={{kind:"add_entry",categoryIndex}} onCommit={onStructure}/><JournalStructureButton label="Supprimer la catégorie" danger action={{kind:"remove_category",categoryIndex}} onCommit={onStructure}/></div></div>;
}

function JournalEntryEditor({document,categoryIndex,entryIndex,onCommit,onStructure}:{document:NarrativeDocument;categoryIndex:number;entryIndex:number;onCommit:NarrativeCommit;onStructure:JournalStructureCommit}) {
  const categories=document.raw.root.fields.find(field=>["Categories","CategoryList"].includes(field.label)&&field.value.kind==="list");
  const category=(categories?.value.value as GenericGff["root"][]|undefined)?.[categoryIndex];
  const entries=category?.fields.find(field=>["EntryList","Entries"].includes(field.label)&&field.value.kind==="list");
  const entry=(entries?.value.value as GenericGff["root"][]|undefined)?.[entryIndex];
  if(!categories||!entries||!entry)return null;
  const base=`/${categories.label}/${categoryIndex}/${entries.label}/${entryIndex}`;
  return <div className="gff-field-editor journal-entry-editor"><strong>Étape JRL</strong>{entry.fields.filter(field=>field.label==="Text"&&["string","localized_string"].includes(field.value.kind)).map(field=><EditableDialogueField key={field.label} field={{label:"Texte localisé",path:`${base}/${field.label}`,value:field.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>)}{entry.fields.filter(field=>["ID","Id","Delay"].includes(field.label)&&isIntegerGffValue(field.value)).map(field=><EditableIntegerGffField key={field.label} field={{label:field.label==="Delay"?"Délai":"Identifiant",path:`${base}/${field.label}`,value:field.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>)}{entry.fields.filter(field=>["End","IsEnd"].includes(field.label)&&isIntegerGffValue(field.value)).map(field=><EditableBooleanGffField key={field.label} field={{label:"État final",path:`${base}/${field.label}`,value:field.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>)}<JournalStructureButton label="Supprimer l'étape" danger action={{kind:"remove_entry",categoryIndex,entryIndex}} onCommit={onStructure}/></div>;
}

function JournalStructureButton({label,action,onCommit,danger=false}:{label:string;action:JournalStructureAction;onCommit:JournalStructureCommit;danger?:boolean}) {
  const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  const run=async()=>{setBusy(true);setMessage("");try{await onCommit(action)}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <span className="structure-action"><button type="button" className={danger?"danger-button":""} disabled={busy} onClick={()=>void run()}>{busy?"…":label}</button>{message&&<small>{message}</small>}</span>;
}

function isIntegerGffValue(value:GenericGffValue){return ["byte","char","word","short","dword","int"].includes(value.kind)&&typeof value.value==="number"}

function EditableIntegerGffField({field,onCommit}:{field:{label:string;path:string;value:GenericGffValue};onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>}) {
  const original=Number(field.value.value);const [draft,setDraft]=useState(original);const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{setDraft(original);setMessage("")},[original,field.path]);
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(field.path,field.value,{kind:field.value.kind,value:draft});setMessage("Enregistré dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <label className="gff-field-row"><span>{field.label}</span><input type="number" value={draft} onChange={event=>setDraft(Number(event.currentTarget.value))}/><button type="button" disabled={busy||!Number.isInteger(draft)||draft===original} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button>{message&&<small>{message}</small>}</label>;
}

function EditableBooleanGffField({field,onCommit}:{field:{label:string;path:string;value:GenericGffValue};onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>}) {
  const original=Number(field.value.value)!==0;const [draft,setDraft]=useState(original);const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{setDraft(original);setMessage("")},[original,field.path]);
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(field.path,field.value,{kind:field.value.kind,value:draft?1:0});setMessage("Enregistré dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <label className="gff-field-row"><span>{field.label}</span><input type="checkbox" checked={draft} onChange={event=>setDraft(event.currentTarget.checked)}/><button type="button" disabled={busy||draft===original} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button>{message&&<small>{message}</small>}</label>;
}

function FactionMatrix({ narrative, query, document, onCommit, onStructure }: { narrative: WorldIndex["narrative"]; query: string; document?:NarrativeDocument; onCommit:NarrativeCommit; onStructure:FactionStructureCommit }) {
  const indexedFactions=narrative.factions.map((value,index)=>({value,index})).filter(({value}) => value.name.toLocaleLowerCase().includes(query)).slice(0, 24);
  const factions=indexedFactions.map(({value})=>value);
  const reputation = new globalThis.Map(narrative.reputations.map((value) => [String(value.sourceId) + ":" + String(value.targetId), value.value]));
  return <div className="faction-matrix"><h3>Matrice des factions · {narrative.factions.length}</h3>{document&&<FactionStructureToolbar factions={narrative.factions} onCommit={onStructure}/>} {document&&<div className="faction-editors">{indexedFactions.map(({value,index})=><FactionEditor key={`${value.id}-${index}`} document={document} factionIndex={index} onCommit={onCommit} onStructure={onStructure}/>)}</div>}<div className="faction-table-wrap"><table><thead><tr><th>Faction</th>{factions.map((value) => <th key={value.id} title={value.name}>{value.id}</th>)}</tr></thead><tbody>{factions.map((source) => <tr key={source.id}><th>{source.name}</th>{factions.map((target) => { const score = reputation.get(String(source.id) + ":" + String(target.id)); return <td key={target.id} className={score === undefined ? "" : score < 10 ? "hostile" : score > 50 ? "friendly" : "neutral"}>{score ?? "—"}</td>; })}</tr>)}</tbody></table></div>{document&&<FactionReputationEditors document={document} factions={narrative.factions} onCommit={onCommit} onStructure={onStructure}/>}</div>;
}

function FactionStructureToolbar({factions,onCommit}:{factions:WorldIndex["narrative"]["factions"];onCommit:FactionStructureCommit}) {
  const [name,setName]=useState("");const [parentId,setParentId]=useState("");const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  const add=async()=>{setBusy(true);setMessage("Ajout…");try{await onCommit({kind:"add_faction",name,parentId:parentId===""?null:Number(parentId)});setName("");setParentId("");setMessage("Faction et matrice créées dans l’overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="faction-structure-toolbar"><input value={name} maxLength={255} placeholder="Nouvelle faction" onChange={event=>setName(event.currentTarget.value)}/><select aria-label="Faction parente" value={parentId} onChange={event=>setParentId(event.currentTarget.value)}><option value="">Sans parent</option>{factions.map(faction=><option key={faction.id} value={faction.id}>{faction.id} · {faction.name}</option>)}</select><button type="button" disabled={busy||!name.trim()} onClick={()=>void add()}>{busy?"…":"+ Faction"}</button>{message&&<small>{message}</small>}</div>;
}

function FactionEditor({document,factionIndex,onCommit,onStructure}:{document:NarrativeDocument;factionIndex:number;onCommit:NarrativeCommit;onStructure:FactionStructureCommit}) {
  const list=document.raw.root.fields.find(field=>["FactionList","Factions"].includes(field.label)&&field.value.kind==="list");
  const faction=(list?.value.value as GenericGff["root"][]|undefined)?.[factionIndex];
  if(!list||!faction)return null;
  const base=`/${list.label}/${factionIndex}`;
  const name=faction.fields.find(field=>["FactionName","Name"].includes(field.label)&&field.value.kind==="string");
  const integers=faction.fields.filter(field=>["FactionID","ID","FactionParentID","ParentID"].includes(field.label)&&isIntegerGffValue(field.value));
  const global=faction.fields.find(field=>["FactionGlobal","Global"].includes(field.label)&&isIntegerGffValue(field.value));
  return <details className="faction-field-editor"><summary>{String(name?.value.value??`Faction ${factionIndex}`)}</summary><div className="gff-field-editor"><strong>Faction {factionIndex}</strong>{name&&<EditableDialogueField field={{label:"Nom",path:`${base}/${name.label}`,value:name.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>} {integers.map(field=><EditableIntegerGffField key={field.label} field={{label:["FactionParentID","ParentID"].includes(field.label)?"Faction parente":"Identifiant",path:`${base}/${field.label}`,value:field.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>)}{global&&<EditableBooleanGffField field={{label:"Globale",path:`${base}/${global.label}`,value:global.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/>} {factionIndex>0&&<FactionStructureButton label="Supprimer la faction" danger action={{kind:"remove_faction",factionIndex}} onCommit={onStructure}/>}</div></details>;
}

function FactionReputationEditors({document,factions,onCommit,onStructure}:{document:NarrativeDocument;factions:WorldIndex["narrative"]["factions"];onCommit:NarrativeCommit;onStructure:FactionStructureCommit}) {
  const list=document.raw.root.fields.find(field=>["RepList","ReputationList"].includes(field.label)&&field.value.kind==="list");
  const entries=(list?.value.value as GenericGff["root"][]|undefined)??[];
  const visibleIds=new Set(factions.map(faction=>faction.id));
  const visible=entries.map((entry,index)=>({entry,index})).filter(({entry})=>{const ids=entry.fields.filter(field=>["FactionID1","SourceID","FactionID2","TargetID"].includes(field.label)&&isIntegerGffValue(field.value)).map(field=>Number(field.value.value));return ids.length===2&&ids.every(id=>visibleIds.has(id))}).slice(0,100);
  if(!list)return null;
  return <div className="gff-field-editor reputation-field-editor"><strong>Réputations FAC</strong><FactionReputationToolbar factions={factions} onCommit={onStructure}/>{visible.map(({entry,index})=>{const source=entry.fields.find(field=>["FactionID1","SourceID"].includes(field.label));const target=entry.fields.find(field=>["FactionID2","TargetID"].includes(field.label));const score=entry.fields.find(field=>["FactionRep","Reputation"].includes(field.label)&&isIntegerGffValue(field.value));return score?<div className="reputation-structure-row" key={index}><EditableIntegerGffField field={{label:`${String(source?.value.value)} → ${String(target?.value.value)}`,path:`/${list.label}/${index}/${score.label}`,value:score.value}} onCommit={(path,before,after)=>onCommit(document,path,before,after)}/><FactionStructureButton label="Supprimer" danger action={{kind:"remove_reputation",reputationIndex:index}} onCommit={onStructure}/></div>:null})}</div>;
}

function FactionReputationToolbar({factions,onCommit}:{factions:WorldIndex["narrative"]["factions"];onCommit:FactionStructureCommit}) {
  const [sourceId,setSourceId]=useState(factions[0]?.id??0);const [targetId,setTargetId]=useState(factions.find(faction=>faction.id!==0)?.id??1);const [value,setValue]=useState(50);const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  const add=async()=>{setBusy(true);setMessage("");try{await onCommit({kind:"add_reputation",sourceId,targetId,value});setMessage("Réputation ajoutée dans l’overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="faction-structure-toolbar reputation-add"><label>Source<select value={sourceId} onChange={event=>setSourceId(Number(event.currentTarget.value))}>{factions.map(faction=><option key={faction.id} value={faction.id}>{faction.id} · {faction.name}</option>)}</select></label><label>Cible<select value={targetId} onChange={event=>setTargetId(Number(event.currentTarget.value))}>{factions.filter(faction=>faction.id!==0).map(faction=><option key={faction.id} value={faction.id}>{faction.id} · {faction.name}</option>)}</select></label><label>Valeur<input type="number" min={0} max={100} value={value} onChange={event=>setValue(Number(event.currentTarget.value))}/></label><button type="button" disabled={busy||!Number.isInteger(value)||value<0||value>100||!factions.some(faction=>faction.id===targetId)} onClick={()=>void add()}>{busy?"…":"+ Réputation"}</button>{message&&<small>{message}</small>}</div>;
}

function FactionStructureButton({label,action,onCommit,danger=false}:{label:string;action:FactionStructureAction;onCommit:FactionStructureCommit;danger?:boolean}) {
  const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  const run=async()=>{setBusy(true);setMessage("");try{await onCommit(action)}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <span className="structure-action"><button type="button" className={danger?"danger-button":""} disabled={busy} onClick={()=>void run()}>{busy?"…":label}</button>{message&&<small>{message}</small>}</span>;
}

function AreaMapView({ jobId, world, filter, editWorkspace, onWorkspace }: { jobId: string; world: WorldIndex; filter: string; editWorkspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [createdAreas, setCreatedAreas] = useState<AreaMap[]>([]);
  const [stagedAreas, setStagedAreas] = useState<Record<string, AreaMap>>({});
  useEffect(() => {
    let active = true;
    if (!editWorkspace) {
      setCreatedAreas([]);
      return () => { active = false; };
    }
    void listWorkspaceCreatedAreas({ workspaceId: editWorkspace.workspaceId })
      .then((areas) => {
        if (!active) return;
        setCreatedAreas((current) => {
          const byResref = new globalThis.Map(areas.map((area) => [area.resref, area]));
          for (const area of current) {
            const stillCreated = editWorkspace.modifiedResources.some((resource) =>
              resource.resource.resref === area.resref && resource.resource.resourceType === 2012 && resource.sourceSha256 === null,
            );
            if (stillCreated && !byResref.has(area.resref)) byResref.set(area.resref, area);
          }
          return [...byResref.values()];
        });
      })
      .catch(() => undefined);
    return () => { active = false; };
  }, [editWorkspace?.workspaceId, editWorkspace?.cursor]);
  const activeCreatedAreas = createdAreas.filter((area) => editWorkspace?.modifiedResources.some((resource) => resource.resource.resref === area.resref && resource.resource.resourceType === 2012));
  const deletedAreas = new Set(editWorkspace?.deletedResources.filter((resource) => resource.resourceType === 2012).map((resource) => resource.resref) ?? []);
  const areas = [...world.areas.filter((area) => !deletedAreas.has(area.resref)), ...activeCreatedAreas].filter((value) => (value.resref + " " + (value.name.text ?? "") + " " + (value.tileset ?? "")).toLocaleLowerCase().includes(filter.toLocaleLowerCase()));
  const [selected, setSelected] = useState<string>(); const [instance, setInstance] = useState<string>(); const [tile, setTile] = useState<string>();
  useEffect(() => { if (!areas.some((value) => value.resref === selected)) setSelected(areas[0]?.resref); }, [areas, selected]);
  useEffect(() => {
    let active = true;
    if (!editWorkspace || !selected) {
      if (!editWorkspace) setStagedAreas({});
      return () => { active = false; };
    }
    void inspectWorkspaceArea({ jobId, workspaceId: editWorkspace.workspaceId, area: selected })
      .then((area) => { if (active) setStagedAreas((current) => ({ ...current, [area.resref]: area })); })
      .catch(() => undefined);
    return () => { active = false; };
  }, [editWorkspace?.workspaceId, editWorkspace?.cursor, jobId, selected]);
  const sourceArea = (selected ? stagedAreas[selected] : undefined) ?? areas.find((value) => value.resref === selected);
  const area = useMemo(() => sourceArea ? applyAreaWorkspaceValues(sourceArea, editWorkspace?.values ?? {}) : undefined, [editWorkspace?.values, sourceArea]);
  const removeArea = async (resref: string) => {
    if (!editWorkspace || !window.confirm(`Supprimer la zone ${resref} de la sortie ? Cette action reste annulable.`)) return;
    onWorkspace(await deleteWorkspaceArea({ jobId, workspaceId: editWorkspace.workspaceId, resref }));
    setSelected(undefined);
  };
  return <div className="area-workspace"><div className="area-list">{editWorkspace && <AreaCreationForm jobId={jobId} workspace={editWorkspace} onAreaCreated={(result) => { onWorkspace(result.workspace); setCreatedAreas((current) => [...current.filter((area) => area.resref !== result.area.resref), result.area]); setSelected(result.area.resref); }} />}{areas.map((value) => <button type="button" className={value.resref === selected ? "selected" : ""} key={value.resref} onClick={() => { setSelected(value.resref); setInstance(undefined); setTile(undefined); }}><strong>{value.name.text ?? value.resref}</strong><code>{value.resref}</code><span>{value.width}×{value.height} · {value.instances.length} instances</span></button>)}</div>{area ? <><div className="area-actions">{editWorkspace && <button type="button" className="danger-button" onClick={() => void removeArea(area.resref)}>Supprimer la zone</button>}</div><AreaCanvas area={area} selected={instance} selectedTile={tile} onSelect={(id) => { setInstance(id); setTile(undefined); }} onSelectTile={(id) => { setTile(id); setInstance(undefined); }} /><AreaInspector jobId={jobId} area={area} selected={instance} selectedTile={tile} editWorkspace={editWorkspace} onWorkspace={onWorkspace} /></> : <p>Aucune zone.</p>}</div>;
}

function AreaCreationForm({ jobId, workspace, onAreaCreated }: { jobId: string; workspace: WorkspaceSnapshot; onAreaCreated: (result: { workspace: WorkspaceSnapshot; area: AreaMap }) => void }) {
  const [openPanel, setOpenPanel] = useState(false);
  const [draft, setDraft] = useState({ resref: "newarea", name: "Nouvelle zone", tileset: "tno01", width: 1, height: 1, tileId: 0 });
  const [status, setStatus] = useState("");
  const create = async () => {
    setStatus("Création des ressources ARE/GIT/GIC…");
    try {
      onAreaCreated(await createWorkspaceArea({ jobId, workspaceId: workspace.workspaceId, ...draft }));
      setStatus("Zone créée dans l’overlay et ouverte immédiatement.");
    } catch (error) { setStatus(normalizeAppError(error).technicalMessage); }
  };
  return <div className="area-create"><button type="button" onClick={() => setOpenPanel((value) => !value)}>+ Nouvelle zone</button>{openPanel && <div className="area-create-fields"><input aria-label="ResRef de la zone" maxLength={16} value={draft.resref} onChange={(event) => setDraft((value) => ({ ...value, resref: event.currentTarget.value.toLocaleLowerCase() }))} /><input aria-label="Nom de la zone" value={draft.name} onChange={(event) => setDraft((value) => ({ ...value, name: event.currentTarget.value }))} /><input aria-label="Tileset" value={draft.tileset} onChange={(event) => setDraft((value) => ({ ...value, tileset: event.currentTarget.value.toLocaleLowerCase() }))} /><label>Largeur<input type="number" min="1" max="64" value={draft.width} onChange={(event) => setDraft((value) => ({ ...value, width: Number(event.currentTarget.value) }))} /></label><label>Hauteur<input type="number" min="1" max="64" value={draft.height} onChange={(event) => setDraft((value) => ({ ...value, height: Number(event.currentTarget.value) }))} /></label><button type="button" className="secondary-button" onClick={() => void create()}>Créer ARE/GIT/GIC</button>{status && <small>{status}</small>}</div>}</div>;
}

function applyAreaWorkspaceValues(area: AreaMap, values: Record<string, unknown>): AreaMap {
  const instances = area.instances.filter((instance) => values[`instance:${area.resref}:${instance.id}:exists`] !== false).map((instance) => {
      const transform = values[`instance:${area.resref}:${instance.id}:transform`] as Partial<{ x: number; y: number; z: number; bearing: number }> | undefined;
      return transform ? { ...instance, ...transform } : instance;
    });
  const prefix = `instance:${area.resref}:`;
  for (const [key, raw] of Object.entries(values)) {
    if (!key.startsWith(prefix) || !key.endsWith(":exists") || typeof raw !== "object" || raw === null) continue;
    const placement = raw as { category: string; templateResref: string; tag: string; x: number; y: number; z: number; bearing: number; linkedTo?: string | null };
    const id = key.slice(prefix.length, -":exists".length);
    if (instances.some((value) => value.id === id)) continue;
    instances.push({ id, category: placement.category, tag: placement.tag, templateResref: placement.templateResref, x: placement.x, y: placement.y, z: placement.z, bearing: placement.bearing, appearance: null, transitionDestination: placement.linkedTo ?? null, transitionFlags: null, loadScreenId: null, geometry: [], spawnPoints: [], inventory: [], sourcePath: `workspace::${area.resref}.git` });
  }
  return { ...area, instances,
    tiles: area.tiles.map((tile) => {
      const state = values[`tile:${area.resref}:${tile.x}:${tile.y}`] as Partial<{ tileId: number; orientation: number }> | undefined;
      return state ? { ...tile, ...state } : tile;
    }),
  };
}

function AreaCanvas({ area, selected, selectedTile, onSelect, onSelectTile }: { area: AreaMap; selected?: string; selectedTile?: string; onSelect: (id: string) => void; onSelectTile: (id: string) => void }) {
  const width = Math.max(area.width, 1); const height = Math.max(area.height, 1);
  return <div className="area-map-frame"><div className="area-map" style={{ aspectRatio: String(width) + "/" + String(height) }}>
    {area.tiles.map((tile) => { const id = `${tile.x}:${tile.y}`; return <button type="button" key={id} onClick={() => onSelectTile(id)} className={"area-tile" + (selectedTile === id ? " selected" : "")} title={"Tuile " + String(tile.tileId) + " · orientation " + String(tile.orientation)} style={{ left: String(tile.x / width * 100) + "%", top: String(tile.y / height * 100) + "%", width: String(100 / width) + "%", height: String(100 / height) + "%" }}><span style={{ transform: `rotate(${tile.orientation * 90}deg)` }}>{tile.tileId}</span></button>; })}
    <svg className="area-polygons" viewBox={`0 0 ${width * 10} ${height * 10}`} preserveAspectRatio="none" aria-hidden="true">{area.instances.filter((value) => (value.geometry ?? []).length >= 3).map((value) => <polygon key={value.id} className={value.category + (selected === value.id ? " selected" : "")} points={(value.geometry ?? []).map((point) => `${value.x + point.x},${height * 10 - (value.y + point.y)}`).join(" ")} />)}</svg>
    {area.instances.map((value) => <button type="button" aria-label={value.category + " " + (value.tag ?? value.templateResref ?? "")} key={value.id} onClick={() => onSelect(value.id)} className={"area-marker " + value.category + (selected === value.id ? " selected" : "")} style={{ left: String(Math.max(0, Math.min(100, value.x / (width * 10) * 100))) + "%", bottom: String(Math.max(0, Math.min(100, value.y / (height * 10) * 100))) + "%" }} />)}
  </div></div>;
}

function AreaInspector({ jobId, area, selected, selectedTile, editWorkspace, onWorkspace }: { jobId: string; area: AreaMap; selected?: string; selectedTile?: string; editWorkspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const value = area.instances.find((candidate) => candidate.id === selected);
  const tile = area.tiles.find((candidate) => `${candidate.x}:${candidate.y}` === selectedTile);
  return <aside className="area-detail"><h3>{area.name.text ?? area.resref}</h3><Property label="Tileset" value={area.tileset ?? "non résolu"} /><Property label="Tuiles" value={String(area.tiles.length)} /><Property label="Instances" value={String(area.instances.length)} />{editWorkspace && <AreaPlacementForm jobId={jobId} area={area.resref} workspace={editWorkspace} onWorkspace={onWorkspace} />}{value && <AreaInstanceEditor jobId={jobId} area={area.resref} value={value} workspace={editWorkspace} onWorkspace={onWorkspace} />}{tile && <AreaTileEditor jobId={jobId} area={area.resref} tile={tile} workspace={editWorkspace} onWorkspace={onWorkspace} />}{area.diagnostics.map((item) => <p className="world-diagnostic" key={item.code + ":" + item.resource}>{item.code} · {item.message}</p>)}</aside>;
}

function AreaPlacementForm({ jobId, area, workspace, onWorkspace }: { jobId: string; area: string; workspace: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [openPanel, setOpenPanel] = useState(false);
  const [draft, setDraft] = useState({ category: "placeable", templateResref: "", tag: "", x: 5, y: 5, z: 0, bearing: 0, linkedTo: null as string | null });
  const [status, setStatus] = useState("");
  const save = async () => {
    setStatus("Placement…");
    try {
      const result = await addWorkspaceAreaInstance({ jobId, workspaceId: workspace.workspaceId, area, placement: draft });
      onWorkspace(result.workspace); setStatus(`Instance ${result.instanceId} ajoutée.`);
    } catch (error) { setStatus(normalizeAppError(error).technicalMessage); }
  };
  return <div className="instance-detail area-edit-form"><button type="button" className="secondary-button" onClick={() => setOpenPanel((value) => !value)}>+ Placer une instance</button>{openPanel && <><label>Catégorie<select value={draft.category} onChange={(event) => setDraft((value) => ({ ...value, category: event.currentTarget.value }))}>{["creature", "door", "encounter", "item", "placeable", "sound", "store", "trigger", "waypoint"].map((category) => <option key={category}>{category}</option>)}</select></label><label>Blueprint ResRef<input maxLength={16} value={draft.templateResref} onChange={(event) => setDraft((value) => ({ ...value, templateResref: event.currentTarget.value.toLocaleLowerCase() }))} /></label><label>Tag<input value={draft.tag} onChange={(event) => setDraft((value) => ({ ...value, tag: event.currentTarget.value }))} /></label><div className="area-number-grid">{(["x", "y", "z", "bearing"] as const).map((field) => <label key={field}>{field.toUpperCase()}<input type="number" step="0.01" value={draft[field]} onChange={(event) => setDraft((value) => ({ ...value, [field]: Number(event.currentTarget.value) }))} /></label>)}</div><button type="button" className="primary-button" disabled={!draft.templateResref} onClick={() => void save()}>Ajouter à la zone</button>{status && <small>{status}</small>}</>}</div>;
}

function AreaInstanceEditor({ jobId, area, value, workspace, onWorkspace }: { jobId: string; area: string; value: AreaMap["instances"][number]; workspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [draft, setDraft] = useState({ x: value.x, y: value.y, z: value.z, bearing: value.bearing ?? 0 });
  const [geometry, setGeometry] = useState(value.geometry ?? []);
  const [spawnPoints, setSpawnPoints] = useState(value.spawnPoints ?? []);
  const [transition, setTransition] = useState({ destination: value.transitionDestination ?? "", flags: value.transitionFlags ?? 0, loadScreenId: value.loadScreenId ?? 0 });
  const [inventoryDraft, setInventoryDraft] = useState({ resref: "", stackSize: 1, x: 0, y: 0, infinite: false, categoryIndex: 0 });
  const [message, setMessage] = useState("");
  useEffect(() => {
    setDraft({ x: value.x, y: value.y, z: value.z, bearing: value.bearing ?? 0 });
    setGeometry(value.geometry ?? []);
    setSpawnPoints(value.spawnPoints ?? []);
    setTransition({ destination: value.transitionDestination ?? "", flags: value.transitionFlags ?? 0, loadScreenId: value.loadScreenId ?? 0 });
  }, [value]);
  const commitStructure = async (action: AreaStructureAction, success: string) => {
    if (!workspace) return;
    setMessage("Enregistrement…");
    try {
      onWorkspace(await editAreaStructure({ jobId, workspaceId: workspace.workspaceId, area, action }));
      setMessage(success);
    } catch (error) { setMessage(normalizeAppError(error).technicalMessage); }
  };
  const save = async () => {
    if (!workspace) return;
    setMessage("Enregistrement…");
    try {
      const snapshot = await moveAreaInstance({ jobId, workspaceId: workspace.workspaceId, area, instanceId: value.id, before: { x: value.x, y: value.y, z: value.z, bearing: value.bearing ?? 0 }, after: draft });
      onWorkspace(snapshot);
      setMessage("Transformation enregistrée dans l’overlay.");
    } catch (error) { setMessage(normalizeAppError(error).technicalMessage); }
  };
  const remove = async () => {
    if (!workspace || !window.confirm(`Supprimer l’instance ${value.tag ?? value.id} ? Cette action reste annulable.`)) return;
    setMessage("Suppression…");
    try {
      onWorkspace(await removeWorkspaceAreaInstance({ jobId, workspaceId: workspace.workspaceId, area, instanceId: value.id }));
    } catch (error) { setMessage(normalizeAppError(error).technicalMessage); }
  };
  const supportsGeometry = value.category === "trigger" || value.category === "encounter";
  const supportsTransition = value.category === "trigger" || value.category === "door";
  const supportsInventory = value.category === "placeable" || value.category === "store";
  return <div className="instance-detail area-edit-form"><h4>{value.tag ?? value.templateResref ?? value.category}</h4><span>{value.category}</span><div className="area-number-grid">{(["x", "y", "z", "bearing"] as const).map((field) => <label key={field}>{field.toUpperCase()}<input type="number" step="0.01" value={draft[field]} disabled={!workspace} onChange={(event) => setDraft((current) => ({ ...current, [field]: Number(event.currentTarget.value) }))} /></label>)}</div><button type="button" className="secondary-button" disabled={!workspace} onClick={() => void save()}>Enregistrer la position</button>
    {supportsGeometry && <AreaPointEditor title="Polygone local" points={geometry} onChange={setGeometry} disabled={!workspace} onSave={() => void commitStructure({ kind: "set_geometry", instanceId: value.id, points: geometry }, "Polygone enregistré dans l’overlay.")} />}
    {value.category === "encounter" && <AreaSpawnPointEditor points={spawnPoints} onChange={setSpawnPoints} disabled={!workspace} onSave={() => void commitStructure({ kind: "set_spawn_points", instanceId: value.id, points: spawnPoints }, "Points d’apparition enregistrés.")} />}
    {supportsTransition && <fieldset className="area-structure-panel"><legend>Transition</legend><label>Destination (tag)<input maxLength={64} value={transition.destination} disabled={!workspace} onChange={(event) => setTransition((current) => ({ ...current, destination: event.currentTarget.value }))} /></label><div className="area-number-grid"><label>Flags<input type="number" min="0" max="255" value={transition.flags} disabled={!workspace} onChange={(event) => setTransition((current) => ({ ...current, flags: Number(event.currentTarget.value) }))} /></label><label>Écran de chargement<input type="number" min="0" max="65535" value={transition.loadScreenId} disabled={!workspace} onChange={(event) => setTransition((current) => ({ ...current, loadScreenId: Number(event.currentTarget.value) }))} /></label></div><button type="button" className="secondary-button" disabled={!workspace || transition.flags < 0 || transition.flags > 255 || transition.loadScreenId < 0 || transition.loadScreenId > 65535} onClick={() => void commitStructure({ kind: "set_transition", instanceId: value.id, ...transition }, "Transition enregistrée.")}>Enregistrer la transition</button></fieldset>}
    {supportsInventory && <fieldset className="area-structure-panel"><legend>Inventaire</legend><div className="area-inventory-list">{(value.inventory ?? []).map((item) => <div className="area-inventory-row" key={`${item.categoryIndex ?? "p"}:${item.itemIndex}`}><span><strong>{item.resref}</strong> ×{item.stackSize}{item.categoryIndex !== null ? ` · catégorie ${item.categoryIndex}` : ""}{item.infinite ? " · infini" : ""}</span><button type="button" className="danger-button" disabled={!workspace} onClick={() => void commitStructure({ kind: "remove_inventory_item", instanceId: value.id, itemIndex: item.itemIndex, categoryIndex: item.categoryIndex }, "Objet retiré de l’inventaire.")}>Retirer</button></div>)}</div><label>Blueprint UTI<input maxLength={16} value={inventoryDraft.resref} disabled={!workspace} onChange={(event) => setInventoryDraft((current) => ({ ...current, resref: event.currentTarget.value.toLocaleLowerCase() }))} /></label><div className="area-number-grid"><label>Quantité<input type="number" min="1" max="65535" value={inventoryDraft.stackSize} onChange={(event) => setInventoryDraft((current) => ({ ...current, stackSize: Number(event.currentTarget.value) }))} /></label><label>Colonne<input type="number" min="0" max="65535" value={inventoryDraft.x} onChange={(event) => setInventoryDraft((current) => ({ ...current, x: Number(event.currentTarget.value) }))} /></label><label>Ligne<input type="number" min="0" max="65535" value={inventoryDraft.y} onChange={(event) => setInventoryDraft((current) => ({ ...current, y: Number(event.currentTarget.value) }))} /></label>{value.category === "store" && <label>Catégorie<input type="number" min="0" max="4" value={inventoryDraft.categoryIndex} onChange={(event) => setInventoryDraft((current) => ({ ...current, categoryIndex: Number(event.currentTarget.value) }))} /></label>}</div>{value.category === "store" && <label className="area-checkbox"><input type="checkbox" checked={inventoryDraft.infinite} onChange={(event) => setInventoryDraft((current) => ({ ...current, infinite: event.currentTarget.checked }))} /> Stock infini</label>}<button type="button" className="secondary-button" disabled={!workspace || !inventoryDraft.resref || inventoryDraft.stackSize < 1} onClick={() => void commitStructure({ kind: "add_inventory_item", instanceId: value.id, resref: inventoryDraft.resref, stackSize: inventoryDraft.stackSize, x: inventoryDraft.x, y: inventoryDraft.y, infinite: inventoryDraft.infinite, categoryIndex: value.category === "store" ? inventoryDraft.categoryIndex : null }, "Objet UTI incorporé dans l’inventaire.")}>Ajouter l’objet</button></fieldset>}
    <button type="button" className="danger-button" disabled={!workspace} onClick={() => void remove()}>Supprimer l’instance</button>{message && <small>{message}</small>}{value.transitionDestination && <b>Transition → {value.transitionDestination}</b>}<small>{value.sourcePath}</small></div>;
}

function AreaPointEditor({ title, points, onChange, disabled, onSave }: { title: string; points: Array<{ x: number; y: number; z: number }>; onChange: (points: Array<{ x: number; y: number; z: number }>) => void; disabled: boolean; onSave: () => void }) {
  const update = (index: number, field: "x" | "y" | "z", value: number) => onChange(points.map((point, position) => position === index ? { ...point, [field]: value } : point));
  return <fieldset className="area-structure-panel"><legend>{title}</legend>{points.map((point, index) => <div className="area-point-row" key={index}><span>{index + 1}</span>{(["x", "y", "z"] as const).map((field) => <input key={field} aria-label={`${title} ${index + 1} ${field}`} type="number" step="0.01" value={point[field]} disabled={disabled} onChange={(event) => update(index, field, Number(event.currentTarget.value))} />)}<button type="button" className="danger-button" disabled={disabled} onClick={() => onChange(points.filter((_, position) => position !== index))}>×</button></div>)}<div className="area-structure-actions"><button type="button" disabled={disabled} onClick={() => onChange(points.length === 0 ? [{ x: 0, y: 0, z: 0 }, { x: 1, y: 0, z: 0 }, { x: 0, y: 1, z: 0 }] : [...points, { x: 0, y: 0, z: 0 }])}>{points.length === 0 ? "Initialiser un triangle" : "+ Point"}</button><button type="button" className="secondary-button" disabled={disabled || points.length < 3} onClick={onSave}>Enregistrer</button></div></fieldset>;
}

function AreaSpawnPointEditor({ points, onChange, disabled, onSave }: { points: Array<{ x: number; y: number; z: number; orientation: number }>; onChange: (points: Array<{ x: number; y: number; z: number; orientation: number }>) => void; disabled: boolean; onSave: () => void }) {
  const update = (index: number, field: "x" | "y" | "z" | "orientation", value: number) => onChange(points.map((point, position) => position === index ? { ...point, [field]: value } : point));
  return <fieldset className="area-structure-panel"><legend>Points d’apparition</legend>{points.map((point, index) => <div className="area-spawn-row" key={index}><span>{index + 1}</span>{(["x", "y", "z", "orientation"] as const).map((field) => <input key={field} aria-label={`Apparition ${index + 1} ${field}`} type="number" step="0.01" value={point[field]} disabled={disabled} onChange={(event) => update(index, field, Number(event.currentTarget.value))} />)}<button type="button" className="danger-button" disabled={disabled} onClick={() => onChange(points.filter((_, position) => position !== index))}>×</button></div>)}<div className="area-structure-actions"><button type="button" disabled={disabled} onClick={() => onChange([...points, { x: 0, y: 0, z: 0, orientation: 0 }])}>+ Apparition</button><button type="button" className="secondary-button" disabled={disabled} onClick={onSave}>Enregistrer</button></div></fieldset>;
}

function AreaTileEditor({ jobId, area, tile, workspace, onWorkspace }: { jobId: string; area: string; tile: AreaMap["tiles"][number]; workspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [draft, setDraft] = useState({ tileId: tile.tileId, orientation: tile.orientation, height: 0 });
  const [message, setMessage] = useState("");
  useEffect(() => setDraft({ tileId: tile.tileId, orientation: tile.orientation, height: 0 }), [tile]);
  const save = async () => {
    if (!workspace) return;
    setMessage("Enregistrement…");
    try {
      const snapshot = await setAreaTile({ jobId, workspaceId: workspace.workspaceId, area, x: tile.x, y: tile.y, before: { tileId: tile.tileId, orientation: tile.orientation, height: 0 }, after: draft });
      onWorkspace(snapshot);
      setMessage("Tuile enregistrée dans l’overlay.");
    } catch (error) { setMessage(normalizeAppError(error).technicalMessage); }
  };
  return <div className="instance-detail area-edit-form"><h4>Tuile {tile.x}:{tile.y}</h4><div className="area-number-grid"><label>ID<input type="number" min="0" value={draft.tileId} disabled={!workspace} onChange={(event) => setDraft((current) => ({ ...current, tileId: Number(event.currentTarget.value) }))} /></label><label>Orientation<input type="number" min="0" max="3" value={draft.orientation} disabled={!workspace} onChange={(event) => setDraft((current) => ({ ...current, orientation: Number(event.currentTarget.value) }))} /></label></div><button type="button" className="secondary-button" disabled={!workspace || draft.orientation < 0 || draft.orientation > 3} onClick={() => void save()}>Enregistrer la tuile</button>{message && <small>{message}</small>}</div>;
}

const DEFAULT_WALKMESH: WalkmeshDraft = {
  vertices: [[0, 0, 0], [10, 0, 0], [10, 10, 0], [0, 10, 0]],
  faces: [[0, 1, 2], [0, 2, 3]],
  surfaceIds: [1, 1],
  variants: [],
  hooks: [],
};

export function WalkmeshWorkbench({ jobId, workspace, onWorkspace }: { jobId: string; workspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void }) {
  const [open, setOpen] = useState(false);
  const [resref, setResref] = useState("onf_walkmesh");
  const [kind, setKind] = useState<WalkmeshKind>("wok");
  const [draft, setDraft] = useState<WalkmeshDraft>(DEFAULT_WALKMESH);
  const [selectedFace, setSelectedFace] = useState(0);
  const [selectedVertex, setSelectedVertex] = useState(0);
  const [vertexPosition, setVertexPosition] = useState<[number, number, number]>([0, 0, 0]);
  const [surfaceId, setSurfaceId] = useState(1);
  const [extrusionDistance, setExtrusionDistance] = useState(1);
  const [weldTolerance, setWeldTolerance] = useState(0.001);
  const [replaceExisting, setReplaceExisting] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("Creation locale: aucune ressource source n'est modifiee.");
  const points = useMemo(() => {
    const xs = draft.vertices.map((value) => value[0]);
    const ys = draft.vertices.map((value) => value[1]);
    const minX = Math.min(...xs, 0);
    const maxX = Math.max(...xs, 1);
    const minY = Math.min(...ys, 0);
    const maxY = Math.max(...ys, 1);
    return draft.vertices.map(([x, y]) => [18 + ((x - minX) / Math.max(1, maxX - minX)) * 264, 282 - ((y - minY) / Math.max(1, maxY - minY)) * 264] as [number, number]);
  }, [draft.vertices]);

  const transform = async (operation: WalkmeshOperation, success: string) => {
    setBusy(true);
    try {
      const result = await transformWalkmeshDraft(draft, operation);
      setDraft(result.draft);
      const nextFace = Math.min(selectedFace, Math.max(0, result.draft.faces.length - 1));
      const nextVertex = Math.min(selectedVertex, Math.max(0, result.draft.vertices.length - 1));
      setSelectedFace(nextFace);
      setSelectedVertex(nextVertex);
      setSurfaceId(result.draft.surfaceIds[nextFace] ?? 0);
      setVertexPosition(result.draft.vertices[nextVertex] ?? [0, 0, 0]);
      setMessage(result.validation.valid ? success : result.validation.diagnostics.join(" "));
    } catch (error) {
      setMessage(normalizeAppError(error).technicalMessage);
    } finally {
      setBusy(false);
    }
  };

  const validate = async () => {
    setBusy(true);
    try {
      const result = await validateWalkmeshDraft(draft, kind);
      setMessage(result.valid ? `Walkmesh valide: ${draft.vertices.length} sommets, ${draft.faces.length} faces.` : result.diagnostics.join(" "));
    } catch (error) {
      setMessage(normalizeAppError(error).technicalMessage);
    } finally {
      setBusy(false);
    }
  };

  const load = async () => {
    if (!workspace) return;
    setBusy(true);
    try {
      const document = await inspectWorkspaceWalkmesh({ jobId, workspaceId: workspace.workspaceId, resref, kind });
      setDraft(document.draft);
      setSelectedFace(0);
      setSelectedVertex(0);
      setSurfaceId(document.draft.surfaceIds[0] ?? 0);
      setVertexPosition(document.draft.vertices[0] ?? [0, 0, 0]);
      setReplaceExisting(true);
      setMessage(`${document.sourceFormat.toUpperCase()} charge sans modifier la source. L'enregistrement sera un remplacement complet explicite.`);
    } catch (error) {
      setMessage(normalizeAppError(error).technicalMessage);
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    if (!workspace) return;
    setBusy(true);
    try {
      const result = await saveWorkspaceWalkmesh({ jobId, workspaceId: workspace.workspaceId, resref, kind, draft, replaceExisting });
      onWorkspace(result.workspace);
      setMessage(`${result.document.resref}.${kind} serialise en ASCII NWN autonome, relu et place dans l'overlay.`);
    } catch (error) {
      setMessage(normalizeAppError(error).technicalMessage);
    } finally {
      setBusy(false);
    }
  };

  const selectFace = (index: number) => {
    const face = Math.max(0, Math.min(index, Math.max(0, draft.faces.length - 1)));
    setSelectedFace(face);
    setSurfaceId(draft.surfaceIds[face] ?? 0);
  };
  const selectVertex = (index: number) => {
    const vertex = Math.max(0, Math.min(index, Math.max(0, draft.vertices.length - 1)));
    setSelectedVertex(vertex);
    setVertexPosition(draft.vertices[vertex] ?? [0, 0, 0]);
  };

  return (
    <section className="walkmesh-workbench">
      <header>
        <div><span className="eyebrow">LOT 20 · WOK / PWK / DWK</span><h3>Atelier de walkmesh</h3></div>
        <button type="button" className="secondary-button" onClick={() => setOpen((value) => !value)}>{open ? "Fermer" : "Ouvrir"}</button>
      </header>
      {open && <div className="walkmesh-editor">
        <div className="walkmesh-controls">
          <label>ResRef<input maxLength={16} value={resref} onChange={(event) => setResref(event.currentTarget.value.toLocaleLowerCase().replace(/[^a-z0-9_]/g, ""))} /></label>
          <label>Format<select value={kind} onChange={(event) => setKind(event.currentTarget.value as WalkmeshKind)}><option value="wok">WOK · tuile</option><option value="pwk">PWK · placeable</option><option value="dwk">DWK · porte</option></select></label>
          <label>Face selectionnee<input type="number" min="0" max={Math.max(0, draft.faces.length - 1)} value={selectedFace} onChange={(event) => selectFace(Number(event.currentTarget.value))} /></label>
          <label>Surface<div className="walkmesh-inline"><input type="number" min={kind === "wok" ? 0 : undefined} max={kind === "wok" ? 19 : undefined} value={surfaceId} onChange={(event) => setSurfaceId(Number(event.currentTarget.value))} /><button type="button" disabled={busy || !draft.faces[selectedFace]} onClick={() => void transform({ kind: "set_surface", faceIndex: selectedFace, surfaceId }, `Surface ${surfaceId} appliquee a la face ${selectedFace}.`)}>Appliquer</button></div></label>
          <fieldset className="walkmesh-tool"><legend>Topologie</legend><div className="walkmesh-actions"><button type="button" onClick={() => void transform({ kind: "split_face", faceIndex: selectedFace }, `Face ${selectedFace} decoupee au centroide.`)} disabled={busy || !draft.faces[selectedFace]}>Decouper la face</button><button type="button" className="danger-button" onClick={() => void transform({ kind: "remove_face", faceIndex: selectedFace }, `Face ${selectedFace} supprimee et sommets orphelins compactes.`)} disabled={busy || draft.faces.length <= 1}>Supprimer la face</button></div></fieldset>
          <fieldset className="walkmesh-tool"><legend>Extrusion</legend><div className="walkmesh-inline"><input aria-label="Distance d'extrusion" type="number" step="0.1" value={extrusionDistance} onChange={(event) => setExtrusionDistance(Number(event.currentTarget.value))} /><button type="button" disabled={busy || !draft.faces[selectedFace]} onClick={() => void transform({ kind: "extrude_face", faceIndex: selectedFace, distance: extrusionDistance }, `Face ${selectedFace} extrudee de ${extrusionDistance}.`)}>Extruder</button></div></fieldset>
          <fieldset className="walkmesh-tool"><legend>Soudure</legend><div className="walkmesh-inline"><input aria-label="Tolerance de soudure" type="number" min="0.000001" step="0.001" value={weldTolerance} onChange={(event) => setWeldTolerance(Number(event.currentTarget.value))} /><button type="button" disabled={busy} onClick={() => void transform({ kind: "weld_vertices", tolerance: weldTolerance }, `Sommets soudes avec une tolerance de ${weldTolerance}.`)}>Souder les sommets</button></div></fieldset>
          <fieldset className="walkmesh-tool walkmesh-vertex-tool"><legend>Sommet</legend><label>Index<input type="number" min="0" max={Math.max(0, draft.vertices.length - 1)} value={selectedVertex} onChange={(event) => selectVertex(Number(event.currentTarget.value))} /></label>{(["X", "Y", "Z"] as const).map((axis, index) => <label key={axis}>{axis}<input aria-label={`Sommet ${axis}`} type="number" step="0.1" value={vertexPosition[index]} onChange={(event) => { const position = [...vertexPosition] as [number, number, number]; position[index] = Number(event.currentTarget.value); setVertexPosition(position); }} /></label>)}<button type="button" disabled={busy || !draft.vertices[selectedVertex]} onClick={() => void transform({ kind: "move_vertex", vertexIndex: selectedVertex, position: vertexPosition }, `Sommet ${selectedVertex} deplace.`)}>Deplacer</button></fieldset>
          <div className="walkmesh-metrics"><span>{draft.vertices.length} sommets</span><span>{draft.faces.length} faces</span><span>{draft.variants.length} variantes</span><span>{draft.hooks.length || (kind === "pwk" ? 2 : kind === "dwk" ? 6 : 0)} hooks {draft.hooks.length ? "explicites" : "generes"}</span></div>
          <label className="walkmesh-confirm"><input type="checkbox" checked={replaceExisting} onChange={(event) => setReplaceExisting(event.currentTarget.checked)} /> Autoriser le remplacement complet d'une ressource existante</label>
          <div className="walkmesh-actions"><button type="button" onClick={() => void validate()} disabled={busy}>Valider</button><button type="button" onClick={() => void load()} disabled={busy || !workspace}>Charger</button><button type="button" className="primary-button" onClick={() => void save()} disabled={busy || !workspace || !resref}>Enregistrer dans l'overlay</button></div>
          <small role="status">{message}</small>
        </div>
        <svg className="walkmesh-canvas" viewBox="0 0 300 300" role="img" aria-label="Apercu topologique du walkmesh">
          {draft.faces.map((face, index) => <polygon key={`${face.join("-")}-${index}`} points={face.map((vertex) => points[vertex]?.join(",") ?? "0,0").join(" ")} className={index === selectedFace ? "selected" : ""} onClick={() => selectFace(index)} />)}
          {points.map((point, index) => <g className={index === selectedVertex ? "selected-vertex" : ""} key={`${point.join("-")}-${index}`} onClick={() => selectVertex(index)}><circle cx={point[0]} cy={point[1]} r="4" /><text x={point[0] + 6} y={point[1] - 6}>{index}</text></g>)}
        </svg>
      </div>}
    </section>
  );
}

function AssetView({ jobId, assets, filter }: { jobId: string; assets: AssetRecord[]; filter: string }) {
  const textureTypes = [2033, 2073, 3, 2080, 2081, 2079, 6];
  const query = filter.toLocaleLowerCase(); const values = assets.filter((value) => (value.key.resref + " " + value.format + " " + value.modelNodes.join(" ") + " " + value.animations.join(" ")).toLocaleLowerCase().includes(query)).slice(0, 250);
  const [selected, setSelected] = useState<AssetRecord>();
  useEffect(() => { if (selected && !values.includes(selected)) setSelected(undefined); }, [selected, values]);
  const texture = selected?.textures.map((resref) => assets.filter((value) => value.key.resref === resref && textureTypes.includes(value.key.resourceType)).sort((left, right) => textureTypes.indexOf(left.key.resourceType) - textureTypes.indexOf(right.key.resourceType))[0]).find(Boolean);
  const imageSelected = selected && textureTypes.includes(selected.key.resourceType) && selected.support === "preview";
  return <div className="asset-workspace"><div className="asset-grid">{values.map((value) => <button type="button" onClick={() => setSelected(value)} className={"asset-card " + value.support + (selected === value ? " selected" : "")} key={value.key.resref + ":" + String(value.key.resourceType)}><div className="asset-preview"><Box size={28} /><span>{value.width && value.height ? String(value.width) + "×" + String(value.height) : value.modelNodes[0] ?? value.format}</span></div><strong>{value.key.resref}</strong><code>{value.format} · {value.support}</code><small>{value.meshCount} mesh · {value.triangleCount.toLocaleString("fr-FR")} triangles · {value.animations.length} animations</small>{value.glbPreview && <b className="glb-ready">GLB disponible</b>}{value.diagnostics.map((item) => <em key={item.code}>{item.code}</em>)}</button>)}</div>{selected && <aside className="model-preview-panel"><header><div><span className="eyebrow">CACHE VERSIONNÉ · SOURCE IMMUABLE</span><h3>{selected.key.resref}</h3></div><button type="button" onClick={() => setSelected(undefined)} aria-label="Fermer l’aperçu"><X size={15} /></button></header>{selected.glbPreview ? <ModelPreview jobId={jobId} asset={selected} texture={texture} /> : imageSelected ? <ImagePreview jobId={jobId} asset={selected} /> : <div className="model-preview-empty"><AlertTriangle size={24} /><p>Aucun aperçu visuel disponible pour cette ressource.</p></div>}{selected.key.resourceType === 2002 ? <div className="model-preview-metrics"><span>{selected.meshCount} mesh</span><span>{selected.skinCount} skin</span><span>{selected.walkmeshCount} walkmesh</span><span>{selected.textures.length} textures</span><span>supermodel · {selected.supermodel ?? "aucun"}</span></div> : <div className="model-preview-metrics"><span>{selected.format.toUpperCase()}</span><span>{selected.width ?? "?"}×{selected.height ?? "?"}</span><span>{selected.sha256.slice(0, 12)}…</span></div>}</aside>}</div>;
}

function ImagePreview({ jobId, asset }: { jobId: string; asset: AssetRecord }) {
  const [preview, setPreview] = useState<string>();
  const [status, setStatus] = useState("Chargement de la texture…");
  useEffect(() => {
    let objectUrl: string | undefined;
    let disposed = false;
    void assetPreviewBytes({ jobId, resref: asset.key.resref, resourceType: asset.key.resourceType }).then((bytes) => {
      if (disposed) return;
      objectUrl = URL.createObjectURL(new Blob([bytes], { type: textureMime(asset.format) }));
      setPreview(objectUrl);
      setStatus(asset.format === "plt" ? "PLT · aperçu local des couches recolorables" : asset.format.toUpperCase() + " · ressource résolue");
    }).catch((error: unknown) => { if (!disposed) setStatus("Échec de l’aperçu · " + normalizeAppError(error).technicalMessage); });
    return () => { disposed = true; if (objectUrl) URL.revokeObjectURL(objectUrl); };
  }, [asset.format, asset.key.resref, asset.key.resourceType, jobId]);
  return <div className="texture-preview">{preview ? <img src={preview} alt={"Aperçu " + asset.key.resref} /> : <CircleGauge size={28} />}<span>{status}</span></div>;
}

function ModelPreview({ jobId, asset, texture }: { jobId: string; asset: AssetRecord; texture?: AssetRecord }) {
  const [status, setStatus] = useState("Conversion MDL → GLB…");
  useEffect(() => {
    const selector = 'canvas[data-model="' + CSS.escape(asset.key.resref) + '"]';
    const canvas = document.querySelector<HTMLCanvasElement>(selector);
    if (!canvas || !canvas.getContext("webgl")) return;
    let disposed = false; let cleanup = () => {};
    void Promise.all([import("@babylonjs/core"), import("@babylonjs/loaders/glTF"), import("@babylonjs/core/Materials/Textures/Loaders/ddsTextureLoader"), import("@babylonjs/core/Materials/Textures/Loaders/tgaTextureLoader"), import("@babylonjs/core/Materials/Textures/Loaders/ktxTextureLoader")]).then(async ([B]) => {
      const buffer = await modelPreviewGlb({ jobId, resref: asset.key.resref });
      if (disposed) return;
      const engine = new B.Engine(canvas, true, { preserveDrawingBuffer: true, stencil: true }); const scene = new B.Scene(engine); scene.clearColor = new B.Color4(0.035, 0.05, 0.065, 1);
      const camera = new B.ArcRotateCamera("model-camera", -Math.PI / 2, Math.PI / 2.6, 8, B.Vector3.Zero(), scene); camera.attachControl(canvas, true); camera.lowerRadiusLimit = 0.05; camera.wheelPrecision = 20;
      new B.HemisphericLight("model-light", new B.Vector3(0.3, 1, -0.4), scene).intensity = 1.1;
      const file = new File([buffer], asset.key.resref + ".glb", { type: "model/gltf-binary" });
      const result = await B.SceneLoader.ImportMeshAsync(null, "", file, scene);
      if (disposed) { scene.dispose(); engine.dispose(); return; }
      if (texture) { const textureBuffer = await assetPreviewBytes({ jobId, resref: texture.key.resref, resourceType: texture.key.resourceType }); const previewTexture = new B.Texture("data:" + texture.key.resref + "." + texture.format, scene, false, true, B.Texture.TRILINEAR_SAMPLINGMODE, undefined, undefined, textureBuffer, true, undefined, textureMime(texture.format)); for (const mesh of result.meshes) { if (mesh.material instanceof B.PBRMaterial) mesh.material.albedoTexture = previewTexture; else if (mesh.material instanceof B.StandardMaterial) mesh.material.diffuseTexture = previewTexture; } }
      let minimum = new B.Vector3(Number.POSITIVE_INFINITY, Number.POSITIVE_INFINITY, Number.POSITIVE_INFINITY); let maximum = new B.Vector3(Number.NEGATIVE_INFINITY, Number.NEGATIVE_INFINITY, Number.NEGATIVE_INFINITY);
      for (const mesh of result.meshes) { const bounds = mesh.getHierarchyBoundingVectors(true); minimum = B.Vector3.Minimize(minimum, bounds.min); maximum = B.Vector3.Maximize(maximum, bounds.max); }
      if (result.meshes.length > 0) { const extent = maximum.subtract(minimum); camera.target = minimum.add(extent.scale(0.5)); camera.radius = Math.max(extent.length() * 1.25, 0.5); }
      scene.animationGroups[0]?.start(true);
      setStatus(result.meshes.length + " nœuds chargés · " + scene.animationGroups.length + " animation(s)");
      engine.runRenderLoop(() => scene.render()); const resize = () => engine.resize(); window.addEventListener("resize", resize); cleanup = () => { window.removeEventListener("resize", resize); scene.dispose(); engine.dispose(); };
    }).catch((error: unknown) => { if (!disposed) setStatus("Échec de l’aperçu · " + normalizeAppError(error).technicalMessage); });
    return () => { disposed = true; cleanup(); };
  }, [asset.key.resref, jobId, texture]);
  return <div className="model-canvas"><canvas data-model={asset.key.resref} aria-label={"Aperçu 3D " + asset.key.resref} /><span>{status}</span></div>;
}

function textureMime(format: string) { return ({ dds: "image/vnd-ms.dds", tga: "image/x-tga", ktx: "image/ktx", plt: "image/png", png: "image/png", jpg: "image/jpeg", gif: "image/gif" } as Record<string, string>)[format] ?? "application/octet-stream"; }
function textureMimeForResourceType(resourceType: number) { return ({ 2033: "image/vnd-ms.dds", 3: "image/x-tga", 2073: "image/ktx", 6: "image/png", 2080: "image/png", 2081: "image/jpeg", 2079: "image/gif" } as Record<number, string>)[resourceType] ?? "application/octet-stream"; }

function SceneView({ jobId, world, filter }: { jobId: string; world: WorldIndex; filter: string }) {
  const scenes = world.scenes.filter((value) => value.area.toLocaleLowerCase().includes(filter.toLocaleLowerCase())); const [selected, setSelected] = useState<string>();
  const [showOverlays, setShowOverlays] = useState(true); const [showWalkmeshes, setShowWalkmeshes] = useState(false); const [wireframe, setWireframe] = useState(false); const [cameraMode, setCameraMode] = useState<"orbit" | "aurora">("orbit");
  useEffect(() => { if (!scenes.some((value) => value.area === selected)) setSelected(scenes[0]?.area); }, [scenes, selected]);
  const scene = scenes.find((value) => value.area === selected);
  return <div className="scene-workspace"><div className="scene-toolbar"><select value={selected ?? ""} onChange={(event) => setSelected(event.currentTarget.value)}>{scenes.map((value) => <option key={value.area}>{value.area}</option>)}</select>{scene && <span>{scene.resolvedAssets}/{scene.objects.length + scene.overlays.length} modèles résolus · {scene.uniqueModels} GLB uniques · {scene.missingAssets} dégradé(s)</span>}<label><input type="checkbox" checked={showOverlays} onChange={(event) => setShowOverlays(event.currentTarget.checked)} /> Overlays</label><label><input type="checkbox" checked={showWalkmeshes} onChange={(event) => setShowWalkmeshes(event.currentTarget.checked)} /> Walkmeshes</label><label><input type="checkbox" checked={wireframe} onChange={(event) => setWireframe(event.currentTarget.checked)} /> Filaire</label><button type="button" onClick={() => setCameraMode((value) => value === "orbit" ? "aurora" : "orbit")}>{cameraMode === "orbit" ? "Vue Aurora" : "Vue orbitale"}</button></div>{scene ? <BabylonScene jobId={jobId} manifest={scene} showOverlays={showOverlays} showWalkmeshes={showWalkmeshes} wireframe={wireframe} cameraMode={cameraMode} /> : <p>Aucune scène.</p>}</div>;
}

function BabylonScene({ jobId, manifest, showOverlays, showWalkmeshes, wireframe, cameraMode }: { jobId: string; manifest: SceneManifest; showOverlays: boolean; showWalkmeshes: boolean; wireframe: boolean; cameraMode: "orbit" | "aurora" }) {
  const [picked, setPicked] = useState<string>(); const [status, setStatus] = useState("Préparation de la scène…");
  useEffect(() => {
    const canvas = document.querySelector<HTMLCanvasElement>('canvas[data-area="' + CSS.escape(manifest.area) + '"]');
    if (!canvas || !canvas.getContext("webgl")) return;
    let disposed = false; let cleanup = () => {};
    setStatus("Résolution des GLB…");
    void Promise.all([import("@babylonjs/core"), import("@babylonjs/loaders/glTF"), import("@babylonjs/core/Materials/Textures/Loaders/ddsTextureLoader"), import("@babylonjs/core/Materials/Textures/Loaders/tgaTextureLoader"), import("@babylonjs/core/Materials/Textures/Loaders/ktxTextureLoader")]).then(async ([B]) => {
      if (disposed) return;
      const engine = new B.Engine(canvas, true, { preserveDrawingBuffer: true, stencil: true }); const scene = new B.Scene(engine); scene.clearColor = new B.Color4(0.045, 0.065, 0.085, 1);
      const resize = () => engine.resize(); window.addEventListener("resize", resize); cleanup = () => { window.removeEventListener("resize", resize); scene.dispose(); engine.dispose(); };
      const radius = Math.max(manifest.width, manifest.height) * 14 + 20; const target = new B.Vector3(manifest.width * 5, 0, manifest.height * 5);
      const camera = new B.ArcRotateCamera("camera", -Math.PI / 2, cameraMode === "aurora" ? 0.32 : Math.PI / 3, cameraMode === "aurora" ? radius * 0.82 : radius, target, scene); camera.attachControl(canvas, true); camera.wheelPrecision = 10; camera.panningSensibility = 70; camera.lowerRadiusLimit = 2;
      new B.HemisphericLight("light", new B.Vector3(0, 1, 0), scene).intensity = 0.85;
      const ground = B.MeshBuilder.CreateGround("ground", { width: Math.max(10, manifest.width * 10), height: Math.max(10, manifest.height * 10) }, scene); const groundMaterial = new B.StandardMaterial("ground-material", scene); groundMaterial.diffuseColor = new B.Color3(0.12, 0.18, 0.22); ground.material = groundMaterial;
      const marker = (value: SceneManifest["objects"][number], failed = false) => { const mesh = B.MeshBuilder.CreateBox("marker:" + value.id, { size: value.kind === "tile" ? 9.7 : 1.4, height: value.kind === "tile" ? 0.18 : 2.2 }, scene); mesh.position = new B.Vector3(value.x, value.kind === "tile" ? -0.08 : value.y + 1, value.z); mesh.rotation.y = value.rotation; mesh.metadata = value; const material = new B.StandardMaterial("marker-material:" + value.id, scene); material.diffuseColor = failed || value.marker ? new B.Color3(0.83, 0.45, 0.19) : new B.Color3(0.25, 0.55, 0.68); material.alpha = value.kind === "tile" ? 0.45 : 0.72; material.wireframe = wireframe || value.kind !== "tile"; mesh.material = material; return mesh; };
      const visibleObjects = manifest.objects.slice(0, 1200); const technical = showOverlays ? manifest.overlays.slice(0, Math.max(0, 1200 - visibleObjects.length)) : [];
      for (const value of [...visibleObjects.filter((item) => item.marker), ...technical]) marker(value);
      const groups = new globalThis.Map<string, Array<SceneManifest["objects"][number]>>();
      for (const value of visibleObjects) { if (value.marker) continue; for (const resref of value.modelResrefs) { const values = groups.get(resref) ?? []; values.push(value); groups.set(resref, values); } }
      const componentCount = [...groups.values()].reduce((total, values) => total + values.length, 0);
      let loaded = 0; let failed = 0; let texturesLoaded = 0; let bytesLoaded = 0;
      const textureCache = new globalThis.Map<string, Promise<InstanceType<typeof B.Texture> | null>>();
      const loadTexture = (resref: string) => { const existing = textureCache.get(resref); if (existing) return existing; const pending = (async () => { const key = await resolveTexture({ jobId, resref }); if (!key || disposed) return null; const bytes = await assetPreviewBytes({ jobId, resref: key.resref, resourceType: key.resourceType }); if (disposed || bytesLoaded + bytes.byteLength > manifest.memoryBudgetBytes) return null; bytesLoaded += bytes.byteLength; texturesLoaded += 1; return new B.Texture("data:" + key.resref, scene, false, true, B.Texture.TRILINEAR_SAMPLINGMODE, undefined, undefined, bytes, true, undefined, textureMimeForResourceType(key.resourceType)); })().catch(() => null); textureCache.set(resref, pending); return pending; };
      for (const [resref, values] of groups) {
        if (disposed) break;
        try {
          const buffer = await modelPreviewGlb({ jobId, resref });
          if (disposed) break;
          if (bytesLoaded + buffer.byteLength > manifest.memoryBudgetBytes) { failed += values.length; values.forEach((value) => marker(value, true)); setStatus("Budget mémoire atteint · " + loaded + " objets chargés"); continue; }
          bytesLoaded += buffer.byteLength;
          const file = new File([buffer], resref + ".glb", { type: "model/gltf-binary" }); const container = await B.SceneLoader.LoadAssetContainerAsync("", file, scene);
          for (const material of container.materials) { const extras = (material.metadata as { gltf?: { extras?: { nwnTextures?: Array<string | null> } } } | undefined)?.gltf?.extras; const textureResref = extras?.nwnTextures?.find((value): value is string => Boolean(value && value !== "null")); if (!textureResref) continue; const texture = await loadTexture(textureResref.toLocaleLowerCase()); if (texture) { if (material instanceof B.PBRMaterial) material.albedoTexture = texture; else if (material instanceof B.StandardMaterial) material.diffuseTexture = texture; } }
          for (const value of values) {
            const instance = container.instantiateModelsToScene((name) => value.id + ":" + name, true); const anchor = new B.TransformNode("anchor:" + value.id, scene); anchor.position = new B.Vector3(value.x, value.y, value.z); anchor.rotation.y = value.rotation; anchor.metadata = value;
            for (const root of instance.rootNodes) { root.parent = anchor; const meshes = root instanceof B.AbstractMesh ? [root, ...root.getChildMeshes(false)] : root.getChildMeshes(false); for (const mesh of meshes) { mesh.metadata = value; const extras = (mesh.material?.metadata as { gltf?: { extras?: { walkmesh?: boolean } } } | undefined)?.gltf?.extras; const isWalkmesh = extras?.walkmesh === true; mesh.isVisible = !isWalkmesh || showWalkmeshes; if (mesh.material) mesh.material.wireframe = wireframe || isWalkmesh; } }
            instance.animationGroups[0]?.start(true); loaded += 1;
          }
          setStatus(loaded + "/" + componentCount + " composants · " + texturesLoaded + " textures · " + (bytesLoaded / 1048576).toFixed(1) + " Mio");
        } catch { failed += values.length; values.forEach((value) => marker(value, true)); setStatus(loaded + " chargés · " + failed + " en mode dégradé"); }
      }
      if (disposed) return;
      if (!disposed) setStatus(loaded + " composants · " + texturesLoaded + " textures · " + failed + " dégradé(s) · " + (bytesLoaded / 1048576).toFixed(1) + " Mio");
      let highlighted: (typeof scene.meshes)[number] | undefined;
      scene.onPointerObservable.add((event) => { const mesh = event.pickInfo?.pickedMesh; const metadata = mesh?.metadata as SceneManifest["objects"][number] | undefined; if (mesh && metadata) { if (highlighted) highlighted.showBoundingBox = false; highlighted = mesh; mesh.showBoundingBox = true; setPicked(metadata.label + " · " + metadata.kind + (metadata.modelResref ? " · " + metadata.modelResref + ".mdl" : "") + " · " + metadata.sourcePath); } });
      engine.runRenderLoop(() => scene.render());
    }).catch((error: unknown) => { if (!disposed) { cleanup(); setStatus("Échec de la scène · " + normalizeAppError(error).technicalMessage); } });
    return () => { disposed = true; cleanup(); };
  }, [cameraMode, jobId, manifest, showOverlays, showWalkmeshes, wireframe]);
  return <div className="babylon-frame"><canvas data-area={manifest.area} aria-label={"Vue 3D " + manifest.area} /><div className="scene-status"><span>Babylon.js · {cameraMode === "aurora" ? "caméra Aurora" : "caméra orbitale"} · {status}</span>{picked && <code>{picked}</code>}</div></div>;
}

function GlobalGraphView({ jobId, world, filter }: { jobId: string; world: WorldIndex; filter: string }) {
  const query = filter.toLocaleLowerCase(); const nodes = world.graphNodes.filter((value) => (value.id + " " + value.kind + " " + value.label).toLocaleLowerCase().includes(query)).slice(0, 120); const ids = new Set(nodes.map((value) => value.id)); const edges = world.graphEdges.filter((value) => ids.has(value.source) || ids.has(value.target)).slice(0, 300);
  const reportQuery = useQuery({ queryKey: ["diagnostic-report", jobId], queryFn: () => diagnosticReport({ jobId }), enabled: false });
  const download = async (kind: "json" | "html") => { const result = reportQuery.data ?? (await reportQuery.refetch()).data; if (!result) return; const content = kind === "json" ? result.json : result.html; const blob = new Blob([content], { type: kind === "json" ? "application/json" : "text/html" }); const link = document.createElement("a"); link.href = URL.createObjectURL(blob); link.download = "opennever-diagnostic." + kind; link.click(); URL.revokeObjectURL(link.href); };
  return <div className="global-report"><div className="report-actions"><button type="button" onClick={() => void download("json")}><Download size={13} /> Rapport JSON stable</button><button type="button" onClick={() => void download("html")}><Download size={13} /> Rapport HTML autonome</button><span>Schéma v1 · chemins de preuve anonymisés</span></div><div className="targeted-graph"><div><h3>Nœuds ciblés · {nodes.length}/{world.graphNodes.length}</h3>{nodes.map((value) => <article key={value.id}><code>{value.kind}</code><strong>{value.label}</strong><small>{value.id}</small></article>)}</div><div><h3>Relations proches · {edges.length}</h3>{edges.map((value) => <article key={value.id} className={value.confidence}><span>{value.source} → {value.target}</span><b>{value.kind} · {value.confidence}</b><small>{value.evidence.resource} · {value.evidence.fieldPath}</small></article>)}</div><div><h3>Diagnostics · {world.diagnostics.length}</h3>{world.diagnostics.slice(0, 250).map((value, index) => <article key={value.code + ":" + value.resource + ":" + String(index)} className={value.severity}><b>{value.code}</b><span>{value.message}</span><small>{value.resource}</small></article>)}</div></div></div>;
}

function StructuredSummaryView({ summary }: { summary: StructuredResourceSummary }) {
  return (
    <section className="inventory-card structured-card" aria-label="Données NWN structurées">
      <div className="inventory-heading">
        <div>
          <span className="eyebrow">GFF · TLK · 2DA</span>
          <h2>Compréhension métier</h2>
        </div>
        <span className={summary.gff.failed ? "format-badge warning" : "format-badge"}>
          {summary.gff.parsed}/{summary.gff.discovered} GFF
        </span>
      </div>
      <div className="inventory-metrics">
        <Metric label="Zones" value={summary.areas.length.toLocaleString("fr-FR")} />
        <Metric label="Blueprints" value={summary.blueprints.length.toLocaleString("fr-FR")} />
        <Metric label="Tables 2DA" value={summary.twoDaTables.length.toLocaleString("fr-FR")} />
        <Metric label="Tables TLK" value={summary.talkTables.length.toLocaleString("fr-FR")} />
      </div>
      <p className="structured-note">
        {summary.gff.structCount.toLocaleString("fr-FR")} structures et {summary.gff.fieldCount.toLocaleString("fr-FR")} champs conservés avec leur type et leur ordre.
      </p>
      {summary.diagnostics.length > 0 && (
        <p className="inventory-limit">{summary.diagnostics.length} ressource(s) structurée(s) nécessitent une vérification.</p>
      )}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function resourceKeyName(key: { resref: string; resourceType: number }) {
  return `${key.resref}.#${key.resourceType}`;
}

function localizedPrimary(value: { values: Array<{ languageId: number; text: string }> } | undefined) {
  return value?.values.find((item) => item.languageId === 0)?.text ?? value?.values[0]?.text;
}

function Property({ label, value }: { label: string; value: string }) {
  return (
    <div className="property-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

export default App;
