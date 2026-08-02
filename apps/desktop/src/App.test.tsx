import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { startModuleAnalysis } from "./lib/tauri";

vi.mock("./lib/tauri", () => ({
  getAppStatus: vi.fn().mockResolvedValue({
    appVersion: "0.1.0-test",
    readOnly: true,
    databaseSchemaVersion: 1,
  }),
  getJob: vi.fn().mockResolvedValue({
    id: "job-1",
    kind: "module_analysis",
    state: "completed",
    sourcePath: "C:/module.mod",
    progress: { bytesRead: 512, totalBytes: 512, percent: 100 },
    result: {
      fingerprint: { sha256: "ABC123", sizeBytes: 512 },
      moduleInfo: {
        name: { stringRef: null, values: [{ languageId: 0, text: "Forge Test" }] },
        description: {
          stringRef: null,
          values: [{ languageId: 0, text: "Synthetic module" }],
        },
        tag: "MODULE",
        minimumGameVersion: "1.69",
        customTlk: null,
        entryArea: "startarea",
        hakFiles: [],
      },
      inventory: {
        fileType: "MOD ",
        fileVersion: "V1.0",
        buildYear: 2026,
        buildDay: 213,
        resourceCount: 2,
        resources: [
          {
            key: { resref: "module", resourceType: 2014 },
            resourceId: 0,
            extension: "ifo",
            offset: 224,
            size: 128,
          },
          {
            key: { resref: "start", resourceType: 2009 },
            resourceId: 1,
            extension: "nss",
            offset: 352,
            size: 160,
          },
        ],
        typeSummaries: [
          { resourceType: 2009, extension: "nss", count: 1, totalSize: 160 },
          { resourceType: 2014, extension: "ifo", count: 1, totalSize: 128 },
        ],
      },
    },
  }),
  startModuleAnalysis: vi.fn().mockResolvedValue({
    id: "job-1",
    kind: "module_analysis",
    state: "queued",
    sourcePath: "C:/module.mod",
    progress: { bytesRead: 0, totalBytes: 0, percent: 0 },
  }),
  cancelJob: vi.fn(),
  selectDirectory: vi.fn(),
  selectModule: vi.fn(),
  normalizeAppError: vi.fn((error) => error),
}));

function renderApp() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  );
}

describe("OpenNever Forge shell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the explorer, workbench, inspector and diagnostics", async () => {
    renderApp();

    expect(screen.getByLabelText("Explorateur du module")).toBeInTheDocument();
    expect(screen.getByLabelText("Zone de travail")).toBeInTheDocument();
    expect(screen.getByLabelText("Inspecteur")).toBeInTheDocument();
    expect(screen.getByLabelText("Diagnostics")).toBeInTheDocument();
    expect(await screen.findByText("Cœur Rust · v0.1.0-test")).toBeInTheDocument();
  });

  it("starts the hash job only after a module path is provided", async () => {
    renderApp();
    const action = screen.getByRole("button", { name: "Analyser la copie" });
    expect(action).toBeDisabled();

    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), {
      target: { value: "C:/module.mod" },
    });
    fireEvent.click(action);

    await waitFor(() => expect(startModuleAnalysis).toHaveBeenCalledWith("C:/module.mod"));
  });

  it("renders and filters the ERF inventory returned by Rust", async () => {
    renderApp();
    fireEvent.change(screen.getByPlaceholderText("Sélectionner un fichier .mod"), {
      target: { value: "C:/module.mod" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Analyser la copie" }));

    expect(await screen.findByRole("table", { name: "Ressources du module" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Forge Test" })).toBeInTheDocument();
    expect(screen.getByText("1 ressource(s) dans cette catégorie.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Ressources (2)" }));
    expect(screen.getByText("2 ressource(s) dans cette catégorie.")).toBeInTheDocument();
    expect(screen.getByText("module")).toBeInTheDocument();
    expect(screen.getByText("start")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Filtrer les ressources"), {
      target: { value: "module.ifo" },
    });
    expect(screen.getByText("module")).toBeInTheDocument();
    expect(screen.queryByText("start")).not.toBeInTheDocument();
  });
});
