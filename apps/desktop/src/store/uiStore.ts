import { create } from "zustand";

type UiState = {
  explorerOpen: boolean;
  inspectorOpen: boolean;
  diagnosticsOpen: boolean;
  setExplorerOpen: (open: boolean) => void;
  toggleExplorerOpen: () => void;
  setInspectorOpen: (open: boolean) => void;
  toggleInspectorOpen: () => void;
  setDiagnosticsOpen: (open: boolean) => void;
  toggleDiagnosticsOpen: () => void;
};

export const useUiStore = create<UiState>((set) => ({
  explorerOpen: true,
  inspectorOpen: true,
  diagnosticsOpen: false,
  setExplorerOpen: (explorerOpen) => set({ explorerOpen }),
  toggleExplorerOpen: () =>
    set((state) => ({ explorerOpen: !state.explorerOpen })),
  setInspectorOpen: (inspectorOpen) => set({ inspectorOpen }),
  toggleInspectorOpen: () =>
    set((state) => ({ inspectorOpen: !state.inspectorOpen })),
  setDiagnosticsOpen: (diagnosticsOpen) => set({ diagnosticsOpen }),
  toggleDiagnosticsOpen: () =>
    set((state) => ({ diagnosticsOpen: !state.diagnosticsOpen })),
}));
