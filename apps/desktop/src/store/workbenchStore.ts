import { create } from "zustand";

type WorkbenchState = {
  activeExplorerItem: string;
  setActiveExplorerItem: (id: string) => void;
  lastContentView: string;
  setLastContentView: (view: string) => void;
  agentObjectiveDraft: string;
  setAgentObjectiveDraft: (draft: string) => void;
};

export const useWorkbenchStore = create<WorkbenchState>((set) => ({
  activeExplorerItem: "module",
  setActiveExplorerItem: (activeExplorerItem) => set({ activeExplorerItem }),
  lastContentView: "module",
  setLastContentView: (lastContentView) => set({ lastContentView }),
  agentObjectiveDraft: "",
  setAgentObjectiveDraft: (agentObjectiveDraft) => set({ agentObjectiveDraft }),
}));
