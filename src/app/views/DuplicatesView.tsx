import { ask } from "@tauri-apps/plugin-dialog";
import { useDedup } from "@/app/dedupStore";
import { useScan } from "@/app/scanStore";
import { useApp } from "@/app/store";
import { formatBytes, formatDuration, formatInt, middleTruncate } from "@/shared/format";
import { revealItem, trashItem, type DupGroup } from "@/shared/ipc";

const MIN_SIZES = [
  { label: "All files", value: 1 },
  { label: "≥ 64 KB", value: 64 * 1024 },
  { label: "≥ 1 MB", value: 1024 * 1024 },
  { label: "≥ 16 MB", value: 16 * 1024 * 1024 },
];

const STAGE_LABEL: Record<string, string> = {
  collecting: "Collecting candidates",
  prehashing: "Prehashing (first 16 KB)",
  hashing: "Full BLAKE3 verification",
};

function fmtDate(mtime: number) {
  return mtime > 0 ? new Date(mtime * 1000).toLocaleDateString() : "—";
}

/* ---------------- states ---------------- */

function ReadyState() {
  const { minSize, setMinSize, start } = useDedup();
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="max-w-md text-center">
        <h1 className="text-gradient text-3xl font-bold tracking-tight">Duplicate Lab</h1>
        <p className="mt-3 text-sm leading-relaxed text-ink-mute">
          Three-stage pipeline: size groups → 16 KB prehash → full BLAKE3. Only
          byte-identical files are reported; hardlinks are recognized and never
          counted twice. Hashes are cached — re-runs are instant.
        </p>
        <div className="mt-6 flex justify-center gap-1.5">
          {MIN_SIZES.map((o) => (
            <button
              key={o.value}
              onClick={() => setMinSize(o.value)}
              className={`rounded-lg border px-3 py-1.5 text-[12px] font-semibold transition-colors ${
                minSize === o.value
                  ? "border-transparent bg-overlay text-ink"
                  : "border-edge text-ink-faint hover:text-ink-mute"
              }`}
            >
              {o.label}
            </button>
          ))}
        </div>
        <button
          onClick={() => void start()}
          className="mt-6 rounded-xl bg-gradient-glow px-6 py-2.5 text-sm font-semibold text-void"
        >
          Find duplicates
        </button>
      </div>
    </div>
  );
}

