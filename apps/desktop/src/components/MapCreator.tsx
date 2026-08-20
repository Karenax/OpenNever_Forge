import { useQuery } from "@tanstack/react-query";
import { Bot, Check, Download, Image, LoaderCircle, Map, RefreshCw, ShieldCheck, Sparkles, WandSparkles } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  applyMapGeneration,
  draftMapWithAi,
  getMapAuthoringContext,
  inspectWorld,
  listWorkspaceCreatedAreas,
  normalizeAppError,
  previewMapGeneration,
  type AreaMap,
  type MapDensityRule,
  type MapGenerationPlan,
  type MapGenerationSpec,
  type ProviderKind,
  type ProviderProfile,
  type WorkspaceSnapshot,
} from "../lib/tauri";
import "./MapCreator.css";

const densityDefinitions = [
  { category: "placeable", label: "Plaçables", hint: "Tables, végétation, mobilier, décor", color: "#d49a52" },
  { category: "creature", label: "Créatures", hint: "PNJ, habitants et adversaires", color: "#df6f68" },
  { category: "door", label: "Portes", hint: "Accès et passages", color: "#9e7ad7" },
  { category: "waypoint", label: "Repères", hint: "Points de passage et destinations", color: "#58a7d8" },
  { category: "sound", label: "Ambiances", hint: "Sources sonores spatialisées", color: "#68b987" },
] as const;

const defaultSpec: MapGenerationSpec = {
  schemaVersion: 1,
  brief: "Une zone de départ lisible avec un chemin central, quelques décors et des espaces libres pour les rencontres.",
  resref: "vibe_map",
  name: "Carte vibecodée",
  tileset: "tno01",
  width: 12,
  height: 10,
  seed: 20260811,
  baseTileId: 0,
  variantTileIds: [],
  borderMargin: 1,
  reservedPercent: 25,
  densities: densityDefinitions.map(({ category }) => ({
    category,
    perHundredTiles: category === "placeable" ? 18 : category === "creature" ? 5 : category === "waypoint" ? 2 : 0,
    minSpacingTiles: category === "creature" ? 3 : 1,
    templateResrefs: [],
  })),
};

const mapAiStorageKey = "opennever.map-ai-provider.v1";
const providerDefaults: Record<Exclude<ProviderKind, "manual">, { endpoint: string; name: string }> = {
  open_ai_responses: { endpoint: "https://api.openai.com/v1/responses", name: "OpenAI Responses" },
  open_ai_chat_completions: { endpoint: "https://api.openai.com/v1/chat/completions", name: "OpenAI Chat Completions" },
  ollama: { endpoint: "http://127.0.0.1:11434/v1/chat/completions", name: "Ollama local" },
  compatible: { endpoint: "http://127.0.0.1:1234/v1/chat/completions", name: "API compatible" },
};

function defaultMapAiProvider(): ProviderProfile {
  return {
    id: "map-creator",
    name: providerDefaults.open_ai_responses.name,
    kind: "open_ai_responses",
    endpoint: providerDefaults.open_ai_responses.endpoint,
    model: "",
    supportsTools: true,
    supportsParallelTools: false,
    supportsStructuredOutput: true,
    storeResponses: false,
    inputCostMicroUsdPerMillionTokens: 0,
    outputCostMicroUsdPerMillionTokens: 0,
  };
}

function loadMapAiProvider(): ProviderProfile {
  try {
    const stored = JSON.parse(localStorage.getItem(mapAiStorageKey) ?? "null") as Partial<ProviderProfile> | null;
    if (!stored || stored.kind === "manual" || !stored.kind || !providerDefaults[stored.kind]) return defaultMapAiProvider();
    return { ...defaultMapAiProvider(), ...stored, id: "map-creator", supportsTools: true, supportsParallelTools: false, storeResponses: false };
  } catch {
    return defaultMapAiProvider();
  }
}

