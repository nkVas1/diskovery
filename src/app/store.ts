import { create } from "zustand";

export type ViewId =
  | "dashboard"
  | "treemap"
  | "duplicates"
  | "advisor"
  | "ai"
  | "settings";

interface AppState {
  view: ViewId;
  setView: (view: ViewId) => void;
}

export const useApp = create<AppState>((set) => ({
  view: "dashboard",
  setView: (view) => set({ view }),
}));
