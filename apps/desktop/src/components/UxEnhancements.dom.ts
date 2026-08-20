import {
  densityLevel,
  domains,
  explorerItemLabels,
  normalizeSearch,
  type DiagnosticFilter,
  type MapView,
  type WorkbenchDomain,
} from "./UxEnhancements.model";

function setTextContent(element: Element | null, value: string): void {
  if (element && element.textContent !== value) element.textContent = value;
}

function replaceInlineLabel(element: Element | null, value: string): void {
  if (!element) return;
  const textNode = [...element.childNodes].find(
    (node) => node.nodeType === Node.TEXT_NODE && node.textContent?.trim(),
  );
  if (textNode) {
    if (textNode.textContent?.trim() !== value) textNode.textContent = ` ${value}`;
    return;
  }
  element.append(document.createTextNode(` ${value}`));
}

function replaceButtonLabel(oldLabel: string, newLabel: string, ariaLabel?: string): void {
  for (const button of document.querySelectorAll<HTMLButtonElement>("button")) {
    const text = button.textContent?.replace(/\s+/g, " ").trim();
    if (text !== oldLabel && text !== newLabel) continue;
    const textNode = [...button.childNodes].find(
      (node) => node.nodeType === Node.TEXT_NODE && node.textContent?.trim(),
    );
    if (textNode && textNode.textContent?.trim() !== newLabel) {
      textNode.textContent = ` ${newLabel}`;
    }
    if (ariaLabel && button.getAttribute("aria-label") !== ariaLabel) {
      button.setAttribute("aria-label", ariaLabel);
    }
  }
}

function annotateMapCreator(): void {
  const page = document.querySelector<HTMLElement>(".map-creator-page");
  const briefPanel = page?.querySelector<HTMLElement>(".map-brief-panel");
  if (!page || !briefPanel) return;

  let stage: Exclude<MapView, "create" | "atlas"> = "describe";
  for (const child of [...briefPanel.children]) {
    if (child.classList.contains("map-step")) {
      const label = normalizeSearch(child.textContent ?? "");
      if (label.includes("fixer les regles")) stage = "generate";
      if (label.includes("regler les densites")) stage = "adjust";
    }
    const element = child as HTMLElement;
    if (element.dataset.uxStage !== stage) element.dataset.uxStage = stage;
  }

  for (const label of briefPanel.querySelectorAll<HTMLLabelElement>("label")) {
    const text = normalizeSearch(label.childNodes[0]?.textContent ?? label.textContent ?? "");
    label.classList.toggle(
      "ux-map-technical-field",
      text.startsWith("resref") || text.startsWith("tuile de base"),
    );
    label.classList.toggle(
      "ux-map-connection-field",
      text.startsWith("endpoint") || text.startsWith("cle api temporaire"),
    );
    label.classList.toggle("ux-map-density-field", text.startsWith("blueprints"));
  }

  for (const article of briefPanel.querySelectorAll<HTMLElement>(".map-density-list article")) {
    const slider = article.querySelector<HTMLInputElement>('input[type="range"]');
    if (slider) article.dataset.uxDensityLevel = densityLevel(Number(slider.value));
  }

  briefPanel
    .querySelector<HTMLButtonElement>(".map-ai-actions .secondary-button")
    ?.classList.add("ux-hidden-duplicate-action");

  const status = page.querySelector<HTMLElement>(".map-preview-panel header p");
  if (status) {
    status.setAttribute("role", "status");
    status.setAttribute("aria-live", "polite");
  }

  setTextContent(page.querySelector(".map-creator-header h1"), "Créer une zone à partir d’un brief");
  replaceInlineLabel(
    page.querySelector(".map-creator-header .rpg-kicker"),
    "GÉNÉRATION ASSISTÉE ET REPRODUCTIBLE",
  );
  replaceButtonLabel("Créer ARE/GIT/GIC", "Créer la zone", "Créer la zone dans l’espace d’édition");
}

export function annotateWorkbench(activeDomain: WorkbenchDomain): void {
  const domain = domains.find((candidate) => candidate.id === activeDomain) ?? domains[0];
  const treeRoot = document.querySelector<HTMLElement>(".tree-root");
  if (treeRoot?.dataset.uxDomainLabel !== domain.label) {
    treeRoot?.setAttribute("data-ux-domain-label", domain.label);
  }

  for (const button of document.querySelectorAll<HTMLButtonElement>(".tree-item[title]")) {
    const itemId = Object.entries(explorerItemLabels).find(
      ([, label]) => label === button.title,
    )?.[0];
    const section = button.closest<HTMLElement>(".tree-section");
    if (!section) continue;
    const visible = Boolean(itemId && itemId !== "help" && domain.itemIds.includes(itemId));
    if (section.hidden === visible) section.hidden = !visible;
    if (itemId && section.dataset.uxItem !== itemId) section.dataset.uxItem = itemId;
  }

  for (const label of document.querySelectorAll<HTMLElement>(".tree-section-label")) {
    if (label.getAttribute("aria-hidden") !== "true") label.setAttribute("aria-hidden", "true");
  }

  const explorerTitle = document.querySelector<HTMLElement>(".explorer .panel-title > span");
  if (explorerTitle) {
    setTextContent(explorerTitle, "Explorateur");
    explorerTitle.title = "Chronique du module";
  }
  const inspectorTitle = document.querySelector<HTMLElement>(".inspector .panel-title > span");
  if (inspectorTitle) {
    setTextContent(inspectorTitle, "Inspecteur");
    inspectorTitle.title = "Grimoire de l’objet";
  }

  const search = document.querySelector<HTMLInputElement>(".explorer .search-box input");
  const placeholder = `Rechercher dans ${domain.label.toLocaleLowerCase()}…`;
  if (search && search.placeholder !== placeholder) search.placeholder = placeholder;

  for (const row of document.querySelectorAll<HTMLElement>(".diagnostic-row")) {
    const code = row.querySelector("code")?.textContent?.trim();
    row.classList.toggle("ux-permanent-information", code === "SOURCE_READ_ONLY");
    if (row.title !== "Double-cliquer pour copier ce diagnostic") {
      row.title = "Double-cliquer pour copier ce diagnostic";
    }
  }

  annotateMapCreator();
}

export function countDiagnostics(): Record<DiagnosticFilter, number> {
  const rows = [...document.querySelectorAll<HTMLElement>(".diagnostic-row")].filter(
    (row) => !row.classList.contains("ux-permanent-information"),
  );
  return {
    all: rows.length,
    error: rows.filter((row) => row.classList.contains("error")).length,
    warning: rows.filter((row) => row.classList.contains("warning")).length,
    info: rows.filter((row) => row.classList.contains("info")).length,
  };
}
