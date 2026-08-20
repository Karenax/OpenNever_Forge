import { CheckCircle2, Copy, Map, TriangleAlert } from "lucide-react";
import {
  useCallback,
  type ChangeEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { createPortal } from "react-dom";
import { clamp, type DiagnosticFilter, type MapView } from "./UxEnhancements.model";

export function WorkspaceSplitters({
  host,
  explorerWidth,
  inspectorWidth,
  onExplorerWidth,
  onInspectorWidth,
}: {
  host: HTMLElement;
  explorerWidth: number;
  inspectorWidth: number;
  onExplorerWidth: (value: number) => void;
  onInspectorWidth: (value: number) => void;
}) {
  const startDrag = useCallback(
    (side: "explorer" | "inspector", event: ReactPointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      const bounds = host.getBoundingClientRect();
      const move = (moveEvent: PointerEvent) => {
        if (side === "explorer") {
          onExplorerWidth(clamp(moveEvent.clientX - bounds.left, 210, 420));
        } else {
          onInspectorWidth(clamp(bounds.right - moveEvent.clientX, 240, 460));
        }
      };
      const stop = () => {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", stop);
        window.removeEventListener("pointercancel", stop);
      };
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", stop, { once: true });
      window.addEventListener("pointercancel", stop, { once: true });
    },
    [host, onExplorerWidth, onInspectorWidth],
  );

  return createPortal(
    <>
      <div
        className="ux-panel-splitter ux-panel-splitter-explorer"
        role="separator"
        aria-label="Redimensionner l’explorateur"
        aria-orientation="vertical"
        aria-valuemin={210}
        aria-valuemax={420}
        aria-valuenow={Math.round(explorerWidth)}
        onPointerDown={(event: ReactPointerEvent<HTMLDivElement>) => startDrag("explorer", event)}
      />
      <div
        className="ux-panel-splitter ux-panel-splitter-inspector"
        role="separator"
        aria-label="Redimensionner l’inspecteur"
        aria-orientation="vertical"
        aria-valuemin={240}
        aria-valuemax={460}
        aria-valuenow={Math.round(inspectorWidth)}
        onPointerDown={(event: ReactPointerEvent<HTMLDivElement>) => startDrag("inspector", event)}
      />
    </>,
    host,
  );
}

export function DiagnosticsControls({
  host,
  counts,
  filter,
  onFilter,
}: {
  host: HTMLElement;
  counts: Record<DiagnosticFilter, number>;
  filter: DiagnosticFilter;
  onFilter: (filter: DiagnosticFilter) => void;
}) {
  const copyAll = async () => {
    const lines = [...document.querySelectorAll<HTMLElement>(".diagnostic-row")]
      .filter((row) => !row.classList.contains("ux-permanent-information"))
      .map((row) => row.innerText.replace(/\s+/g, " ").trim());
    if (lines.length) await navigator.clipboard?.writeText(lines.join("\n"));
  };

  return createPortal(
    <div className="ux-diagnostic-controls" aria-label="Filtres de diagnostics">
      <button
        type="button"
        className={filter === "error" ? "active error" : "error"}
        onClick={() => onFilter(filter === "error" ? "all" : "error")}
        title="Afficher uniquement les erreurs"
      >
        <TriangleAlert size={12} /> {counts.error}
      </button>
      <button
        type="button"
        className={filter === "warning" ? "active warning" : "warning"}
        onClick={() => onFilter(filter === "warning" ? "all" : "warning")}
        title="Afficher uniquement les avertissements"
      >
        <TriangleAlert size={12} /> {counts.warning}
      </button>
      <button
        type="button"
        className={filter === "info" ? "active info" : "info"}
        onClick={() => onFilter(filter === "info" ? "all" : "info")}
        title="Afficher uniquement les informations"
      >
        <CheckCircle2 size={12} /> {counts.info}
      </button>
      <button type="button" onClick={() => void copyAll()} title="Copier les diagnostics">
        <Copy size={12} />
      </button>
    </div>,
    host,
  );
}

export function MapStageControls({
  host,
  view,
  connectionExpert,
  densityExpert,
  onView,
  onConnectionExpert,
  onDensityExpert,
}: {
  host: HTMLElement;
  view: MapView;
  connectionExpert: boolean;
  densityExpert: boolean;
  onView: (view: MapView) => void;
  onConnectionExpert: (value: boolean) => void;
  onDensityExpert: (value: boolean) => void;
}) {
  const steps: Array<{ id: MapView; label: string }> = [
    { id: "describe", label: "Décrire" },
    { id: "generate", label: "Générer" },
    { id: "adjust", label: "Ajuster" },
    { id: "create", label: "Créer" },
  ];
  return createPortal(
    <div className="ux-map-stage-controls">
      <nav aria-label="Étapes du créateur de cartes">
        {steps.map((step, index) => (
          <button
            key={step.id}
            type="button"
            className={view === step.id ? "active" : ""}
            aria-current={view === step.id ? "step" : undefined}
            onClick={() => onView(step.id)}
          >
            <span>{index + 1}</span>
            {step.label}
          </button>
        ))}
        <button
          type="button"
          className={view === "atlas" ? "active atlas" : "atlas"}
          onClick={() => onView("atlas")}
        >
          <Map size={13} /> Atlas
        </button>
      </nav>
      <div className="ux-map-expert-toggles">
        <label>
          <input
            type="checkbox"
            checked={connectionExpert}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              onConnectionExpert(event.currentTarget.checked)
            }
          />
          Connexion avancée
        </label>
        <label>
          <input
            type="checkbox"
            checked={densityExpert}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              onDensityExpert(event.currentTarget.checked)
            }
          />
          Blueprints précis
        </label>
      </div>
    </div>,
    host,
  );
}
