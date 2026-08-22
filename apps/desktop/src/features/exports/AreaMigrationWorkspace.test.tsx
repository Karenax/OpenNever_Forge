import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AreaMigrationWorkspace } from "./AreaMigrationWorkspace";

const api = vi.hoisted(() => ({
  listAreaMigrationCandidates: vi.fn(),
  previewAreaMigration: vi.fn(),
  selectDirectory: vi.fn(),
  startAreaMigrationExport: vi.fn(),
  getJob: vi.fn(),
  getAreaMigrationJob: vi.fn(),
  cancelJob: vi.fn(),
}));

vi.mock("../../lib/tauri", () => ({
  ...api,
  normalizeAppError: (error: unknown) => {
    if (typeof error === "object" && error && "userMessage" in error) return error;
    return { userMessage: error instanceof Error ? error.message : String(error) };
  },
}));

const counts = {
  tiles: 12, instances: 7, uniqueModels: 4, textures: 9, preservedNavigation: 3,
  missingItems: 1, fallbacks: 2, diagnostics: 2, warnings: 2, errors: 0, byStatus: {},
};

const preview = {
  schemaVersion: "area-migration-bundle@1.0.0",
  areaResref: "forest01",
  areaName: "Foret synthetique",
  suggestedDirectoryName: "forest01.area-migration-v1",
  ready: true,
  complete: false,
  counts,
  diagnostics: [{ sequence: 1, severity: "warning", status: "placeholder", phase: "audit", code: "AREA_KIND_UNKNOWN", message: "Type preserve comme inconnu.", resource: "forest01.are", identity: null }],
  classification: "local_only_proprietary",
  redistribution: "not_redistributable_without_separate_rights",
  navigationStatus: "preserved-not-converted",
};

