import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { createPortal } from "react-dom";
import { useUiStore } from "../store/uiStore";
import { annotateWorkbench, countDiagnostics } from "./UxEnhancements.dom";
import { CommandPalette, DomainNavigation } from "./UxEnhancements.navigation";
import { DiagnosticsControls, MapStageControls, WorkspaceSplitters } from "./UxEnhancements.portals";
import {
  densityLevel,
  domainForItem,
  domains,
  explorerItemLabels,
  itemIcons,
  loadBoolean,
  loadMapView,
  loadNumber,
  preferenceKeys,
  queryHosts,
  sameHosts,
  savePreference,
  type DiagnosticFilter,
  type DomainDefinition,
  type HostState,
  type MapView,
  type NavigationCommand,
} from "./UxEnhancements.model";
import "./UxEnhancements.css";

export { densityLevel, domainForItem, filterNavigationCommands } from "./UxEnhancements.model";
export type { WorkbenchDomain } from "./UxEnhancements.model";

export function UxEnhancements() {
  const activeExplorerItem = useUiStore((state) => state.activeExplorerItem);
  const setActiveExplorerItem = useUiStore((state) => state.setActiveExplorerItem);
  const activeDomain = domainForItem(activeExplorerItem);
  const [hosts, setHosts] = useState<HostState>({
    topMenu: null,
    workspaceGrid: null,
    diagnostics: null,
    mapHeader: null,
  });
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [explorerWidth, setExplorerWidth] = useState(() =>
    loadNumber(preferenceKeys.explorerWidth, 270, 210, 420),
  );
  const [inspectorWidth, setInspectorWidth] = useState(() =>
    loadNumber(preferenceKeys.inspectorWidth, 304, 240, 460),
  );
  const [diagnosticFilter, setDiagnosticFilter] = useState<DiagnosticFilter>("all");
  const [diagnosticCounts, setDiagnosticCounts] = useState(() => countDiagnostics());
  const [mapView, setMapView] = useState<MapView>(loadMapView);
  const [mapConnectionExpert, setMapConnectionExpert] = useState(() =>
    loadBoolean(preferenceKeys.mapConnectionExpert, false),
  );
  const [mapDensityExpert, setMapDensityExpert] = useState(() =>
    loadBoolean(preferenceKeys.mapDensityExpert, false),
  );
  const initialPanelRestoreDone = useRef(false);
  const mapPlanSeen = useRef(false);

  const commands = useMemo<NavigationCommand[]>(
    () =>
      domains.flatMap((domain) =>
        domain.itemIds.map((itemId) => ({
          id: `open-${itemId}`,
          label: explorerItemLabels[itemId] ?? itemId,
          description: `Ouvrir l’atelier ${explorerItemLabels[itemId] ?? itemId}`,
          itemId,
          domain: domain.id,
          icon: itemIcons[itemId] ?? domain.icon,
          keywords: [...domain.keywords, itemId],
        })),
      ),
    [],
  );

  const navigateDomain = useCallback(
    (domain: DomainDefinition) => setActiveExplorerItem(domain.defaultItem),
    [setActiveExplorerItem],
  );

  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--ux-explorer-width", `${explorerWidth}px`);
    root.style.setProperty("--ux-inspector-width", `${inspectorWidth}px`);
    savePreference(preferenceKeys.explorerWidth, String(Math.round(explorerWidth)));
    savePreference(preferenceKeys.inspectorWidth, String(Math.round(inspectorWidth)));
  }, [explorerWidth, inspectorWidth]);

  useEffect(() => {
    savePreference(preferenceKeys.mapView, mapView);
    savePreference(preferenceKeys.mapConnectionExpert, String(mapConnectionExpert));
    savePreference(preferenceKeys.mapDensityExpert, String(mapDensityExpert));
    const page = document.querySelector<HTMLElement>(".map-creator-page");
    if (page) {
      page.dataset.uxMapView = mapView;
      page.classList.toggle("ux-map-connection-expert", mapConnectionExpert);
      page.classList.toggle("ux-map-density-expert", mapDensityExpert);
    }
  }, [mapView, mapConnectionExpert, mapDensityExpert, hosts.mapHeader]);

  useEffect(() => {
    const diagnostics = document.querySelector<HTMLElement>(".diagnostics");
    if (diagnostics) diagnostics.dataset.uxFilter = diagnosticFilter;
  }, [diagnosticFilter, hosts.diagnostics]);

  useEffect(() => {
    let frame = 0;
    const synchronize = () => {
      frame = 0;
      const nextHosts = queryHosts();
      setHosts((current) => (sameHosts(current, nextHosts) ? current : nextHosts));
      annotateWorkbench(activeDomain);
      const nextCounts = countDiagnostics();
      setDiagnosticCounts((current) =>
        current.all === nextCounts.all &&
        current.error === nextCounts.error &&
        current.warning === nextCounts.warning &&
        current.info === nextCounts.info
          ? current
          : nextCounts,
      );

      const grid = nextHosts.workspaceGrid;
      if (grid) {
        savePreference(
          preferenceKeys.explorerCollapsed,
          String(grid.classList.contains("explorer-collapsed")),
        );
        savePreference(
          preferenceKeys.inspectorCollapsed,
          String(grid.classList.contains("inspector-collapsed")),
        );
      }

      const page = document.querySelector<HTMLElement>(".map-creator-page");
      if (page) {
        page.dataset.uxMapView = mapView;
        page.classList.toggle("ux-map-connection-expert", mapConnectionExpert);
        page.classList.toggle("ux-map-density-expert", mapDensityExpert);
      }
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(synchronize);
    };
    schedule();
    const observer = new MutationObserver(schedule);
    observer.observe(document.body, { childList: true, subtree: true });
    return () => {
      observer.disconnect();
      if (frame) cancelAnimationFrame(frame);
    };
  }, [activeDomain, mapView, mapConnectionExpert, mapDensityExpert]);

  useEffect(() => {
    if (initialPanelRestoreDone.current || !hosts.workspaceGrid) return;
    initialPanelRestoreDone.current = true;
    const grid = hosts.workspaceGrid;
    if (
      loadBoolean(preferenceKeys.explorerCollapsed, false) &&
      !grid.classList.contains("explorer-collapsed")
    ) {
      document
        .querySelector<HTMLButtonElement>('button[aria-label="Réduire l\'explorateur"]')
        ?.click();
    }
    if (
      loadBoolean(preferenceKeys.inspectorCollapsed, false) &&
      !grid.classList.contains("inspector-collapsed")
    ) {
      document
        .querySelector<HTMLButtonElement>('button[aria-label="Réduire l\'inspecteur"]')
        ?.click();
    }
  }, [hosts.workspaceGrid]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((current) => !current);
        return;
      }
      if (event.key === "Escape") setPaletteOpen(false);
      if (event.altKey && /^[1-6]$/.test(event.key)) {
        const domain = domains[Number(event.key) - 1];
        if (domain) {
          event.preventDefault();
          navigateDomain(domain);
        }
      }
    };
    const onDoubleClick = (event: MouseEvent) => {
      const row = (event.target as HTMLElement).closest<HTMLElement>(".diagnostic-row");
      if (row && !row.classList.contains("ux-permanent-information")) {
        void navigator.clipboard?.writeText(row.innerText.replace(/\s+/g, " ").trim());
      }
    };
    const onInput = (event: Event) => {
      const input = event.target as HTMLInputElement;
      if (input.matches('.map-density-list input[type="range"]')) {
        const article = input.closest<HTMLElement>("article");
        if (article) article.dataset.uxDensityLevel = densityLevel(Number(input.value));
      }
    };
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("dblclick", onDoubleClick);
    document.addEventListener("input", onInput);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("dblclick", onDoubleClick);
      document.removeEventListener("input", onInput);
    };
  }, [navigateDomain]);

  useEffect(() => {
    if (!hosts.mapHeader) {
      mapPlanSeen.current = false;
      return;
    }
    const page = document.querySelector<HTMLElement>(".map-creator-page");
    if (!page) return;
    mapPlanSeen.current = Boolean(page.querySelector(".map-plan-metrics"));
    const observer = new MutationObserver(() => {
      const hasPlan = Boolean(page.querySelector(".map-plan-metrics"));
      if (!hasPlan) {
        mapPlanSeen.current = false;
        return;
      }
      if (!mapPlanSeen.current) {
        mapPlanSeen.current = true;
        setMapView("create");
      }
    });
    observer.observe(page, { childList: true, subtree: true });
    return () => observer.disconnect();
  }, [hosts.mapHeader]);

  return (
    <>
      {hosts.topMenu &&
        createPortal(
          <DomainNavigation
            activeDomain={activeDomain}
            onNavigate={navigateDomain}
            onOpenPalette={() => setPaletteOpen(true)}
          />,
          hosts.topMenu,
        )}
      {hosts.workspaceGrid && (
        <WorkspaceSplitters
          host={hosts.workspaceGrid}
          explorerWidth={explorerWidth}
          inspectorWidth={inspectorWidth}
          onExplorerWidth={setExplorerWidth}
          onInspectorWidth={setInspectorWidth}
        />
      )}
      {hosts.diagnostics && (
        <DiagnosticsControls
          host={hosts.diagnostics}
          counts={diagnosticCounts}
          filter={diagnosticFilter}
          onFilter={setDiagnosticFilter}
        />
      )}
      {hosts.mapHeader && (
        <MapStageControls
          host={hosts.mapHeader}
          view={mapView}
          connectionExpert={mapConnectionExpert}
          densityExpert={mapDensityExpert}
          onView={setMapView}
          onConnectionExpert={setMapConnectionExpert}
          onDensityExpert={setMapDensityExpert}
        />
      )}
      <CommandPalette
        open={paletteOpen}
        commands={commands}
        onClose={() => setPaletteOpen(false)}
        onNavigate={(command) => {
          setActiveExplorerItem(command.itemId);
          setPaletteOpen(false);
        }}
      />
    </>
  );
}

export default UxEnhancements;
