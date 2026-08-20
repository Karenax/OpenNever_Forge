import { LayoutDashboard } from "lucide-react";
import { describe, expect, it } from "vitest";
import {
  densityLevel,
  domainForItem,
  filterNavigationCommands,
  nodeAffectsWorkbench,
} from "./UxEnhancements";

describe("UX navigation helpers", () => {
  it("maps every principal workshop to its functional domain", () => {
    expect(domainForItem("module")).toBe("project");
    expect(domainForItem("map_creator")).toBe("world");
    expect(domainForItem("dialogues")).toBe("narration");
    expect(domainForItem("blueprints")).toBe("content");
    expect(domainForItem("build")).toBe("validation");
    expect(domainForItem("agent")).toBe("agent");
  });

  it("describes density values without exposing raw numbers as the only cue", () => {
    expect(densityLevel(0)).toBe("Aucune");
    expect(densityLevel(8)).toBe("Faible");
    expect(densityLevel(18)).toBe("Normale");
    expect(densityLevel(32)).toBe("Riche");
    expect(densityLevel(50)).toBe("Très riche");
  });

  it("searches commands without being sensitive to accents", () => {
    const commands = [
      {
        id: "open-map",
        label: "Créateur de cartes",
        description: "Créer une zone à partir d’un brief",
        itemId: "map_creator",
        domain: "world" as const,
        icon: LayoutDashboard,
        keywords: ["generation", "zone"],
      },
      {
        id: "open-build",
        label: "Construire et tester",
        description: "Valider puis lancer le module",
        itemId: "build",
        domain: "validation" as const,
        icon: LayoutDashboard,
        keywords: ["build", "test"],
      },
    ];

    expect(filterNavigationCommands(commands, "createur")).toHaveLength(1);
    expect(filterNavigationCommands(commands, "génération")[0]?.itemId).toBe("map_creator");
    expect(filterNavigationCommands(commands, "build")[0]?.itemId).toBe("build");
  });

  it("ignores large dialogue subtrees while still detecting workbench hosts", () => {
    const dialogue = document.createElement("article");
    dialogue.className = "dialogue-line-card";
    dialogue.innerHTML = "<textarea>Une longue réplique</textarea><button>Associer</button>";
    expect(nodeAffectsWorkbench(dialogue)).toBe(false);

    const diagnostics = document.createElement("section");
    diagnostics.innerHTML = '<div class="diagnostic-row warning">Avertissement</div>';
    expect(nodeAffectsWorkbench(diagnostics)).toBe(true);
  });
});