describe("AreaMigrationWorkspace", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.listAreaMigrationCandidates.mockResolvedValue([{ resref: "forest01", name: "Foret synthetique", width: 4, height: 3, tileCount: 12, instanceCount: 7, sourceDiagnosticCount: 0 }]);
    api.previewAreaMigration.mockResolvedValue(preview);
    api.selectDirectory.mockResolvedValue("C:\\Exports");
    api.getJob.mockResolvedValue(null);
    api.getAreaMigrationJob.mockResolvedValue(null);
  });

  it("keeps the workspace visible and locked until analysis completes", () => {
    render(<AreaMigrationWorkspace analysisReady={false} />);
    expect(screen.getByRole("region", { name: "Migration de zone" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Analyse requise" })).toBeInTheDocument();
    expect(screen.getByText(/sources NWN resteront en lecture seule/)).toBeInTheDocument();
    expect(api.listAreaMigrationCandidates).not.toHaveBeenCalled();
  });

  it("shows preview counts and keeps export disabled until destination and legal consent", async () => {
    api.startAreaMigrationExport.mockResolvedValue({
      id: "migration-1", kind: "area_migration_export", state: "completed", sourcePath: "C:\\module.mod",
      progress: { bytesRead: 0, totalBytes: 0, percent: 100, phase: "persisting" },
      migrationResult: {
        bundlePath: "C:\\Exports\\forest01.area-migration-v1",
        manifestFile: { path: "manifest.json", sizeBytes: 512, sha256: "a".repeat(64), role: "manifest" },
        report: { schemaVersion: preview.schemaVersion, areaResref: "forest01", complete: false, counts, navigationConverted: false, navigationStatus: "preserved-not-converted", diagnosticsFile: "diagnostics.jsonl", bundleIsLocalOnly: true, sourceModuleImmutable: true, payloadFileCount: 8, payloadSizeBytes: 2048 },
        diagnostics: preview.diagnostics,
      },
    });
    render(<AreaMigrationWorkspace jobId="analysis-1" analysisReady />);

    expect(await screen.findByText("Export possible avec réserves")).toBeInTheDocument();
    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText("9")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Aperçu du bundle" })).toHaveTextContent("manifest.json");
    expect(screen.getByText("Avertissements")).toBeInTheDocument();
    expect(screen.getByText(/Ressources propriétaires/)).toBeInTheDocument();
    const exportButton = screen.getByRole("button", { name: "Exporter le bundle" });
    expect(exportButton).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Choisir la destination" }));
    expect(await screen.findByText("C:\\Exports\\forest01.area-migration-v1")).toBeInTheDocument();
    expect(exportButton).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: /ce bundle reste local/ }));
    expect(exportButton).toBeEnabled();
    fireEvent.click(exportButton);

    expect(await screen.findByText("Bundle produit avec réserves")).toBeInTheDocument();
    expect(screen.getAllByText("C:\\Exports\\forest01.area-migration-v1").length).toBeGreaterThan(0);
    expect(api.startAreaMigrationExport).toHaveBeenCalledWith({ analysisJobId: "analysis-1", areaResref: "forest01", destination: "C:\\Exports\\forest01.area-migration-v1" });
  });

  it("presents a safe failure without claiming a bundle or stale progress", async () => {
    api.startAreaMigrationExport.mockResolvedValue({
      id: "migration-2", kind: "area_migration_export", state: "failed", sourcePath: "C:\\module.mod",
      progress: { bytesRead: 0, totalBytes: 0, percent: 1, phase: "persisting" },
      migrationProgress: { phase: "preparing", percent: 1, current: null },
      error: { code: "MIGRATION_SOURCE_CHANGED", userMessage: "Le module source a change.", technicalMessage: "hash mismatch", severity: "error" },
    });
    render(<AreaMigrationWorkspace jobId="analysis-1" analysisReady />);
    await screen.findByText("Export possible avec réserves");
    fireEvent.click(screen.getByRole("button", { name: "Choisir la destination" }));
    await screen.findByText("C:\\Exports\\forest01.area-migration-v1");
    fireEvent.click(screen.getByRole("checkbox", { name: /ce bundle reste local/ }));
    fireEvent.click(screen.getByRole("button", { name: "Exporter le bundle" }));
    expect(await screen.findByText("Export interrompu")).toBeInTheDocument();
    expect(screen.getByText("Le module source a change.")).toBeInTheDocument();
    expect(screen.queryByRole("progressbar", { name: "Progression de la migration" })).not.toBeInTheDocument();
    expect(screen.queryByText("Bundle complet")).not.toBeInTheDocument();
  });

  it("returns the report panel to the failure summary", async () => {
    api.startAreaMigrationExport.mockResolvedValue({
      id: "migration-2", kind: "area_migration_export", state: "failed", sourcePath: "C:\\module.mod",
      progress: { bytesRead: 0, totalBytes: 0, percent: 1, phase: "persisting" },
      migrationProgress: { phase: "preparing", percent: 1, current: null },
      error: { code: "MIGRATION_SOURCE_CHANGED", userMessage: "Le module source a change.", technicalMessage: "hash mismatch", severity: "error" },
    });
    render(<AreaMigrationWorkspace jobId="analysis-1" analysisReady />);
    const reportPanel = screen.getByRole("heading", { name: "Résultat et diagnostics" }).closest("aside");
    expect(reportPanel).not.toBeNull();
    if (!reportPanel) return;
    reportPanel.scrollTop = 900;

    await screen.findByText("Export possible avec réserves");
    fireEvent.click(screen.getByRole("button", { name: "Choisir la destination" }));
    await screen.findByText("C:\\Exports\\forest01.area-migration-v1");
    fireEvent.click(screen.getByRole("checkbox", { name: /ce bundle reste local/ }));
    fireEvent.click(screen.getByRole("button", { name: "Exporter le bundle" }));

    expect(await screen.findByText("Export interrompu")).toBeInTheDocument();
    await waitFor(() => expect(reportPanel.scrollTop).toBe(0));
  });

  it("keeps a preview with blocking diagnostics non-exportable", async () => {
    api.previewAreaMigration.mockResolvedValue({
      ...preview,
      ready: false,
      complete: false,
      counts: { ...counts, errors: 1 },
      diagnostics: [{
        ...preview.diagnostics[0],
        severity: "error",
        status: "missing",
        code: "MIGRATION_TILE_COUNT_MISMATCH",
        message: "La grille synthétique est incomplète.",
      }],
    });

    render(<AreaMigrationWorkspace jobId="analysis-1" analysisReady />);

    expect(await screen.findByText("Export bloqué")).toBeInTheDocument();
    expect(screen.getByText("MIGRATION_TILE_COUNT_MISMATCH")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Exporter le bundle" })).toBeDisabled();
  });

  it("shows live progress and exposes safe cancellation", async () => {
    const runningJob = {
      id: "migration-3", kind: "area_migration_export", state: "running", sourcePath: "C:\\module.mod",
      progress: { bytesRead: 0, totalBytes: 0, percent: 48, phase: "persisting" },
      migrationProgress: { phase: "models", percent: 48, current: "tile_a" },
    };
    api.startAreaMigrationExport.mockResolvedValue(runningJob);
    api.cancelJob.mockResolvedValue({ ...runningJob, state: "cancelled" });

    render(<AreaMigrationWorkspace jobId="analysis-1" analysisReady />);
    await screen.findByText("Export possible avec réserves");
    fireEvent.click(screen.getByRole("button", { name: "Choisir la destination" }));
    await screen.findByText("C:\\Exports\\forest01.area-migration-v1");
    fireEvent.click(screen.getByRole("checkbox", { name: /ce bundle reste local/ }));
    fireEvent.click(screen.getByRole("button", { name: "Exporter le bundle" }));

    const progress = await screen.findByRole("progressbar", { name: "Progression de la migration" });
    expect(progress).toHaveAttribute("aria-valuenow", "48");
    expect(screen.getByText("Conversion des modèles")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Annuler" }));

    expect(await screen.findByText("Export annulé")).toBeInTheDocument();
    expect(screen.getByText("Aucun bundle partiel n’a été publié.")).toBeInTheDocument();
    expect(api.cancelJob).toHaveBeenCalledWith("migration-3");
  });

  it("recovers the latest migration job after the workspace remounts", async () => {
    const recoveredJob = {
      id: "migration-recovered", kind: "area_migration_export", state: "running", sourcePath: "C:\\module.mod",
      migrationAnalysisJobId: "analysis-1", migrationAreaResref: "forest01",
      migrationDestination: "C:\\Exports\\forest01.area-migration-v1",
      progress: { bytesRead: 0, totalBytes: 0, percent: 37, phase: "persisting" },
      migrationProgress: { phase: "textures", percent: 37, current: "stone" },
    };
    api.getAreaMigrationJob.mockResolvedValue(recoveredJob);
    const view = render(<AreaMigrationWorkspace jobId="analysis-1" analysisReady />);
    expect(await screen.findByText("Conversion des textures")).toBeInTheDocument();
    expect(screen.getByText("C:\\Exports\\forest01.area-migration-v1")).toBeInTheDocument();
    view.unmount();
    render(<AreaMigrationWorkspace jobId="analysis-1" analysisReady />);
    await waitFor(() => expect(api.getAreaMigrationJob).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Conversion des textures")).toBeInTheDocument();
  });
});
