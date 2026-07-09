import { create } from "zustand";
import {
  cancelScan,
  getScanSummary,
  listVolumes,
  startScan,
  type ScanSummary,
  type VolumeInfo,
} from "@/shared/ipc";

export type ScanStatus = "idle" | "scanning" | "done" | "error";

interface ScanProgress {
  files: number;
  dirs: number;
  bytes: number;
  errors: number;
  currentPath: string;
  elapsedMs: number;
}

const emptyProgress: ScanProgress = {
  files: 0,
  dirs: 0,
  bytes: 0,
  errors: 0,
  currentPath: "",
  elapsedMs: 0,
};

interface ScanStore {
  status: ScanStatus;
  volumes: VolumeInfo[];
  target: string | null;
  /** Used bytes of the volume when scanning a volume root — drives the % estimate. */
  expectedBytes: number | null;
  progress: ScanProgress;
  summary: ScanSummary | null;
  error: string | null;
  loadVolumes: () => Promise<void>;
  start: (path: string, expectedBytes?: number) => Promise<void>;
  cancel: () => Promise<void>;
  reset: () => void;
}

export const useScan = create<ScanStore>((set, get) => ({
  status: "idle",
  volumes: [],
  target: null,
  expectedBytes: null,
  progress: emptyProgress,
  summary: null,
  error: null,

  loadVolumes: async () => {
    try {
      set({ volumes: await listVolumes() });
    } catch {
      // non-fatal; cards simply won't render
    }
  },

  start: async (path, expectedBytes) => {
    if (get().status === "scanning") return;
    set({
      status: "scanning",
      target: path,
      expectedBytes: expectedBytes ?? null,
      progress: emptyProgress,
      summary: null,
      error: null,
    });
    try {
      await startScan(path, (e) => {
        if (e.type === "progress") {
          const { type: _, ...progress } = e;
          set({ progress });
        } else if (e.type === "done") {
          void getScanSummary()
            .then((summary) => set({ status: "done", summary }))
            .catch((err) => set({ status: "error", error: String(err) }));
        } else if (e.message === "cancelled") {
          set({ status: "idle", target: null });
        } else {
          set({ status: "error", error: e.message });
        }
      });
    } catch (err) {
      set({ status: "error", error: String(err) });
    }
  },

  cancel: async () => {
    await cancelScan().catch(() => undefined);
  },

  reset: () => set({ status: "idle", target: null, summary: null, error: null }),
}));
