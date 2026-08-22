import {
  BookOpen,
  Box,
  CircleHelp,
  Code2,
  Download,
  Hammer,
  LayoutDashboard,
  Map,
  MessageSquareText,
  ShieldCheck,
  Sparkles,
  Users,
  type LucideIcon,
} from "lucide-react";

export type WorkbenchDomain =
  | "project"
  | "world"
  | "narration"
  | "content"
  | "validation"
  | "export"
  | "agent";

export type DomainDefinition = {
  id: WorkbenchDomain;
  label: string;
  shortLabel: string;
  icon: LucideIcon;
  defaultItem: string;
  itemIds: string[];
  keywords: string[];
};

export type NavigationCommand = {
  id: string;
  label: string;
  description: string;
  itemId: string;
  domain: WorkbenchDomain;
  icon: LucideIcon;
  keywords: string[];
};

export type DiagnosticFilter = "all" | "error" | "warning" | "info";
export type MapView = "describe" | "generate" | "adjust" | "create" | "atlas";

export type HostState = {
  topMenu: HTMLElement | null;
  workspaceGrid: HTMLElement | null;
  diagnostics: HTMLElement | null;
  mapHeader: HTMLElement | null;
};

export const preferenceKeys = {
  explorerWidth: "opennever.ux.explorer-width.v1",
  inspectorWidth: "opennever.ux.inspector-width.v1",
  explorerCollapsed: "opennever.ux.explorer-collapsed.v1",
  inspectorCollapsed: "opennever.ux.inspector-collapsed.v1",
  mapView: "opennever.ux.map-view.v1",
  mapConnectionExpert: "opennever.ux.map-connection-expert.v1",
  mapDensityExpert: "opennever.ux.map-density-expert.v1",
} as const;

export const explorerItemLabels: Record<string, string> = {
  module: "Table de campagne",
  resources: "Ressources",
  areas: "Zones",
  map_creator: "Créateur de cartes",
  migration: "Exporter une carte",
  asset_export: "Exporter des assets",
  dialogue_export: "Exporter des dialogues",
  scene: "Vue 3D",
  assets: "Assets",
  dialogues: "Dialogues",
  journal: "Journal et quêtes",
  factions: "Factions",
  scripts: "Scripts",
  blueprints: "Blueprints",
  tables: "2DA et TLK",
  graph: "Références",
  build: "Construire et tester",
  agent: "Agent Studio",
  help: "Guide et manuel",
};

export const domains: DomainDefinition[] = [
  {
    id: "project",
    label: "Projet",
    shortLabel: "Projet",
    icon: LayoutDashboard,
    defaultItem: "module",
    itemIds: ["module", "resources"],
    keywords: ["campagne", "module", "ressources", "dépendances"],
  },
  {
    id: "world",
    label: "Monde",
    shortLabel: "Monde",
    icon: Map,
    defaultItem: "areas",
    itemIds: ["areas", "map_creator", "scene", "assets"],
    keywords: ["zones", "carte", "atlas", "3d", "assets"],
  },
  {
    id: "narration",
    label: "Narration",
    shortLabel: "Récit",
    icon: MessageSquareText,
    defaultItem: "dialogues",
    itemIds: ["dialogues", "journal", "factions"],
    keywords: ["dialogues", "quêtes", "journal", "factions", "récit"],
  },
  {
    id: "content",
    label: "Contenu",
    shortLabel: "Contenu",
    icon: Box,
    defaultItem: "blueprints",
    itemIds: ["scripts", "blueprints", "tables"],
    keywords: ["scripts", "blueprints", "2da", "tlk", "contenu"],
  },
  {
    id: "validation",
    label: "Validation",
    shortLabel: "Valider",
    icon: ShieldCheck,
    defaultItem: "build",
    itemIds: ["graph", "build"],
    keywords: ["références", "diagnostics", "build", "test", "validation"],
  },
  {
    id: "export",
    label: "Export",
    shortLabel: "Export",
    icon: Download,
    defaultItem: "migration",
    itemIds: ["migration", "asset_export", "dialogue_export"],
    keywords: ["export", "migration", "carte", "asset", "glb", "animation", "dialogue", "dlg", "transcript", "bundle"],
  },
  {
    id: "agent",
    label: "Agent",
    shortLabel: "Agent",
    icon: Sparkles,
    defaultItem: "agent",
    itemIds: ["agent"],
    keywords: ["ia", "agent", "automatisation", "modèle"],
  },
];

export const itemIcons: Record<string, LucideIcon> = {
  module: LayoutDashboard,
  resources: Box,
  areas: Map,
  map_creator: Map,
  migration: Download,
  asset_export: Box,
  dialogue_export: MessageSquareText,
  scene: Map,
  assets: Box,
  dialogues: MessageSquareText,
  journal: BookOpen,
  factions: Users,
  scripts: Code2,
  blueprints: Box,
  tables: Code2,
  graph: ShieldCheck,
  build: Hammer,
  agent: Sparkles,
  help: CircleHelp,
};

export function domainForItem(itemId: string): WorkbenchDomain {
  return domains.find((domain) => domain.itemIds.includes(itemId))?.id ?? "project";
}

export function densityLevel(value: number): string {
  if (value <= 0) return "Aucune";
  if (value < 10) return "Faible";
  if (value < 25) return "Normale";
  if (value < 40) return "Riche";
  return "Très riche";
}

export function filterNavigationCommands(
  commands: NavigationCommand[],
  query: string,
): NavigationCommand[] {
  const normalized = normalizeSearch(query);
  if (!normalized) return commands;
  return commands.filter((command) =>
    normalizeSearch(
      [command.label, command.description, command.domain, ...command.keywords].join(" "),
    ).includes(normalized),
  );
}

export function normalizeSearch(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLocaleLowerCase()
    .trim();
}

export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function loadNumber(key: string, fallback: number, min: number, max: number): number {
  try {
    const stored = localStorage.getItem(key);
    if (stored === null || stored.trim() === "") return fallback;
    const parsed = Number(stored);
    return Number.isFinite(parsed) ? clamp(parsed, min, max) : fallback;
  } catch {
    return fallback;
  }
}

export function loadBoolean(key: string, fallback: boolean): boolean {
  try {
    const value = localStorage.getItem(key);
    return value === null ? fallback : value === "true";
  } catch {
    return fallback;
  }
}

export function savePreference(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // The desktop application normally provides localStorage. Failure is non-blocking.
  }
}

export function loadMapView(): MapView {
  try {
    const stored = localStorage.getItem(preferenceKeys.mapView);
    return ["describe", "generate", "adjust", "create", "atlas"].includes(stored ?? "")
      ? (stored as MapView)
      : "describe";
  } catch {
    return "describe";
  }
}

export function sameHosts(left: HostState, right: HostState): boolean {
  return (
    left.topMenu === right.topMenu &&
    left.workspaceGrid === right.workspaceGrid &&
    left.diagnostics === right.diagnostics &&
    left.mapHeader === right.mapHeader
  );
}

export function queryHosts(): HostState {
  return {
    topMenu: document.querySelector<HTMLElement>(".main-menu"),
    workspaceGrid: document.querySelector<HTMLElement>(".workspace-grid"),
    diagnostics: document.querySelector<HTMLElement>(".diagnostic-tabs"),
    mapHeader: document.querySelector<HTMLElement>(".map-creator-header"),
  };
}
