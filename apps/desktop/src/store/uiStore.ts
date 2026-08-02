import { create } from "zustand";

type UiState = {
  activeExplorerItem: string;
  setActiveExplorerItem: (id: string) => void;
};

export const useUiStore = create<UiState>((set) => ({
  activeExplorerItem: "module",
  setActiveExplorerItem: (activeExplorerItem) => set({ activeExplorerItem }),
}));