function RunningState() {
  const { progress, cancel } = useDedup();
  const isBytes = progress.stage === "hashing";
  const pct = progress.total > 0 ? Math.min(100, (progress.done / progress.total) * 100) : 0;
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="w-full max-w-md">
        <div className="flex items-center gap-3">
          <span className="relative flex h-2.5 w-2.5">
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-glow-violet opacity-60" />
            <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-glow-violet" />
          </span>
          <h1 className="text-lg font-bold">{STAGE_LABEL[progress.stage] ?? progress.stage}</h1>
        </div>
        <div className="mt-5 h-2 w-full overflow-hidden rounded-full bg-data-track">
          <div className="h-full rounded-full bg-data-violet" style={{ width: `${pct}%` }} />
        </div>
        <p className="mt-2 text-right font-mono text-[12px] tabular-nums text-ink-faint">
          {isBytes
            ? `${formatBytes(progress.done)} / ${formatBytes(progress.total)}`
            : `${formatInt(progress.done)} / ${formatInt(progress.total)} files`}
        </p>
        <button
          onClick={() => void cancel()}
          className="mt-6 rounded-xl border border-edge px-5 py-2 text-[13px] font-semibold text-ink-mute hover:border-danger hover:text-danger"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

function GroupCard({ group }: { group: DupGroup }) {
  const refresh = useDedup((s) => s.refresh);
  const refreshSummary = useScan((s) => s.refreshSummary);

  const onTrashFile = async (id: number, name: string) => {
    const yes = await ask(`Move "${name}" to the Recycle Bin?`, {
      title: "Diskovery",
      kind: "warning",
    });
    if (!yes) return;
    await trashItem(id).catch(() => undefined);
    await refresh();
    await refreshSummary();
  };

  const onKeepNewest = async () => {
    const newest = group.files.reduce((a, b) => (b.mtime > a.mtime ? b : a));
    const victims = group.files.filter((f) => f.id !== newest.id);
    const yes = await ask(
      `Keep "${newest.name}" (newest) and move ${victims.length} older cop${
        victims.length === 1 ? "y" : "ies"
      } (${formatBytes(group.size * victims.length)}) to the Recycle Bin?`,
      { title: "Diskovery", kind: "warning" },
    );
    if (!yes) return;
    for (const v of victims) {
      await trashItem(v.id).catch(() => undefined);
    }
    await refresh();
    await refreshSummary();
  };

  return (
    <div className="rounded-2xl border border-edge-soft bg-panel p-4">
      <div className="flex items-center gap-3">
        <span className="font-mono text-[13px] font-semibold tabular-nums">
          {group.files.length} × {formatBytes(group.size)}
        </span>
        <span className="rounded-md bg-overlay px-1.5 py-0.5 font-mono text-[10px] text-ink-faint">
          {group.hash}
        </span>
        <span className="text-[12px] text-warn">wastes {formatBytes(group.wasted)}</span>
        <div className="flex-1" />
        <button
          onClick={() => void onKeepNewest()}
          className="rounded-lg border border-edge px-3 py-1 text-[12px] font-semibold text-ink-mute hover:border-ink-faint hover:text-ink"
        >
          Keep newest
        </button>
      </div>
      <div className="mt-3 space-y-1">
        {group.files.map((f) => (
          <div
            key={f.id}
            className="flex items-center gap-3 rounded-lg px-2 py-1 hover:bg-raised"
          >
            <div className="min-w-0 flex-1">
              <p className="truncate text-[13px]">{f.name}</p>
              <p className="truncate font-mono text-[11px] text-ink-faint" title={f.dir}>
                {middleTruncate(f.dir, 64)}
              </p>
            </div>
            <span className="shrink-0 font-mono text-[11px] tabular-nums text-ink-faint">
              {fmtDate(f.mtime)}
            </span>
            <div className="flex shrink-0 gap-1">
              <button
                onClick={() => void revealItem(f.id)}
                className="rounded-md px-2 py-0.5 text-[11px] font-semibold text-ink-faint hover:bg-overlay hover:text-ink"
              >
                Reveal
              </button>
              <button
                onClick={() => void onTrashFile(f.id, f.name)}
                className="rounded-md px-2 py-0.5 text-[11px] font-semibold text-ink-faint hover:bg-overlay hover:text-danger"
              >
                Recycle
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function DoneState() {
  const { results, loaded, loadMore, reset } = useDedup();
  if (!results) return null;

  return (
    <div className="mx-auto max-w-4xl px-8 py-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            <span className="text-gradient">{formatBytes(results.totalWasted)}</span> wasted
          </h1>
          <p className="mt-1 text-sm text-ink-mute">
            {formatInt(results.totalGroups)} duplicate groups · hashed{" "}
            {formatBytes(results.hashedBytes)} in {formatDuration(results.elapsedMs)} ·{" "}
            {formatInt(results.cacheHits)} cache hits
          </p>
        </div>
        <button
          onClick={reset}
          className="rounded-xl border border-edge px-4 py-2 text-[13px] font-semibold text-ink-mute hover:text-ink"
        >
          New search
        </button>
      </div>

      {results.totalGroups === 0 ? (
        <div className="mt-16 text-center">
          <p className="text-lg font-semibold text-ok">No duplicates found</p>
          <p className="mt-1 text-sm text-ink-mute">Every file above the size threshold is unique.</p>
        </div>
      ) : (
        <>
          <div className="mt-7 space-y-3">
            {results.groups.map((g) => (
              <GroupCard key={`${g.hash}-${g.size}`} group={g} />
            ))}
          </div>
          {loaded < results.totalGroups && (
            <button
              onClick={() => void loadMore()}
              className="mt-5 w-full rounded-xl border border-edge py-2.5 text-[13px] font-semibold text-ink-mute hover:text-ink"
            >
              Show more ({formatInt(results.totalGroups - loaded)} groups left)
            </button>
          )}
        </>
      )}
    </div>
  );
}

/* ---------------- root ---------------- */

export default function DuplicatesView() {
  const scanReady = useScan((s) => s.summary !== null);
  const setView = useApp((s) => s.setView);
  const { status, error, reset } = useDedup();

  if (!scanReady) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <h1 className="text-gradient text-2xl font-bold">No scan yet</h1>
          <p className="mt-2 text-sm text-ink-mute">Run a scan first — the lab works on its results.</p>
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

  if (status === "running") return <RunningState />;
  if (status === "done") return <DoneState />;
  if (status === "error") {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <div className="max-w-md text-center">
          <h1 className="text-xl font-bold text-danger">Duplicate search failed</h1>
          <p className="mt-2 text-sm break-words text-ink-mute">{error}</p>
          <button
            onClick={reset}
            className="mt-6 rounded-xl border border-edge px-5 py-2 text-[13px] font-semibold text-ink-mute hover:text-ink"
          >
            Back
          </button>
        </div>
      </div>
    );
  }
  return <ReadyState />;
}