export function MapCreator({ jobId, workspace, onWorkspace, onCreateWorkspace, onOpenAgent }: {
  jobId: string;
  workspace?: WorkspaceSnapshot;
  onWorkspace: (workspace: WorkspaceSnapshot) => void;
  onCreateWorkspace: () => Promise<void>;
  onOpenAgent: (objective: string) => void;
}) {
  const [spec, setSpec] = useState(defaultSpec);
  const [plan, setPlan] = useState<MapGenerationPlan>();
  const [generatedArea, setGeneratedArea] = useState<AreaMap>();
  const [selectedAtlasArea, setSelectedAtlasArea] = useState("");
  const [busy, setBusy] = useState<"preview" | "apply" | "ai">();
  const [aiProvider, setAiProvider] = useState(loadMapAiProvider);
  const [apiKey, setApiKey] = useState("");
  const [includeBlueprintResrefs, setIncludeBlueprintResrefs] = useState(true);
  const [message, setMessage] = useState("Décrivez la carte, puis verrouillez ses contraintes reproductibles.");
  useEffect(() => {
    localStorage.setItem(mapAiStorageKey, JSON.stringify(aiProvider));
  }, [aiProvider]);
  const worldQuery = useQuery({ queryKey: ["map-creator-world", jobId], queryFn: () => inspectWorld({ jobId }) });
  const authoringQuery = useQuery({
    queryKey: ["map-authoring-context", jobId, spec.tileset],
    queryFn: () => getMapAuthoringContext({ jobId, tileset: spec.tileset }),
    enabled: Boolean(jobId && spec.tileset),
    retry: false,
  });
  const createdAreasQuery = useQuery({
    queryKey: ["map-creator-created", workspace?.workspaceId, workspace?.cursor],
    queryFn: () => listWorkspaceCreatedAreas({ workspaceId: workspace?.workspaceId ?? "" }),
    enabled: Boolean(workspace),
  });
  const atlasAreas = useMemo(() => {
    const values = [...(worldQuery.data?.areas ?? []), ...(createdAreasQuery.data ?? [])];
    if (generatedArea) values.push(generatedArea);
    const unique = new globalThis.Map<string, AreaMap>();
    for (const value of values) unique.set(value.resref, value);
    return [...unique.values()].sort((left, right) => left.resref.localeCompare(right.resref));
  }, [createdAreasQuery.data, generatedArea, worldQuery.data?.areas]);
  const atlasArea = atlasAreas.find((area) => area.resref === selectedAtlasArea) ?? generatedArea ?? atlasAreas[0];
  const previewArea = generatedArea ?? (plan ? areaFromPlan(plan) : undefined);

  const patchSpec = <Key extends keyof MapGenerationSpec>(key: Key, value: MapGenerationSpec[Key]) => {
    setSpec((current) => ({ ...current, [key]: value }));
    setPlan(undefined);
    setGeneratedArea(undefined);
  };
  const patchDensity = (category: string, patch: Partial<MapDensityRule>) => {
    patchSpec("densities", spec.densities.map((rule) => rule.category === category ? { ...rule, ...patch } : rule));
  };
  const preview = async () => {
    setBusy("preview");
    setMessage("Calcul du plan déterministe…");
    try {
      const next = await previewMapGeneration({ jobId, spec });
      setPlan(next);
      setGeneratedArea(undefined);
      setMessage(`Plan ${next.planSha256.slice(0, 12)} prêt · ${next.metrics.placementCount} placement(s).`);
    } catch (error) {
      setMessage(normalizeAppError(error).userMessage);
    } finally {
      setBusy(undefined);
    }
  };
  const apply = async () => {
    if (!workspace || !plan) return;
    setBusy("apply");
    setMessage("Création atomique de ARE/GIT/GIC…");
    try {
      const result = await applyMapGeneration({
        jobId,
        workspaceId: workspace.workspaceId,
        spec: plan.spec,
        expectedPlanSha256: plan.planSha256,
      });
      onWorkspace(result.workspace);
      setGeneratedArea(result.area);
      setSelectedAtlasArea(result.area.resref);
      setMessage(`Carte ${result.area.resref} créée et relue depuis l’overlay. Une seule annulation retire le lot complet.`);
    } catch (error) {
      setMessage(normalizeAppError(error).userMessage);
    } finally {
      setBusy(undefined);
    }
  };
  const draftWithAi = async () => {
    setBusy("ai");
    setMessage("L’IA prépare un contrat borné ; aucune ressource NWN brute n’est transmise…");
    try {
      const result = await draftMapWithAi({
        jobId,
        currentSpec: spec,
        provider: aiProvider,
        apiKey: apiKey || undefined,
        includeBlueprintResrefs,
      });
      setSpec(result.plan.spec);
      setPlan(result.plan);
      setGeneratedArea(undefined);
      setMessage(`Plan IA reçu de ${result.model} via ${result.endpointOrigin} · ${result.sharedBlueprintCount} ResRef partagées · contrôle local réussi.`);
    } catch (error) {
      setMessage(normalizeAppError(error).userMessage);
    } finally {
      setApiKey("");
      setBusy(undefined);
    }
  };
  const changeProviderKind = (kind: Exclude<ProviderKind, "manual">) => {
    const defaults = providerDefaults[kind];
    setAiProvider((current) => ({
      ...current,
      kind,
      name: defaults.name,
      endpoint: defaults.endpoint,
      supportsStructuredOutput: kind !== "ollama",
    }));
  };
  const openAgent = () => onOpenAgent([
    "Construis entièrement une carte NWN à partir du brief ci-dessous.",
    "Utilise resource.search pour remplacer les listes de templateResrefs vides par des blueprints réellement résolus.",
    "Ajuste les densités et espacements selon le brief, puis appelle map.generate une seule fois avec un contrat valide.",
    "Conserve la graine pour rendre le résultat reproductible et termine par module.validate.",
    `Brief : ${spec.brief}`,
    `Contrat initial : ${JSON.stringify(spec)}`,
  ].join("\n"));

  return <section className="map-creator-page workspace-page" aria-label="Créateur de carte">
    <header className="workspace-page-header map-creator-header">
      <div><span className="rpg-kicker"><WandSparkles size={13}/> CONSTRUCTION DÉTERMINISTE</span><h1>Vibecoder une carte complète</h1><p>Le brief décrit l’intention. La graine, les densités et les espacements garantissent un résultat reproductible et contrôlable.</p></div>
      <span className="format-badge"><ShieldCheck size={13}/> Source NWN protégée</span>
    </header>
    <div className="map-creator-layout">
      <aside className="map-brief-panel scroll-panel">
        <div className="map-step"><span>1</span><div><strong>Décrire l’expérience</strong><small>Ambiance, circulation, lieux importants et population.</small></div></div>
        <label className="map-brief-input">Brief de la carte<textarea aria-label="Brief de la carte" value={spec.brief} onChange={(event) => patchSpec("brief", event.currentTarget.value)} placeholder="Ex. une auberge chaleureuse avec salle commune, comptoir, cuisine, chambres et accès à la cave…"/></label>
        <div className="map-identity-grid">
          <label>Nom lisible<input value={spec.name} onChange={(event) => patchSpec("name", event.currentTarget.value)}/></label>
          <label>ResRef<input maxLength={16} value={spec.resref} onChange={(event) => patchSpec("resref", normalizeResref(event.currentTarget.value))}/></label>
          <label>Tileset<input list="map-tilesets" maxLength={16} value={spec.tileset} onChange={(event) => patchSpec("tileset", normalizeResref(event.currentTarget.value))}/><datalist id="map-tilesets">{authoringQuery.data?.availableTilesets.map((tileset)=><option key={tileset} value={tileset}/>)}</datalist></label>
          <label>Tuile de base<input list="map-tile-ids" type="number" min="0" value={spec.baseTileId} onChange={(event) => patchSpec("baseTileId", Number(event.currentTarget.value))}/><datalist id="map-tile-ids">{authoringQuery.data?.selectedTileset?.tileIds.map((tileId)=><option key={tileId} value={tileId}/>)}</datalist></label>
        </div>
        <div className="map-step"><span>2</span><div><strong>Fixer les règles</strong><small>Ces valeurs restent identiques même si le plan est recalculé.</small></div></div>
        {authoringQuery.data?.selectedTileset ? <div className="map-compat-summary">
          <ShieldCheck size={14}/><div><strong>SET {authoringQuery.data.selectedTileset.resref} résolu</strong><small>{authoringQuery.data.selectedTileset.tileCount} tuiles · SHA-256 {authoringQuery.data.selectedTileset.sha256.slice(0,12)}… · limite {authoringQuery.data.limits.maxWidth}×{authoringQuery.data.limits.maxHeight}</small></div>
        </div> : authoringQuery.data ? <p className="map-context-error">SET {spec.tileset} introuvable. Choisissez un tileset proposé ou configurez les chemins NWN/HAK.</p> : authoringQuery.error ? <p className="map-context-error">{normalizeAppError(authoringQuery.error).userMessage}</p> : <p className="map-context-loading">Vérification du SET…</p>}
        <details className="map-ai-panel" open>
          <summary><WandSparkles size={14}/> Générer directement avec une IA</summary>
          <p>Pour un PC peu puissant, choisissez une API distante. La clé reste uniquement en mémoire ; seuls le brief, les limites, les identifiants SET et, si autorisé, des ResRef sont transmis.</p>
          <div className="map-ai-grid">
            <label>Fournisseur<select aria-label="Fournisseur IA de carte" value={aiProvider.kind} onChange={(event) => changeProviderKind(event.currentTarget.value as Exclude<ProviderKind,"manual">)}><option value="open_ai_responses">OpenAI Responses</option><option value="open_ai_chat_completions">OpenAI Chat Completions</option><option value="ollama">Ollama local</option><option value="compatible">API compatible</option></select></label>
            <label>Modèle<input aria-label="Modèle IA de carte" value={aiProvider.model} onChange={(event) => { const model=event.currentTarget.value; setAiProvider((current) => ({...current,model})); }} placeholder="Nom exact du modèle"/></label>
          </div>
          <label>Endpoint<input aria-label="Endpoint IA de carte" value={aiProvider.endpoint} onChange={(event) => { const endpoint=event.currentTarget.value; setAiProvider((current) => ({...current,endpoint})); }}/></label>
          <label>Clé API temporaire<input aria-label="Clé API temporaire de carte" type="password" autoComplete="off" value={apiKey} onChange={(event) => setApiKey(event.currentTarget.value)} placeholder={aiProvider.kind==="ollama"?"Généralement vide":"Jamais enregistrée"}/></label>
          <label className="map-ai-sharing"><input type="checkbox" checked={includeBlueprintResrefs} onChange={(event) => setIncludeBlueprintResrefs(event.currentTarget.checked)}/><span>Partager uniquement les noms ResRef des blueprints disponibles</span></label>
          <div className="map-ai-actions"><button type="button" className="secondary-button" onClick={openAgent}><Bot size={14}/> Agent Studio avancé</button><button type="button" className="primary-button" disabled={Boolean(busy)||!aiProvider.model.trim()||!authoringQuery.data?.selectedTileset} onClick={() => void draftWithAi()}>{busy==="ai"?<LoaderCircle className="agent-spinner" size={14}/>:<Sparkles size={14}/>} Générer le plan avec l’IA</button></div>
          <small>Les paramètres non secrets sont mémorisés sur ce PC. La carte n’est écrite qu’après votre prévisualisation et votre validation.</small>
        </details>
        <div className="map-number-grid">
          <label>Largeur<input type="number" min="1" max="32" value={spec.width} onChange={(event) => patchSpec("width", Number(event.currentTarget.value))}/></label>
          <label>Hauteur<input type="number" min="1" max="32" value={spec.height} onChange={(event) => patchSpec("height", Number(event.currentTarget.value))}/></label>
          <label>Marge<input type="number" min="0" max="31" value={spec.borderMargin} onChange={(event) => patchSpec("borderMargin", Number(event.currentTarget.value))}/></label>
          <label>Réserve libre<input type="number" min="0" max="90" value={spec.reservedPercent} onChange={(event) => patchSpec("reservedPercent", Number(event.currentTarget.value))}/><small>% des tuiles constructibles</small></label>
        </div>
        <label className="map-seed-field">Graine déterministe<div><input type="number" min="0" value={spec.seed} onChange={(event) => patchSpec("seed", Number(event.currentTarget.value))}/><button type="button" onClick={() => patchSpec("seed", seedFromBrief(spec.brief))}><RefreshCw size={13}/> Depuis le brief</button></div></label>
        <details className="map-advanced-tiles"><summary>Tuiles alternatives (avancé)</summary><label>Identifiants séparés par des virgules<input aria-label="Tuiles alternatives" value={spec.variantTileIds.join(", ")} onChange={(event) => patchSpec("variantTileIds", numberList(event.currentTarget.value))} placeholder="12, 18, 24"/></label><p>Les identifiants sont distribués de façon stable. Leur compatibilité visuelle doit être confirmée avec le SET du tileset.</p></details>
        <div className="map-step"><span>3</span><div><strong>Régler les densités</strong><small>Nombre cible pour cent tuiles, limité par l’espacement et la réserve.</small></div></div>
        <div className="map-density-list">{densityDefinitions.map((definition) => {
          const rule = spec.densities.find((value) => value.category === definition.category) as MapDensityRule;
          return <article key={definition.category} style={{"--density-color":definition.color} as React.CSSProperties}>
            <header><div><strong>{definition.label}</strong><small>{definition.hint}</small></div><b>{rule.perHundredTiles}/100</b></header>
            <input aria-label={`Densité ${definition.label}`} type="range" min="0" max="50" value={rule.perHundredTiles} onChange={(event) => patchDensity(rule.category,{perHundredTiles:Number(event.currentTarget.value)})}/>
            <div><label>Espacement<input aria-label={`Espacement ${definition.label}`} type="number" min="0" max="64" value={rule.minSpacingTiles} onChange={(event) => patchDensity(rule.category,{minSpacingTiles:Number(event.currentTarget.value)})}/></label><label>Blueprints<input aria-label={`Blueprints ${definition.label}`} value={rule.templateResrefs.join(", ")} onChange={(event) => patchDensity(rule.category,{templateResrefs:resrefList(event.currentTarget.value)})} placeholder="ResRefs séparés par des virgules"/></label></div>
          </article>;
        })}</div>
        <div className="map-vibe-actions"><button type="button" className="secondary-button" onClick={openAgent}><Bot size={14}/> Confier le brief à l’Agent</button><button type="button" className="primary-button" disabled={Boolean(busy)} onClick={() => void preview()}>{busy==="preview"?<LoaderCircle className="agent-spinner" size={14}/>:<Sparkles size={14}/>} Prévisualiser</button></div>
      </aside>

      <main className="map-preview-panel scroll-panel">
        <header><div><span className="eyebrow">PLAN DE CONSTRUCTION</span><h2>{plan?.spec.name ?? "Aucun plan calculé"}</h2><p>{message}</p></div>{plan&&<code>{plan.planSha256.slice(0,16)}…</code>}</header>
        {previewArea ? <MapAtlasGraphic area={previewArea} source={generatedArea?"ARE/GIT relus depuis l’overlay":"Prévisualisation du plan déterministe"}/> : <div className="map-preview-empty"><Map size={52}/><h3>Votre carte apparaîtra ici</h3><p>Les tuiles, zones libres et placements sont calculés avant toute écriture.</p></div>}
        {plan&&<><div className="map-plan-metrics"><Metric label="Tuiles" value={plan.metrics.totalTiles}/><Metric label="Constructibles" value={plan.metrics.buildableTiles}/><Metric label="Réservées" value={plan.metrics.reservedTiles}/><Metric label="Placements" value={plan.metrics.placementCount}/><Metric label="Occupation" value={`${plan.metrics.occupiedPercent} %`}/></div>{plan.warnings.length>0&&<div className="map-plan-warnings">{plan.warnings.map((warning)=><p key={warning}>{warning}</p>)}</div>}</>}
        <div className="map-apply-bar"><span>{workspace?`Overlay prêt · révision ${workspace.cursor}`:"Un espace d’édition est requis pour appliquer."}</span>{!workspace?<button type="button" className="primary-button" onClick={() => void onCreateWorkspace()}>Créer l’espace d’édition</button>:<button type="button" className="primary-button" disabled={!plan||Boolean(busy)||Boolean(generatedArea)} onClick={() => void apply()}>{busy==="apply"?<LoaderCircle className="agent-spinner" size={14}/>:<Check size={14}/>} Créer ARE/GIT/GIC</button>}</div>
        {plan && <div className="map-plan-compatibility" aria-label="Compatibilité Neverwinter Nights">
          <span className={plan.compatibility.tilesetResolved?"valid":"invalid"}><ShieldCheck size={12}/> SET résolu</span>
          <span className={plan.compatibility.tileIdsVerified?"valid":"invalid"}><Check size={12}/> IDs de tuiles vérifiés</span>
          <span className={plan.compatibility.edgeCompatibilityVerified?"valid":"warning"}>{plan.compatibility.edgeCompatibilityVerified?"Raccords vérifiés":spec.variantTileIds.length?"Raccords de variantes non prouvés":"Mode homogène recommandé"}</span>
          <small>Format ARE/GIT/GIC borné · ResRef ≤ 16 · {plan.spec.width*plan.spec.height}/1024 tuiles · {plan.metrics.placementCount}/2048 placements</small>
        </div>}
      </main>

      <aside className="map-atlas-panel scroll-panel">
        <header><div><span className="eyebrow">ATLAS DU MODULE</span><h2>Cartes de repérage</h2><p>Une image schématique est produite depuis les tuiles et instances réellement relues.</p></div><Image size={24}/></header>
        <label>Zone<select aria-label="Zone de l’atlas" value={atlasArea?.resref??""} onChange={(event)=>setSelectedAtlasArea(event.currentTarget.value)}>{atlasAreas.map((area)=><option key={area.resref} value={area.resref}>{area.name.text??area.resref}</option>)}</select></label>
        {atlasArea?<MapAtlasGraphic area={atlasArea} source="Ressources ARE/GIT relues" compact/>:<p>Aucune zone disponible.</p>}
      </aside>
    </div>
  </section>;
}

