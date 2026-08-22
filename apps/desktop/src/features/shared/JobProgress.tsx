import { LoaderCircle } from "lucide-react";
import { useEffect, useState } from "react";
import type { JobSnapshot } from "../../lib/tauri";

const terminalStates = new Set(["completed", "failed", "cancelled"]);

function formatElapsedTime(seconds: number) {
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return minutes > 0
    ? `${minutes} min ${remainingSeconds.toString().padStart(2, "0")} s`
    : `${remainingSeconds} s`;
}

export function JobProgress({ job }: { job: JobSnapshot }) {
  const active = !terminalStates.has(job.state);
  const finalizing = active && job.progress.phase === "persisting";
  const visiblePercent = Math.min(job.progress.percent, active ? 99 : 100);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);

  useEffect(() => {
    setElapsedSeconds(0);
    if (!active) return;
    const startedAt = Date.now();
    const timer = window.setInterval(() => {
      setElapsedSeconds(Math.floor((Date.now() - startedAt) / 1_000));
    }, 1_000);
    return () => window.clearInterval(timer);
  }, [job.id, active]);

  const phaseLabels: Record<JobSnapshot["progress"]["phase"], string> = {
    hashing: "Empreinte du module",
    inventory: "Inventaire du conteneur",
    dependencies: "Résolution des dépendances",
    resource_catalog: "Catalogue des ressources",
    structured_resources: "Structures Aurora",
    scripts: "Index des scripts",
    dialogues: "Index des dialogues",
    world: "Modèle du monde",
    persisting: "Persistance de l’index",
  };
  const phase =
    job.state === "queued"
      ? "Préparation de l’analyse"
      : job.state === "cancelling"
        ? "Annulation en cours"
        : job.state === "completed"
          ? "Analyse terminée"
          : job.state === "failed"
            ? "Analyse interrompue"
            : job.state === "cancelled"
              ? "Analyse annulée"
              : finalizing
                ? "Finalisation de l’index"
                : phaseLabels[job.progress.phase];
  const detail = finalizing
    ? "Enregistrement du catalogue et préparation des vues — le travail continue."
    : job.state === "running"
      ? `Lecture et indexation des ressources · ${visiblePercent.toFixed(1)} % parcourus`
      : job.state === "queued"
        ? "La tâche va démarrer dans un instant."
        : job.state === "cancelling"
          ? "La demande d’arrêt a été transmise au moteur."
          : job.state === "completed"
            ? "Le module et ses ressources sont prêts."
            : job.error?.userMessage ?? "La tâche ne travaille plus.";

  return (
    <div
      className={`job-progress ${active ? "is-active" : `is-${job.state}`}`}
      aria-live="polite"
      aria-busy={active}
      role="status"
    >
      <div className="job-progress-heading">
        <span>Analyse du module</span>
        <strong>
          {active ? (
            <><LoaderCircle className="job-progress-spinner" size={13} /> ACTIF · {formatElapsedTime(elapsedSeconds)}</>
          ) : (
            `${visiblePercent.toFixed(1)} %`
          )}
        </strong>
      </div>
      <div
        className={`progress-track ${finalizing ? "is-indeterminate" : ""}`}
        role="progressbar"
        aria-label={phase}
        aria-valuemin={0}
        aria-valuemax={100}
        {...(!finalizing ? { "aria-valuenow": visiblePercent } : {})}
      >
        <div
          className={finalizing ? "progress-sweep" : `progress-fill ${active ? "is-animated" : "is-static"}`}
          style={finalizing ? undefined : { width: `${visiblePercent}%` }}
        />
      </div>
      <div className="job-progress-activity">
        <span className="activity-pulse" aria-hidden="true" />
        <span>
          <strong>{phase}</strong>
          <small>{detail}</small>
        </span>
      </div>
    </div>
  );
}
