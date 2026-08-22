import { lazy, Suspense, useEffect, useLayoutEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { AlertTriangle, ChevronRight, Code2, LoaderCircle, MessageSquareText, Search, X } from "lucide-react";
import {
  editDialogueField,
  editDialogueStructure,
  inspectDialogue,
  normalizeAppError,
  queryDialogues,
  type DialogueGraph,
  type DialogueIndexSummary,
  type DialogueNodeRef,
  type DialogueStructureAction,
  type GenericGff,
  type GenericGffValue,
  type WorkspaceSnapshot,
} from "../../lib/tauri";

const DialogueFlow = lazy(() => import("./DialogueFlow"));
export function DialogueSummaryView({ summary }: { summary: DialogueIndexSummary }) {
  return (
    <section className="inventory-card dialogue-summary-card" aria-label="Index des dialogues">
      <div className="inventory-heading"><div><span className="eyebrow">DLG · ARBRE · GRAPHE</span><h2>Dialogues fidèles</h2></div><span className={summary.diagnostics ? "format-badge warning" : "format-badge"}>{summary.dialogues.toLocaleString("fr-FR")} DLG</span></div>
      <div className="inventory-metrics"><Metric label="Nœuds" value={summary.nodes.toLocaleString("fr-FR")} /><Metric label="Liens" value={summary.links.toLocaleString("fr-FR")} /><Metric label="Partagés" value={summary.sharedNodes.toLocaleString("fr-FR")} /><Metric label="Références" value={summary.references.toLocaleString("fr-FR")} /></div>
      <p className="structured-note">{summary.cycles.toLocaleString("fr-FR")} cycles · {summary.unreachableNodes.toLocaleString("fr-FR")} nœuds inaccessibles · {summary.brokenLinks.toLocaleString("fr-FR")} liens cassés · {summary.scriptLinks.toLocaleString("fr-FR")} liens vers des scripts.</p>
    </section>
  );
}

export function DialogueWorkspace({ jobId, summary, filter, editWorkspace, onWorkspace, onOpenScript }: { jobId: string; summary: DialogueIndexSummary; filter: string; editWorkspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void; onOpenScript: (script: string) => void }) {
  const [page, setPage] = useState(0); const [selected, setSelected] = useState<string>(); const [query,setQuery]=useState(filter); const pageSize=50;
  useEffect(()=>setQuery(filter),[filter]);
  useEffect(()=>setPage(0),[query]);
  const pageQuery=useQuery({queryKey:["dialogues",jobId,query,page],queryFn:()=>queryDialogues({jobId,query,offset:page*pageSize,limit:pageSize})});
  const items=pageQuery.data?.items??[];
  const graphQuery=useQuery({queryKey:["dialogue",jobId,selected,editWorkspace?.workspaceId,editWorkspace?.cursor],queryFn:()=>inspectDialogue({jobId,resref:selected as string,workspaceId:editWorkspace?.workspaceId}),enabled:Boolean(selected)});
  const total=pageQuery.data?.total??0; const pages=Math.max(1,Math.ceil(total/pageSize));
  return <section className="inventory-card dialogue-workspace" aria-label="Explorateur de dialogues">
    <div className="inventory-heading dialogue-workspace-heading"><div><span className="eyebrow">ATELIER DE CONVERSATION</span><h2>Dialogues</h2><p>Choisissez un dialogue, puis travaillez sur une seule ligne à la fois.</p></div><span className="format-badge">{summary.dialogues.toLocaleString("fr-FR")} DLG</span></div>
    <div className="dialogue-layout"><aside className="dialogue-list" aria-label="Liste des dialogues"><label className="dialogue-list-search"><Search size={16}/><input aria-label="Rechercher un dialogue" value={query} onChange={event=>setQuery(event.currentTarget.value)} placeholder="Nom, texte ou ResRef…"/></label><div className="dialogue-list-results">{items.map(item=><button type="button" aria-label={`Ouvrir le dialogue ${item.resref}`} key={item.resref} className={selected===item.resref?"dialogue-list-item selected":"dialogue-list-item"} onClick={()=>setSelected(item.resref)}><span><MessageSquareText size={15}/><code>{item.resref}</code></span><small>{item.nodeCount.toLocaleString("fr-FR")} lignes · {item.linkCount.toLocaleString("fr-FR")} liens{item.cycleCount?` · ${item.cycleCount} cycle(s)`:""}</small>{item.preview&&<em>{item.preview}</em>}</button>)}</div>
      {!items.length&&<p className="resource-empty">{pageQuery.isLoading?"Recherche des dialogues…":"Aucun dialogue ne correspond."}</p>}
      <div className="catalog-pagination compact"><button type="button" aria-label="Page précédente des dialogues" disabled={page===0} onClick={()=>setPage(value=>Math.max(0,value-1))}>‹</button><span>{total.toLocaleString("fr-FR")} résultat(s) · page {page+1}/{pages}</span><button type="button" aria-label="Page suivante des dialogues" disabled={page+1>=pages} onClick={()=>setPage(value=>value+1)}>›</button></div>
    </aside><DialogueGraphView jobId={jobId} resref={selected} graph={graphQuery.data} loading={graphQuery.isLoading} error={graphQuery.error} editWorkspace={editWorkspace} onWorkspace={onWorkspace} onOpenScript={onOpenScript} onClose={()=>setSelected(undefined)}/></div>
    <span className="script-total-hidden">{summary.nodes}</span>
  </section>;
}

function DialogueGraphView({ jobId, resref, graph, loading, error, editWorkspace, onWorkspace, onOpenScript, onClose }: { jobId: string; resref?: string; graph?: DialogueGraph; loading: boolean; error: unknown; editWorkspace?: WorkspaceSnapshot; onWorkspace: (workspace: WorkspaceSnapshot) => void; onOpenScript: (script: string)=>void; onClose:()=>void }) {
  const [tab,setTab]=useState<"lines"|"graph"|"raw">("lines"); const [selectedNode,setSelectedNode]=useState<string>();
  const [editedGraph,setEditedGraph]=useState<DialogueGraph>();
  const [structureBusy,setStructureBusy]=useState(false);const [structureMessage,setStructureMessage]=useState("");
  useEffect(()=>{setEditedGraph(undefined);setSelectedNode(undefined);setStructureMessage("");setTab("lines")},[resref]);
  useEffect(()=>{if(graph?.key.resref===resref)setEditedGraph(graph)},[graph,resref]);
  const currentGraph=editedGraph??graph;
  useEffect(()=>{if(currentGraph&&!currentGraph.nodes.some(node=>node.id===selectedNode))setSelectedNode(currentGraph.roots[0]??currentGraph.nodes[0]?.id)},[currentGraph,selectedNode]);
  if(!resref)return <div className="dialogue-empty dialogue-welcome"><MessageSquareText size={44}/><div><h2>Choisissez un dialogue</h2><p>Rien n’est chargé tant que vous n’avez pas sélectionné un DLG dans la liste.</p><span>La recherche et la pagination restent disponibles à gauche.</span></div></div>;
  if(loading&&!currentGraph)return <div className="dialogue-empty dialogue-welcome"><LoaderCircle className="agent-spinner" size={28}/><div><h2>Ouverture de {resref}</h2><p>Lecture de sa structure de conversation…</p></div></div>;
  if(error&&!currentGraph)return <div className="dialogue-empty dialogue-welcome warning"><AlertTriangle size={32}/><div><h2>Dialogue impossible à ouvrir</h2><p>{normalizeAppError(error).technicalMessage}</p><button type="button" onClick={onClose}>Revenir à la liste</button></div></div>;
  if(!currentGraph)return <div className="dialogue-empty">Sélectionnez un dialogue.</div>;
  const node=currentGraph.nodes.find(value=>value.id===selectedNode);
  const commitField=async(path:string,before:GenericGffValue,after:GenericGffValue)=>{
    if(!editWorkspace)return;
    const result=await editDialogueField({jobId,workspaceId:editWorkspace.workspaceId,resref:currentGraph.key.resref,path,before,after});
    onWorkspace(result.workspace); setEditedGraph(result.graph);
  };
  const commitStructure=async(action:DialogueStructureAction)=>{
    if(!editWorkspace)return;
    setStructureBusy(true);setStructureMessage("Mise à jour de la structure…");
    try{const result=await editDialogueStructure({jobId,workspaceId:editWorkspace.workspaceId,resref:currentGraph.key.resref,action});onWorkspace(result.workspace);setEditedGraph(result.graph);if(action.kind==="add_node"){const added=result.graph.nodes.filter(value=>value.kind===action.nodeKind).at(-1);if(added)setSelectedNode(added.id)}setStructureMessage("Structure enregistrée dans l'overlay.")}catch(error){setStructureMessage(normalizeAppError(error).technicalMessage);throw error}finally{setStructureBusy(false)}
  };
  const connectNodes=async(sourceId:string,targetId:string)=>{const source=dialogueNodeRef(sourceId);const target=dialogueNodeRef(targetId);if(!source||!target)throw new Error("Nœud DLG invalide");await commitStructure({kind:"add_link",source,target})};
  return <div className="dialogue-document"><div className="script-tabs dialogue-tabs"><button type="button" className={tab==="lines"?"active":""} onClick={()=>setTab("lines")}>Lignes</button><button type="button" className={tab==="graph"?"active":""} onClick={()=>setTab("graph")}>Graphe (avancé)</button><button type="button" className={tab==="raw"?"active":""} onClick={()=>setTab("raw")}>GFF (avancé)</button><strong>{currentGraph.key.resref}</strong><button type="button" className="dialogue-close" aria-label="Fermer le dialogue" title="Fermer le dialogue" onClick={onClose}><X size={16}/></button></div>
    {tab==="lines"?<DialogueLinesEditor graph={currentGraph} selectedId={selectedNode} onSelect={setSelectedNode} editWorkspace={editWorkspace} busy={structureBusy} message={structureMessage} onCommitField={commitField} onCommitStructure={commitStructure} onOpenScript={onOpenScript}/>:tab==="graph"?<div className="dialogue-editor-surface"><div className="dialogue-content"><Suspense fallback={<div className="dialogue-empty">Chargement du graphe…</div>}><DialogueFlow graph={currentGraph} selectedId={selectedNode} onSelect={setSelectedNode} onConnect={editWorkspace?connectNodes:undefined}/></Suspense></div><DialogueInspector graph={currentGraph} node={node} editWorkspace={editWorkspace} onCommitField={commitField} onCommitStructure={commitStructure} onOpenScript={onOpenScript}/></div>:<pre className="dialogue-raw">{JSON.stringify(currentGraph.raw,null,2)}</pre>}
  </div>;
}

const dialogueNodePageSize=60;

function DialogueLinesEditor({graph,selectedId,onSelect,editWorkspace,busy,message,onCommitField,onCommitStructure,onOpenScript}:{graph:DialogueGraph;selectedId?:string;onSelect:(nodeId:string)=>void;editWorkspace?:WorkspaceSnapshot;busy:boolean;message:string;onCommitField:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>;onCommitStructure:(action:DialogueStructureAction)=>Promise<void>;onOpenScript:(script:string)=>void}) {
  const [query,setQuery]=useState("");const [page,setPage]=useState(0);
  const normalized=query.trim().toLocaleLowerCase();
  const nodeById=useMemo(()=>new globalThis.Map(graph.nodes.map(node=>[node.id,node])),[graph.nodes]);
  const outgoingBySource=useMemo(()=>{const result=new globalThis.Map<string,DialogueGraph["links"]>();for(const link of graph.links){if(link.source===null)continue;const values=result.get(link.source)??[];values.push(link);result.set(link.source,values)}return result},[graph.links]);
  const incomingByTarget=useMemo(()=>{const result=new globalThis.Map<string,DialogueGraph["links"]>();for(const link of graph.links){const values=result.get(link.target)??[];values.push(link);result.set(link.target,values)}return result},[graph.links]);
  const nodes=useMemo(()=>graph.nodes.filter(node=>!normalized||[node.id,node.displayText,node.speaker,node.comment,node.actionScript,node.quest].some(value=>value?.toLocaleLowerCase().includes(normalized))),[graph.nodes,normalized]);
  useEffect(()=>setPage(0),[normalized]);
  const pages=Math.max(1,Math.ceil(nodes.length/dialogueNodePageSize));
  useEffect(()=>setPage(value=>Math.min(value,pages-1)),[pages]);
  useEffect(()=>{if(!selectedId)return;const position=nodes.findIndex(node=>node.id===selectedId);if(position>=0)setPage(Math.floor(position/dialogueNodePageSize))},[nodes,selectedId]);
  const visibleNodes=nodes.slice(page*dialogueNodePageSize,(page+1)*dialogueNodePageSize);
  const selectedNode=selectedId?nodeById.get(selectedId):undefined;
  const starts=graph.links.filter(link=>link.source===null);
  return <div className="dialogue-lines-workspace">
    <aside className="dialogue-line-browser" aria-label="Lignes du dialogue">
      <header><div><span>PARCOURS</span><strong>{graph.nodes.length.toLocaleString("fr-FR")} lignes</strong></div><small>{graph.links.length.toLocaleString("fr-FR")} liens</small></header>
      <label className="dialogue-line-search"><Search size={15}/><input aria-label="Rechercher une ligne" placeholder="Texte, locuteur, script ou quête…" value={query} onChange={event=>setQuery(event.currentTarget.value)}/></label>
      <section className="dialogue-starts" aria-label="Départs du dialogue"><div><strong>Début du dialogue</strong><span>{starts.length?`${starts.length} départ(s)`:"Aucun départ"}</span></div><div className="dialogue-root-list">{starts.slice(0,12).map(link=>{const target=nodeById.get(link.target);return <button type="button" key={link.id} className={selectedId===link.target?"selected":""} onClick={()=>onSelect(link.target)}><ChevronRight size={14}/><span>{target?.displayText??link.target}</span></button>})}{starts.length>12&&<small>+ {starts.length-12} autres départs</small>}</div>{editWorkspace&&<DialogueAddLinkEditor graph={graph} source={null} label="Ajouter une ligne de départ" onAdd={onCommitStructure}/>}</section>
      <nav className="dialogue-node-list" aria-label="Résultats des lignes">{visibleNodes.map(node=><button type="button" key={node.id} aria-label={`Ouvrir la ligne ${node.id}`} className={selectedId===node.id?`selected ${node.kind}`:node.kind} onClick={()=>onSelect(node.id)}><span>{node.kind==="entry"?"PNJ":"Joueur"}</span><strong>{node.displayText??"Texte non résolu"}</strong><small>{node.speaker?`${node.speaker} · `:""}{node.id}</small></button>)}{nodes.length===0&&<p>Aucune ligne ne correspond à cette recherche.</p>}</nav>
      <div className="catalog-pagination compact"><button type="button" aria-label="Page précédente des lignes" disabled={page===0} onClick={()=>setPage(value=>Math.max(0,value-1))}>‹</button><span>{nodes.length.toLocaleString("fr-FR")} résultat(s) · {page+1}/{pages}</span><button type="button" aria-label="Page suivante des lignes" disabled={page+1>=pages} onClick={()=>setPage(value=>value+1)}>›</button></div>
    </aside>
    <main className="dialogue-line-focus"><header className="dialogue-lines-toolbar"><div><span>ÉDITION DE LA CONVERSATION</span><strong>{selectedNode?selectedNode.kind==="entry"?"Réplique du PNJ":"Réponse du joueur":"Sélectionnez une ligne"}</strong></div>{editWorkspace?<div><button type="button" disabled={busy} onClick={()=>void onCommitStructure({kind:"add_node",nodeKind:"entry"}).catch(()=>undefined)}>+ Réplique PNJ</button><button type="button" disabled={busy} onClick={()=>void onCommitStructure({kind:"add_node",nodeKind:"reply"}).catch(()=>undefined)}>+ Réponse joueur</button></div>:<small>Lecture seule · créez un espace d’édition pour modifier.</small>}</header>
      {message&&<p className="dialogue-lines-status" role="status">{message}</p>}
      {selectedNode?<DialogueLineCard graph={graph} node={selectedNode} nodeById={nodeById} outgoing={outgoingBySource.get(selectedNode.id)??[]} incoming={incomingByTarget.get(selectedNode.id)??[]} editable={Boolean(editWorkspace)} onSelect={onSelect} onCommitField={onCommitField} onCommitStructure={onCommitStructure} onOpenScript={onOpenScript}/>:<div className="dialogue-empty dialogue-focus-empty"><MessageSquareText size={36}/><p>Choisissez une ligne à gauche pour lire son texte et parcourir ses réponses.</p></div>}
    </main>
  </div>;
}

function DialogueLineCard({graph,node,nodeById,outgoing,incoming,editable,onSelect,onCommitField,onCommitStructure,onOpenScript}:{graph:DialogueGraph;node:DialogueGraph["nodes"][number];nodeById:ReadonlyMap<string,DialogueGraph["nodes"][number]>;outgoing:DialogueGraph["links"];incoming:DialogueGraph["links"];editable:boolean;onSelect:(nodeId:string)=>void;onCommitField:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>;onCommitStructure:(action:DialogueStructureAction)=>Promise<void>;onOpenScript:(script:string)=>void}) {
  const nodeRef={kind:node.kind,index:node.index} satisfies DialogueNodeRef;
  const fields=dialogueNodeEditableFields(graph,node.id);const textField=fields.find(field=>field.path.endsWith("/Text"));const details=fields.filter(field=>field!==textField);
  const startLinks=incoming.filter(link=>link.source===null);
  const [confirmDelete,setConfirmDelete]=useState(false);useEffect(()=>setConfirmDelete(false),[node.id]);
  return <article className={`dialogue-line-card ${node.kind}`} aria-label={`Ligne ${node.id}`}>
    <header><div><span>{node.kind==="entry"?"PNJ":"Joueur"}</span><code>{node.id}</code>{node.speaker&&<strong>{node.speaker}</strong>}</div>{editable&&(confirmDelete?<div className="dialogue-delete-confirm"><span>Supprimer cette ligne et ses liens ?</span><button type="button" onClick={()=>setConfirmDelete(false)}>Annuler</button><button type="button" className="danger-button" onClick={()=>void onCommitStructure({kind:"remove_node",node:nodeRef}).catch(()=>undefined)}>Confirmer</button></div>:<button type="button" className="danger-button" onClick={()=>setConfirmDelete(true)}>Supprimer la ligne</button>)}</header>
    {incoming.length>0&&<section className="dialogue-line-origin"><strong>{incoming.some(link=>link.source===null)?"Point de départ":"Arrive depuis"}</strong><div>{incoming.slice(0,20).map(link=>{const source=link.source?nodeById.get(link.source):undefined;return <button type="button" key={link.id} disabled={!link.source} onClick={()=>link.source&&onSelect(link.source)}><ChevronRight size={13}/><span>{link.source?source?.displayText??link.source:"Début du dialogue"}</span></button>})}{incoming.length>20&&<small>+ {incoming.length-20} autres liens entrants</small>}</div></section>}
    {startLinks.length>0&&<DialogueStartRules links={startLinks} editable={editable} onCommitStructure={onCommitStructure} onOpenScript={onOpenScript}/>}
    <div className="dialogue-line-text">{editable&&textField?<EditableDialogueField field={{...textField,label:"Texte de la ligne"}} onCommit={onCommitField}/>:<p>{node.displayText??"Texte non résolu"}</p>}</div>
    <div className="dialogue-line-links"><strong>{node.kind==="entry"?"Réponses proposées":"Suite du dialogue"}</strong>{outgoing.map(link=><DialogueLineLink key={link.id} link={link} target={nodeById.get(link.target)} editable={editable} onSelect={onSelect} onCommitStructure={onCommitStructure} onOpenScript={onOpenScript}/>)}{outgoing.length===0&&<span>Aucune ligne associée.</span>}{editable&&<DialogueAddLinkEditor graph={graph} source={nodeRef} label={node.kind==="entry"?"Associer une réponse joueur":"Associer la prochaine réplique PNJ"} onAdd={onCommitStructure}/>}</div>
    {(details.length>0||node.actionScript||node.comment||node.quest)&&<details className="dialogue-line-details"><summary>Autres réglages de la ligne</summary>{editable&&details.map(field=><EditableDialogueField key={field.path} field={field} onCommit={onCommitField}/>)}{node.actionScript&&<button type="button" onClick={()=>onOpenScript(node.actionScript as string)}><Code2 size={13}/> Ouvrir l’action {node.actionScript}</button>}{node.comment&&<span>Commentaire · {node.comment}</span>}{node.quest&&<span>Quête · {node.quest}</span>}</details>}
  </article>;
}

function DialogueStartRules({links,editable,onCommitStructure,onOpenScript}:{links:DialogueGraph["links"];editable:boolean;onCommitStructure:(action:DialogueStructureAction)=>Promise<void>;onOpenScript:(script:string)=>void}) {
  return <section className="dialogue-start-rules"><strong>Conditions d’entrée</strong>{links.map(link=>{const position=Number(link.id.split(":").at(-1));return <div className="dialogue-start-rule" key={`${link.id}:rules`}><span>Départ {position+1}</span>{editable?<DialogueLinkTriggerEditor link={link} source={null} position={position} conditionLabel="Condition d’entrée" actionLabel="Action au démarrage" onCommit={onCommitStructure}/>:<div className="dialogue-link-scripts">{link.conditionScript&&<button type="button" onClick={()=>onOpenScript(link.conditionScript as string)}>Condition · {link.conditionScript}</button>}{link.actionScript&&<button type="button" onClick={()=>onOpenScript(link.actionScript as string)}>Action · {link.actionScript}</button>}{!link.conditionScript&&!link.actionScript&&<span>Toujours disponible</span>}</div>}{editable&&<button type="button" className="danger-button compact" onClick={()=>void onCommitStructure({kind:"remove_link",source:null,position}).catch(()=>undefined)}>Retirer ce départ</button>}</div>})}</section>;
}

function DialogueLineLink({link,target,editable,onSelect,onCommitStructure,onOpenScript}:{link:DialogueGraph["links"][number];target?:DialogueGraph["nodes"][number];editable:boolean;onSelect:(nodeId:string)=>void;onCommitStructure:(action:DialogueStructureAction)=>Promise<void>;onOpenScript:(script:string)=>void}) {
  const source=dialogueNodeRefFromId(link.source);
  const position=Number(link.id.split(":").at(-1));
  return <div className="dialogue-line-link"><button type="button" className="dialogue-line-link-target" onClick={()=>onSelect(link.target)}><span>→</span><strong>{target?.displayText??link.target}</strong><code>{link.target}</code>{link.broken&&<em>Lien cassé</em>}</button>{editable?<DialogueLinkTriggerEditor link={link} source={source} position={position} onCommit={onCommitStructure}/>:<div className="dialogue-link-scripts">{link.conditionScript&&<button type="button" onClick={()=>onOpenScript(link.conditionScript as string)}>Déclencheur · {link.conditionScript}</button>}{link.actionScript&&<button type="button" onClick={()=>onOpenScript(link.actionScript as string)}>Action · {link.actionScript}</button>}{!link.conditionScript&&!link.actionScript&&<span>Toujours disponible</span>}</div>}{editable&&<button type="button" className="danger-button compact" onClick={()=>void onCommitStructure({kind:"remove_link",source,position}).catch(()=>undefined)}>Dissocier</button>}</div>;
}

function DialogueLinkTriggerEditor({link,source,position,conditionLabel="Déclencheur (condition)",actionLabel="Action après la ligne",onCommit}:{link:DialogueGraph["links"][number];source:DialogueNodeRef|null;position:number;conditionLabel?:string;actionLabel?:string;onCommit:(action:DialogueStructureAction)=>Promise<void>}) {
  const condition=link.conditionScript??"";const action=link.actionScript??"";
  const [conditionDraft,setConditionDraft]=useState(condition);const [actionDraft,setActionDraft]=useState(action);const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{setConditionDraft(condition);setActionDraft(action);setMessage("")},[condition,action,link.id]);
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit({kind:"set_link_scripts",source,position,conditionScript:conditionDraft.trim()||null,actionScript:actionDraft.trim()||null});setMessage("Déclencheurs enregistrés.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="dialogue-trigger-editor"><label><span>{conditionLabel}</span><input aria-label={`Déclencheur de ${link.id}`} placeholder="script_condition" maxLength={16} value={conditionDraft} onChange={event=>setConditionDraft(event.currentTarget.value.toLocaleLowerCase())}/></label><label><span>{actionLabel}</span><input aria-label={`Action de ${link.id}`} placeholder="script_action" maxLength={16} value={actionDraft} onChange={event=>setActionDraft(event.currentTarget.value.toLocaleLowerCase())}/></label><button type="button" disabled={busy||(conditionDraft===condition&&actionDraft===action)} onClick={()=>void commit()}>{busy?"…":"Enregistrer"}</button>{message&&<small>{message}</small>}</div>;
}

function dialogueNodeRef(id:string):DialogueNodeRef|undefined {const [kind,indexText]=id.split(":");const index=Number(indexText);return (kind==="entry"||kind==="reply")&&Number.isInteger(index)&&index>=0?{kind,index}:undefined}

function DialogueInspector({ graph, node, editWorkspace, onCommitField, onCommitStructure, onOpenScript }: { graph: DialogueGraph; node?: DialogueGraph["nodes"][number]; editWorkspace?: WorkspaceSnapshot; onCommitField:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>; onCommitStructure:(action:DialogueStructureAction)=>Promise<void>; onOpenScript:(script:string)=>void }) {
  const nodeRef=node?{kind:node.kind,index:node.index} satisfies DialogueNodeRef:undefined;
  return <div className="dialogue-inspection"><div><h3>Nœud sélectionné</h3>{node?<><strong>{node.id}</strong><p>{node.displayText??"Texte non résolu"}</p>{node.speaker&&<span>Locuteur · {node.speaker}</span>}{node.comment&&<span>Commentaire · {node.comment}</span>}{node.animation!==null&&<span>Animation · {node.animation}{node.animationLoop?" · boucle":""}</span>}{node.sound&&<span>Son · {node.sound}</span>}{node.quest&&<span>Quête · {node.quest}</span>}{node.actionScript&&<button type="button" onClick={()=>onOpenScript(node.actionScript as string)}><Code2 size={12}/> Action · {node.actionScript}</button>}{editWorkspace&&<DialogueNodeFieldEditor graph={graph} nodeId={node.id} onCommit={onCommitField}/>} {editWorkspace&&nodeRef&&<><DialogueAddLinkEditor graph={graph} source={nodeRef} label="Ajouter un lien sortant" onAdd={onCommitStructure}/><button type="button" className="danger-button" onClick={()=>void onCommitStructure({kind:"remove_node",node:nodeRef}).catch(()=>undefined)}>Supprimer ce nœud</button></>}</>:<span>Aucun nœud.</span>}</div>
    <div><h3>Scripts et cibles des liens</h3>{graph.links.filter(link=>link.source===node?.id).map(link=><div className="dialogue-link-meta" key={link.id}><span>→ {link.target}{link.isChild?" · partagé":""}</span>{link.conditionScript&&<button type="button" onClick={()=>onOpenScript(link.conditionScript as string)}><Code2 size={12}/> Condition · {link.conditionScript}</button>}{link.actionScript&&<button type="button" onClick={()=>onOpenScript(link.actionScript as string)}><Code2 size={12}/> Action · {link.actionScript}</button>}{editWorkspace&&<DialogueLinkFieldEditor graph={graph} link={link} onCommit={onCommitField} onRemove={onCommitStructure}/>}</div>)}{editWorkspace&&graph.links.filter(link=>link.source===null).map(link=><div className="dialogue-link-meta" key={link.id}><span>Départ → {link.target}</span><DialogueLinkFieldEditor graph={graph} link={link} onCommit={onCommitField} onRemove={onCommitStructure}/></div>)}</div>
    <div><h3>Références entrantes</h3>{graph.references.slice(0,100).map(value=><span key={`${value.resource.resref}-${value.fieldPath}`}>{value.resource.resref}.#{value.resource.resourceType} · {value.fieldPath}</span>)}{graph.references.length===0&&<span>Aucune référence GFF détectée.</span>}</div>
    {graph.diagnostics.length>0&&<div><h3>Diagnostics</h3>{graph.diagnostics.slice(0,50).map((value,index)=><span className="missing" key={`${value.code}-${index}`}>{value.code} · {value.message}</span>)}</div>}
  </div>;
}

function DialogueNodeFieldEditor({ graph, nodeId, onCommit }: { graph: DialogueGraph; nodeId: string; onCommit: (path: string, before: GenericGffValue, after: GenericGffValue) => Promise<void> }) {
  const fields = dialogueNodeEditableFields(graph, nodeId);
  if (!fields.length) return <small>Aucun champ texte existant n'est éditable sur ce nœud.</small>;
  return <div className="gff-field-editor dialogue-node-editor"><strong>Édition DLG transactionnelle</strong>{fields.map((field)=><EditableDialogueField key={field.path} field={field} onCommit={onCommit}/>)}</div>;
}

function dialogueNodeEditableFields(graph: DialogueGraph, nodeId: string): Array<{ label: string; path: string; value: GenericGffValue }> {
  const [kind,indexText]=nodeId.split(":");
  const index=Number(indexText);
  if(!Number.isInteger(index)||index<0)return [];
  const raw=graph.raw as GenericGff;
  const listCandidates=kind==="entry"?["EntryList","EntriesList"]:kind==="reply"?["ReplyList","RepliesList"]:[];
  const listField=raw?.root?.fields?.find((field)=>listCandidates.includes(field.label));
  if(!listField||listField.value.kind!=="list"||!Array.isArray(listField.value.value))return [];
  const child=(listField.value.value as GenericGff["root"][])[index];
  if(!child)return [];
  const labels:Record<string,string>={Text:"Texte localisé",Speaker:"Locuteur",Comment:"Commentaire",Script:"Script d'action",ActionScript:"Script d'action"};
  return child.fields.filter((field)=>Object.hasOwn(labels,field.label)&&["string","res_ref","localized_string"].includes(field.value.kind)).map((field)=>({label:labels[field.label],path:`/${listField.label}/${index}/${field.label}`,value:field.value}));
}

export function EditableDialogueField({ field, onCommit }: { field: { label: string; path: string; value: GenericGffValue }; onCommit: (path: string, before: GenericGffValue, after: GenericGffValue) => Promise<void> }) {
  if(field.value.kind==="localized_string")return <EditableLocalizedDialogueField field={field} onCommit={onCommit}/>;
  return <EditableScalarDialogueField field={field} onCommit={onCommit}/>;
}

function EditableScalarDialogueField({ field, onCommit }: { field: { label: string; path: string; value: GenericGffValue }; onCommit: (path: string, before: GenericGffValue, after: GenericGffValue) => Promise<void> }) {
  const original=String(field.value.value??""); const [draft,setDraft]=useState(original); const [busy,setBusy]=useState(false); const [message,setMessage]=useState("");
  useEffect(()=>{setDraft(original);setMessage("")},[original,field.path]);
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(field.path,field.value,{kind:field.value.kind,value:draft});setMessage("Enregistré dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <label className="gff-field-row"><span>{field.label}</span><input value={draft} onChange={(event)=>setDraft(event.currentTarget.value)}/><button type="button" disabled={busy||draft===original} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button>{message&&<small>{message}</small>}</label>;
}

type DialogueLocalizedValue={languageId:number;text:string};
type DialogueLocalizedString={stringRef:number|null;values:DialogueLocalizedValue[]};

function asDialogueLocalizedString(value:unknown):DialogueLocalizedString {
  if(!value||typeof value!=="object")return {stringRef:null,values:[]};
  const candidate=value as {stringRef?:unknown;values?:unknown};
  const stringRef=typeof candidate.stringRef==="number"?candidate.stringRef:null;
  const values=Array.isArray(candidate.values)?candidate.values.flatMap((entry)=>{
    if(!entry||typeof entry!=="object")return [];
    const item=entry as {languageId?:unknown;text?:unknown};
    return typeof item.languageId==="number"&&typeof item.text==="string"?[{languageId:item.languageId,text:item.text}]:[];
  }):[];
  return {stringRef,values};
}

export function EditableLocalizedDialogueField({field,onCommit}:{field:{label:string;path:string;value:GenericGffValue};onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>}) {
  const original=asDialogueLocalizedString(field.value.value);
  const originalJson=JSON.stringify(original);
  const [draft,setDraft]=useState<DialogueLocalizedString>(original);
  const [busy,setBusy]=useState(false); const [message,setMessage]=useState("");
  useLayoutEffect(()=>{setDraft(original);setMessage("")},[originalJson,field.path]);
  const update=(index:number,text:string)=>setDraft(value=>({...value,values:value.values.map((entry,position)=>position===index?{...entry,text}:entry)}));
  const addVariant=()=>setDraft(value=>{const used=new Set(value.values.map(entry=>entry.languageId));let languageId=0;while(used.has(languageId))languageId+=2;return {...value,values:[...value.values,{languageId,text:""}]}});
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(field.path,field.value,{kind:"localized_string",value:draft});setMessage("Variantes localisées enregistrées dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="gff-field-row localized-dialogue-field"><span>{field.label}{draft.stringRef!==null?` · StrRef ${draft.stringRef}`:""}</span>{draft.values.map((entry,index)=><label key={`${entry.languageId}-${index}`}><small>Langue/genre {entry.languageId}</small><textarea value={entry.text} onChange={(event)=>update(index,event.currentTarget.value)}/></label>)}<div><button type="button" disabled={busy} onClick={addVariant}>Ajouter une variante</button><button type="button" disabled={busy||JSON.stringify(draft)===originalJson} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button></div>{message&&<small>{message}</small>}</div>;
}

const dialogueTargetResultLimit=40;

function dialogueTargetMatches(nodes:DialogueGraph["nodes"],targetKind:"entry"|"reply",query:string){
  const normalized=query.trim().toLocaleLowerCase();let total=0;const items:DialogueGraph["nodes"]=[];
  for(const node of nodes){if(node.kind!==targetKind)continue;if(normalized&&![node.id,node.displayText,node.speaker,node.comment].some(value=>value?.toLocaleLowerCase().includes(normalized)))continue;total+=1;if(items.length<dialogueTargetResultLimit)items.push(node)}
  return {items,total};
}

function DialogueAddLinkEditor({graph,source,label,onAdd}:{graph:DialogueGraph;source:DialogueNodeRef|null;label:string;onAdd:(action:DialogueStructureAction)=>Promise<void>}) {
  const targetKind=source?.kind==="entry"?"reply":"entry";const sourceKey=source?`${source.kind}:${source.index}`:"start";
  const [open,setOpen]=useState(false);const [query,setQuery]=useState("");const [targetId,setTargetId]=useState<string>();const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{setOpen(false);setQuery("");setTargetId(undefined);setMessage("")},[sourceKey]);
  const matches=useMemo(()=>open?dialogueTargetMatches(graph.nodes,targetKind,query):{items:[],total:0},[graph.nodes,open,query,targetKind]);
  const target=targetId?graph.nodes.find(node=>node.id===targetId):undefined;
  const add=async()=>{if(!target)return;setBusy(true);setMessage("Ajout…");try{await onAdd({kind:"add_link",source,target:{kind:target.kind,index:target.index}});setOpen(false);setQuery("");setTargetId(undefined);setMessage("Lien ajouté dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
  return <div className="dialogue-add-link"><button type="button" className="dialogue-add-link-toggle" aria-expanded={open} onClick={()=>{setOpen(value=>!value);setMessage("")}}>{open?"Annuler":`+ ${label}`}</button>{open&&<div className="dialogue-target-picker"><header><strong>{label}</strong><span>{targetKind==="entry"?"Réplique PNJ":"Réponse joueur"}</span></header><label><Search size={14}/><input aria-label={`Rechercher une cible pour ${label}`} autoFocus value={query} onChange={event=>{setQuery(event.currentTarget.value);setTargetId(undefined)}} placeholder="Rechercher le texte ou l’identifiant…"/></label><div className="dialogue-target-results">{matches.items.map(node=><button type="button" key={node.id} className={targetId===node.id?"selected":""} onClick={()=>setTargetId(node.id)}><strong>{node.displayText??"Texte non résolu"}</strong><small>{node.speaker?`${node.speaker} · `:""}{node.id}</small></button>)}{matches.total===0&&<span>Aucune ligne compatible.</span>}</div>{matches.total>matches.items.length&&<small>{matches.items.length} sur {matches.total} lignes affichées · précisez la recherche.</small>}<footer><span>{target?target.displayText??target.id:"Choisissez une ligne."}</span><button type="button" disabled={!target||busy} onClick={()=>void add()}>{busy?"Ajout…":"Associer cette ligne"}</button></footer></div>}{message&&<small role="status">{message}</small>}</div>;
}

function DialogueLinkFieldEditor({graph,link,onCommit,onRemove}:{graph:DialogueGraph;link:DialogueGraph["links"][number];onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>;onRemove:(action:DialogueStructureAction)=>Promise<void>}) {
  const context=dialogueLinkEditContext(graph,link);
  if(!context)return <small>Structure brute du lien introuvable.</small>;
  const indexField=context.structure.fields.find(field=>field.label==="Index"&&["byte","word","dword"].includes(field.value.kind));
  const textLabels:Record<string,string>={Active:"Condition",Conditional:"Condition",Script:"Action",ActionScript:"Action",LinkComment:"Commentaire",Comment:"Commentaire"};
  const textFields=context.structure.fields.filter(field=>Object.hasOwn(textLabels,field.label)&&["string","res_ref"].includes(field.value.kind));
  const source=dialogueNodeRefFromId(link.source);const position=Number(link.id.split(":").at(-1));
  return <div className="gff-field-editor dialogue-link-editor"><strong>Édition du lien</strong>{indexField&&<EditableDialogueTargetField graph={graph} path={`${context.path}/Index`} value={indexField.value} targetKind={context.targetKind} onCommit={onCommit}/>} {textFields.map(field=><EditableDialogueField key={field.label} field={{label:textLabels[field.label],path:`${context.path}/${field.label}`,value:field.value}} onCommit={onCommit}/>)}<button type="button" className="danger-button" onClick={()=>void onRemove({kind:"remove_link",source,position}).catch(()=>undefined)}>Supprimer ce lien</button></div>;
}

function dialogueNodeRefFromId(id:string|null):DialogueNodeRef|null {
  if(id===null)return null;const [kind,indexText]=id.split(":");const index=Number(indexText);return (kind==="entry"||kind==="reply")&&Number.isInteger(index)&&index>=0?{kind,index}:null;
}

function dialogueLinkEditContext(graph:DialogueGraph,link:DialogueGraph["links"][number]):{path:string;structure:GenericGff["root"];targetKind:"entry"|"reply"}|undefined {
  const raw=graph.raw as GenericGff;
  const position=Number(link.id.split(":").at(-1));
  if(!Number.isInteger(position)||position<0)return undefined;
  if(link.source===null){const list=raw.root.fields.find(field=>field.label==="StartingList"&&field.value.kind==="list");const structure=(list?.value.value as GenericGff["root"][]|undefined)?.[position];return list&&structure?{path:`/${list.label}/${position}`,structure,targetKind:"entry"}:undefined;}
  const [sourceKind,indexText]=link.source.split(":");const sourceIndex=Number(indexText);
  if(!Number.isInteger(sourceIndex)||sourceIndex<0||!(["entry","reply"].includes(sourceKind)))return undefined;
  const nodeCandidates=sourceKind==="entry"?["EntryList","EntriesList"]:["ReplyList","RepliesList"];
  const linkCandidates=sourceKind==="entry"?["RepliesList","ReplyList"]:["EntriesList","EntryList"];
  const nodeList=raw.root.fields.find(field=>nodeCandidates.includes(field.label)&&field.value.kind==="list");
  const node=(nodeList?.value.value as GenericGff["root"][]|undefined)?.[sourceIndex];
  const linkList=node?.fields.find(field=>linkCandidates.includes(field.label)&&field.value.kind==="list");
  const structure=(linkList?.value.value as GenericGff["root"][]|undefined)?.[position];
  return nodeList&&linkList&&structure?{path:`/${nodeList.label}/${sourceIndex}/${linkList.label}/${position}`,structure,targetKind:sourceKind==="entry"?"reply":"entry"}:undefined;
}

function EditableDialogueTargetField({graph,path,value,targetKind,onCommit}:{graph:DialogueGraph;path:string;value:GenericGffValue;targetKind:"entry"|"reply";onCommit:(path:string,before:GenericGffValue,after:GenericGffValue)=>Promise<void>}) {
  const original=Number(value.value);const [draft,setDraft]=useState(original);const [open,setOpen]=useState(false);const [query,setQuery]=useState("");const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");
  useEffect(()=>{setDraft(original);setOpen(false);setQuery("");setMessage("")},[original,path]);
  const matches=useMemo(()=>open?dialogueTargetMatches(graph.nodes,targetKind,query):{items:[],total:0},[graph.nodes,open,query,targetKind]);
  const current=graph.nodes.find(node=>node.kind===targetKind&&node.index===draft);
  const commit=async()=>{setBusy(true);setMessage("Enregistrement…");try{await onCommit(path,value,{kind:value.kind,value:draft});setMessage("Cible enregistrée dans l'overlay.")}catch(error){setMessage(normalizeAppError(error).technicalMessage)}finally{setBusy(false)}};
   return <div className="gff-field-row dialogue-target-field"><span>Cible</span><button type="button" onClick={()=>setOpen(value=>!value)}>{current?.displayText??current?.id??`Index ${draft}`}</button><button type="button" disabled={busy||draft===original} onClick={()=>void commit()}>{busy?"…":"Appliquer"}</button>{open&&<div className="dialogue-target-picker compact"><label><Search size={13}/><input aria-label="Rechercher une nouvelle cible" autoFocus value={query} onChange={event=>setQuery(event.currentTarget.value)} placeholder="Texte ou identifiant…"/></label><div className="dialogue-target-results">{matches.items.map(node=><button type="button" key={node.id} className={draft===node.index?"selected":""} onClick={()=>{setDraft(node.index);setOpen(false)}}><strong>{node.displayText??"Texte non résolu"}</strong><small>{node.id}</small></button>)}</div>{matches.total>matches.items.length&&<small>{matches.items.length} sur {matches.total} lignes · précisez la recherche.</small>}</div>}{message&&<small>{message}</small>}</div>;
}

export function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
