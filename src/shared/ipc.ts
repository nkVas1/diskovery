import { Channel, invoke } from "@tauri-apps/api/core";

export interface AppInfo {
  name: string;
  version: string;
}

export interface VolumeInfo {
  path: string;
  label: string;
  fs: string;
  total: number;
  free: number;
  kind: "ssd" | "hdd" | "unknown";
  removable: boolean;
}

export type ScanEvent =
  | {
      type: "progress";
      files: number;
      dirs: number;
      bytes: number;
      errors: number;
      currentPath: string;
      elapsedMs: number;
    }
  | {
      type: "done";
      files: number;
      dirs: number;
      bytes: number;
      errors: number;
      elapsedMs: number;
    }
  | { type: "error"; message: string };

export interface NodeDto {
  id: number;
  name: string;
  size: number;
  mtime: number;
  isDir: boolean;
  childCount: number;
}

export interface FileDto {
  id: number;
  name: string;
  path: string;
  size: number;
}

export interface ExtStat {
  ext: string;
  bytes: number;
  count: number;
}

export interface ScanSummary {
  rootPath: string;
  files: number;
  dirs: number;
  bytes: number;
  errors: number;
  elapsedMs: number;
  topDirs: NodeDto[];
  topFiles: FileDto[];
  topExts: ExtStat[];
}

export const getAppInfo = () => invoke<AppInfo>("app_info");
export const listVolumes = () => invoke<VolumeInfo[]>("list_volumes");

export function startScan(path: string, onEvent: (e: ScanEvent) => void) {
  const channel = new Channel<ScanEvent>();
  channel.onmessage = onEvent;
  return invoke<void>("start_scan", { path, onEvent: channel });
}

export const cancelScan = () => invoke<void>("cancel_scan");
export const getScanSummary = () => invoke<ScanSummary>("scan_summary");
export const getChildren = (id: number) => invoke<NodeDto[]>("get_children", { id });
export const getNodePath = (id: number) => invoke<string>("node_path", { id });

/* ---- treemap ---- */

export interface LabelRect {
  id: number;
  name: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Crumb {
  id: number;
  name: string;
}

export interface TreemapMeta {
  root: number;
  labels: LabelRect[];
  breadcrumb: Crumb[];
}

export interface TreemapHit {
  id: number;
  topDir: number;
  name: string;
  path: string;
  size: number;
  isDir: boolean;
  category: number;
  rect: [number, number, number, number];
}

export const treemapRender = (root: number, width: number, height: number) =>
  invoke<ArrayBuffer>("treemap_render", { root, width, height });
export const treemapMeta = () => invoke<TreemapMeta>("treemap_meta");
export const treemapHit = (x: number, y: number) =>
  invoke<TreemapHit | null>("treemap_hit", { x, y });

/* ---- file ops ---- */

export const openItem = (id: number) => invoke<void>("open_item", { id });
export const revealItem = (id: number) => invoke<void>("reveal_item", { id });
export const trashItem = (id: number) => invoke<number>("trash_item", { id });
