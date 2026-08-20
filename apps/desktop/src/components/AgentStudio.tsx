import { useEffect, useMemo, useState } from "react";
import { Bot, Check as CheckIcon, CircleDollarSign, Gauge, KeyRound, LoaderCircle, Play, PlugZap, Save, Shield, SlidersHorizontal, Square, Workflow, X } from "lucide-react";
import {
  advanceAgentRun,
  cancelAgentRun,
  createAgentRun,
  getAgentStudioState,
  normalizeAppError,
  saveAgentPolicy,
  resolveAgentApproval,
  testAgentProvider,
  type AgentPolicy,
  type AgentStudioState,
  type ApprovalMode,
  type CapabilityAccess,
  type CapabilityOverride,
  type ProviderKind,
  type ProviderProfile,
  type ResourceKey,
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

export function AgentStudio({ jobId, workspace, selectedResource, activeView, initialObjective, onError }: {
  jobId: string;
  workspace: WorkspaceSnapshot;
  selectedResource?: ResourceKey;
  activeView?: string;
  initialObjective?: string;
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
  const [activeRunId, setActiveRunId] = useState<string>();
  const [activity, setActivity] = useState("");
  const [message, setMessage] = useState("");
  const [providerTest, setProviderTest] = useState<{ ok: boolean; message: string }>();

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

  useEffect(() => {
    if (initialObjective?.trim()) setObjective(initialObjective);
  }, [initialObjective]);

  const categories = useMemo(() => ["Toutes", ...new Set(studio?.registry.capabilities.map((item) => item.category) ?? [])], [studio]);
  const capabilities = useMemo(() => studio?.registry.capabilities.filter((item) => category === "Toutes" || item.category === category) ?? [], [studio, category]);

  function patchPolicy(patch: Partial<AgentPolicy>) {
    setPolicy((current) => current ? { ...current, ...patch } : current);
  }

  function changeProviderKind(kind: ProviderKind) {
    const defaults: Partial<Record<ProviderKind, Partial<ProviderProfile>>> = {
      open_ai_responses: { id: "openai-responses", name: "OpenAI Responses API", endpoint: "https://api.openai.com/v1/responses" },
      open_ai_chat_completions: { id: "openai-chat", name: "OpenAI compatible Chat Completions", endpoint: "https://api.openai.com/v1/chat/completions" },
      ollama: { id: "local-ollama", name: "Ollama local", endpoint: "http://127.0.0.1:11434/v1/chat/completions" },
      manual: { id: "manual", name: "Mode manuel" },
    };
    setProvider((current) => ({ ...current, kind, ...defaults[kind] }));
    setProviderTest(undefined);
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

  function useCurrentSelection() {
    if (!selectedResource) return;
    setPolicy((current) => current ? {
      ...current,
      scopeGrants: {
        ...current.scopeGrants,
        selectedResources: [selectedResource, ...current.scopeGrants.selectedResources.filter((value) => value.resref !== selectedResource.resref || value.resourceType !== selectedResource.resourceType)],
      },
    } : current);
    setMessage(`La ressource ${selectedResource.resref} a été ajoutée au périmètre de cette exécution.`);
  }

  async function savePolicy() {
    if (!policy) return;
    setBusy(true);
    setActivity("Enregistrement du profil…");
    setMessage("");
    try {
      const next = await saveAgentPolicy(workspace.workspaceId, policy);
      setStudio(next);
      setPolicy(next.policy);
      setMessage("Profil enregistré dans le workspace.");
    } catch (error) {
      setMessage(`Impossible d’enregistrer le profil : ${normalizeAppError(error).userMessage}`);
      onError(error);
    } finally {
      setActivity("");
      setBusy(false);
    }
  }

  async function prepareRun() {
    if (!policy || !objective.trim()) return;
    setBusy(true);
    setActivity("Préparation de l’exécution…");
    setMessage("");
    try {
      const blueprint = blueprintText.trim() ? JSON.parse(blueprintText) : undefined;
      const run = await createAgentRun({ jobId, workspaceId: workspace.workspaceId, objective, provider, policy, blueprint });
      setStudio((current) => current ? { ...current, runs: [run, ...current.runs] } : current);
      setMessage(`Exécution ${run.id.slice(0, 8)} préparée avec toutes ses limites et son journal de reprise.`);
    } catch (error) {
      setMessage(`Impossible de préparer l’exécution : ${normalizeAppError(error).userMessage}`);
      onError(error);
    } finally {
      setActivity("");
      setBusy(false);
    }
  }

  async function testProvider() {
    if (!policy) return;
    setBusy(true);
    setActivity("Test de communication avec le modèle…");
    setProviderTest(undefined);
    try {
      const report = await testAgentProvider({ provider, policy, apiKey: apiKey || undefined });
      setProviderTest({
        ok: true,
        message: `Connexion réussie · ${report.model} · ${report.latencyMs} ms · réponse : ${report.reply}`,
      });
    } catch (error) {
      const normalized = normalizeAppError(error);
      setProviderTest({ ok: false, message: `Échec du test : ${normalized.userMessage}` });
      onError(error);
    } finally {
      setApiKey("");
      setActivity("");
      setBusy(false);
    }
  }

  function replaceRun(run: AgentStudioState["runs"][number]) {
    setStudio((current) => current ? { ...current, runs: [run, ...current.runs.filter((item) => item.id !== run.id)] } : current);
  }

  async function advance(runId: string) {
    setBusy(true);
    setActiveRunId(runId);
    setActivity("Le modèle analyse la demande et choisit sa prochaine action…");
    setMessage("");
    try {
      const run = await advanceAgentRun({ workspaceId: workspace.workspaceId, runId, apiKey: apiKey || undefined });
      replaceRun(run);
      setMessage(run.status === "completed" ? "Exécution terminée." : run.status === "waiting_approval" ? "L’agent attend votre décision." : `État de l’exécution : ${run.status}.`);
    } catch (error) {
      setMessage(`L’agent n’a pas pu démarrer : ${normalizeAppError(error).userMessage}`);
      onError(error);
    } finally {
      setApiKey("");
      setActiveRunId(undefined);
      setActivity("");
      setBusy(false);
    }
  }

  async function resolve(runId: string, approvalId: string, approved: boolean) {
    setBusy(true);
    setActivity(approved ? "Application de l’action autorisée…" : "Enregistrement du refus…");
    try {
      const run = await resolveAgentApproval({ workspaceId: workspace.workspaceId, runId, approvalId, approved });
      replaceRun(run);
      setMessage(approved ? "Appel approuvé et exécuté. Vous pouvez poursuivre la boucle." : "Appel refusé et enregistré.");
    } catch (error) {
      setMessage(`La décision n’a pas pu être appliquée : ${normalizeAppError(error).userMessage}`);
      onError(error);
    } finally {
      setActivity("");
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
        <div><span className="eyebrow">PARCOURS GUIDÉ · AGENT MULTI-ÉTAPES</span><h2><Bot size={19} /> Décrire puis superviser un résultat</h2><p>Aucun modèle n’est contacté avant Tester, Lancer ou Poursuivre. Créer l’exécution enregistre seulement votre objectif et ses limites dans le workspace.</p></div>
        <span className={`agent-level level-${policy.level}`}><Shield size={14} /> {levels.find((item) => item.value === policy.level)?.label}</span>
      </header>

      <div className="agent-section agent-objective agent-quick-start">
        <div className="agent-guided-step">
          <div className="agent-step-number">1</div><div><h3>Choisir le moteur IA</h3><p>Le test envoie uniquement une demande « OK », sans contenu NWN.</p></div>
        </div>
        <div className="agent-provider-quick"><label>Fournisseur<select value={provider.kind} onChange={(event) => changeProviderKind(event.currentTarget.value as ProviderKind)}><option value="open_ai_responses">OpenAI · Responses API</option><option value="open_ai_chat_completions">OpenAI compatible</option><option value="ollama">Ollama local</option><option value="compatible">Serveur compatible personnalisé</option><option value="manual">Manuel · sans réseau</option></select></label><label>Modèle<input value={provider.model} disabled={provider.kind==="manual"} onChange={event=>{setProvider({...provider,model:event.currentTarget.value});setProviderTest(undefined)}} placeholder="Ex. qwen2.5-coder:7b"/></label><button type="button" className="provider-test-button" disabled={busy||provider.kind==="manual"||!provider.endpoint.trim()||!provider.model.trim()} onClick={()=>void testProvider()}>{busy&&activity.startsWith("Test")?<LoaderCircle className="agent-spinner" size={13}/>:<PlugZap size={13}/>} Tester</button></div>
        {provider.kind!=="manual"&&<label className="agent-quick-key"><span><KeyRound size={12}/> Clé temporaire si nécessaire</span><input type="password" autoComplete="off" value={apiKey} onChange={event=>setApiKey(event.currentTarget.value)} placeholder="Jamais persistée"/></label>}
        {providerTest&&<p className={`provider-test-result ${providerTest.ok?"success":"failure"}`} role="status">{providerTest.message}</p>}
        <div className="agent-guided-step"><div className="agent-step-number">2</div><div><h3>Contrôler le contexte</h3><p>L’agent voit seulement les données cochées et les ressources accordées.</p></div></div>
        <div className="agent-context-summary"><span>Atelier précédent <strong>{activeView??"module"}</strong></span>{selectedResource?<><span>Ressource sélectionnée <strong>{selectedResource.resref}</strong> · type {selectedResource.resourceType}</span><button type="button" className="secondary-button" onClick={useCurrentSelection}>Utiliser cette sélection</button></>:<span>Aucune ressource sélectionnée. L’objectif peut rester limité au module ou à une zone.</span>}</div>
        <div className="agent-guided-step"><div className="agent-step-number">3</div><div><h3>Décrire le résultat vérifiable</h3><p>Précisez les ressources à créer ou modifier et la validation attendue.</p></div></div>
        <textarea aria-label="Objectif de l’agent" value={objective} onChange={(event) => setObjective(event.currentTarget.value)} placeholder="Exemple : créer une zone de départ forestière, un PNJ et un dialogue, puis valider les scripts…" />
        <div className="profile-actions"><button type="button" className="secondary-button" disabled={busy} onClick={() => void savePolicy()}><Save size={13} /> Enregistrer le profil</button><button type="button" className="primary-button" disabled={busy || !objective.trim() || (provider.kind !== "manual" && !provider.model.trim())} onClick={() => void prepareRun()}><Workflow size={13} /> 4 · Créer l’exécution</button></div>
        {activity && !activeRunId && <div className="agent-working" role="status"><LoaderCircle className="agent-spinner" size={16} /><div><strong>{activity}</strong><span>Veuillez patienter.</span></div></div>}
        {message && <p className="profile-result" role="status">{message}</p>}
        {studio.runs.length > 0 && <div className="agent-runs"><strong>2 · Exécutions préparées</strong>{studio.runs.slice(0, 5).map((run) => {
          const pendingApproval = run.approvals.some((approval) => approval.status === "pending");
          const isWorking = activeRunId === run.id;
          return <article key={run.id} className={`agent-run run-${run.status}`}>
            <div><code>{run.id.slice(0, 8)}</code><strong>{agentStatusLabel(run.status)}</strong><span>{run.objective}</span></div>
            <small>{run.provider.name} · {run.provider.model || "sans modèle"} · {run.currentTurn}/{run.policy.limits.maxTurns} tours · {run.toolCalls.length}/{run.policy.limits.maxToolCalls} appels</small>
            {isWorking && <div className="agent-working" role="status"><LoaderCircle className="agent-spinner" size={17} /><div><strong>Le modèle travaille</strong><span>{activity} Un modèle local volumineux peut prendre plusieurs minutes.</span></div></div>}
            {run.approvals.filter((approval) => approval.status === "pending").map((approval) => <div className="agent-approval" key={approval.id}><span><Shield size={12} /> <strong>{approval.capabilityId}</strong> · {approval.summary}</span><button type="button" disabled={busy} onClick={() => void resolve(run.id, approval.id, false)}><X size={12} /> Refuser</button><button type="button" disabled={busy} onClick={() => void resolve(run.id, approval.id, true)}><CheckIcon size={12} /> Autoriser</button></div>)}
            {!pendingApproval && !["completed", "failed", "cancelled"].includes(run.status) && run.provider.kind !== "manual" && <div className="agent-run-actions"><button type="button" className="primary-button" disabled={busy} onClick={() => void advance(run.id)}>{isWorking ? <LoaderCircle className="agent-spinner" size={12} /> : <Play size={12} />} {run.status === "planned" ? "Lancer l’agent" : "Poursuivre l’agent"}</button><button type="button" className="secondary-button" onClick={() => void cancel(run.id)}><Square size={11} /> Arrêter</button></div>}
            <details open={run.status === "failed"}><summary>Journal détaillé ({run.events.length})</summary>{run.events.slice(-12).map((event) => <p key={event.sequence}><code>#{event.sequence}</code> {event.message}</p>)}</details>
          </article>;
        })}</div>}
      </div>

      <details className="agent-advanced">
        <summary><SlidersHorizontal size={15} /> Réglages avancés : fournisseur, sécurité, budgets et fonctions accessibles</summary>

      <div className="agent-blueprint-advanced"><label>ModuleBlueprint JSON (facultatif)<textarea value={blueprintText} onChange={(event) => setBlueprintText(event.currentTarget.value)} placeholder='{"schemaVersion":1,"name":"…","areas":[…]}' spellCheck={false} /></label><p>Ce contrat expert décrit un module complet. Une demande en langage naturel n’en a pas besoin.</p></div>

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
            <label>Protocole<select value={provider.kind} onChange={(event) => changeProviderKind(event.currentTarget.value as ProviderKind)}>
              <option value="open_ai_responses">OpenAI · Responses API</option><option value="open_ai_chat_completions">OpenAI compatible · Chat Completions</option><option value="ollama">Ollama local · API compatible OpenAI</option><option value="compatible">Serveur compatible OpenAI · personnalisé</option><option value="manual">Manuel · sans réseau</option>
            </select></label>
            <p className="agent-hint">« OpenAI compatible » désigne tout serveur qui expose le contrat <code>/v1/chat/completions</code>, notamment Ollama, LM Studio ou un proxy compatible.</p>
            <label>Endpoint<input value={provider.endpoint} disabled={provider.kind === "manual"} onChange={(event) => { setProvider({ ...provider, endpoint: event.currentTarget.value }); setProviderTest(undefined); }} spellCheck={false} /></label>
            <label>Modèle<input value={provider.model} disabled={provider.kind === "manual"} onChange={(event) => { setProvider({ ...provider, model: event.currentTarget.value }); setProviderTest(undefined); }} placeholder="Ex. qwen2.5-coder:7b" spellCheck={false} /></label>
            <button type="button" className="provider-test-button" disabled={busy || provider.kind === "manual" || !provider.endpoint.trim() || !provider.model.trim()} onClick={() => void testProvider()}>{busy && activity.startsWith("Test") ? <LoaderCircle className="agent-spinner" size={13} /> : <PlugZap size={13} />} Tester la communication avec le modèle</button>
            <p className="agent-hint">{!provider.model.trim() && provider.kind !== "manual" ? "Indiquez le nom exact d’un modèle installé pour activer le test. " : ""}Cliquer sur ce bouton envoie seulement « Répondez uniquement par OK » au modèle, sans ressource NWN.</p>
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
            <h3><SlidersHorizontal size={15} /> Données transmises et confidentialité</h3>
            <p className="agent-hint">Le modèle est contacté uniquement lorsque vous cliquez sur Tester, Lancer ou Poursuivre. HTTP reste limité aux modèles locaux ; un fournisseur distant doit utiliser HTTPS.</p>
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

          <div className="agent-section agent-tool-runtime">
            <h3>Outils externes <span className="agent-optional-badge">Facultatif</span></h3>
            <p className="agent-hint agent-tool-intro">Aucun de ces chemins n’est nécessaire pour consulter ou analyser un module. Renseignez seulement le groupe correspondant à l’action que vous souhaitez confier à l’agent.</p>

            <section className="agent-tool-group" aria-labelledby="agent-tool-compile-title">
              <div className="agent-tool-group-heading">
                <div><h4 id="agent-tool-compile-title">Compiler les scripts</h4><p>Transforme un fichier source <code>.nss</code> en script exécutable <code>.ncs</code>.</p></div>
                <span>NSS → NCS</span>
              </div>
              <RuntimePathField id="agent-compiler-path" label="Compilateur de scripts NWScript" kind="Fichier .exe · requis pour compiler" help="Chemin complet vers nwn_script_comp.exe (ou un compilateur compatible)." placeholder="C:\…\nwn_script_comp.exe" value={policy.toolRuntime.compilerPath} onChange={(value) => patchRuntime("compilerPath", value)} />
              <RuntimePathField id="agent-game-install-path" label="Dossier d’installation de NWN:EE" kind="Dossier · requis pour compiler" help="Racine du jeu contenant notamment les dossiers bin, data et lang." placeholder="C:\…\Neverwinter Nights" value={policy.toolRuntime.gameInstallPath} onChange={(value) => patchRuntime("gameInstallPath", value)} />
              <RuntimePathField id="agent-user-data-path" label="Dossier utilisateur de NWN:EE" kind="Dossier · requis pour les HAK/TLK utilisateur" help="Racine des données utilisateur contenant notamment hak, tlk, override et development. Le MCP l’utilise pour résoudre les tilesets et blueprints personnalisés." placeholder="C:\Users\…\Documents\Neverwinter Nights" value={policy.toolRuntime.userDataPath} onChange={(value) => patchRuntime("userDataPath", value)} />
              <RuntimePathField id="agent-include-paths" label="Dossiers d’includes supplémentaires" kind="Dossiers · facultatif" help="Emplacements de scripts .nss partagés. Séparez plusieurs dossiers par un point-virgule (;)." placeholder="C:\Mes scripts\includes; D:\NWN\includes" value={policy.toolRuntime.includePaths.join("; ")} onChange={(value) => patchRuntime("includePaths", value.split(";").map((item) => item.trim()).filter(Boolean))} />
            </section>

            <section className="agent-tool-group" aria-labelledby="agent-tool-output-title">
              <div className="agent-tool-group-heading">
                <div><h4 id="agent-tool-output-title">Produire, tester et synchroniser</h4><p>Ces trois chemins sont indépendants : renseignez uniquement ceux correspondant aux sorties que vous autorisez.</p></div>
                <span>Sorties contrôlées</span>
              </div>
              <RuntimePathField id="agent-output-roots" label="Dossiers de sortie autorisés" kind="Dossiers · requis pour créer ou construire un .mod" help="Barrière de sécurité : l’agent ne peut produire un module que dans ces dossiers. Séparez plusieurs dossiers par un point-virgule (;)." placeholder="D:\Mes modules\builds" value={policy.toolRuntime.allowedOutputRoots.join("; ")} onChange={(value) => patchRuntime("allowedOutputRoots", value.split(";").map((item) => item.trim()).filter(Boolean))} />
              <RuntimePathField id="agent-development-path" label="Dossier development de NWN" kind="Dossier · requis pour le test en direct" help="Dossier de surcharge temporaire de NWN, généralement dans vos données utilisateur. Le module source reste intact." placeholder="C:\Users\…\Documents\Neverwinter Nights\development" value={policy.toolRuntime.developmentPath} onChange={(value) => patchRuntime("developmentPath", value)} />
              <RuntimePathField id="agent-toolset-temp-path" label="Dossier temporaire d’Aurora Toolset" kind="Dossier · requis pour la synchronisation Toolset" help="Dossier extrait du module actuellement ouvert par Aurora. OpenNever compare les fichiers avant toute synchronisation." placeholder="C:\Users\…\AppData\Local\Temp\…" value={policy.toolRuntime.toolsetTempPath} onChange={(value) => patchRuntime("toolsetTempPath", value)} />
            </section>

            <section className="agent-tool-group" aria-labelledby="agent-tool-launch-title">
              <div className="agent-tool-group-heading">
                <div><h4 id="agent-tool-launch-title">Lancer le jeu ou le serveur</h4><p>Ces réglages servent uniquement à démarrer un test ; ils ne sont pas utilisés pendant l’édition.</p></div>
                <span>Test NWN</span>
              </div>
              <RuntimePathField id="agent-nwn-executable-path" label="Programme à lancer" kind="Fichier .exe · requis pour lancer" help="Choisissez nwmain.exe pour le jeu ou nwserver.exe pour un serveur." placeholder="C:\…\Neverwinter Nights\bin\win32\nwmain.exe" value={policy.toolRuntime.nwnExecutablePath} onChange={(value) => patchRuntime("nwnExecutablePath", value)} />
              <RuntimePathField id="agent-nwn-working-directory" label="Dossier de démarrage du programme" kind="Dossier · requis pour lancer" help="Dossier depuis lequel NWN ou nwserver sera démarré, généralement celui contenant l’exécutable." placeholder="C:\…\Neverwinter Nights\bin\win32" value={policy.toolRuntime.nwnWorkingDirectory} onChange={(value) => patchRuntime("nwnWorkingDirectory", value)} />
              <div className="agent-runtime-field">
                <label htmlFor="agent-nwn-arguments">Arguments de lancement</label>
                <span id="agent-nwn-arguments-help"><strong>Facultatif</strong> · Un argument par ligne, transmis directement au programme sans passer par un terminal.</span>
                <textarea id="agent-nwn-arguments" aria-describedby="agent-nwn-arguments-help" value={policy.toolRuntime.nwnArguments.join("\n")} onChange={(event) => patchRuntime("nwnArguments", event.currentTarget.value.split(/\r?\n/).map((value) => value.trim()).filter(Boolean))} placeholder={'-module\nma_copie_test'} spellCheck={false} />
              </div>
            </section>
          </div>

          <div className="agent-section">
            <h3>Actions externes</h3>
            <p className="agent-hint">Cocher une action l’autorise, mais ne la déclenche pas. Son chemin correspondant doit aussi être configuré ci-dessus.</p>
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

        </div>
      </div>
      </details>
    </section>
  );
}

function agentStatusLabel(status: AgentStudioState["runs"][number]["status"]): string {
  return {
    planned: "Prête à lancer",
    running: "En cours",
    waiting_approval: "Votre validation est requise",
    completed: "Terminée",
    failed: "Échec",
    cancelled: "Arrêtée",
  }[status];
}

function RuntimePathField({ id, label, kind, help, placeholder, value, onChange }: {
  id: string;
  label: string;
  kind: string;
  help: string;
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
}) {
  const helpId = `${id}-help`;
  return <div className="agent-runtime-field">
    <label htmlFor={id}>{label}</label>
    <span id={helpId}><strong>{kind}</strong> · {help}</span>
    <input id={id} aria-describedby={helpId} value={value} onChange={(event) => onChange(event.currentTarget.value)} placeholder={placeholder} spellCheck={false} />
  </div>;
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
