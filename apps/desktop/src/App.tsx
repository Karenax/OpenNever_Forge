import {
  AlertTriangle,
  Archive,
  Box,
  Braces,
  ChevronRight,
  CircleGauge,
  Code2,
  Database,
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
  X,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import {
  cancelJob,
  getAppStatus,
  getJob,
  normalizeAppError,
  selectDirectory,
  selectModule,
  startModuleAnalysis,
  type ContainerResource,
  type JobSnapshot,
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
  { id: "dialogues", label: "Dialogues", icon: MessageSquareText },
  { id: "scripts", label: "Scripts", icon: Code2 },
  { id: "blueprints", label: "Blueprints", icon: Box },
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
  const moduleName = localizedPrimary(moduleInfo?.name);
  const appReady = statusQuery.data;
  const explorerGroups = useMemo(
    () =>
      explorerGroupDefinitions.map((item) => ({
        ...item,
        count:
          item.id === "resources"
            ? inventory?.resourceCount ?? 0
            : inventory?.resources.filter((resource) =>
                resourceTypesByGroup[item.id]?.has(resource.key.resourceType),
              ).length ?? 0,
      })),
    [inventory],
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
      const created = await startModuleAnalysis(project.modulePath);
      setJobId(created.id);
      setDiagnostics((current) => [
        ...current.filter((item) => item.id !== "MODULE_ANALYSIS_STARTED"),
        {
          id: "MODULE_ANALYSIS_STARTED",
          level: "info",
          code: "MODULE_ANALYSIS_STARTED",
          message: "Lecture de l'en-tête et de l'inventaire ERF lancée en arrière-plan.",
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
              <span>{inventory ? `${inventory.resourceCount} ressources` : "Module non indexé"}</span>
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
                : inventory
                  ? "Inventaire ERF prêt"
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
              <span className="eyebrow">LOT 1 · CONTENEUR ERF/MOD</span>
              <h1>{moduleName ?? "Ouvrir une copie de travail"}</h1>
              {moduleInfo ? (
                <p>
                  Module <code>{moduleInfo.tag}</code> · NWN {moduleInfo.minimumGameVersion} minimum ·
                  zone d'entrée <code>{moduleInfo.entryArea}</code>
                </p>
              ) : (
                <p>
                  Sélectionnez un module et les deux racines NWN. L'analyse calcule son empreinte puis
                  lit uniquement l'index ERF : aucun contenu de ressource n'est extrait ou modifié.
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

            {inventory && (
              <InventoryView
                inventory={inventory}
                activeGroup={activeExplorerItem}
                filter={resourceFilter}
              />
            )}

            <div className="safety-note">
              <ShieldCheck size={18} />
              <div>
                <strong>Garantie du premier lot</strong>
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
              {inventory
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
              <Property label="Version minimale" value={moduleInfo.minimumGameVersion} />
              <Property label="Zone d'entrée" value={moduleInfo.entryArea} />
              <Property label="HAK requis" value={moduleInfo.hakFiles.length.toLocaleString("fr-FR")} />
              <Property label="TLK personnalisé" value={moduleInfo.customTlk ?? "Aucun"} />
            </div>
          )}
        </aside>
      </section>

      <section className="diagnostics panel" aria-label="Diagnostics">
        <div className="diagnostic-tabs">
          <button type="button" className="active">Diagnostics</button>
          <button type="button">Import</button>
          <button type="button">Journal</button>
          <span>{diagnostics.length} message{diagnostics.length > 1 ? "s" : ""}</span>
        </div>
        <div className="diagnostic-list">
          {diagnostics.map((item) => (
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

type InventoryViewProps = {
  inventory: NonNullable<JobSnapshot["result"]>["inventory"];
  activeGroup: string;
  filter: string;
};

function InventoryView({ inventory, activeGroup, filter }: InventoryViewProps) {
  const normalizedFilter = filter.trim().toLocaleLowerCase("fr-FR");
  const groupTypes = resourceTypesByGroup[activeGroup];
  const resources = inventory.resources.filter((resource) => {
    const matchesGroup = activeGroup === "resources" || groupTypes?.has(resource.key.resourceType);
    const filename = resourceName(resource).toLocaleLowerCase("fr-FR");
    return matchesGroup && (!normalizedFilter || filename.includes(normalizedFilter));
  });
  const visibleResources = resources.slice(0, 200);

  return (
    <section className="inventory-card" aria-label="Inventaire ERF">
      <div className="inventory-heading">
        <div>
          <span className="eyebrow">INDEX BINAIRE VALIDÉ</span>
          <h2>Inventaire du module</h2>
        </div>
        <span className="format-badge">{inventory.fileType.trim()} · {inventory.fileVersion}</span>
      </div>
      <div className="inventory-metrics">
        <Metric label="Ressources" value={inventory.resourceCount.toLocaleString("fr-FR")} />
        <Metric label="Types présents" value={inventory.typeSummaries.length.toLocaleString("fr-FR")} />
        <Metric label="Affichées" value={resources.length.toLocaleString("fr-FR")} />
      </div>
      <div className="resource-table" role="table" aria-label="Ressources du module">
        <div className="resource-row resource-header" role="row">
          <span role="columnheader">ResRef</span>
          <span role="columnheader">Type</span>
          <span role="columnheader">Taille</span>
          <span role="columnheader">Offset</span>
        </div>
        {visibleResources.map((resource) => (
          <div
            className="resource-row"
            role="row"
            key={`${resource.resourceId}-${resource.key.resourceType}`}
          >
            <code role="cell">{resource.key.resref}</code>
            <span role="cell">{resource.extension?.toUpperCase() ?? `#${resource.key.resourceType}`}</span>
            <span role="cell">{resource.size.toLocaleString("fr-FR")}</span>
            <span role="cell">{resource.offset.toLocaleString("fr-FR")}</span>
          </div>
        ))}
        {visibleResources.length === 0 && (
          <div className="resource-empty">Aucune ressource ne correspond à cette vue.</div>
        )}
      </div>
      {resources.length > visibleResources.length && (
        <p className="inventory-limit">
          Affichage limité aux 200 premières ressources sur {resources.length.toLocaleString("fr-FR")}.
        </p>
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

function resourceName(resource: ContainerResource) {
  return resource.extension ? `${resource.key.resref}.${resource.extension}` : resource.key.resref;
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
