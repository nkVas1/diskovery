import { create } from "zustand";
import {
  cancelDedup,
  dedupResults,
  startDedup,
  type DedupResults,
} from "@/shared/ipc";

type DedupStatus = "idle" | "running" | "done" | "error";

interface DedupProgress {
  stage: "collecting" | "prehashing" | "hashing";
  done: number;
  total: number;
}

interface DedupStore {
  status: DedupStatus;
  progress: DedupProgress;
  results: DedupResults | null;
  /** how many groups are currently loaded into `results.groups` */
  loaded: number;
  error: string | null;
  minSize: number;
  setMinSize: (n: number) => void;
  start: () => Promise<void>;
  cancel: () => Promise<void>;
  refresh: () => Promise<void>;
  loadMore: () => Promise<void>;
  reset: () => void;
}

const PAGE = 50;

export const useDedup = create<DedupStore>((set, get) => ({
  status: "idle",
  progress: { stage: "collecting", done: 0, total: 0 },
  results: null,
  loaded: 0,
  error: null,
  minSize: 1024 * 1024,

  setMinSize: (minSize) => set({ minSize }),

  start: async () => {
    if (get().status === "running") return;
    set({ status: "running", results: null, loaded: 0, error: null });
    try {
      await startDedup(get().minSize, (e) => {
        if (e.type === "progress") {
          set({ progress: { stage: e.stage, done: e.done, total: e.total } });
        } else if (e.type === "done") {
          void get().refresh();
        } else if (e.message === "cancelled") {
          set({ status: "idle" });
        } else {
          set({ status: "error", error: e.message });
        }
      });
    } catch (err) {
      set({ status: "error", error: String(err) });
    }
  },

  cancel: async () => {
    await cancelDedup().catch(() => undefined);
  },

  refresh: async () => {
    try {
      const count = Math.max(get().loaded, PAGE);
      const results = await dedupResults(0, count);
      set({ status: "done", results, loaded: results.groups.length });
    } catch (err) {
      set({ status: "error", error: String(err) });
    }
  },

  loadMore: async () => {
    const { results, loaded } = get();
    if (!results) return;
    try {
      const next = await dedupResults(loaded, PAGE);
      set({
        results: { ...next, groups: [...results.groups, ...next.groups] },
        loaded: loaded + next.groups.length,
      });
    } catch {
      // keep current page
    }
  },

  reset: () => set({ status: "idle", results: null, loaded: 0, error: null }),
}));
