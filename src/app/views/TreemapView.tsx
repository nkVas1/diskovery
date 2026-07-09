import { useCallback, useEffect, useRef, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";
import { useScan } from "@/app/scanStore";
import { useApp } from "@/app/store";
import { formatBytes, middleTruncate } from "@/shared/format";
import {
  openItem,
  revealItem,
  trashItem,
  treemapHit,
  treemapMeta,
  treemapRender,
  type LabelRect,
  type TreemapHit,
  type TreemapMeta,
} from "@/shared/ipc";

/** Mirror of the Rust category palette — fixed order, index = category id. */
const CATEGORIES = [
  { name: "Video", color: "#3987e5" },
  { name: "Images", color: "#199e70" },
  { name: "Documents", color: "#c98500" },
  { name: "Code", color: "#008300" },
  { name: "Other", color: "#9085e9" },
  { name: "Apps", color: "#e66767" },
  { name: "Audio", color: "#d55181" },
  { name: "Archives", color: "#d95926" },
  { name: "Folders", color: "#515d71" },
] as const;

/** Root persists across view switches. */
const useTreemapRoot = create<{ root: number; setRoot: (n: number) => void }>((set) => ({
  root: 0,
  setRoot: (root) => set({ root }),
}));

function drawLabels(ctx: CanvasRenderingContext2D, labels: LabelRect[]) {
  ctx.font = "600 11px 'Inter Variable', sans-serif";
  ctx.textBaseline = "middle";
  for (const l of labels) {
    if (l.w < 76 || l.h < 26) continue;
    const text = l.name;
    const tw = Math.min(ctx.measureText(text).width, l.w - 20);
    const bx = l.x + 5;
    const by = l.y + 5;
    ctx.fillStyle = "rgba(6, 9, 17, 0.62)";
    ctx.beginPath();
    ctx.roundRect(bx, by, tw + 14, 18, 5);
    ctx.fill();
    ctx.fillStyle = "#e7ecf5";
    ctx.save();
    ctx.beginPath();
    ctx.rect(bx + 7, by, tw, 18);
    ctx.clip();
    ctx.fillText(text, bx + 7, by + 9.5);
    ctx.restore();
  }
}

export default function TreemapView() {
  const summary = useScan((s) => s.summary);
  const refreshSummary = useScan((s) => s.refreshSummary);
  const setView = useApp((s) => s.setView);
  const { root, setRoot } = useTreemapRoot();

  const containerRef = useRef<HTMLDivElement>(null);
  const baseRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const lastHitAt = useRef(0);
  const renderSeq = useRef(0);

  const [meta, setMeta] = useState<TreemapMeta | null>(null);
  const [hover, setHover] = useState<TreemapHit | null>(null);
  const [selected, setSelected] = useState<TreemapHit | null>(null);
  const [busy, setBusy] = useState(false);

  const render = useCallback(
    async (rootId: number) => {
      const el = containerRef.current;
      if (!el || !summary) return;
      const w = Math.floor(el.clientWidth);
      const h = Math.floor(el.clientHeight);
      if (w < 64 || h < 64) return;
      const seq = ++renderSeq.current;
      setBusy(true);
      try {
        const buf = await treemapRender(rootId, w, h);
        if (seq !== renderSeq.current) return;
        const canvas = baseRef.current;
        const overlay = overlayRef.current;
        if (!canvas || !overlay) return;
        canvas.width = w;
        canvas.height = h;
        overlay.width = w;
        overlay.height = h;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;
        ctx.putImageData(new ImageData(new Uint8ClampedArray(buf), w, h), 0, 0);
        const m = await treemapMeta();
        if (seq !== renderSeq.current) return;
        drawLabels(ctx, m.labels);
        setMeta(m);
        setHover(null);
        setSelected(null);
        setRoot(rootId);
      } catch {
        // no scan yet — empty state handles it
      } finally {
        if (seq === renderSeq.current) setBusy(false);
      }
    },
    [summary, setRoot],
  );

  // initial render + re-render when a new scan lands
  useEffect(() => {
    void render(root);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [summary]);

  // re-render on container resize (debounced)
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    let t: ReturnType<typeof setTimeout> | null = null;
    const ro = new ResizeObserver(() => {
      if (t) clearTimeout(t);
      t = setTimeout(() => void render(useTreemapRoot.getState().root), 220);
    });
    ro.observe(el);
    return () => {
      if (t) clearTimeout(t);
      ro.disconnect();
    };
  }, [render]);

  const drawOutline = useCallback((hit: TreemapHit | null, strong: boolean) => {
    const overlay = overlayRef.current;
    if (!overlay) return;
    const ctx = overlay.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, overlay.width, overlay.height);
    if (!hit) return;
    const [x, y, w, h] = hit.rect;
    ctx.strokeStyle = strong ? "#ffffff" : "rgba(255,255,255,0.75)";
    ctx.lineWidth = strong ? 2 : 1.25;
    ctx.strokeRect(x + 0.5, y + 0.5, Math.max(1, w - 1), Math.max(1, h - 1));
  }, []);

  const onMove = useCallback(
    async (e: React.MouseEvent<HTMLCanvasElement>) => {
      const now = performance.now();
      if (now - lastHitAt.current < 30) return;
      lastHitAt.current = now;
      const rect = e.currentTarget.getBoundingClientRect();
      try {
        const hit = await treemapHit(e.clientX - rect.left, e.clientY - rect.top);
        setHover(hit);
        drawOutline(selected ?? hit, selected !== null);
      } catch {
        /* layout gone */
      }
    },
    [drawOutline, selected],
  );

  const onClick = useCallback(() => {
    setSelected(hover);
    drawOutline(hover, true);
  }, [hover, drawOutline]);

  const onDoubleClick = useCallback(() => {
    if (!hover) return;
    const target = hover.topDir;
    if (target !== root && (target !== hover.id || hover.isDir)) {
      void render(target);
    }
  }, [hover, root, render]);

  const onTrash = useCallback(async (hit: TreemapHit) => {
    const yes = await ask(
      `Move "${hit.name}" (${formatBytes(hit.size)}) to the Recycle Bin?`,
      { title: "Diskovery", kind: "warning" },
    );
    if (!yes) return;
    try {
      await trashItem(hit.id);
      await refreshSummary();
      await render(useTreemapRoot.getState().root);
    } catch (err) {
      await ask(`Could not delete: ${String(err)}`, { title: "Diskovery", kind: "error" });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshSummary, render]);

  if (!summary) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <h1 className="text-gradient text-2xl font-bold">No scan yet</h1>
          <p className="mt-2 text-sm text-ink-mute">Run a scan to draw the treemap.</p>
          <button
            onClick={() => setView("dashboard")}
            className="mt-6 rounded-xl bg-gradient-glow px-5 py-2.5 text-sm font-semibold text-void"
          >
            Go to scan
          </button>
        </div>
      </div>
    );
  }

  const active = selected ?? hover;

  return (
    <div className="flex h-full flex-col">
      {/* breadcrumb bar */}
      <div className="flex h-11 shrink-0 items-center gap-1 overflow-x-auto border-b border-edge-soft px-4">
        {meta?.breadcrumb.map((c, i) => (
          <span key={c.id} className="flex items-center gap-1">
            {i > 0 && <span className="text-ink-faint">›</span>}
            <button
              onClick={() => void render(c.id)}
              className={`max-w-48 truncate rounded-md px-1.5 py-0.5 font-mono text-[12px] transition-colors ${
                c.id === root ? "text-ink" : "text-ink-faint hover:bg-panel hover:text-ink"
              }`}
            >
              {c.name}
            </button>
          </span>
        ))}
        <div className="flex-1" />
        {busy && (
          <span className="shrink-0 animate-pulse font-mono text-[11px] text-ink-faint">
            rendering…
          </span>
        )}
      </div>

      {/* canvas */}
      <div ref={containerRef} className="relative min-h-0 flex-1">
        <canvas ref={baseRef} className="absolute inset-0" />
        <canvas
          ref={overlayRef}
          className="absolute inset-0 cursor-crosshair"
          onMouseMove={(e) => void onMove(e)}
          onClick={onClick}
          onDoubleClick={onDoubleClick}
          onMouseLeave={() => {
            setHover(null);
            drawOutline(selected, true);
          }}
        />
      </div>

      {/* info + legend bar */}
      <div className="shrink-0 border-t border-edge-soft bg-panel px-4 py-2">
        <div className="flex h-9 items-center gap-3">
          {active ? (
            <>
              <span
                className="h-2.5 w-2.5 shrink-0 rounded-full"
                style={{ background: CATEGORIES[active.category]?.color }}
              />
              <span className="shrink-0 text-[13px] font-semibold">{active.name}</span>
              <span className="min-w-0 truncate font-mono text-[11px] text-ink-faint" title={active.path}>
                {middleTruncate(active.path, 80)}
              </span>
              <span className="shrink-0 font-mono text-[12px] tabular-nums text-ink-mute">
                {formatBytes(active.size)}
              </span>
              <div className="flex-1" />
              {selected && (
                <div className="flex shrink-0 gap-1.5">
                  {!selected.isDir && (
                    <BarButton onClick={() => void openItem(selected.id)}>Open</BarButton>
                  )}
                  <BarButton onClick={() => void revealItem(selected.id)}>Reveal</BarButton>
                  <BarButton danger onClick={() => void onTrash(selected)}>
                    Recycle
                  </BarButton>
                </div>
              )}
            </>
          ) : (
            <>
              <span className="text-[12px] text-ink-faint">
                Hover to inspect · click to select · double-click to drill in
              </span>
              <div className="flex-1" />
              <div className="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1">
                {CATEGORIES.map((c) => (
                  <span key={c.name} className="flex items-center gap-1.5 text-[11px] text-ink-faint">
                    <span className="h-2 w-2 rounded-full" style={{ background: c.color }} />
                    {c.name}
                  </span>
                ))}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

function BarButton({
  children,
  onClick,
  danger = false,
}: {
  children: React.ReactNode;
  onClick: () => void;
  danger?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      className={`rounded-lg border border-edge px-3 py-1 text-[12px] font-semibold transition-colors ${
        danger
          ? "text-ink-mute hover:border-danger hover:text-danger"
          : "text-ink-mute hover:border-ink-faint hover:text-ink"
      }`}
    >
      {children}
    </button>
  );
}
