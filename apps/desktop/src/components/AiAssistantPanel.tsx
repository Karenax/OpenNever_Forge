import { useState } from "react";
import { CheckCircle2, KeyRound, ShieldCheck, Sparkles, TriangleAlert } from "lucide-react";
import {
  applyAiChangeSet,
  previewAiChangeSet,
  requestAiChangeSet,
  type AiChangeSet,
  type AiProviderProposal,
  type ResourceKey,
  type WorkspaceSnapshot,
} from "../lib/tauri";

export function AiAssistantPanel({ jobId, workspace, selectedResource, onWorkspaceChange, onError }: {
  jobId: string;
  workspace: WorkspaceSnapshot;
  selectedResource?: ResourceKey;
  onWorkspaceChange: (workspace: WorkspaceSnapshot) => void;
  onError: (error: unknown) => void;
}) {
  const [endpoint, setEndpoint] = useState("http://127.0.0.1:11434/v1/chat/completions");
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [prompt, setPrompt] = useState("");
  const [allowNetwork, setAllowNetwork] = useState(false);
  const [includeModuleMetadata, setIncludeModuleMetadata] = useState(false);
  const [includeResourceContents, setIncludeResourceContents] = useState(false);
  const [manualJson, setManualJson] = useState("");
  const [proposal, setProposal] = useState<AiProviderProposal>();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState("");

  async function askProvider() {
    setBusy(true);
    setResult("");
    try {
      const next = await requestAiChangeSet({
        jobId,
        workspaceId: workspace.workspaceId,
        endpoint,
        model,
        apiKey: apiKey || undefined,
        prompt,
        selectedResources: selectedResource ? [selectedResource] : [],
        consent: { allowNetwork, includeModuleMetadata, includeResourceContents },
      });
      setProposal(next);
      setResult(next.preview.allValid
        ? "Proposition validée. Vérifiez chaque opération avant confirmation."
        : "Proposition refusée : au moins une précondition ne correspond plus au workspace.");
    } catch (error) {
      onError(error);
    } finally {
      setApiKey("");
      setBusy(false);
    }
  }

  async function previewManualProposal() {
    setBusy(true);
    setResult("");
    try {
      const changeSet = JSON.parse(manualJson) as AiChangeSet;
      const preview = await previewAiChangeSet({ jobId, workspaceId: workspace.workspaceId, changeSet });
      setProposal({ endpointOrigin: "manuel", model: "proposition JSON locale", proposalSha256: preview.proposalSha256, changeSet, preview, sharedResources: 0, warnings: [] });
      setResult(preview.allValid ? "Proposition locale validée." : "Proposition locale refusée.");
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }

  async function applyProposal() {
    if (!proposal?.preview.allValid) return;
    if (!window.confirm(`Appliquer ${proposal.changeSet.commands.length} opération(s) IA annulable(s) ?`)) return;
    setBusy(true);
    try {
      const report = await applyAiChangeSet({
        jobId,
        workspaceId: workspace.workspaceId,
        proposalSha256: proposal.proposalSha256,
        changeSet: proposal.changeSet,
        confirmed: true,
      });
      onWorkspaceChange(report.workspace);
      setResult(`${report.appliedCommands} opération(s) appliquée(s). Utilisez Annuler pour les inverser.`);
      setProposal(undefined);
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="ai-assistant-card" aria-label="Assistant IA contrôlé">
      <header className="dependency-heading">
        <div><span className="eyebrow">LOT 25 · ASSISTANCE CONTRÔLÉE</span><h2><Sparkles size={18} /> Proposition d’opérations</h2></div>
        <span>Réseau {allowNetwork ? "autorisé pour cet appel" : "désactivé"}</span>
      </header>
      <p className="ai-safety"><ShieldCheck size={15} /> Le modèle ne reçoit rien sans consentement, ne peut proposer que des champs GFF ou du NSS, et n’écrit jamais directement dans le module.</p>
      <div className="ai-provider-grid">
        <label>Endpoint compatible OpenAI<input value={endpoint} onChange={(event) => setEndpoint(event.currentTarget.value)} spellCheck={false} /></label>
        <label>Modèle choisi<input value={model} onChange={(event) => setModel(event.currentTarget.value)} placeholder="nom exact du modèle" spellCheck={false} /></label>
        <label className="ai-key"><span><KeyRound size={12} /> Clé temporaire</span><input type="password" autoComplete="off" value={apiKey} onChange={(event) => setApiKey(event.currentTarget.value)} placeholder="jamais enregistrée" /></label>
      </div>
      <label className="ai-prompt">Demande<textarea value={prompt} onChange={(event) => setPrompt(event.currentTarget.value)} placeholder="Décrivez le changement voulu et ses limites…" /></label>
      <div className="ai-consents">
        <label><input type="checkbox" checked={allowNetwork} onChange={(event) => setAllowNetwork(event.currentTarget.checked)} /> Autoriser cet appel réseau</label>
        <label><input type="checkbox" checked={includeModuleMetadata} onChange={(event) => setIncludeModuleMetadata(event.currentTarget.checked)} /> Envoyer les métadonnées minimales</label>
        <label title={selectedResource ? selectedResource.resref : "Sélectionnez d’abord une ressource"}><input type="checkbox" disabled={!selectedResource} checked={includeResourceContents && Boolean(selectedResource)} onChange={(event) => setIncludeResourceContents(event.currentTarget.checked)} /> Envoyer la ressource sélectionnée {selectedResource ? <code>{selectedResource.resref}.{selectedResource.resourceType}</code> : null}</label>
      </div>
      <div className="profile-actions"><button type="button" disabled={busy || !allowNetwork || !model.trim() || !prompt.trim()} onClick={() => void askProvider()}><Sparkles size={13} /> Générer et prévisualiser</button></div>
      <details className="ai-manual-proposal"><summary>Prévisualiser une proposition JSON locale, sans réseau</summary><textarea aria-label="Proposition JSON locale" value={manualJson} onChange={(event) => setManualJson(event.currentTarget.value)} placeholder={'{"summary":"…","commands":[…]}'} spellCheck={false} /><button type="button" disabled={busy || !manualJson.trim()} onClick={() => void previewManualProposal()}>Valider localement</button></details>
      {proposal && <div className="ai-proposal">
        <div className="ai-proposal-heading"><div><strong>{proposal.changeSet.summary}</strong><small>{proposal.model} · {proposal.endpointOrigin} · {proposal.sharedResources} ressource(s) transmise(s)</small></div><span className={proposal.preview.allValid ? "valid" : "invalid"}>{proposal.preview.allValid ? <CheckCircle2 size={14} /> : <TriangleAlert size={14} />}{proposal.preview.allValid ? "Valide" : "Refusée"}</span></div>
        {proposal.warnings.map((warning) => <p className="ai-warning" key={warning}>{warning}</p>)}
        <div className="ai-command-list">{proposal.preview.previews.map((entry, index) => <article className={entry.valid ? "valid" : "invalid"} key={`${entry.target}-${index}`}><div><code>{entry.command.kind}</code><strong>{entry.target}</strong></div><span>{entry.valid ? "Précondition vérifiée" : entry.diagnostic ?? "Opération invalide"}</span></article>)}</div>
        <code className="ai-digest">SHA-256 · {proposal.proposalSha256}</code>
        <div className="profile-actions"><button type="button" disabled={busy || !proposal.preview.allValid} onClick={() => void applyProposal()}>Confirmer et appliquer</button></div>
      </div>}
      {result && <p className="profile-result">{result}</p>}
    </section>
  );
}
