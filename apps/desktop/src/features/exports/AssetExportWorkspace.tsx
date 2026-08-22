import {
  AlertTriangle,
  Box,
  Clapperboard,
  Download,
  FileBox,
  LoaderCircle,
  Search,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  exportAssetBundle,
  listAssetExportCandidates,
  normalizeAppError,
  previewAssetExport,
  selectDirectory,
  type AssetExportCandidate,
  type AssetExportPreview,
  type AssetExportResult,
} from "../../lib/tauri";
import {
  ExportConsentLabel,
  ExportDestinationSection,
  ExportLaunchButton,
  ExportMetric,
  ExportResultSection,
  ExportWarningsSection,
  ExportWorkshopLockedState,
  ExportWorkshopPageHeader,
  composeBundleDestinationPath,
  formatExportBytes,
} from "./ExportWorkshopShell";
import "./AssetExportWorkspace.css";

type AssetExportWorkspaceProps = {
  jobId?: string;
  analysisReady: boolean;
};

type AnimationFilter = "all" | "animated" | "static";

export function AssetExportWorkspace({ jobId, analysisReady }: AssetExportWorkspaceProps) {
  const analysisJobId = analysisReady ? jobId : undefined;
  const [candidates, setCandidates] = useState<AssetExportCandidate[]>([]);
  const [selectedResref, setSelectedResref] = useState("");
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<AnimationFilter>("all");
  const [preview, setPreview] = useState<AssetExportPreview>();
  const [destination, setDestination] = useState("");
  const [acceptedWarning, setAcceptedWarning] = useState(false);
  const [result, setResult] = useState<AssetExportResult>();
  const [loadingCandidates, setLoadingCandidates] = useState(false);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [exportBusy, setExportBusy] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => {
    if (!analysisJobId) {
      setCandidates([]);
      setSelectedResref("");
      setPreview(undefined);
      return;
    }
    let disposed = false;
    setLoadingCandidates(true);
    setError(undefined);
    void listAssetExportCandidates(analysisJobId)
      .then((values) => {
        if (disposed) return;
        setCandidates(values);
        setSelectedResref(values.find((candidate) => candidate.exportable)?.resref ?? values[0]?.resref ?? "");
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(normalizeAppError(reason).userMessage);
      })
      .finally(() => {
        if (!disposed) setLoadingCandidates(false);
      });
    return () => {
      disposed = true;
    };
  }, [analysisJobId]);

  useEffect(() => {
    setDestination("");
    setAcceptedWarning(false);
    setResult(undefined);
    if (!analysisJobId || !selectedResref) {
      setPreview(undefined);
      return;
    }
    let disposed = false;
    setPreview(undefined);
    setPreviewBusy(true);
    setError(undefined);
    void previewAssetExport(analysisJobId, selectedResref)
      .then((value) => {
        if (!disposed) setPreview(value);
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
  }, [analysisJobId, selectedResref]);

  const visibleCandidates = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    return candidates.filter((candidate) => {
      const matchesFilter = filter === "all"
        || (filter === "animated" && candidate.declaredAnimationCount > 0)
        || (filter === "static" && candidate.declaredAnimationCount === 0);
      const matchesQuery = !normalized
        || candidate.resref.toLocaleLowerCase().includes(normalized)
        || candidate.declaredAnimations.some((animation) => animation.toLocaleLowerCase().includes(normalized));
      return matchesFilter && matchesQuery;
    });
  }, [candidates, filter, query]);

  const chooseDestination = async () => {
    if (!preview) return;
    try {
      const parent = await selectDirectory();
      if (!parent) return;
      setDestination(composeBundleDestinationPath(parent, preview.suggestedDirectoryName));
      setResult(undefined);
    } catch (reason) {
      setError(normalizeAppError(reason).userMessage);
    }
  };

  const launchExport = async () => {
    if (!analysisJobId || !preview?.ready || !destination || !acceptedWarning) return;
    setExportBusy(true);
    setResult(undefined);
    setError(undefined);
    try {
      setResult(await exportAssetBundle({
        analysisJobId,
        resref: preview.resref,
        destination,
        localOnlyAcknowledged: acceptedWarning,
      }));
    } catch (reason) {
      setError(normalizeAppError(reason).userMessage);
    } finally {
      setExportBusy(false);
    }
  };

  if (!analysisJobId) {
    return (
      <ExportWorkshopLockedState
        className="asset-export-workspace asset-export-locked"
        ariaLabel="Export d’assets"
        icon={<FileBox size={32} />}
        kicker="EXPORT GLB · STATIQUE OU ANIMÉ"
        title="Analyse requise"
        description="Analysez une carte ou un module pour exporter ses modèles résolus."
        note="Les fichiers NWN sources restent en lecture seule."
      />
    );
  }

  return (
    <section className="asset-export-workspace" aria-label="Export d’assets">
      <aside className="asset-export-browser">
        <header>
          <span className="rpg-kicker"><FileBox size={13} /> MODÈLES RÉSOLUS</span>
          <h2>Choisir un asset</h2>
          <p>{candidates.length.toLocaleString("fr-FR")} modèle(s) indexé(s)</p>
        </header>
        <label className="asset-export-search">
          <Search size={14} />
          <input
            aria-label="Rechercher un asset à exporter"
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="ResRef ou animation…"
          />
        </label>
        <nav aria-label="Filtrer les assets par animation">
          <button type="button" className={filter === "all" ? "active" : ""} onClick={() => setFilter("all")}>Tous</button>
          <button type="button" className={filter === "animated" ? "active" : ""} onClick={() => setFilter("animated")}>Animés</button>
          <button type="button" className={filter === "static" ? "active" : ""} onClick={() => setFilter("static")}>Statiques</button>
        </nav>
        {loadingCandidates ? <p className="asset-export-loading"><LoaderCircle size={14} /> Index des modèles…</p> : null}
        <div className="asset-export-list">
          {visibleCandidates.map((candidate) => (
            <button
              type="button"
              key={candidate.resref}
              className={candidate.resref === selectedResref ? "selected" : ""}
              onClick={() => setSelectedResref(candidate.resref)}
            >
              <span>{candidate.declaredAnimationCount > 0 ? <Clapperboard size={14} /> : <Box size={14} />}</span>
              <strong>{candidate.resref}</strong>
              <small>{candidate.meshCount} mesh · {candidate.declaredAnimationCount} animation(s)</small>
              {!candidate.exportable ? <em>À vérifier</em> : null}
            </button>
          ))}
          {!loadingCandidates && visibleCandidates.length === 0 ? <p>Aucun modèle ne correspond.</p> : null}
        </div>
      </aside>

      <main className="asset-export-main">
        <ExportWorkshopPageHeader
          icon={<Download size={13} />}
          kicker="EXPORT ASSET V1"
          title="Exporter des assets"
          description="Convertir un modèle Aurora en GLB avec ses textures et ses clips d’animation."
        />

        {previewBusy ? <div className="asset-export-audit"><LoaderCircle size={18} /> Analyse de {selectedResref}…</div> : null}
        {error ? <div className="asset-export-error" role="alert"><AlertTriangle size={17} /> {error}</div> : null}

        {preview ? (
          <div className="asset-export-scroll">
            <section className="asset-export-summary">
              <div className={`asset-export-mode ${preview.mode}`}>
                {preview.mode === "animated" ? <Clapperboard size={24} /> : <Box size={24} />}
                <span>
                  <strong>{preview.mode === "animated" ? "Asset animé" : "Asset statique"}</strong>
                  <code>{preview.resref}.mdl → {preview.resref}.glb</code>
                </span>
              </div>
              <div className="asset-export-metrics">
                <ExportMetric label="Nœuds" value={preview.nodeCount} />
                <ExportMetric label="Mesh" value={preview.meshCount} />
                <ExportMetric label="Primitives" value={preview.primitiveCount} />
                <ExportMetric label="Skins" value={preview.skinCount} />
                <ExportMetric label="Animations" value={preview.animationCount} />
                <ExportMetric label="Textures" value={preview.textures.length} />
              </div>
            </section>

            <section className="asset-export-detail-grid">
              <article>
                <header><h3>Animations</h3><span>{preview.animationCount} clip(s) GLB</span></header>
                {preview.animations.length ? (
                  <div className="asset-export-clips">
                    {preview.animations.map((animation) => (
                      <div key={animation.name} className={animation.exported ? "exported" : "fallback"}>
                        <strong>{animation.name}</strong>
                        <small>{animation.lengthSeconds.toFixed(2)} s · {animation.trackCount} piste(s) · {animation.eventCount} événement(s)</small>
                        <span>{animation.exported ? "Exportée" : "Sans piste transformable"}</span>
                      </div>
                    ))}
                  </div>
                ) : <p>Aucune animation déclarée : le GLB sera statique.</p>}
              </article>
              <article>
                <header><h3>Textures</h3><span>{preview.textures.length} référence(s)</span></header>
                {preview.textures.length ? (
                  <div className="asset-export-textures">
                    {preview.textures.map((texture) => (
                      <div key={texture.resref}>
                        <code>{texture.resref}</code>
                        <span className={texture.status}>{texture.status === "planned" ? "PNG prévu" : "Repli matériau"}</span>
                      </div>
                    ))}
                  </div>
                ) : <p>Ce modèle ne référence aucune texture.</p>}
              </article>
            </section>

            <ExportWarningsSection
              className="asset-export-warnings"
              title="Points à contrôler"
              warnings={preview.warnings}
            />

            <ExportDestinationSection
              className="asset-export-destination"
              heading="Destination"
              description={<>Un nouveau dossier atomique contiendra le GLB, <code>manifest.json</code> et les textures PNG.</>}
              destination={destination}
              onChoose={() => void chooseDestination()}
            />

            <ExportConsentLabel
              className="asset-export-rights"
              checked={acceptedWarning}
              onToggle={setAcceptedWarning}
            >
              <strong>Cet export reste local.</strong> Les modèles et textures NWN ne deviennent pas redistribuables par leur conversion.
            </ExportConsentLabel>

            <ExportLaunchButton
              className="asset-export-launch"
              disabled={!preview.ready || !destination || !acceptedWarning || exportBusy}
              busy={exportBusy}
              idleIcon={<Download size={16} />}
              idleLabel="Exporter l’asset"
              busyLabel="Conversion et écriture…"
              onLaunch={() => void launchExport()}
            />

            {result ? (
              <ExportResultSection
                className="asset-export-result"
                ariaLabel="Résultat de l’export d’asset"
                title="Asset exporté"
                summary={`${result.mode === "animated" ? `${result.animationCount} animation(s)` : "GLB statique"} · ${result.textureCount} texture(s) · ${formatExportBytes(result.glbSizeBytes)}`}
                destination={result.destination}
                footnote={`SHA-256 GLB · ${result.glbSha256}`}
              />
            ) : null}
          </div>
        ) : null}
      </main>
    </section>
  );
}
