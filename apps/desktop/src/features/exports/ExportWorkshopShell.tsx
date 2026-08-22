import type { ReactNode } from "react";
import { AlertTriangle, CheckCircle2, FolderOpen, LoaderCircle, ShieldAlert } from "lucide-react";
import "./ExportWorkshopShell.css";

type ExportWorkshopLockedStateProps = {
  className: string;
  ariaLabel: string;
  panelClassName?: string;
  icon: ReactNode;
  kicker: string;
  title: string;
  description: string;
  note: string;
};

export function ExportWorkshopLockedState({
  className,
  ariaLabel,
  panelClassName,
  icon,
  kicker,
  title,
  description,
  note,
}: ExportWorkshopLockedStateProps) {
  return (
    <section className={className} aria-label={ariaLabel}>
      <div className={panelClassName}>
        {icon}
        <span className="rpg-kicker">{kicker}</span>
        <h1>{title}</h1>
        <p>{description}</p>
        <small>{note}</small>
      </div>
    </section>
  );
}

type ExportWorkshopPageHeaderProps = {
  icon: ReactNode;
  kicker: string;
  title: string;
  description: string;
};

export function ExportWorkshopPageHeader({ icon, kicker, title, description }: ExportWorkshopPageHeaderProps) {
  return (
    <header className="workspace-page-header">
      <div>
        <span className="rpg-kicker">{icon} {kicker}</span>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
    </header>
  );
}

export function composeBundleDestinationPath(parent: string, suggestedDirectoryName: string): string {
  const separator = parent.includes("\\") ? "\\" : "/";
  return `${parent.replace(/[\\/]$/, "")}${separator}${suggestedDirectoryName}`;
}

export function formatExportBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Kio`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mio`;
}

export function ExportMetric({ label, value, warning = false }: { label: string; value: number; warning?: boolean }) {
  return <div className={warning ? "warning" : ""}><strong>{value.toLocaleString("fr-FR")}</strong><span>{label}</span></div>;
}

type ExportWarningsSectionProps = {
  className: string;
  title: ReactNode;
  warnings: readonly string[];
};

export function ExportWarningsSection({ className, title, warnings }: ExportWarningsSectionProps) {
  if (!warnings.length) return null;
  return (
    <section className={className}>
      <h3><AlertTriangle size={16} /> {title}</h3>
      {warnings.map((warning) => <p key={warning}>{warning}</p>)}
    </section>
  );
}

type ExportDestinationSectionProps = {
  className: string;
  heading: string;
  description: ReactNode;
  destination: string;
  onChoose: () => void;
};

export function ExportDestinationSection({ className, heading, description, destination, onChoose }: ExportDestinationSectionProps) {
  return (
    <section className={className}>
      <div>
        <h3>{heading}</h3>
        <p>{description}</p>
      </div>
      <button type="button" onClick={onChoose}><FolderOpen size={15} /> Choisir la destination</button>
      <code>{destination || "Aucune destination sélectionnée"}</code>
    </section>
  );
}

type ExportConsentLabelProps = {
  className: string;
  checked: boolean;
  onToggle: (checked: boolean) => void;
  children: ReactNode;
};

export function ExportConsentLabel({ className, checked, onToggle, children }: ExportConsentLabelProps) {
  return (
    <label className={className}>
      <input type="checkbox" checked={checked} onChange={(event) => onToggle(event.currentTarget.checked)} />
      <ShieldAlert size={19} />
      <span>{children}</span>
    </label>
  );
}

type ExportLaunchButtonProps = {
  className: string;
  busyClassName?: string;
  disabled: boolean;
  busy: boolean;
  idleIcon: ReactNode;
  idleLabel: string;
  busyLabel: string;
  onLaunch: () => void;
};

export function ExportLaunchButton({
  className,
  busyClassName = "",
  disabled,
  busy,
  idleIcon,
  idleLabel,
  busyLabel,
  onLaunch,
}: ExportLaunchButtonProps) {
  return (
    <button
      type="button"
      className={busy ? `${className}${busyClassName}` : className}
      disabled={disabled}
      onClick={onLaunch}
    >
      {busy ? <LoaderCircle size={16} /> : idleIcon}
      {busy ? busyLabel : idleLabel}
    </button>
  );
}

type ExportResultSectionProps = {
  className: string;
  ariaLabel: string;
  title: string;
  summary: ReactNode;
  destination: string;
  footnote: string;
};

export function ExportResultSection({ className, ariaLabel, title, summary, destination, footnote }: ExportResultSectionProps) {
  return (
    <section className={className} aria-label={ariaLabel}>
      <CheckCircle2 size={24} />
      <div>
        <h3>{title}</h3>
        <p>{summary}</p>
        <code>{destination}</code>
        <small>{footnote}</small>
      </div>
    </section>
  );
}
