import { useEffect, useMemo, useState } from "react";
import { Bot, Check as CheckIcon, CircleDollarSign, Gauge, KeyRound, Play, Save, Shield, SlidersHorizontal, Square, Workflow, X } from "lucide-react";
import {
  advanceAgentRun,
  cancelAgentRun,
  createAgentRun,
  getAgentStudioState,
  saveAgentPolicy,
  resolveAgentApproval,
  type AgentPolicy,
  type AgentStudioState,
  type ApprovalMode,
  type CapabilityAccess,
  type CapabilityOverride,
  type ProviderKind,
  type ProviderProfile,
  type SecurityLevel,
  type ToolScope,
  type WorkspaceSnapshot,
} from "../lib/tauri";

const levels: Array<{ value: SecurityLevel; label: string; description: string }> = [
  { value: "observer", label: "0 · Observateur", description: "Lecture et diagnostics uniquement" },
  { value: "advisor", label: "1 · Conseiller", description: "Plans et prévisualisations sans application" },
  { value: "assisted", label: "2 · Assistant", description: "Confirmation de chaque lot d’écriture" },
  { value: "supervised", label: "3 · Agent supervisé", description: "Éditions réversibles automatiques, risques confirmés" },
  { value: "autonomous", label: "4 · Constructeur autonome", description: "Création et validation dans le workspace" },
  { value: "operator", label: "5 · Opérateur expert", description: "Déploiement et Toolset selon la matrice" },
];

const defaultProvider: ProviderProfile = {
  id: "local-ollama",
  name: "Ollama local",
  kind: "ollama",
  endpoint: "http://127.0.0.1:11434/v1/chat/completions",
  model: "",
  supportsTools: true,
  supportsParallelTools: false,
  supportsStructuredOutput: true,
  storeResponses: false,
  inputCostMicroUsdPerMillionTokens: 0,
  outputCostMicroUsdPerMillionTokens: 0,
};

