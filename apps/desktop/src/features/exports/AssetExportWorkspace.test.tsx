import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AssetExportWorkspace } from "./AssetExportWorkspace";

const api = vi.hoisted(() => ({
  listAssetExportCandidates: vi.fn(),
  previewAssetExport: vi.fn(),
  exportAssetBundle: vi.fn(),
  selectDirectory: vi.fn(),
}));

vi.mock("../../lib/tauri", () => ({
  ...api,
  normalizeAppError: (error: unknown) => ({
    userMessage: error instanceof Error ? error.message : String(error),
  }),
}));

const animatedCandidate = {
  resref: "hero", format: "mdl_binary", source: "module.mod", exportable: true,
  declaredAnimationCount: 2, declaredAnimations: ["idle", "walk"], meshCount: 3,
  triangleCount: 120, skinCount: 1, textureCount: 2, diagnosticCount: 0,
};
const staticCandidate = {
  resref: "crate", format: "mdl_ascii", source: "module.mod", exportable: true,
  declaredAnimationCount: 0, declaredAnimations: [], meshCount: 1,
  triangleCount: 12, skinCount: 0, textureCount: 1, diagnosticCount: 0,
};
const animatedPreview = {
  schemaVersion: "opennever-asset-export@1.0.0", resref: "hero", mode: "animated", ready: true,
  suggestedDirectoryName: "hero.asset-export-v1", nodeCount: 8, meshCount: 3,
  primitiveCount: 3, skinCount: 1, animationCount: 2,
  animations: [
    { name: "idle", lengthSeconds: 1, transitionSeconds: 0.2, rootNode: "root", trackCount: 2, eventCount: 0, exported: true },
    { name: "walk", lengthSeconds: 1.2, transitionSeconds: 0.2, rootNode: "root", trackCount: 4, eventCount: 1, exported: true },
  ],
  textures: [
    { resref: "hero_diff", resourceType: 2033, outputPath: null, status: "planned", diagnostic: null },
    { resref: "hero_mask", resourceType: 3, outputPath: null, status: "planned", diagnostic: null },
  ],
  warnings: [], classification: "local_only_proprietary",
  redistribution: "not_redistributable_without_separate_rights",
};

describe("AssetExportWorkspace", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.listAssetExportCandidates.mockResolvedValue([animatedCandidate, staticCandidate]);
    api.previewAssetExport.mockImplementation(async (_jobId: string, resref: string) => resref === "hero"
      ? animatedPreview
      : { ...animatedPreview, resref: "crate", mode: "static", animationCount: 0, animations: [], suggestedDirectoryName: "crate.asset-export-v1" });
    api.selectDirectory.mockResolvedValue("C:\\Exports");
    api.exportAssetBundle.mockResolvedValue({
      schemaVersion: animatedPreview.schemaVersion, destination: "C:\\Exports\\hero.asset-export-v1",
      resref: "hero", mode: "animated", glbPath: "hero.glb", glbSha256: "a".repeat(64),
      glbSizeBytes: 2048, animationCount: 2, textureCount: 2, warnings: [],
      manifest: { schemaVersion: animatedPreview.schemaVersion, generator: "test", classification: animatedPreview.classification,
        redistribution: animatedPreview.redistribution, sourceModuleSha256: "module", sourceModel: "hero.mdl",
        sourceModelSha256: "model", sourceDependencies: {}, mode: "animated", animations: animatedPreview.animations,
        textures: animatedPreview.textures, warnings: [], files: [], sourceModuleImmutable: true },
    });
  });

  it("stays locked and read-only until an analysis is available", () => {
    render(<AssetExportWorkspace analysisReady={false} />);
    expect(screen.getByRole("region", { name: "Export d’assets" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Analyse requise" })).toBeInTheDocument();
    expect(screen.getByText(/sources restent en lecture seule/)).toBeInTheDocument();
    expect(api.listAssetExportCandidates).not.toHaveBeenCalled();
  });

  it("distinguishes animated and static models and exposes exported clips", async () => {
    render(<AssetExportWorkspace jobId="analysis-1" analysisReady />);
    expect(await screen.findByText("Asset animé")).toBeInTheDocument();
    expect(screen.getByText("walk")).toBeInTheDocument();
    expect(screen.getByText("2 clip(s) GLB")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Statiques" }));
    expect(screen.queryByRole("button", { name: /hero/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /crate/i })).toBeInTheDocument();
  });

  it("exports only after destination and local-only acknowledgement", async () => {
    render(<AssetExportWorkspace jobId="analysis-1" analysisReady />);
    await screen.findByText("Asset animé");
    const launch = screen.getByRole("button", { name: "Exporter l’asset" });
    expect(launch).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Choisir la destination" }));
    expect(await screen.findByText("C:\\Exports\\hero.asset-export-v1")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox", { name: /cet export reste local/i }));
    expect(launch).toBeEnabled();
    fireEvent.click(launch);
    expect(await screen.findByRole("region", { name: "Résultat de l’export d’asset" })).toHaveTextContent("2 animation(s)");
    await waitFor(() => expect(api.exportAssetBundle).toHaveBeenCalledWith({
      analysisJobId: "analysis-1", resref: "hero",
      destination: "C:\\Exports\\hero.asset-export-v1", localOnlyAcknowledged: true,
    }));
  });
});