function MapAtlasGraphic({ area, source, compact=false }: { area: AreaMap; source: string; compact?: boolean }) {
  const svgRef = useRef<SVGSVGElement>(null);
  const cell = 24;
  const pad = 32;
  const mapWidth = Math.max(1, area.width) * cell;
  const mapHeight = Math.max(1, area.height) * cell;
  const width = mapWidth + pad * 2;
  const height = mapHeight + pad * 2 + 42;
  const exportImage = async (kind: "svg"|"png") => {
    if (!svgRef.current) return;
    const serialized = new XMLSerializer().serializeToString(svgRef.current);
    if (kind === "svg") return downloadBlob(new Blob([serialized],{type:"image/svg+xml;charset=utf-8"}),`${area.resref}-map.svg`);
    const url=URL.createObjectURL(new Blob([serialized],{type:"image/svg+xml;charset=utf-8"}));
    try { const imageElement=new globalThis.Image(); await new Promise<void>((resolve,reject)=>{imageElement.onload=()=>resolve();imageElement.onerror=()=>reject(new Error("SVG preview failed"));imageElement.src=url}); const scale=Math.min(4,Math.max(2,1600/width));const canvas=document.createElement("canvas");canvas.width=Math.round(width*scale);canvas.height=Math.round(height*scale);const context=canvas.getContext("2d");if(!context)return;context.fillStyle="#111a24";context.fillRect(0,0,canvas.width,canvas.height);context.drawImage(imageElement,0,0,canvas.width,canvas.height);const blob=await new Promise<Blob|null>((resolve)=>canvas.toBlob(resolve,"image/png"));if(blob)downloadBlob(blob,`${area.resref}-map.png`);} finally {URL.revokeObjectURL(url)}
  };
  return <figure className={`map-atlas-graphic ${compact?"compact":""}`}>
    <svg ref={svgRef} viewBox={`0 0 ${width} ${height}`} role="img" aria-label={`Carte de repérage ${area.name.text??area.resref}`} xmlns="http://www.w3.org/2000/svg">
      <rect width={width} height={height} fill="#111a24" rx="10"/>
      <text x={pad} y="21" fill="#f3e7c4" fontSize="12" fontFamily="Segoe UI, sans-serif" fontWeight="700">{area.name.text??area.resref}</text>
      <text x={width-pad} y="21" textAnchor="end" fill="#8fa6bb" fontSize="8" fontFamily="Segoe UI, sans-serif">{area.resref} · {area.tileset??"tileset ?"}</text>
      <g transform={`translate(${pad} ${pad})`}>
        {area.tiles.map((tile)=><g key={`${tile.x}:${tile.y}`} transform={`translate(${tile.x*cell} ${(area.height-1-tile.y)*cell})`}><rect width={cell-1} height={cell-1} fill={tileColor(tile.tileId)} stroke="#314253" strokeWidth="0.6"/><path d={`M${cell-6} 4 l3 3 -3 3`} fill="none" stroke="#dce7ed" strokeWidth="1" transform={`rotate(${tile.orientation*90} ${cell/2} ${cell/2})`} opacity=".58"/><text x="3" y="9" fill="#dce7ed" fontSize="5" fontFamily="Consolas, monospace" opacity=".7">{tile.tileId}</text></g>)}
        {area.instances.map((instance,index)=>{const x=(instance.x/10)*cell;const y=mapHeight-(instance.y/10)*cell;return <g key={instance.id||index} transform={`translate(${x} ${y})`}><circle r={instance.category==="creature"?4:3.4} fill={instanceColor(instance.category)} stroke="#fff4d3" strokeWidth="1"/><text x="6" y="3" fill="#fff4d3" fontSize="6" fontFamily="Segoe UI, sans-serif">{instance.tag??instance.templateResref??instance.category}</text></g>})}
        <path d={`M${mapWidth-10} 18 v-12 m0 0 -4 6 m4-6 4 6`} stroke="#fff4d3" fill="none" strokeWidth="1.5"/><text x={mapWidth-10} y="27" textAnchor="middle" fill="#fff4d3" fontSize="7">N</text>
      </g>
      <text x={pad} y={height-13} fill="#8fa6bb" fontSize="7" fontFamily="Segoe UI, sans-serif">{source} · {area.width}×{area.height} tuiles · {area.instances.length} instance(s)</text>
    </svg>
    <figcaption><span>{source}</span><div><button type="button" onClick={()=>void exportImage("svg")}><Download size={12}/> SVG</button><button type="button" onClick={()=>void exportImage("png")}><Download size={12}/> PNG</button></div></figcaption>
  </figure>;
}

