import { Background, Controls, MiniMap, ReactFlow, type Edge, type Node } from "@xyflow/react";
import type { DialogueGraph } from "../lib/tauri";

export default function DialogueFlow({ graph, onSelect }: { graph: DialogueGraph; onSelect: (id: string) => void }) {
  const nodes: Node[] = graph.nodes.map((value) => ({
    id: value.id,
    position: { x: value.kind === "entry" ? 0 : 430, y: value.index * 92 },
    data: {
      label: (
        <div className={`flow-dialogue-node ${value.kind}`}>
          <code>{value.id}</code>
          <span>{value.displayText?.slice(0, 100) ?? "Texte non résolu"}</span>
        </div>
      ),
    },
    className: graph.sharedNodes.includes(value.id) ? "shared" : "",
  }));
  const edges: Edge[] = graph.links
    .filter((value) => value.source && !value.broken)
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

  return (
    <div className="dialogue-flow">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        fitView
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable
        onNodeClick={(_, value) => onSelect(value.id)}
      >
        <Background color="#27323c" gap={24} />
        <MiniMap pannable zoomable nodeColor={(node) => (node.id.startsWith("entry") ? "#567d9d" : "#9a7044")} />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}
