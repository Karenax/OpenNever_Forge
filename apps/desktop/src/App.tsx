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
  Map,
  MessageSquareText,
  PanelLeftClose,
  Search,
  ShieldCheck,
  SquareStack,
  Orbit,
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
  inspectScript,
  inspectWorld,
  modelPreviewGlb,
  diagnosticReport,
  normalizeAppError,
  queryResources,
  queryDialogues,
  queryScripts,
  resolveTexture,
  selectDirectory,
  selectModule,
  startModuleAnalysis,
  type JobSnapshot,
  type ModuleDependency,
  type ModuleDependencyReport,
  type ResolvedResource,
  type ResourceCatalogSummary,
  type ResourceInspection,
  type DialogueGraph,
  type DialogueIndexSummary,
  type DialogueTreeNode,
  type ScriptDocument,
  type ScriptIndexSummary,
  type StructuredResourceSummary,
  type AreaMap,
  type AssetRecord,
  type SceneManifest,
  type WorldIndex,
  type WorldSummary,
} from "./lib/tauri";
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
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([
    {
      id: "readonly",
      level: "info",
      code: "READ_ONLY_PHASE",
      message: "Phase 1 active : aucune ressource NWN ne sera modifiée.",
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
        }),
      );
    } catch (error) {
      pushError(error);
    } finally {
      setInspectionBusy(false);
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
          <span className="readonly-badge">
            <ShieldCheck size={13} /> Lecture seule
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

            {dependencyReport && <DependencyReportView report={dependencyReport} />}

            {structuredSummary && <StructuredSummaryView summary={structuredSummary} />}

            {scriptIndexSummary && <ScriptSummaryView summary={scriptIndexSummary} />}

            {dialogueIndexSummary && <DialogueSummaryView summary={dialogueIndexSummary} />}

            {worldSummary && <WorldSummaryView summary={worldSummary} />}

            {scriptIndexSummary && jobId && activeExplorerItem === "scripts" && (
              <ScriptWorkspace jobId={jobId} summary={scriptIndexSummary} filter={resourceFilter} />
            )}

            {dialogueIndexSummary && jobId && activeExplorerItem === "dialogues" && (
              <DialogueWorkspace jobId={jobId} summary={dialogueIndexSummary} filter={resourceFilter} onOpenScript={(script) => { setResourceFilter(script); setActiveExplorerItem("scripts"); }} />
            )}

            {worldSummary && jobId && ["narrative", "areas", "assets", "scene", "graph"].includes(activeExplorerItem) && (
              <PhaseOneWorkspace jobId={jobId} activeView={activeExplorerItem} />
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
                <strong>Garantie de la Phase 1</strong>
                <p>Le module source, les HAK et l'installation du jeu ne sont jamais ouverts en écriture.</p>
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
            <Property label="Mode" value="Lecture seule" />
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
                <pre className="raw-inspector">{JSON.stringify(inspection.value, null, 2)}</pre>
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

function DependencyReportView({ report }: { report: ModuleDependencyReport }) {
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

function DialogueWorkspace({ jobId, summary, filter, onOpenScript }: { jobId: string; summary: DialogueIndexSummary; filter: string; onOpenScript: (script: string) => void }) {
  const [page, setPage] = useState(0); const [selected, setSelected] = useState<string>(); const pageSize=50;
  useEffect(()=>setPage(0),[filter]);
  const pageQuery=useQuery({queryKey:["dialogues",jobId,filter,page],queryFn:()=>queryDialogues({jobId,query:filter,offset:page*pageSize,limit:pageSize})});
  const items=pageQuery.data?.items??[];
  useEffect(()=>{if(!selected&&items[0])setSelected(items[0].resref);if(selected&&items.length&&!items.some(value=>value.resref===selected))setSelected(items[0].resref)},[items,selected]);
  const graphQuery=useQuery({queryKey:["dialogue",jobId,selected],queryFn:()=>inspectDialogue({jobId,resref:selected as string}),enabled:Boolean(selected)});
  const total=pageQuery.data?.total??0; const pages=Math.max(1,Math.ceil(total/pageSize));
  return <section className="inventory-card dialogue-workspace" aria-label="Explorateur de dialogues">
    <div className="inventory-heading"><div><span className="eyebrow">STRUCTURE COMPLÈTE · PROVENANCE GFF</span><h2>Dialogues</h2></div><span className="format-badge">{total} résultat(s)</span></div>
    <div className="dialogue-layout"><div className="dialogue-list">{items.map(item=><button type="button" key={item.resref} className={selected===item.resref?"dialogue-list-item selected":"dialogue-list-item"} onClick={()=>setSelected(item.resref)}><span><MessageSquareText size={13}/><code>{item.resref}</code></span><small>{item.nodeCount} nœuds · {item.linkCount} liens · {item.cycleCount} cycles</small>{item.preview&&<em>{item.preview}</em>}</button>)}
      {!items.length&&<p className="resource-empty">{pageQuery.isLoading?"Indexation…":"Aucun dialogue ne correspond."}</p>}
      <div className="catalog-pagination compact"><button type="button" disabled={page===0} onClick={()=>setPage(value=>Math.max(0,value-1))}>‹</button><span>{page+1}/{pages}</span><button type="button" disabled={page+1>=pages} onClick={()=>setPage(value=>value+1)}>›</button></div>
    </div><DialogueGraphView graph={graphQuery.data} loading={graphQuery.isLoading} onOpenScript={onOpenScript}/></div>
    <span className="script-total-hidden">{summary.nodes}</span>
  </section>;
}

function DialogueGraphView({ graph, loading, onOpenScript }: { graph?: DialogueGraph; loading: boolean; onOpenScript: (script: string)=>void }) {
  const [tab,setTab]=useState<"tree"|"graph"|"raw">("tree"); const [selectedNode,setSelectedNode]=useState<string>();
  useEffect(()=>setSelectedNode(graph?.roots[0]??graph?.nodes[0]?.id),[graph]);
  if(loading)return <div className="dialogue-empty">Ouverture du dialogue…</div>; if(!graph)return <div className="dialogue-empty">Sélectionnez un dialogue.</div>;
  const node=graph.nodes.find(value=>value.id===selectedNode);
  return <div className="dialogue-document"><div className="script-tabs"><button type="button" className={tab==="tree"?"active":""} onClick={()=>setTab("tree")}>Arbre simplifié</button><button type="button" className={tab==="graph"?"active":""} onClick={()=>setTab("graph")}>Graphe complet</button><button type="button" className={tab==="raw"?"active":""} onClick={()=>setTab("raw")}>GFF brut</button><strong>{graph.key.resref}</strong></div>
    <div className="dialogue-content">{tab==="tree"?<div className="dialogue-tree">{graph.tree.map(value=><DialogueTreeBranch key={value.nodeId} value={value} onSelect={setSelectedNode}/>)}</div>:tab==="graph"?<DialogueFlow graph={graph} onSelect={setSelectedNode}/>:<pre className="dialogue-raw">{JSON.stringify(graph.raw,null,2)}</pre>}</div>
    <DialogueInspector graph={graph} node={node} onOpenScript={onOpenScript}/>
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

function DialogueInspector({ graph, node, onOpenScript }: { graph: DialogueGraph; node?: DialogueGraph["nodes"][number]; onOpenScript:(script:string)=>void }) {
  return <div className="dialogue-inspection"><div><h3>Nœud sélectionné</h3>{node?<><strong>{node.id}</strong><p>{node.displayText??"Texte non résolu"}</p>{node.speaker&&<span>Locuteur · {node.speaker}</span>}{node.comment&&<span>Commentaire · {node.comment}</span>}{node.animation!==null&&<span>Animation · {node.animation}{node.animationLoop?" · boucle":""}</span>}{node.sound&&<span>Son · {node.sound}</span>}{node.quest&&<span>Quête · {node.quest}</span>}{node.actionScript&&<button type="button" onClick={()=>onOpenScript(node.actionScript as string)}><Code2 size={12}/> Action · {node.actionScript}</button>}</>:<span>Aucun nœud.</span>}</div>
    <div><h3>Scripts des liens</h3>{graph.links.filter(link=>link.source===node?.id).map(link=><div className="dialogue-link-meta" key={link.id}><span>→ {link.target}{link.isChild?" · partagé":""}</span>{link.conditionScript&&<button type="button" onClick={()=>onOpenScript(link.conditionScript as string)}><Code2 size={12}/> Condition · {link.conditionScript}</button>}{link.actionScript&&<button type="button" onClick={()=>onOpenScript(link.actionScript as string)}><Code2 size={12}/> Action · {link.actionScript}</button>}</div>)}</div>
    <div><h3>Références entrantes</h3>{graph.references.slice(0,100).map(value=><span key={`${value.resource.resref}-${value.fieldPath}`}>{value.resource.resref}.#{value.resource.resourceType} · {value.fieldPath}</span>)}{graph.references.length===0&&<span>Aucune référence GFF détectée.</span>}</div>
    {graph.diagnostics.length>0&&<div><h3>Diagnostics</h3>{graph.diagnostics.slice(0,50).map((value,index)=><span className="missing" key={`${value.code}-${index}`}>{value.code} · {value.message}</span>)}</div>}
  </div>;
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

function ScriptWorkspace({ jobId, summary, filter }: { jobId: string; summary: ScriptIndexSummary; filter: string }) {
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
        <ScriptDocumentView document={document} loading={inspectionQuery.isLoading} tab={tab} onTab={setTab} />
      </div>
      <p className="inventory-limit">Vérification de compilation désactivée : aucun compilateur NWScript n'est embarqué ni exécuté pendant la Phase 1.</p>
      <span className="script-total-hidden" aria-hidden="true">{summary.paired}</span>
    </section>
  );
}

function ScriptDocumentView({ document, loading, tab, onTab }: { document?: ScriptDocument; loading: boolean; tab: "source" | "bytecode"; onTab: (tab: "source" | "bytecode") => void }) {
  if (loading) return <div className="script-document-empty">Ouverture du script…</div>;
  if (!document) return <div className="script-document-empty">Sélectionnez un script.</div>;
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
            <Editor height="430px" theme="opennever-dark" language="nwscript" value={document.nss.text} beforeMount={configureNwscriptMonaco} options={{ readOnly: true, domReadOnly: true, automaticLayout: true, minimap: { enabled: false }, fontFamily: "Cascadia Code, Consolas, monospace", fontSize: 12, scrollBeyondLastLine: false, renderWhitespace: "selection" }} />
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

function PhaseOneWorkspace({ jobId, activeView }: { jobId: string; activeView: string }) {
  const [filter, setFilter] = useState("");
  const worldQuery = useQuery({ queryKey: ["world", jobId], queryFn: () => inspectWorld({ jobId }), staleTime: Number.POSITIVE_INFINITY });
  const world = worldQuery.data;
  if (worldQuery.isLoading) return <section className="inventory-card world-workspace">Construction de la vue métier…</section>;
  if (!world) return <section className="inventory-card world-workspace">L’index global n’est pas disponible.</section>;
  return <section className="inventory-card world-workspace" aria-label="Explorateur de la Phase 1">
    <div className="inventory-heading"><div><span className="eyebrow">SOURCE RUST · PROVENANCE CONSERVÉE</span><h2>{worldViewTitle(activeView)}</h2></div><label className="world-filter"><Search size={13} /><input value={filter} onChange={(event) => setFilter(event.currentTarget.value)} placeholder="Filtrer cette vue…" /></label></div>
    {activeView === "narrative" && <NarrativeView world={world} filter={filter} />}
    {activeView === "areas" && <AreaMapView world={world} filter={filter} />}
    {activeView === "assets" && <AssetView jobId={jobId} assets={world.assets.assets} filter={filter} />}
    {activeView === "scene" && <SceneView jobId={jobId} world={world} filter={filter} />}
    {activeView === "graph" && <GlobalGraphView jobId={jobId} world={world} filter={filter} />}
  </section>;
}

function worldViewTitle(view: string) {
  return ({ narrative: "Journal, quêtes et factions", areas: "Carte 2D des zones", assets: "Modèles, textures et animations", scene: "Vue 3D des zones", graph: "Graphe global et validation" } as Record<string, string>)[view] ?? "Phase 1";
}

function NarrativeView({ world, filter }: { world: WorldIndex; filter: string }) {
  const query = filter.trim().toLocaleLowerCase();
  const categories = world.narrative.categories.filter((value) => [value.tag, value.name.text ?? "", String(value.name.stringRef ?? "")].some((candidate) => candidate.toLocaleLowerCase().includes(query)));
  return <div className="narrative-layout"><div className="journal-list"><h3>Journal · {categories.length} catégorie(s)</h3>
    {categories.map((category) => <article key={category.tag} className="journal-category"><header><strong>{category.name.text ?? category.tag}</strong><code>{category.tag}</code><span>priorité {category.priority} · {category.xp} XP</span></header>
      {category.entries.map((entry) => <div className={entry.finalState ? "journal-entry final" : "journal-entry"} key={entry.id}><b>Étape {entry.id}</b><span>{entry.text.text ?? "StrRef " + String(entry.text.stringRef ?? "absente")}</span>{entry.finalState && <em>état final</em>}</div>)}
    </article>)}</div><FactionMatrix world={world} query={query} /><div className="confidence-legend"><b>Confiance</b><span className="certain">certain · champ explicite</span><span className="probable">probable · rapprochement nommé</span><span className="possible">possible · cible non résolue</span></div>
  </div>;
}

function FactionMatrix({ world, query }: { world: WorldIndex; query: string }) {
  const factions = world.narrative.factions.filter((value) => value.name.toLocaleLowerCase().includes(query)).slice(0, 24);
  const reputation = new globalThis.Map(world.narrative.reputations.map((value) => [String(value.sourceId) + ":" + String(value.targetId), value.value]));
  return <div className="faction-matrix"><h3>Matrice des factions · {world.narrative.factions.length}</h3><div className="faction-table-wrap"><table><thead><tr><th>Faction</th>{factions.map((value) => <th key={value.id} title={value.name}>{value.id}</th>)}</tr></thead><tbody>{factions.map((source) => <tr key={source.id}><th>{source.name}</th>{factions.map((target) => { const score = reputation.get(String(source.id) + ":" + String(target.id)); return <td key={target.id} className={score === undefined ? "" : score < 10 ? "hostile" : score > 50 ? "friendly" : "neutral"}>{score ?? "—"}</td>; })}</tr>)}</tbody></table></div></div>;
}

function AreaMapView({ world, filter }: { world: WorldIndex; filter: string }) {
  const areas = world.areas.filter((value) => (value.resref + " " + (value.name.text ?? "") + " " + (value.tileset ?? "")).toLocaleLowerCase().includes(filter.toLocaleLowerCase()));
  const [selected, setSelected] = useState<string>(); const [instance, setInstance] = useState<string>();
  useEffect(() => { if (!areas.some((value) => value.resref === selected)) setSelected(areas[0]?.resref); }, [areas, selected]);
  const area = areas.find((value) => value.resref === selected);
  return <div className="area-workspace"><div className="area-list">{areas.map((value) => <button type="button" className={value.resref === selected ? "selected" : ""} key={value.resref} onClick={() => setSelected(value.resref)}><strong>{value.name.text ?? value.resref}</strong><code>{value.resref}</code><span>{value.width}×{value.height} · {value.instances.length} instances</span></button>)}</div>{area ? <><AreaCanvas area={area} selected={instance} onSelect={setInstance} /><AreaInspector area={area} selected={instance} /></> : <p>Aucune zone.</p>}</div>;
}

function AreaCanvas({ area, selected, onSelect }: { area: AreaMap; selected?: string; onSelect: (id: string) => void }) {
  const width = Math.max(area.width, 1); const height = Math.max(area.height, 1);
  return <div className="area-map-frame"><div className="area-map" style={{ aspectRatio: String(width) + "/" + String(height) }}>
    {area.tiles.map((tile) => <div key={String(tile.x) + ":" + String(tile.y)} className="area-tile" title={"Tuile " + String(tile.tileId) + " · orientation " + String(tile.orientation)} style={{ left: String(tile.x / width * 100) + "%", top: String(tile.y / height * 100) + "%", width: String(100 / width) + "%", height: String(100 / height) + "%", transform: "rotate(" + String(tile.orientation * 90) + "deg)" }}><span>{tile.tileId}</span></div>)}
    {area.instances.map((value) => <button type="button" aria-label={value.category + " " + (value.tag ?? value.templateResref ?? "")} key={value.id} onClick={() => onSelect(value.id)} className={"area-marker " + value.category + (selected === value.id ? " selected" : "")} style={{ left: String(Math.max(0, Math.min(100, value.x / (width * 10) * 100))) + "%", bottom: String(Math.max(0, Math.min(100, value.y / (height * 10) * 100))) + "%" }} />)}
  </div></div>;
}

function AreaInspector({ area, selected }: { area: AreaMap; selected?: string }) {
  const value = area.instances.find((candidate) => candidate.id === selected);
  return <aside className="area-detail"><h3>{area.name.text ?? area.resref}</h3><Property label="Tileset" value={area.tileset ?? "non résolu"} /><Property label="Tuiles" value={String(area.tiles.length)} /><Property label="Instances" value={String(area.instances.length)} />{value && <div className="instance-detail"><h4>{value.tag ?? value.templateResref ?? value.category}</h4><span>{value.category}</span><code>X {value.x.toFixed(2)} · Y {value.y.toFixed(2)} · Z {value.z.toFixed(2)}</code>{value.transitionDestination && <b>Transition → {value.transitionDestination}</b>}<small>{value.sourcePath}</small></div>}{area.diagnostics.map((item) => <p className="world-diagnostic" key={item.code + ":" + item.resource}>{item.code} · {item.message}</p>)}</aside>;
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