export function AgentStudio({ jobId, workspace, onError }: {
  jobId: string;
  workspace: WorkspaceSnapshot;
  onError: (error: unknown) => void;
}) {
  const [studio, setStudio] = useState<AgentStudioState>();
  const [policy, setPolicy] = useState<AgentPolicy>();
  const [provider, setProvider] = useState(defaultProvider);
  const [objective, setObjective] = useState("");
  const [blueprintText, setBlueprintText] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [category, setCategory] = useState("Toutes");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    let alive = true;
    void getAgentStudioState(workspace.workspaceId)
      .then((next) => {
        if (!alive) return;
        setStudio(next);
        setPolicy(next.policy);
      })
      .catch(onError);
    return () => { alive = false; };
  }, [workspace.workspaceId, onError]);

  const categories = useMemo(() => ["Toutes", ...new Set(studio?.registry.capabilities.map((item) => item.category) ?? [])], [studio]);
  const capabilities = useMemo(() => studio?.registry.capabilities.filter((item) => category === "Toutes" || item.category === category) ?? [], [studio, category]);

  function patchPolicy(patch: Partial<AgentPolicy>) {
    setPolicy((current) => current ? { ...current, ...patch } : current);
  }

  function loadSecurityPreset(level: SecurityLevel) {
    const preset = studio?.presets.find((item) => item.level === level);
    if (!preset) return;
    setPolicy((current) => current ? {
      ...preset,
      toolRuntime: current.toolRuntime,
      scopeGrants: current.scopeGrants,
      context: {
        ...preset.context,
        allowedProviderHosts: current.context.allowedProviderHosts,
        allowInsecureLocalHttp: current.context.allowInsecureLocalHttp,
      },
    } : current);
    setMessage(`Preset « ${preset.name} » chargé. Les chemins, portées et hôtes autorisés sont conservés.`);
  }

  function patchContext(key: keyof AgentPolicy["context"], value: boolean | number) {
    setPolicy((current) => current ? { ...current, context: { ...current.context, [key]: value } } : current);
  }

  function patchLimit(key: keyof AgentPolicy["limits"], value: number) {
    setPolicy((current) => current ? { ...current, limits: { ...current.limits, [key]: Math.max(0, value) } } : current);
  }

  function patchRuntime(key: keyof AgentPolicy["toolRuntime"], value: string | string[]) {
    setPolicy((current) => current ? { ...current, toolRuntime: { ...current.toolRuntime, [key]: value } } : current);
  }

  function capabilityRule(id: string): CapabilityOverride {
    return policy?.capabilityOverrides[id] ?? policy?.capabilityOverrides["*"] ?? {
      access: "deny", approval: "always", scope: "selected_resource", maxCalls: 0,
    };
  }

  function patchCapability(id: string, patch: Partial<CapabilityOverride>) {
    setPolicy((current) => {
      if (!current) return current;
      return {
        ...current,
        capabilityOverrides: {
          ...current.capabilityOverrides,
          [id]: { ...(current.capabilityOverrides[id] ?? current.capabilityOverrides["*"]), ...patch },
        },
      };
    });
  }

  async function savePolicy() {
    if (!policy) return;
    setBusy(true);
    setMessage("");
    try {
      const next = await saveAgentPolicy(workspace.workspaceId, policy);
      setStudio(next);
      setPolicy(next.policy);
      setMessage("Profil enregistré dans le workspace.");
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }

  async function prepareRun() {
    if (!policy || !objective.trim()) return;
    setBusy(true);
    setMessage("");
    try {
      const blueprint = blueprintText.trim() ? JSON.parse(blueprintText) : undefined;
      const run = await createAgentRun({ jobId, workspaceId: workspace.workspaceId, objective, provider, policy, blueprint });
      setStudio((current) => current ? { ...current, runs: [run, ...current.runs] } : current);
      setMessage(`Exécution ${run.id.slice(0, 8)} préparée avec toutes ses limites et son journal de reprise.`);
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }

  function replaceRun(run: AgentStudioState["runs"][number]) {
    setStudio((current) => current ? { ...current, runs: [run, ...current.runs.filter((item) => item.id !== run.id)] } : current);
  }

  async function advance(runId: string) {
    setBusy(true);
    setMessage("");
    try {
      const run = await advanceAgentRun({ workspaceId: workspace.workspaceId, runId, apiKey: apiKey || undefined });
      replaceRun(run);
      setMessage(run.status === "completed" ? "Exécution terminée." : run.status === "waiting_approval" ? "L’agent attend votre décision." : `État de l’exécution : ${run.status}.`);
    } catch (error) {
      onError(error);
    } finally {
      setApiKey("");
      setBusy(false);
    }
  }

  async function resolve(runId: string, approvalId: string, approved: boolean) {
    setBusy(true);
    try {
      const run = await resolveAgentApproval({ workspaceId: workspace.workspaceId, runId, approvalId, approved });
      replaceRun(run);
      setMessage(approved ? "Appel approuvé et exécuté. Vous pouvez poursuivre la boucle." : "Appel refusé et enregistré.");
    } catch (error) {
      onError(error);
    } finally {
      setBusy(false);
    }
  }

  async function cancel(runId: string) {
    try {
      const run = await cancelAgentRun({ workspaceId: workspace.workspaceId, runId });
      replaceRun(run);
      setMessage("Arrêt demandé ; aucun nouveau tour ni outil ne sera lancé.");
    } catch (error) {
      onError(error);
    }
  }

  if (!studio || !policy) {
    return <section className="agent-studio-card"><p>Chargement des politiques agentiques…</p></section>;
  }

  return (
    <section className="agent-studio-card" aria-label="Agent Studio">
      <header className="agent-studio-header">
        <div><span className="eyebrow">AGENT STUDIO · POLITIQUES FINES</span><h2><Bot size={19} /> Construction agentique</h2></div>
        <span className={`agent-level level-${policy.level}`}><Shield size={14} /> {levels.find((item) => item.value === policy.level)?.label}</span>
      </header>

      <div className="agent-studio-grid">
        <div className="agent-settings-column">
          <div className="agent-section">
            <h3><Shield size={15} /> Profil de sécurité</h3>
            <label>Nom du profil<input value={policy.name} onChange={(event) => patchPolicy({ name: event.currentTarget.value })} /></label>
            <label>Niveau<select value={policy.level} onChange={(event) => loadSecurityPreset(event.currentTarget.value as SecurityLevel)}>{levels.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}</select></label>
            <p className="agent-hint">{levels.find((item) => item.value === policy.level)?.description}. Le changement charge le preset ; la matrice ci-dessous permet ensuite de le spécialiser.</p>
          </div>

          <div className="agent-section">
            <h3><Workflow size={15} /> Fournisseur</h3>
            <label>Protocole<select value={provider.kind} onChange={(event) => setProvider({ ...provider, kind: event.currentTarget.value as ProviderKind })}>
              <option value="open_ai_responses">Responses API</option><option value="open_ai_chat_completions">Chat Completions</option><option value="ollama">Ollama</option><option value="compatible">Compatible</option><option value="manual">Manuel / sans réseau</option>
            </select></label>
            <label>Endpoint<input value={provider.endpoint} disabled={provider.kind === "manual"} onChange={(event) => setProvider({ ...provider, endpoint: event.currentTarget.value })} spellCheck={false} /></label>
            <label>Modèle<input value={provider.model} disabled={provider.kind === "manual"} onChange={(event) => setProvider({ ...provider, model: event.currentTarget.value })} spellCheck={false} /></label>
            <label>Effort de raisonnement<select value={provider.reasoningEffort ?? ""} onChange={(event) => setProvider({ ...provider, reasoningEffort: event.currentTarget.value || undefined })}><option value="">Défaut du fournisseur</option><option value="low">Faible</option><option value="medium">Moyen</option><option value="high">Élevé</option><option value="xhigh">Très élevé</option></select></label>
            <label>Température ×1000<input type="number" min={0} max={2000} value={provider.temperatureMilli ?? ""} placeholder="Défaut" onChange={(event) => setProvider({ ...provider, temperatureMilli: event.currentTarget.value === "" ? undefined : Number(event.currentTarget.value) })} /></label>
            <div className="agent-inline-checks"><Check checked={provider.supportsTools} label="Outils" onChange={(value) => setProvider({ ...provider, supportsTools: value })} /><Check checked={provider.supportsParallelTools} label="Parallèle" onChange={(value) => setProvider({ ...provider, supportsParallelTools: value })} /><Check checked={provider.supportsStructuredOutput} label="JSON strict" onChange={(value) => setProvider({ ...provider, supportsStructuredOutput: value })} /></div>
            {provider.kind === "open_ai_responses" && <>
              <Check checked={provider.storeResponses} label="Stockage de la conversation chez le fournisseur" onChange={(value) => setProvider({ ...provider, storeResponses: value })} />
              <p className="agent-hint">Responses sans stockage rejoue localement les éléments de conversation ; avec stockage, la reprise utilise l’identifiant de réponse du fournisseur.</p>
            </>}
            <div className="agent-limit-grid"><DecimalField label="Entrée USD/M tokens" value={provider.inputCostMicroUsdPerMillionTokens / 1_000_000} onChange={(value) => setProvider({ ...provider, inputCostMicroUsdPerMillionTokens: Math.round(value * 1_000_000) })} /><DecimalField label="Sortie USD/M tokens" value={provider.outputCostMicroUsdPerMillionTokens / 1_000_000} onChange={(value) => setProvider({ ...provider, outputCostMicroUsdPerMillionTokens: Math.round(value * 1_000_000) })} /></div>
          </div>

          <div className="agent-section">
            <h3><SlidersHorizontal size={15} /> Contexte et confidentialité</h3>
            <Check checked={policy.context.allowNetwork} label="Autoriser le réseau" onChange={(value) => patchContext("allowNetwork", value)} />
            <Check checked={policy.context.allowInsecureLocalHttp} label="Autoriser HTTP local" onChange={(value) => patchContext("allowInsecureLocalHttp", value)} />
            <Check checked={policy.context.includeModuleMetadata} label="Métadonnées du module" onChange={(value) => patchContext("includeModuleMetadata", value)} />
            <Check checked={policy.context.includeResourceContents} label="Contenu des ressources" onChange={(value) => patchContext("includeResourceContents", value)} />
            <Check checked={policy.context.includeDiagnostics} label="Diagnostics" onChange={(value) => patchContext("includeDiagnostics", value)} />
            <Check checked={policy.context.includeArchitectureGraph} label="Sous-graphe d’architecture" onChange={(value) => patchContext("includeArchitectureGraph", value)} />
            <Check checked={policy.context.includeLocalPaths} label="Chemins locaux dans le contexte" onChange={(value) => patchContext("includeLocalPaths", value)} />
            <Check checked={policy.context.retainConversation} label="Conserver la conversation" onChange={(value) => patchContext("retainConversation", value)} />
            <label>Rétention (jours)<input type="number" min={0} max={3650} value={policy.context.retentionDays} onChange={(event) => patchContext("retentionDays", Number(event.currentTarget.value))} /></label>
            <label>Hôtes fournisseur autorisés<input value={policy.context.allowedProviderHosts.join(", ")} onChange={(event) => setPolicy({ ...policy, context: { ...policy.context, allowedProviderHosts: event.currentTarget.value.split(",").map((value) => value.trim()).filter(Boolean) } })} spellCheck={false} /></label>
          </div>

          <div className="agent-section">
            <h3><Gauge size={15} /> Budgets d’exécution</h3>
            <div className="agent-limit-grid">
              <NumberField label="Tours" value={policy.limits.maxTurns} onChange={(value) => patchLimit("maxTurns", value)} />
              <NumberField label="Appels outils" value={policy.limits.maxToolCalls} onChange={(value) => patchLimit("maxToolCalls", value)} />
              <NumberField label="Parallèles" value={policy.limits.maxParallelCalls} onChange={(value) => patchLimit("maxParallelCalls", value)} />
              <NumberField label="Tentatives" value={policy.limits.maxRetries} onChange={(value) => patchLimit("maxRetries", value)} />
              <NumberField label="Durée (s)" value={policy.limits.maxDurationSeconds} onChange={(value) => patchLimit("maxDurationSeconds", value)} />
              <NumberField label="Ressources" value={policy.limits.maxContextResources} onChange={(value) => patchLimit("maxContextResources", value)} />
              <NumberField label="Prompt (octets)" value={policy.limits.maxPromptBytes} onChange={(value) => patchLimit("maxPromptBytes", value)} />
              <NumberField label="Ressource contexte (octets)" value={policy.limits.maxContextResourceBytes} onChange={(value) => patchLimit("maxContextResourceBytes", value)} />
              <NumberField label="Réponse (octets)" value={policy.limits.maxResponseBytes} onChange={(value) => patchLimit("maxResponseBytes", value)} />
              <NumberField label="Sortie modèle (tokens)" value={policy.limits.maxOutputTokens} onChange={(value) => patchLimit("maxOutputTokens", value)} />
            </div>
            <label className="agent-cost"><CircleDollarSign size={13} /> Budget maximal (USD)<input type="number" min={0} step="0.01" value={policy.limits.maxCostMicroUsd / 1_000_000} onChange={(event) => patchLimit("maxCostMicroUsd", Math.round(Number(event.currentTarget.value) * 1_000_000))} /></label>
          </div>

          <div className="agent-section">
            <h3>Environnement des outils</h3>
            <label>Compilateur NWScript<input value={policy.toolRuntime.compilerPath} onChange={(event) => patchRuntime("compilerPath", event.currentTarget.value)} spellCheck={false} /></label>
            <label>Installation du jeu<input value={policy.toolRuntime.gameInstallPath} onChange={(event) => patchRuntime("gameInstallPath", event.currentTarget.value)} spellCheck={false} /></label>
            <label>Dossiers d’includes<input value={policy.toolRuntime.includePaths.join("; ")} onChange={(event) => patchRuntime("includePaths", event.currentTarget.value.split(";").map((value) => value.trim()).filter(Boolean))} spellCheck={false} /></label>
            <label>Dossier development<input value={policy.toolRuntime.developmentPath} onChange={(event) => patchRuntime("developmentPath", event.currentTarget.value)} spellCheck={false} /></label>
            <label>Dossier temporaire Toolset<input value={policy.toolRuntime.toolsetTempPath} onChange={(event) => patchRuntime("toolsetTempPath", event.currentTarget.value)} spellCheck={false} /></label>
            <label>Racines de sortie autorisées<input value={policy.toolRuntime.allowedOutputRoots.join("; ")} onChange={(event) => patchRuntime("allowedOutputRoots", event.currentTarget.value.split(";").map((value) => value.trim()).filter(Boolean))} spellCheck={false} /></label>
            <label>Exécutable NWN / nwserver<input value={policy.toolRuntime.nwnExecutablePath} onChange={(event) => patchRuntime("nwnExecutablePath", event.currentTarget.value)} spellCheck={false} /></label>
            <label>Dossier de travail NWN<input value={policy.toolRuntime.nwnWorkingDirectory} onChange={(event) => patchRuntime("nwnWorkingDirectory", event.currentTarget.value)} spellCheck={false} /></label>
            <label>Arguments NWN (un par ligne)<textarea value={policy.toolRuntime.nwnArguments.join("\n")} onChange={(event) => patchRuntime("nwnArguments", event.currentTarget.value.split(/\r?\n/).map((value) => value.trim()).filter(Boolean))} spellCheck={false} /></label>
          </div>

          <div className="agent-section">
            <h3>Actions externes</h3>
            <Check checked={policy.allowDevelopmentDeploy} label="Déploiement development" onChange={(value) => patchPolicy({ allowDevelopmentDeploy: value })} />
            <Check checked={policy.allowToolsetSync} label="Synchronisation Toolset" onChange={(value) => patchPolicy({ allowToolsetSync: value })} />
            <Check checked={policy.allowProcessLaunch} label="Lancement NWN / nwserver" onChange={(value) => patchPolicy({ allowProcessLaunch: value })} />
            <Check checked={policy.stopOnValidationError} label="Arrêt au premier défaut de validation" onChange={(value) => patchPolicy({ stopOnValidationError: value })} />
            <Check checked={policy.checkpointBeforeWrite} label="Checkpoint avant toute écriture" onChange={(value) => patchPolicy({ checkpointBeforeWrite: value })} />
          </div>

          <div className="agent-section">
            <h3>Périmètres autorisés</h3>
            <label>Zones (séparées par une virgule)<input value={policy.scopeGrants.areas.join(", ")} onChange={(event) => patchPolicy({ scopeGrants: { ...policy.scopeGrants, areas: event.currentTarget.value.split(",").map((value) => value.trim()).filter(Boolean) } })} spellCheck={false} /></label>
            <label>Ressources sélectionnées (`resref:type`)<textarea value={policy.scopeGrants.selectedResources.map((resource) => `${resource.resref}:${resource.resourceType}`).join("\n")} onChange={(event) => patchPolicy({ scopeGrants: { ...policy.scopeGrants, selectedResources: event.currentTarget.value.split(/\r?\n/).map((value) => value.trim()).filter(Boolean).map((value) => { const [resref, type] = value.split(":"); return { resref, resourceType: Number(type) }; }).filter((resource) => resource.resref && Number.isInteger(resource.resourceType)) } })} spellCheck={false} /></label>
          </div>
        </div>

        <div className="agent-capabilities-column">
          <div className="agent-section capability-matrix">
            <div className="capability-heading"><div><h3>Fonctions accessibles</h3><p>{studio.registry.capabilities.length} capacités enregistrées · invariants d’intégrité toujours actifs</p></div><select value={category} onChange={(event) => setCategory(event.currentTarget.value)}>{categories.map((item) => <option key={item}>{item}</option>)}</select></div>
            <div className="capability-list">{capabilities.map((capability) => {
              const rule = capabilityRule(capability.id);
              return <article key={capability.id} className={`capability-row risk-${capability.risk}`}>
                <div className="capability-name"><strong>{capability.title}</strong><code>{capability.id}</code><small>{capability.description}</small></div>
                <label>Accès<select value={rule.access} onChange={(event) => patchCapability(capability.id, { access: event.currentTarget.value as CapabilityAccess })}><option value="deny">Interdit</option><option value="read">Lecture</option><option value="preview">Prévisualisation</option><option value="execute">Exécution</option></select></label>
                <label>Approbation<select value={rule.approval} onChange={(event) => patchCapability(capability.id, { approval: event.currentTarget.value as ApprovalMode })}><option value="always">Toujours</option><option value="per_batch">Par lot</option><option value="above_risk">Selon risque</option><option value="never">Jamais</option></select></label>
                <label>Périmètre<select value={rule.scope} onChange={(event) => patchCapability(capability.id, { scope: event.currentTarget.value as ToolScope })}><option value="selected_resource">Ressource</option><option value="area">Zone</option><option value="module">Module</option><option value="workspace">Workspace</option></select></label>
                <label>Max<input type="number" min={0} max={policy.limits.maxToolCalls} value={rule.maxCalls} onChange={(event) => patchCapability(capability.id, { maxCalls: Number(event.currentTarget.value) })} /></label>
                <span className="capability-badges"><i>{capability.risk}</i><i>{capability.sideEffect}</i>{capability.reversible && <i>annulable</i>}</span>
              </article>;
            })}</div>
          </div>

          <div className="agent-section agent-objective">
            <h3>Nouvelle exécution</h3>
            <textarea value={objective} onChange={(event) => setObjective(event.currentTarget.value)} placeholder="Décrivez le module ou la transformation à réaliser, les contraintes et les critères de réussite…" />
            <label>ModuleBlueprint JSON (facultatif)<textarea value={blueprintText} onChange={(event) => setBlueprintText(event.currentTarget.value)} placeholder='{"schemaVersion":1,"name":"…","areas":[…]}' spellCheck={false} /></label>
            {provider.kind !== "manual" && <label><span><KeyRound size={12} /> Clé temporaire</span><input type="password" autoComplete="off" value={apiKey} onChange={(event) => setApiKey(event.currentTarget.value)} placeholder="Jamais persistée" /></label>}
            <div className="profile-actions"><button type="button" className="secondary-button" disabled={busy} onClick={() => void savePolicy()}><Save size={13} /> Enregistrer le profil</button><button type="button" className="primary-button" disabled={busy || !objective.trim() || (provider.kind !== "manual" && !provider.model.trim())} onClick={() => void prepareRun()}><Workflow size={13} /> Préparer l’exécution</button></div>
            {message && <p className="profile-result">{message}</p>}
            {studio.runs.length > 0 && <div className="agent-runs"><strong>Exécutions persistées</strong>{studio.runs.slice(0, 5).map((run) => <article key={run.id} className={`agent-run run-${run.status}`}><div><code>{run.id.slice(0, 8)}</code><strong>{run.status}</strong><span>{run.objective}</span></div><small>{run.currentTurn}/{run.policy.limits.maxTurns} tours · {run.toolCalls.length}/{run.policy.limits.maxToolCalls} appels</small>{run.approvals.filter((approval) => approval.status === "pending").map((approval) => <div className="agent-approval" key={approval.id}><span><Shield size={12} /> <strong>{approval.capabilityId}</strong> · {approval.summary}</span><button type="button" disabled={busy} onClick={() => void resolve(run.id, approval.id, false)}><X size={12} /> Refuser</button><button type="button" disabled={busy} onClick={() => void resolve(run.id, approval.id, true)}><CheckIcon size={12} /> Autoriser</button></div>)}{!run.approvals.some((approval) => approval.status === "pending") && !["completed", "failed", "cancelled"].includes(run.status) && run.provider.kind !== "manual" && <div className="agent-run-actions"><button type="button" className="secondary-button" disabled={busy} onClick={() => void advance(run.id)}><Play size={12} /> Démarrer / poursuivre</button><button type="button" className="secondary-button" onClick={() => void cancel(run.id)}><Square size={11} /> Arrêter</button></div>}<details><summary>Journal ({run.events.length})</summary>{run.events.slice(-12).map((event) => <p key={event.sequence}><code>#{event.sequence}</code> {event.message}</p>)}</details></article>)}</div>}
          </div>
        </div>
      </div>
    </section>
  );
}

function Check({ checked, label, onChange }: { checked: boolean; label: string; onChange: (value: boolean) => void }) {
  return <label className="agent-check"><input type="checkbox" checked={checked} onChange={(event) => onChange(event.currentTarget.checked)} /><span>{label}</span></label>;
}

function NumberField({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return <label>{label}<input type="number" min={0} value={value} onChange={(event) => onChange(Number(event.currentTarget.value))} /></label>;
}

function DecimalField({ label, value, onChange }: { label: string; value: number; onChange: (value: number) => void }) {
  return <label>{label}<input type="number" min={0} step="0.01" value={value} onChange={(event) => onChange(Number(event.currentTarget.value))} /></label>;
}
