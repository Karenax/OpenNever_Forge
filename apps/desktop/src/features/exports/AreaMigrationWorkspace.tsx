import {
  AlertTriangle,
  CheckCircle2,
  FolderOpen,
  LoaderCircle,
  MapPinned,
  PackageOpen,
  ShieldAlert,
  XCircle,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import "./AreaMigrationWorkspace.css";
import {
  ExportWorkshopLockedState,
  ExportWorkshopPageHeader,
  composeBundleDestinationPath,
} from "./ExportWorkshopShell";
import {
  cancelJob,
  getAreaMigrationJob,
  getJob,
  listAreaMigrationCandidates,
  normalizeAppError,
  previewAreaMigration,
  selectDirectory,
  startAreaMigrationExport,
  type AreaMigrationCandidate,
  type AreaMigrationPreview,
  type JobSnapshot,
  type MigrationPhase,
} from "../../lib/tauri";

const terminalStates = new Set(["completed", "failed", "cancelled"]);

type AreaMigrationWorkspaceProps = {
  jobId?: string;
  analysisReady: boolean;
};

export function AreaMigrationWorkspace({ jobId, analysisReady }: AreaMigrationWorkspaceProps) {
  const analysisJobId = analysisReady ? jobId : undefined;
  const [candidates, setCandidates] = useState<AreaMigrationCandidate[]>([]);
  const [selectedArea, setSelectedArea] = useState("");
  const [preview, setPreview] = useState<AreaMigrationPreview>();
  const [destination, setDestination] = useState("");
  const [acceptedWarning, setAcceptedWarning] = useState(false);
  const [migrationJob, setMigrationJob] = useState<JobSnapshot>();
  const [loadingAreas, setLoadingAreas] = useState(false);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [launchBusy, setLaunchBusy] = useState(false);
  const [error, setError] = useState<string>();
  const reportPanelRef = useRef<HTMLElement>(null);

  useEffect(() => {
    if (!analysisJobId) {
      setCandidates([]);
      setSelectedArea("");
      setPreview(undefined);
      setMigrationJob(undefined);
      setLoadingAreas(false);
      return;
    }
    let disposed = false;
    setCandidates([]);
    setSelectedArea("");
    setLoadingAreas(true);
    setError(undefined);
    void listAreaMigrationCandidates(analysisJobId)
      .then((values) => {
        if (disposed) return;
        setCandidates(values);
        setSelectedArea(values[0]?.resref || "");
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(normalizeAppError(reason).userMessage);
      })
      .finally(() => {
        if (!disposed) setLoadingAreas(false);
      });
    return () => {
      disposed = true;
    };
  }, [analysisJobId]);

  useEffect(() => {
    if (!analysisJobId || !selectedArea) {
      setPreview(undefined);
      return;
    }
    let disposed = false;
    setPreviewBusy(true);
    setPreview(undefined);
    setDestination("");
    setAcceptedWarning(false);
    setMigrationJob(undefined);
    setError(undefined);
    void previewAreaMigration(analysisJobId, selectedArea)
      .then((value) => {
        if (!disposed) setPreview(value);
      })
      .then(() => getAreaMigrationJob(analysisJobId, selectedArea))
      .then((job) => {
        if (disposed || !job) return;
        setMigrationJob(job);
        if (job.migrationDestination) setDestination(job.migrationDestination);
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(normalizeAppError(reason).userMessage);
      })
      .finally(() => {
        if (!disposed) setPreviewBusy(false);
      });
    return () => {
      disposed = true;
    };
  }, [analysisJobId, selectedArea]);

  useEffect(() => {
    if (!migrationJob || terminalStates.has(migrationJob.state)) return;
    let disposed = false;
    const refresh = async () => {
      try {
        const value = await getJob(migrationJob.id);
        if (!disposed && value) setMigrationJob(value);
      } catch (reason) {
        if (!disposed) setError(normalizeAppError(reason).userMessage);
      }
    };
    const timer = window.setInterval(() => void refresh(), 350);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [migrationJob?.id, migrationJob?.state]);

  useEffect(() => {
    if (!migrationJob || !terminalStates.has(migrationJob.state)) return;
    if (reportPanelRef.current) reportPanelRef.current.scrollTop = 0;
  }, [migrationJob?.state]);

  const active = Boolean(migrationJob && !terminalStates.has(migrationJob.state));
  const canExport = Boolean(
    preview?.ready && destination && acceptedWarning && !active && !launchBusy,
  );
  const phase = migrationJob?.migrationProgress?.phase;
  const visibleDiagnostics = migrationJob?.migrationResult?.diagnostics ?? preview?.diagnostics ?? [];
  const diagnosticTotal = migrationJob?.migrationResult?.report.counts.diagnostics
    ?? preview?.counts.diagnostics
    ?? visibleDiagnostics.length;
  const selected = useMemo(
    () => candidates.find((candidate) => candidate.resref === selectedArea),
    [candidates, selectedArea],
  );

  const chooseDestination = async () => {
    if (!preview) return;
    try {
      const parent = await selectDirectory();
      if (!parent) return;
      setDestination(composeBundleDestinationPath(parent, preview.suggestedDirectoryName));
    } catch (reason) {
      setError(normalizeAppError(reason).userMessage);
    }
  };

  const launch = async () => {
    if (!preview || !canExport) return;
    setLaunchBusy(true);
    setError(undefined);
    try {
      setMigrationJob(await startAreaMigrationExport({
        analysisJobId: analysisJobId!,
        areaResref: preview.areaResref,
        destination,
      }));
    } catch (reason) {
      setError(normalizeAppError(reason).userMessage);
    } finally {
      setLaunchBusy(false);
    }
  };

  const cancel = async () => {
    if (!migrationJob) return;
    try {
      setMigrationJob(await cancelJob(migrationJob.id));
    } catch (reason) {
      setError(normalizeAppError(reason).userMessage);
    }
  };

  if (!analysisJobId) {
    return (
      <ExportWorkshopLockedState
        className="migration-workspace migration-workspace-locked"
        ariaLabel="Migration de zone"
        panelClassName="migration-locked-state"
        icon={<MapPinned size={30} />}
        kicker="MIGRATION DE ZONE"
        title="Analyse requise"
        description="Choisissez un module puis terminez son analyse avant de préparer un bundle de migration."
        note="Les sources NWN resteront en lecture seule pendant l’audit et l’export."
      />
    );
  }

  return (
    <section className="migration-workspace" aria-label="Migration de zone">
      <aside className="migration-area-panel">
        <header>
          <span className="rpg-kicker"><MapPinned size={13} /> SOURCE ANALYSÉE</span>
          <h2>Choisir une zone</h2>
          <p>Le catalogue actif reste en lecture seule.</p>
        </header>
        {loadingAreas ? <p className="migration-loading"><LoaderCircle size={14} /> Chargement des zones...</p> : null}
        {!loadingAreas && candidates.length === 0 ? <p>Aucune zone analysée dans ce module.</p> : null}
        <div className="migration-area-list">
          {candidates.map((candidate) => (
            <button
              type="button"
              key={candidate.resref}
              className={candidate.resref === selectedArea ? "selected" : ""}
              onClick={() => setSelectedArea(candidate.resref)}
            >
              <strong>{candidate.name}</strong>
              <code>{candidate.resref}</code>
              <small>{candidate.tileCount} tuiles · {candidate.instanceCount} instances</small>
            </button>
          ))}
        </div>
      </aside>

      <main className="migration-plan-panel">
        <ExportWorkshopPageHeader
          icon={<PackageOpen size={13} />}
          kicker="BUNDLE DE MIGRATION V1"
          title="Migration de zone"
          description="Auditer, prévisualiser puis exporter une zone vers un bundle neutre et versionné."
        />

        {previewBusy ? <div className="migration-audit-loading"><LoaderCircle size={18} /> Audit de {selected?.name ?? selectedArea}...</div> : null}
        {preview ? (
          <div className="migration-plan-scroll">
            <section className="migration-readiness" aria-label="État de préparation">
              <div>
                {!preview.ready ? <XCircle size={22} /> : preview.complete ? <CheckCircle2 size={22} /> : <AlertTriangle size={22} />}
                <span><strong>{!preview.ready ? "Export bloqué" : preview.complete ? "Bundle complet prévu" : "Export possible avec réserves"}</strong><small>{preview.areaName} · {preview.areaResref}</small></span>
              </div>
              <div className="migration-counts">
                <MigrationMetric label="Tuiles" value={preview.counts.tiles} />
                <MigrationMetric label="Instances" value={preview.counts.instances} />
                <MigrationMetric label="Modèles" value={preview.counts.uniqueModels} />
                <MigrationMetric label="Textures" value={preview.counts.textures} />
                <MigrationMetric label="Navigation préservée" value={preview.counts.preservedNavigation} />
                <MigrationMetric label="Manquants" value={preview.counts.missingItems} warning={preview.counts.missingItems > 0} />
                <MigrationMetric label="Replis" value={preview.counts.fallbacks} warning={preview.counts.fallbacks > 0} />
                <MigrationMetric label="Avertissements" value={preview.counts.warnings} warning={preview.counts.warnings > 0} />
                <MigrationMetric label="Erreurs" value={preview.counts.errors} warning={preview.counts.errors > 0} />
              </div>
            </section>

            <section className="migration-content-preview" aria-label="Aperçu du bundle">
              <div><span className="migration-step">1</span><div><h3>Contenu prévu</h3><p>Inventaire léger, sans transfert de buffers binaires vers l’interface.</p></div></div>
              <ul>
                <li><code>manifest.json</code>, <code>area.json</code>, <code>identity-map.json</code>, <code>diagnostics.jsonl</code> et <code>migration-report.json</code></li>
                <li>{preview.counts.uniqueModels} modèle(s) GLB unique(s) et {preview.counts.textures} texture(s) PNG résolue(s)</li>
                <li>{preview.counts.preservedNavigation} WOK/PWK/DWK préservé(s) octet pour octet, sans conversion de navigation</li>
                <li>Coordonnées canoniques : NWN <code>[x,y,z]</code> → Y-up main droite <code>[x,z,-y]</code></li>
              </ul>
            </section>

            <section className="migration-destination">
              <div><span className="migration-step">2</span><div><h3>Destination locale</h3><p>Choisissez un dossier parent. Le bundle est créé dans un nouveau sous-dossier atomique.</p></div></div>
              <button type="button" onClick={() => void chooseDestination()}><FolderOpen size={15} /> Choisir la destination</button>
              <code>{destination || "Aucune destination sélectionnée"}</code>
            </section>

            <section className="migration-legal-warning">
              <ShieldAlert size={23} />
              <div>
                <h3>Ressources propriétaires · usage local uniquement</h3>
                <p>Les ressources MOD, HAK, TLK et installation copiées ou converties ne sont pas redistribuables sans droits séparés. Le bundle ne confère aucune licence.</p>
                <label><input type="checkbox" checked={acceptedWarning} onChange={(event) => setAcceptedWarning(event.target.checked)} /> J’ai compris : ce bundle reste local et ne doit pas être redistribué.</label>
              </div>
            </section>

            <section className="migration-launch">
              <div><span className="migration-step">3</span><div><h3>Lancer l’export</h3><p>Aucun tampon binaire n’est envoyé à React ; les artefacts restent sur disque.</p></div></div>
              <div className="migration-actions">
                <button type="button" className="primary-button" disabled={!canExport} onClick={() => void launch()}>
                  {launchBusy ? <LoaderCircle size={15} /> : <PackageOpen size={15} />} Exporter le bundle
                </button>
                {active ? <button type="button" disabled={migrationJob?.state === "cancelling"} onClick={() => void cancel()}>Annuler</button> : null}
              </div>
              {active && migrationJob ? <MigrationJobProgress job={migrationJob} phase={phase} /> : null}
            </section>
          </div>
        ) : null}
        {error ? <p className="migration-error" role="alert"><AlertTriangle size={15} /> {error}</p> : null}
      </main>

      <aside ref={reportPanelRef} className="migration-report-panel">
        <header><span className="rpg-kicker">RAPPORT</span><h2>Résultat et diagnostics</h2></header>
        {migrationJob?.state === "completed" && migrationJob.migrationResult ? (
          <section className="migration-result-success">
            <CheckCircle2 size={24} />
            <h3>{migrationJob.migrationResult.report.complete ? "Bundle complet" : "Bundle produit avec réserves"}</h3>
            <code>{migrationJob.migrationResult.bundlePath}</code>
            <small>{migrationJob.migrationResult.report.payloadFileCount} fichiers de contenu · {formatBytes(migrationJob.migrationResult.report.payloadSizeBytes)} avant rapport/manifeste</small>
            <small>Navigation : préservée, non convertie</small>
          </section>
        ) : migrationJob?.state === "failed" ? (
          <section className="migration-result-failure"><XCircle size={24} /><h3>Export interrompu</h3><p>{migrationJob.error?.userMessage ?? "Le moteur n’a pas pu produire le bundle."}</p></section>
        ) : migrationJob?.state === "cancelled" ? (
          <section className="migration-result-cancelled"><XCircle size={24} /><h3>Export annulé</h3><p>Aucun bundle partiel n’a été publié.</p></section>
        ) : (
          <p className="migration-report-empty">Le rapport final, le chemin du bundle et les diagnostics apparaitront ici.</p>
        )}
        <div className="migration-diagnostics" aria-label="Diagnostics de migration">
          {visibleDiagnostics.slice(0, 100).map((diagnostic) => (
            <article key={`${diagnostic.sequence}:${diagnostic.code}`} className={diagnostic.severity}>
              <strong>{diagnostic.code}</strong><span>{diagnostic.message}</span><small>{diagnostic.status} · {diagnostic.resource ?? diagnostic.phase}</small>
            </article>
          ))}
          {diagnosticTotal > 100 ? <p>{diagnosticTotal - 100} diagnostics supplémentaires dans diagnostics.jsonl.</p> : null}
        </div>
      </aside>
    </section>
  );
}
function MigrationMetric({ label, value, warning = false }: { label: string; value: number; warning?: boolean }) {
  return <div className={warning ? "warning" : ""}><span>{label}</span><strong>{value.toLocaleString("fr-FR")}</strong></div>;
}

function MigrationJobProgress({ job, phase }: { job: JobSnapshot; phase?: MigrationPhase }) {
  const percent = Math.min(job.migrationProgress?.percent ?? job.progress.percent, terminalStates.has(job.state) ? 100 : 99);
  const labels: Record<MigrationPhase, string> = {
    preparing: "Préparation", audit: "Audit", models: "Conversion des modèles", textures: "Conversion des textures",
    navigation: "Préservation de la navigation", bundle: "Écriture atomique", verifying: "Vérification finale",
  };
  return <div className="migration-progress" role="status" aria-live="polite"><div><strong>{job.state === "cancelling" ? "Annulation en cours" : labels[phase ?? "preparing"]}</strong><span>{percent.toFixed(1)} %</span></div><progress aria-label="Progression de la migration" aria-valuenow={percent} max={100} value={percent} /><small>{job.migrationProgress?.current ?? "Traitement local en arrière-plan"}</small></div>;
}

function formatBytes(value: number) {
  return value >= 1024 * 1024 ? `${(value / (1024 * 1024)).toFixed(1)} Mio` : `${Math.ceil(value / 1024)} Kio`;
}
