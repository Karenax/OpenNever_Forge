import { useState } from "react";
import {
  applyAuroraWorkspaceSync,
  planAuroraWorkspaceSync,
  selectDirectory,
  type AuroraSyncDirection,
  type AuroraSyncPlan,
  type WorkspaceSnapshot,
} from "../lib/tauri";

export function AuroraSyncPanel({ jobId, workspace, onWorkspaceChange, onError }: {
  jobId: string;
  workspace: WorkspaceSnapshot;
  onWorkspaceChange: (workspace: WorkspaceSnapshot) => void;
  onError: (error: unknown) => void;
}) {
  const [root, setRoot] = useState("");
  const [plan, setPlan] = useState<AuroraSyncPlan>();
  const [choices, setChoices] = useState<Record<string, AuroraSyncDirection | "">>({});
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState("");
  const keyOf = (resource: { resref: string; resourceType: number }) => `${resource.resref}.${resource.resourceType}`;

  async function browse() {
    const selected = await selectDirectory();
    if (selected) {
      setRoot(selected);
      setPlan(undefined);
      setChoices({});
    }
  }

  async function preview() {
    if (!root) return;
    setBusy(true);
    try {
      const next = await planAuroraWorkspaceSync({ jobId, workspaceId: workspace.workspaceId, root });
      setPlan(next);
      setChoices(Object.fromEntries(next.entries.map((entry) => [
        keyOf(entry.resource),
        entry.state === "toolset_only" || entry.state === "toolset_changed"
          ? "pull_from_toolset"
          : entry.state === "workspace_only" || entry.state === "workspace_changed"
            ? "push_to_toolset"
            : "",
      ])));
      setResult(next.conflictCount ? `${next.conflictCount} conflit(s) à arbitrer explicitement.` : "Comparaison prête.");
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }

  async function synchronize() {
    if (!plan) return;
    const actions = plan.entries.flatMap((entry) => {
      const direction = choices[keyOf(entry.resource)];
      return direction ? [{
        resource: entry.resource,
        direction,
        expectedToolsetSha256: entry.toolsetSha256,
        expectedWorkspaceSha256: entry.workspaceSha256,
      }] : [];
    });
    if (!actions.length) {
      setResult("Aucune opération sélectionnée.");
      return;
    }
    setBusy(true);
    try {
      const report = await applyAuroraWorkspaceSync({ jobId, workspaceId: workspace.workspaceId, root, actions });
      setPlan(report.plan);
      onWorkspaceChange(report.workspace);
      setChoices({});
      setResult(`${report.applied.length} ressource(s) synchronisée(s) · ${report.backups.length} sauvegarde(s) Toolset.`);
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }

  const stateLabels: Record<string, string> = {
    identical: "Identique",
    toolset_only: "Toolset uniquement",
    workspace_only: "OpenNever uniquement",
    toolset_changed: "Toolset modifié",
    workspace_changed: "OpenNever modifié",
    conflict: "Conflit",
  };
  const migrations = workspace.migrationHistory ?? [];

  return (
    <section className="aurora-sync-card" aria-label="Synchronisation avec Aurora Toolset">
      <div className="dependency-heading">
        <div><span className="eyebrow">LOT 23 · SYNCHRONISATION CONTRÔLÉE</span><h2>Workspace temporaire du Toolset</h2></div>
        <span>Schéma projet v{workspace.schemaVersion}</span>
      </div>
      <p className="sync-safety">Chaque fichier est comparé par SHA-256. Une écriture vers le Toolset crée d’abord une copie sous <code>.opennever-backups</code>. Les conflits ne sont jamais résolus automatiquement.</p>
      <div className="sync-root"><input value={root} readOnly placeholder="Dossier temporaire extrait par Aurora Toolset" /><button type="button" onClick={() => void browse()}>Parcourir</button><button type="button" disabled={busy || !root} onClick={() => void preview()}>Comparer</button></div>
      {plan && <>
        <div className="sync-metrics"><span>{plan.incomingCount} entrant(s)</span><span>{plan.outgoingCount} sortant(s)</span><span>{plan.conflictCount} conflit(s)</span><span>{plan.identicalCount} identique(s)</span><span>{plan.baselineFound ? "Baseline active" : "Première comparaison"}</span></div>
        <div className="sync-table" role="table" aria-label="Plan de synchronisation Toolset">
          {plan.entries.map((entry) => <div className={`sync-row ${entry.state}`} role="row" key={keyOf(entry.resource)}>
            <code>{entry.relativePath}</code><span>{stateLabels[entry.state]}</span>
            <select aria-label={`Action pour ${entry.relativePath}`} value={choices[keyOf(entry.resource)] ?? ""} disabled={entry.state === "identical"} onChange={(event) => setChoices({ ...choices, [keyOf(entry.resource)]: event.currentTarget.value as AuroraSyncDirection | "" })}>
              <option value="">Ignorer</option><option value="pull_from_toolset">Importer du Toolset</option><option value="push_to_toolset">Envoyer vers Toolset</option>
            </select>
          </div>)}
        </div>
        <div className="profile-actions"><button type="button" disabled={busy || !Object.values(choices).some(Boolean)} onClick={() => void synchronize()}>Synchroniser la sélection</button></div>
      </>}
      <small className="sync-compiler-warning">Après import d’un NSS : recompiler en NCS avant sauvegarde du module Toolset. Après envoi : sauvegarder explicitement le module dans Aurora, sinon son dossier temporaire sera recréé à la prochaine ouverture.</small>
      {migrations.length > 0 && <details className="migration-history"><summary>{migrations.length} migration(s) de projet appliquée(s)</summary>{migrations.map((migration) => <div key={`${migration.fromVersion}-${migration.toVersion}`}><strong>v{migration.fromVersion} → v{migration.toVersion}</strong><code>{migration.backupPath}</code>{migration.steps.map((step) => <small key={step}>{step}</small>)}</div>)}</details>}
      {result && <p className="profile-result">{result}</p>}
    </section>
  );
}
