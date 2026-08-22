import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DialogueExportWorkspace } from "./DialogueExportWorkspace";

const api = vi.hoisted(() => ({
  listDialogueExportCandidates: vi.fn(),
  previewDialogueExport: vi.fn(),
  exportDialogueBundle: vi.fn(),
  selectDirectory: vi.fn(),
}));

vi.mock("../../lib/tauri", () => ({
  ...api,
  normalizeAppError: (error: unknown) => ({
    userMessage: error instanceof Error ? error.message : String(error),
  }),
}));

const preview = {
  schemaVersion: "opennever-dialogue-export@1.0.0",
  resref: "guard",
  revision: "workspace",
  ready: true,
  suggestedDirectoryName: "guard.dialogue-export-v1",
  sourceResourceSha256: "d".repeat(64),
  nodeCount: 2,
  entryCount: 1,
  replyCount: 1,
  linkCount: 1,
  rootCount: 1,
  sharedNodeCount: 0,
  unreachableNodeCount: 0,
  cycleCount: 0,
  brokenLinkCount: 0,
  diagnosticCount: 0,
  referenceCount: 1,
  scripts: ["can_enter", "open_gate"],
  transcriptPreview: ["- **Gardien** : Bienvenue, voyageur. `entry:0`"],
  warnings: [],
  classification: "local_only_proprietary",
  redistribution: "not_redistributable_without_separate_rights",
};

describe("DialogueExportWorkspace", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.listDialogueExportCandidates.mockResolvedValue([
      { resref: "guard", nodeCount: 2, linkCount: 1, cycleCount: 0, diagnosticCount: 0, preview: "Bienvenue, voyageur." },
      { resref: "merchant", nodeCount: 8, linkCount: 7, cycleCount: 1, diagnosticCount: 1, preview: "Que désirez-vous ?" },
    ]);
    api.previewDialogueExport.mockResolvedValue(preview);
    api.selectDirectory.mockResolvedValue("C:\\Exports");
    api.exportDialogueBundle.mockResolvedValue({
      schemaVersion: preview.schemaVersion,
      destination: "C:\\Exports\\guard.dialogue-export-v1",
      resref: "guard",
      revision: "workspace",
      sourceResourceSha256: preview.sourceResourceSha256,
      nodeCount: 2,
      linkCount: 1,
      fileCount: 4,
      totalSizeBytes: 4096,
      warnings: [],
      manifest: {
        schemaVersion: preview.schemaVersion,
        generator: "test",
        classification: preview.classification,
        redistribution: preview.redistribution,
        resref: "guard",
        revision: "workspace",
        sourceResourceSha256: preview.sourceResourceSha256,
        nodeCount: 2,
        linkCount: 1,
        rootCount: 1,
        brokenLinkCount: 0,
        cycleCount: 0,
        scripts: preview.scripts,
        warnings: [],
        files: [],
        sourceNwnImmutable: true,
      },
    });
  });

  it("stays locked and read-only until an analysis is available", () => {
    render(<DialogueExportWorkspace analysisReady={false} />);
    expect(screen.getByRole("region", { name: "Export de dialogues" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Analyse requise" })).toBeInTheDocument();
    expect(screen.getByText(/sources restent en lecture seule/)).toBeInTheDocument();
    expect(api.listDialogueExportCandidates).not.toHaveBeenCalled();
  });

  it("previews the workspace revision, transcript and scripts", async () => {
    render(<DialogueExportWorkspace jobId="analysis-1" analysisReady workspaceId="workspace-1" />);
    expect(await screen.findByText("Version modifiée du workspace")).toBeInTheDocument();
    expect(screen.getAllByText(/Bienvenue, voyageur/)).toHaveLength(2);
    expect(screen.getByText(/can_enter, open_gate/)).toBeInTheDocument();
    expect(api.previewDialogueExport).toHaveBeenCalledWith({
      analysisJobId: "analysis-1",
      workspaceId: "workspace-1",
      resref: "guard",
    });
    fireEvent.change(screen.getByRole("textbox", { name: "Rechercher un dialogue à exporter" }), { target: { value: "marchand absent" } });
    expect(screen.getByText("Aucun dialogue ne correspond.")).toBeInTheDocument();
  });

  it("exports only after destination and local-only acknowledgement", async () => {
    render(<DialogueExportWorkspace jobId="analysis-1" analysisReady workspaceId="workspace-1" />);
    await screen.findByText("Version modifiée du workspace");
    const launch = screen.getByRole("button", { name: "Exporter le dialogue" });
    expect(launch).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "Choisir la destination" }));
    expect(await screen.findByText("C:\\Exports\\guard.dialogue-export-v1")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("checkbox", { name: /cet export reste local/i }));
    expect(launch).toBeEnabled();
    fireEvent.click(launch);
    expect(await screen.findByRole("region", { name: "Résultat de l’export de dialogue" })).toHaveTextContent("4 fichier(s)");
    await waitFor(() => expect(api.exportDialogueBundle).toHaveBeenCalledWith({
      analysisJobId: "analysis-1",
      workspaceId: "workspace-1",
      resref: "guard",
      destination: "C:\\Exports\\guard.dialogue-export-v1",
      expectedSourceResourceSha256: preview.sourceResourceSha256,
      localOnlyAcknowledged: true,
    }));
  });
});
