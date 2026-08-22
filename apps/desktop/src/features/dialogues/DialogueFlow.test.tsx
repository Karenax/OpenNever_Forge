import { describe, expect, it } from "vitest";
import type { DialogueGraph } from "../../lib/tauri";
import { buildFocusedDialogueView } from "./DialogueFlow";

function largeDialogue(size: number): DialogueGraph {
  const nodes = Array.from({ length: size }, (_, index) => ({
    id: `entry:${index}`,
    kind: "entry" as const,
    index,
    text: null,
    displayText: index === 1100 ? "La cible recherchée" : `Réplique ${index}`,
    speaker: "NPC",
    comment: null,
    animation: null,
    animationLoop: null,
    sound: null,
    quest: null,
    actionScript: null,
  }));
  return {
    key: { resref: "large_dialogue", resourceType: 2029 },
    source: "synthetic",
    nodes,
    links: nodes.slice(1).map((node, index) => ({
      id: `link:${index}`,
      source: nodes[index].id,
      target: node.id,
      conditionScript: null,
      actionScript: null,
      comment: null,
      isChild: false,
      broken: false,
    })),
    roots: [nodes[0].id],
    sharedNodes: [],
    unreachableNodes: [],
    cycles: [],
    diagnostics: [],
    references: [],
    tree: [],
    raw: {},
  };
}

describe("focused dialogue graph", () => {
  it("keeps a 1,200-node dialogue readable around the active branch", () => {
    const graph = largeDialogue(1200);
    const view = buildFocusedDialogueView(graph, "entry:0", "", 4);

    expect(view.nodes).toHaveLength(5);
    expect(view.edges).toHaveLength(4);
    expect(view.nodes.map((node) => node.id)).toEqual([
      "entry:0",
      "entry:1",
      "entry:2",
      "entry:3",
      "entry:4",
    ]);
  });

  it("finds a distant sentence without rendering the complete dialogue", () => {
    const graph = largeDialogue(1200);
    const view = buildFocusedDialogueView(graph, "entry:0", "cible recherchée", 2);

    expect(view.nodes.some((node) => node.id === "entry:1100")).toBe(true);
    expect(view.nodes.length).toBeLessThanOrEqual(5);
    expect(view.nodes.length).toBeLessThan(graph.nodes.length);
  });

  it("caps a very broad branch explicitly", () => {
    const graph = largeDialogue(200);
    graph.links = graph.nodes.slice(1).map((node, index) => ({
      id: `root:${index}`,
      source: "entry:0",
      target: node.id,
      conditionScript: null,
      actionScript: null,
      comment: null,
      isChild: false,
      broken: false,
    }));
    const view = buildFocusedDialogueView(graph, "entry:0", "", 1);

    expect(view.nodes).toHaveLength(36);
    expect(view.truncated).toBe(true);
  });
});
