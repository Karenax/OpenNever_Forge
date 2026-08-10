import { useEffect, useMemo, useState } from "react";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Connection,
  type Edge,
  type Node,
} from "@xyflow/react";
import { Focus, Search } from "lucide-react";
import type { DialogueGraph } from "../lib/tauri";

const MAX_VISIBLE_NODES = 36;

type DialogueFlowProps = {
  graph: DialogueGraph;
  selectedId?: string;
  onSelect: (id: string) => void;
  onConnect?: (sourceId: string, targetId: string) => Promise<void>;
};

export default function DialogueFlow({ graph, selectedId, onSelect, onConnect }: DialogueFlowProps) {
  const [query, setQuery] = useState("");
  const [depth, setDepth] = useState(1);
  const [focusId, setFocusId] = useState(selectedId ?? graph.roots[0] ?? graph.nodes[0]?.id);
  const [connectMessage, setConnectMessage] = useState("");

  useEffect(() => {
    if (selectedId) setFocusId(selectedId);
  }, [selectedId]);

  const view = useMemo(
    () => buildFocusedDialogueView(graph, focusId, query, depth),
    [depth, focusId, graph, query],
  );

  const connect = async (connection: Connection) => {
    if (!onConnect || !connection.source || !connection.target) return;
    setConnectMessage("Ajout du lien…");
    try {
      await onConnect(connection.source, connection.target);
      setConnectMessage("Lien ajouté dans l’overlay.");
    } catch {
      setConnectMessage("Le lien n’a pas pu être ajouté.");
    }
  };

  return (
    <div className="dialogue-flow-shell">
      <div className="dialogue-flow-toolbar">
        <label>
          <Search size={14} />
          <input
            aria-label="Rechercher un nœud de dialogue"
            value={query}
            onChange={(event) => setQuery(event.currentTarget.value)}
            placeholder="Texte, locuteur, commentaire ou identifiant…"
          />
        </label>
        <label className="dialogue-depth-control">
          Voisinage
          <select value={depth} onChange={(event) => setDepth(Number(event.currentTarget.value))}>
            <option value={1}>1 niveau</option>
            <option value={2}>2 niveaux</option>
            <option value={3}>3 niveaux</option>
            <option value={4}>4 niveaux</option>
          </select>
        </label>
        <button
          type="button"
          onClick={() => setFocusId(selectedId ?? graph.roots[0] ?? graph.nodes[0]?.id)}
          title="Recentrer sur la sélection"
        >
          <Focus size={14} /> Recentrer
        </button>
        <span>
          {view.nodes.length.toLocaleString("fr-FR")} / {graph.nodes.length.toLocaleString("fr-FR")} nœuds affichés
        </span>
      </div>
      {view.truncated && (
        <p className="dialogue-flow-notice">
          Vue limitée à {MAX_VISIBLE_NODES} nœuds pour rester lisible. Recherchez un texte ou recentrez
          sur un nœud pour explorer une autre branche.
        </p>
      )}
      {connectMessage && <p className="dialogue-flow-status" role="status">{connectMessage}</p>}
      <div className="dialogue-flow">
        <ReactFlow
          key={`${focusId ?? "root"}:${query}:${depth}`}
          nodes={view.nodes}
          edges={view.edges}
          defaultViewport={{ x: 24, y: 24, zoom: 0.72 }}
          minZoom={0.12}
          maxZoom={1.7}
          nodesDraggable={false}
          nodesConnectable={Boolean(onConnect)}
          elementsSelectable
          onNodeClick={(_, value) => {
            setFocusId(value.id);
            onSelect(value.id);
          }}
          onConnect={(connection) => void connect(connection)}
        >
          <Background color="#27323c" gap={24} />
          <MiniMap
            pannable
            zoomable
            nodeColor={(node) => (node.id.startsWith("entry") ? "#567d9d" : "#9a7044")}
          />
          <Controls showInteractive={false} />
        </ReactFlow>
      </div>
    </div>
  );
}

export function buildFocusedDialogueView(
  graph: DialogueGraph,
  focusId: string | undefined,
  query: string,
  maxDepth: number,
): { nodes: Node[]; edges: Edge[]; truncated: boolean } {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  const neighbors = new Map<string, Set<string>>();
  for (const node of graph.nodes) neighbors.set(node.id, new Set());
  for (const link of graph.links) {
    if (!link.source || link.broken || !byId.has(link.target)) continue;
    neighbors.get(link.source)?.add(link.target);
    neighbors.get(link.target)?.add(link.source);
  }

  const matches = normalizedQuery
    ? graph.nodes.filter((node) =>
      [node.id, node.displayText, node.speaker, node.comment, node.quest, node.actionScript]
        .some((value) => value?.toLocaleLowerCase().includes(normalizedQuery)),
    )
    : [];
  const seeds = matches.length
    ? matches.map((node) => node.id)
    : [focusId, ...graph.roots, graph.nodes[0]?.id].filter((value): value is string => Boolean(value));

  const distance = new Map<string, number>();
  const queue: string[] = [];
  for (const seed of seeds) {
    if (!byId.has(seed) || distance.has(seed)) continue;
    distance.set(seed, 0);
    queue.push(seed);
  }
  while (queue.length && distance.size < MAX_VISIBLE_NODES) {
    const current = queue.shift() as string;
    const currentDepth = distance.get(current) ?? 0;
    if (currentDepth >= maxDepth) continue;
    for (const neighbor of neighbors.get(current) ?? []) {
      if (distance.has(neighbor)) continue;
      distance.set(neighbor, currentDepth + 1);
      queue.push(neighbor);
      if (distance.size >= MAX_VISIBLE_NODES) break;
    }
  }

  const visibleIds = new Set(distance.keys());
  const layers = new Map<number, string[]>();
  for (const [id, layer] of distance) {
    const values = layers.get(layer) ?? [];
    values.push(id);
    layers.set(layer, values);
  }

  const nodes: Node[] = [];
  for (const [layer, ids] of [...layers.entries()].sort(([left], [right]) => left - right)) {
    ids.sort((left, right) => (byId.get(left)?.index ?? 0) - (byId.get(right)?.index ?? 0));
    ids.forEach((id, position) => {
      const value = byId.get(id);
      if (!value) return;
      nodes.push({
        id,
        position: {
          x: layer * 1_650 + (position % 4) * 380,
          y: Math.floor(position / 4) * 170,
        },
        selected: id === focusId,
        data: {
          label: (
            <div className={`flow-dialogue-node ${value.kind}`}>
              <div><code>{value.id}</code>{value.speaker && <b>{value.speaker}</b>}</div>
              <span>{value.displayText?.slice(0, 180) ?? "Texte non résolu"}</span>
              {value.comment && <small>{value.comment.slice(0, 100)}</small>}
            </div>
          ),
        },
        className: graph.sharedNodes.includes(value.id) ? "shared" : "",
      });
    });
  }

  const edges: Edge[] = graph.links
    .filter((value) => value.source && !value.broken && visibleIds.has(value.source) && visibleIds.has(value.target))
    .map((value) => ({
      id: value.id,
      source: value.source as string,
      target: value.target,
      animated: graph.cycles.some(
        (cycle) => cycle.includes(value.source as string) && cycle.includes(value.target),
      ),
      label: value.conditionScript ?? undefined,
      style: { stroke: value.isChild ? "#d59b55" : "#5f819e" },
    }));

  return {
    nodes,
    edges,
    truncated: graph.nodes.length > visibleIds.size && distance.size >= MAX_VISIBLE_NODES,
  };
}
