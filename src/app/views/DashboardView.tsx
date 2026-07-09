import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useScan } from "@/app/scanStore";
import { useApp } from "@/app/store";
import { formatBytes, formatDuration, formatInt, middleTruncate } from "@/shared/format";
import { Icon } from "@/shared/Icon";
import type { VolumeInfo } from "@/shared/ipc";

/* ---------- shared bits ---------- */

function Meter({ ratio, danger = false }: { ratio: number; danger?: boolean }) {
  const pct = Math.min(100, Math.max(0, ratio * 100));
  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-data-track">
      <div
        className={`h-full rounded-full ${danger ? "bg-danger" : "bg-data-cyan"}`}
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

function BarRow({
  label,
  sub,
  value,
  ratio,
  violet = false,
}: {
  label: string;
  sub?: string;
  value: string;
  ratio: number;
  violet?: boolean;
}) {
  return (
    <div className="group flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-panel">
      <div className="w-44 min-w-0 shrink-0">
        <p className="truncate text-[13px] text-ink" title={label}>
          {label}
        </p>
        {sub && <p className="truncate text-[11px] text-ink-faint">{sub}</p>}
      </div>
      <div className="h-2 min-w-0 flex-1 overflow-hidden rounded-full bg-data-track">
        <div
          className={`h-full rounded-full ${violet ? "bg-data-violet" : "bg-data-cyan"}`}
          style={{ width: `${Math.max(0.6, ratio * 100)}%` }}
        />
      </div>
      <span className="w-20 shrink-0 text-right font-mono text-[12px] tabular-nums text-ink-mute">
        {value}
      </span>
    </div>
  );
}

function StatTile({ label, value, sub }: { label: string; value: string; sub?: string }) {
  return (
    <div className="rounded-2xl border border-edge-soft bg-panel px-5 py-4">
      <p className="text-[11px] font-medium tracking-[0.14em] text-ink-faint uppercase">{label}</p>
      <p className="mt-1.5 font-mono text-2xl font-semibold tabular-nums">{value}</p>
      {sub && <p className="mt-0.5 text-[11px] text-ink-faint">{sub}</p>}
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="mb-2 text-[11px] font-semibold tracking-[0.16em] text-ink-faint uppercase">
      {children}
    </h2>
  );
}

/* ---------- idle: target picker ---------- */

function VolumeCard({ vol }: { vol: VolumeInfo }) {
  const start = useScan((s) => s.start);
  const used = vol.total - vol.free;
  const ratio = vol.total > 0 ? used / vol.total : 0;
  const nearlyFull = ratio > 0.92;
  return (
    <div className="group rounded-2xl border border-edge-soft bg-panel p-5 transition-colors hover:border-edge">
      <div className="flex items-baseline justify-between">
        <div className="flex items-baseline gap-2.5">
          <span className="font-mono text-xl font-semibold">{vol.path.replace(/\\$/, "")}</span>
          <span className="max-w-36 truncate text-[13px] text-ink-mute">
            {vol.label || "Local Disk"}
          </span>
        </div>
        <div className="flex gap-1.5">
          {vol.removable && <Chip>USB</Chip>}
          <Chip>{vol.kind === "unknown" ? vol.fs : vol.kind.toUpperCase()}</Chip>
        </div>
      </div>
      <div className="mt-4">
        <Meter ratio={ratio} danger={nearlyFull} />
        <div className="mt-2 flex justify-between text-[12px]">
          <span className="text-ink-mute">
            {formatBytes(used)} used
            {nearlyFull && <span className="ml-2 text-danger">· almost full</span>}
          </span>
          <span className="text-ink-faint">{formatBytes(vol.free)} free</span>
        </div>
      </div>
      <button
        onClick={() => void start(vol.path, used)}
        className="mt-4 w-full rounded-xl border border-edge py-2 text-[13px] font-semibold text-ink-mute transition-colors hover:border-transparent hover:bg-gradient-glow hover:text-void"
      >
        Scan {vol.path.replace(/\\$/, "")}
      </button>
    </div>
  );
}

function Chip({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded-md border border-edge px-1.5 py-0.5 font-mono text-[10px] tracking-wide text-ink-faint uppercase">
      {children}
    </span>
  );
}

function IdleState() {
  const { volumes, loadVolumes, start } = useScan();

  useEffect(() => {
    void loadVolumes();
  }, [loadVolumes]);

  const pickFolder = async () => {
    const dir = await open({ directory: true, title: "Choose a folder to scan" });
    if (typeof dir === "string") void start(dir);
  };

  return (
    <div className="mx-auto max-w-4xl px-8 py-10">
      <h1 className="text-2xl font-bold tracking-tight">Choose a target</h1>
      <p className="mt-1 text-sm text-ink-mute">
        Scan a whole drive or any folder — results stream in live.
      </p>
      <div className="mt-7 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
        {volumes.map((v) => (
          <VolumeCard key={v.path} vol={v} />
        ))}
      </div>
      <button
        onClick={() => void pickFolder()}
        className="mt-6 flex items-center gap-2.5 rounded-xl border border-dashed border-edge px-5 py-3 text-[13px] text-ink-mute transition-colors hover:border-ink-faint hover:text-ink"
      >
        <Icon name="folder" size={16} />
        Or pick a specific folder…
      </button>
    </div>
  );
}

/* ---------- scanning: live telemetry ---------- */

function ScanningState() {
  const { target, progress, expectedBytes, cancel } = useScan();
  const rate = progress.elapsedMs > 0 ? (progress.files / progress.elapsedMs) * 1000 : 0;
  const ratio = expectedBytes ? progress.bytes / expectedBytes : null;

  return (
    <div className="mx-auto max-w-4xl px-8 py-10">
      <div className="flex items-center gap-3">
        <span className="relative flex h-2.5 w-2.5">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-glow-cyan opacity-60" />
          <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-glow-cyan" />
        </span>
        <h1 className="text-xl font-bold tracking-tight">Scanning {target}</h1>
      </div>

      <div className="mt-7 grid grid-cols-2 gap-4 lg:grid-cols-4">
        <StatTile label="Files" value={formatInt(progress.files)} sub={`${formatInt(Math.round(rate))} / s`} />
        <StatTile label="Data seen" value={formatBytes(progress.bytes)} />
        <StatTile label="Folders" value={formatInt(progress.dirs)} />
        <StatTile label="Elapsed" value={formatDuration(progress.elapsedMs)} />
      </div>

      {ratio !== null && (
        <div className="mt-6">
          <Meter ratio={Math.min(ratio, 0.99)} />
          <p className="mt-1.5 text-right font-mono text-[11px] tabular-nums text-ink-faint">
            ~{Math.min(99, Math.round(ratio * 100))}%
          </p>
        </div>
      )}

      <p className="mt-5 h-5 truncate font-mono text-[12px] text-ink-faint" title={progress.currentPath}>
        {middleTruncate(progress.currentPath, 96)}
      </p>
      {progress.errors > 0 && (
        <p className="mt-2 text-[12px] text-warn">
          {formatInt(progress.errors)} items skipped (access denied)
        </p>
      )}

      <button
        onClick={() => void cancel()}
        className="mt-8 rounded-xl border border-edge px-5 py-2 text-[13px] font-semibold text-ink-mute transition-colors hover:border-danger hover:text-danger"
      >
        Cancel scan
      </button>
    </div>
  );
}

/* ---------- done: summary ---------- */

function DoneState() {
  const { summary, reset } = useScan();
  const setView = useApp((s) => s.setView);
  if (!summary) return null;

  const maxDir = summary.topDirs[0]?.size ?? 1;
  const maxExt = summary.topExts[0]?.bytes ?? 1;

  return (
    <div className="mx-auto max-w-5xl px-8 py-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">{summary.rootPath}</h1>
          <p className="mt-1 text-sm text-ink-mute">
            {formatBytes(summary.bytes)} in {formatInt(summary.files)} files ·{" "}
            {formatInt(summary.dirs)} folders · scanned in {formatDuration(summary.elapsedMs)}
            {summary.errors > 0 && (
              <span className="text-warn"> · {formatInt(summary.errors)} skipped</span>
            )}
          </p>
        </div>
        <div className="flex gap-2.5">
          <button
            onClick={reset}
            className="rounded-xl border border-edge px-4 py-2 text-[13px] font-semibold text-ink-mute transition-colors hover:text-ink"
          >
            New scan
          </button>
          <button
            onClick={() => setView("treemap")}
            className="rounded-xl bg-gradient-glow px-4 py-2 text-[13px] font-semibold text-void"
          >
            Open treemap
          </button>
        </div>
      </div>

      <div className="mt-8 grid grid-cols-1 gap-8 lg:grid-cols-2">
        <section>
          <SectionTitle>Top folders</SectionTitle>
          <div className="-mx-2">
            {summary.topDirs.map((d) => (
              <BarRow
                key={d.id}
                label={d.name}
                value={formatBytes(d.size)}
                ratio={d.size / maxDir}
              />
            ))}
          </div>
        </section>

        <section>
          <SectionTitle>Largest files</SectionTitle>
          <div className="space-y-0.5">
            {summary.topFiles.slice(0, 12).map((f) => (
              <div
                key={f.id}
                className="flex items-center justify-between gap-3 rounded-lg px-2 py-1.5 hover:bg-panel"
              >
                <div className="min-w-0">
                  <p className="truncate text-[13px]">{f.name}</p>
                  <p className="truncate text-[11px] text-ink-faint" title={f.path}>
                    {middleTruncate(f.path, 56)}
                  </p>
                </div>
                <span className="shrink-0 font-mono text-[12px] tabular-nums text-ink-mute">
                  {formatBytes(f.size)}
                </span>
              </div>
            ))}
          </div>
        </section>
      </div>

      <section className="mt-8">
        <SectionTitle>File types by size</SectionTitle>
        <div className="-mx-2">
          {summary.topExts.slice(0, 10).map((e) => (
            <BarRow
              key={e.ext}
              label={`.${e.ext}`}
              sub={`${formatInt(e.count)} files`}
              value={formatBytes(e.bytes)}
              ratio={e.bytes / maxExt}
              violet
            />
          ))}
        </div>
      </section>
    </div>
  );
}

/* ---------- error ---------- */

function ErrorState() {
  const { error, reset } = useScan();
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="max-w-md text-center">
        <h1 className="text-xl font-bold text-danger">Scan failed</h1>
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

export default function DashboardView() {
  const status = useScan((s) => s.status);
  if (status === "scanning") return <ScanningState />;
  if (status === "done") return <DoneState />;
  if (status === "error") return <ErrorState />;
  return <IdleState />;
}