function Metric({label,value}:{label:string;value:string|number}){return <span><strong>{value}</strong><small>{label}</small></span>}
function normalizeResref(value:string){return value.toLocaleLowerCase().replace(/[^a-z0-9_]/g,"").slice(0,16)}
function resrefList(value:string){return value.split(",").map(normalizeResref).filter(Boolean).slice(0,128)}
function numberList(value:string){return value.split(",").map((item)=>Number(item.trim())).filter((item)=>Number.isInteger(item)&&item>=0).slice(0,128)}
function seedFromBrief(value:string){let hash=2166136261;for(const character of value){hash^=character.codePointAt(0)??0;hash=Math.imul(hash,16777619)}return hash>>>0}
function tileColor(tileId:number){const hue=(tileId*47+195)%360;return `hsl(${hue} 25% 30%)`}
function instanceColor(category:string){return ({placeable:"#d49a52",creature:"#df6f68",door:"#9e7ad7",waypoint:"#58a7d8",sound:"#68b987",trigger:"#e5cf65",encounter:"#d75ca0",store:"#52c4be",item:"#b5c068"} as Record<string,string>)[category]??"#c8d2dc"}
function downloadBlob(blob:Blob,name:string){const link=document.createElement("a");link.href=URL.createObjectURL(blob);link.download=name;link.click();setTimeout(()=>URL.revokeObjectURL(link.href),0)}
function areaFromPlan(plan:MapGenerationPlan):AreaMap{return {resref:plan.spec.resref,name:{stringRef:null,text:plan.spec.name},width:plan.spec.width,height:plan.spec.height,tileset:plan.spec.tileset,tiles:plan.tiles,instances:plan.placements.map((placement,index)=>({id:`plan:${placement.category}:${index}`,category:placement.category,tag:placement.tag,templateResref:placement.templateResref,x:placement.x,y:placement.y,z:placement.z,bearing:placement.bearing,appearance:null,transitionDestination:null,transitionFlags:null,loadScreenId:null,geometry:[],spawnPoints:[],inventory:[],sourcePath:"map-plan"})),diagnostics:[],areSource:"map-plan",gitSource:"map-plan",gicSource:"map-plan"}}
