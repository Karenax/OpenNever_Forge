import {
  AlertTriangle,
  Download,
  FileJson2,
  FileText,
  LoaderCircle,
  MessageSquareText,
  Search,
  Workflow,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  exportDialogueBundle,
  listDialogueExportCandidates,
  normalizeAppError,
  previewDialogueExport,
  selectDirectory,
  type DialogueExportPreview,
  type DialogueExportResult,
  type DialogueSearchHit,
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
import "./DialogueExportWorkspace.css";

type DialogueExportWorkspaceProps = {
  jobId?: string;
  analysisReady: boolean;
  workspaceId?: string;
};

export function DialogueExportWorkspace({ jobId, analysisReady, workspaceId }: DialogueExportWorkspaceProps) {
  const analysisJobId = analysisReady ? jobId : undefined;
  const [candidates, setCandidates] = useState<DialogueSearchHit[]>([]);
  const [selectedResref, setSelectedResref] = useState("");
  const [query, setQuery] = useState("");
  const [preview, setPreview] = useState<DialogueExportPreview>();
  const [destination, setDestination] = useState("");
  const [acceptedWarning, setAcceptedWarning] = useState(false);
  const [result, setResult] = useState<DialogueExportResult>();
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
    void listDialogueExportCandidates(analysisJobId, workspaceId)
      .then((values) => {
        if (disposed) return;
        setCandidates(values);
        setSelectedResref(values[0]?.resref ?? "");
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
  }, [analysisJobId, workspaceId]);

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
    void previewDialogueExport({ analysisJobId, workspaceId, resref: selectedResref })
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
  }, [analysisJobId, selectedResref, workspaceId]);

  const visibleCandidates = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase();
    if (!normalized) return candidates;
    return candidates.filter((candidate) =>
      candidate.resref.toLocaleLowerCase().includes(normalized)
      || candidate.preview?.toLocaleLowerCase().includes(normalized),
    );
  }, [candidates, query]);

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
      setResult(await exportDialogueBundle({
        analysisJobId,
        workspaceId,
        resref: preview.resref,
        destination,
        expectedSourceResourceSha256: preview.sourceResourceSha256,
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
        className="dialogue-export-workspace dialogue-export-locked"
        ariaLabel="Export de dialogues"
        icon={<MessageSquareText size={34} />}
        kicker="EXPORT DLG · JSON · TRANSCRIPT"
        title="Analyse requise"
        description="Analysez une carte ou un module pour exporter ses dialogues résolus."
        note="Les fichiers NWN sources restent en lecture seule."
      />
    );
  }

  return (
    <section className="dialogue-export-workspace" aria-label="Export de dialogues">
      <aside className="dialogue-export-browser">
        <header>
          <span className="rpg-kicker"><MessageSquareText size={13} /> DIALOGUES INDEXÉS</span>
          <h2>Choisir un dialogue</h2>
          <p>{candidates.length.toLocaleString("fr-FR")} dialogue(s)</p>
        </header>
        <label className="dialogue-export-search">
          <Search size={14} />
          <input
            aria-label="Rechercher un dialogue à exporter"
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="ResRef ou texte…"
          />
        </label>
        {loadingCandidates ? <p className="dialogue-export-loading"><LoaderCircle size={14} /> Index des dialogues…</p> : null}
        <div className="dialogue-export-list">
          {visibleCandidates.map((candidate) => (
            <button
              type="button"
              key={candidate.resref}
              className={candidate.resref === selectedResref ? "selected" : ""}
              onClick={() => setSelectedResref(candidate.resref)}
            >
              <MessageSquareText size={15} />
              <span>
                <strong>{candidate.resref}</strong>
                <small>{candidate.nodeCount} ligne(s) · {candidate.linkCount} lien(s)</small>
                {candidate.preview ? <em>{candidate.preview}</em> : null}
              </span>
              {candidate.diagnosticCount > 0 ? <AlertTriangle size={14} /> : null}
            </button>
          ))}
          {!loadingCandidates && visibleCandidates.length === 0 ? <p>Aucun dialogue ne correspond.</p> : null}
        </div>
      </aside>

      <main className="dialogue-export-main">
        <ExportWorkshopPageHeader
          icon={<Download size={13} />}
          kicker="DIALOGUE EXPORT V1"
          title="Exporter des dialogues"
          description="Préserver le DLG et produire une version JSON et un transcript lisible."
        />

        {previewBusy ? <div className="dialogue-export-audit"><LoaderCircle size={18} /> Analyse de {selectedResref}…</div> : null}
        {error ? <div className="dialogue-export-error" role="alert"><AlertTriangle size={17} /> {error}</div> : null}

        {preview ? (
          <div className="dialogue-export-scroll">
            <section className="dialogue-export-summary">
              <div className={`dialogue-export-revision ${preview.revision}`}>
                <Workflow size={23} />
                <span>
                  <strong>{preview.revision === "workspace" ? "Version modifiée du workspace" : "Version analysée"}</strong>
                  <code>{preview.resref}.dlg</code>
                </span>
              </div>
              <div className="dialogue-export-metrics">
                <ExportMetric label="Lignes" value={preview.nodeCount} />
                <ExportMetric label="Liens" value={preview.linkCount} />
                <ExportMetric label="Racines" value={preview.rootCount} />
                <ExportMetric label="Cycles" value={preview.cycleCount} warning={preview.cycleCount > 0} />
                <ExportMetric label="Scripts" value={preview.scripts.length} />
                <ExportMetric label="Références" value={preview.referenceCount} />
              </div>
            </section>

            <section className="dialogue-export-grid">
              <article>
                <header><h3><FileText size={16} /> Aperçu du transcript</h3><span>{preview.entryCount} PNJ · {preview.replyCount} joueur</span></header>
                <div className="dialogue-export-transcript">
                  {preview.transcriptPreview.length
                    ? preview.transcriptPreview.map((line, index) => <p key={`${index}-${line}`}>{readableTranscriptLine(line)}</p>)
                    : <p>Aucun texte résolu.</p>}
                </div>
              </article>
              <article>
                <header><h3><FileJson2 size={16} /> Contenu produit</h3></header>
                <ul className="dialogue-export-files">
                  <li><code>{preview.resref}.dlg</code><span>Ressource exacte</span></li>
                  <li><code>dialogue.json</code><span>Structure portable</span></li>
                  <li><code>transcript.md</code><span>Lecture humaine</span></li>
                  <li><code>manifest.json</code><span>Tailles et SHA-256</span></li>
                </ul>
                {preview.scripts.length ? <p className="dialogue-export-scripts"><strong>Scripts :</strong> {preview.scripts.join(", ")}</p> : null}
              </article>
            </section>

            <ExportWarningsSection
              className="dialogue-export-warnings"
              title="Structure conservée avec diagnostics"
              warnings={preview.warnings}
            />

            <ExportDestinationSection
              className="dialogue-export-destination"
              heading="Destination"
              description="Un nouveau dossier atomique contiendra les quatre fichiers de l’export."
              destination={destination}
              onChoose={() => void chooseDestination()}
            />

            <ExportConsentLabel
              className="dialogue-export-rights"
              checked={acceptedWarning}
              onToggle={setAcceptedWarning}
            >
              <strong>Cet export reste local.</strong> Le dialogue, ses textes et ses scripts ne deviennent pas redistribuables par leur conversion.
            </ExportConsentLabel>

            <ExportLaunchButton
              className="dialogue-export-launch"
              busyClassName=" busy"
              disabled={!preview.ready || !destination || !acceptedWarning || exportBusy}
              busy={exportBusy}
              idleIcon={<Download size={16} />}
              idleLabel="Exporter le dialogue"
              busyLabel="Sérialisation et écriture…"
              onLaunch={() => void launchExport()}
            />

            {result ? (
              <ExportResultSection
                className="dialogue-export-result"
                ariaLabel="Résultat de l’export de dialogue"
                title="Dialogue exporté"
                summary={`${result.nodeCount} ligne(s) · ${result.linkCount} lien(s) · ${result.fileCount} fichier(s) · ${formatExportBytes(result.totalSizeBytes)}`}
                destination={result.destination}
                footnote={`SHA-256 DLG · ${result.sourceResourceSha256}`}
              />
            ) : null}
          </div>
        ) : null}
      </main>
    </section>
  );
}

function readableTranscriptLine(line: string): string {
  return line.replace(/\*\*/g, "").replace(/`/g, "");
}
