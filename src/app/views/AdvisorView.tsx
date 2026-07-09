import { useState } from "react";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { useScan } from "@/app/scanStore";
import { useApp } from "@/app/store";
import { formatBytes, formatInt, middleTruncate } from "@/shared/format";
import {
  advisorAnalyze,
  advisorClean,
  type FindingDto,
  type Tier,
} from "@/shared/ipc";

const TIER_META: Record<Tier, { label: string; className: string; blurb: string }> = {
  safe: {
    label: "Safe",
    className: "bg-ok/15 text-ok",
    blurb: "Removable with no functional impact — caches and scratch data that rebuild themselves.",
  },
  caution: {
    label: "Caution",
    className: "bg-warn/15 text-warn",
    blurb: "Removable with understood consequences. Read the note on each card first.",
  },
  expert: {
    label: "Expert",
    className: "bg-danger/15 text-danger",
    blurb: "Do not touch by hand — use the dedicated system tool named on the card.",
  },
};

function TierBadge({ tier }: { tier: Tier }) {
  const m = TIER_META[tier];
  return (
    <span
      className={`rounded-md px-2 py-0.5 text-[10px] font-bold tracking-[0.12em] uppercase ${m.className}`}
    >
      {m.label}
    </span>
  );
}

function FindingCard({
  finding,
  onCleaned,
}: {
  finding: FindingDto;
  onCleaned: () => void;
}) {
  const [busy, setBusy] = useState(false);

  const onClean = async () => {
    const yes = await ask(
      `Move ${formatInt(finding.itemCount)} item${finding.itemCount === 1 ? "" : "s"} (${formatBytes(
        finding.bytes,
      )}) from "${finding.title}" to the Recycle Bin?`,
      { title: "Diskovery", kind: "warning" },
    );
    if (!yes) return;
    setBusy(true);
    try {
      const report = await advisorClean(finding.ruleId);
      await message(
        `Recycled ${formatBytes(report.removedBytes)} (${formatInt(report.removedItems)} items)` +
          (report.failedItems > 0 ? `\n${formatInt(report.failedItems)} items were in use and skipped.` : ""),
        { title: "Diskovery" },
      );
      onCleaned();
    } catch (err) {
      await message(`Cleanup failed: ${String(err)}`, { title: "Diskovery", kind: "error" });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="rounded-2xl border border-edge-soft bg-panel p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-[14px] font-semibold">{finding.title}</h3>
            <TierBadge tier={finding.tier} />
          </div>
          <p className="mt-1.5 text-[12px] leading-relaxed text-ink-mute">{finding.rationale}</p>
          <p className="mt-1 text-[11px] text-ink-faint">→ {finding.actionHint}</p>
        </div>
        <div className="shrink-0 text-right">
          <p className="font-mono text-lg font-semibold tabular-nums">{formatBytes(finding.bytes)}</p>
          <p className="text-[11px] text-ink-faint">{formatInt(finding.itemCount)} item(s)</p>
        </div>
      </div>
      <div className="mt-3 space-y-0.5">
        {finding.locations.map((l) => (
          <p key={l} className="truncate font-mono text-[11px] text-ink-faint" title={l}>
            {middleTruncate(l, 88)}
          </p>
        ))}
      </div>
      {finding.deletable && (
        <button
          onClick={() => void onClean()}
          disabled={busy}
          className={`mt-3 rounded-lg px-4 py-1.5 text-[12px] font-semibold transition-colors ${
            finding.tier === "safe"
              ? "bg-gradient-glow text-void disabled:opacity-50"
              : "border border-edge text-ink-mute hover:border-warn hover:text-warn disabled:opacity-50"
          }`}
        >
          {busy ? "Recycling…" : "Clean (to Recycle Bin)"}
        </button>
      )}
    </div>
  );
}

export default function AdvisorView() {
  const scanReady = useScan((s) => s.summary !== null);
  const refreshSummary = useScan((s) => s.refreshSummary);
  const setView = useApp((s) => s.setView);
  const [findings, setFindings] = useState<FindingDto[] | null>(null);
  const [busy, setBusy] = useState(false);

  const analyze = async () => {
    setBusy(true);
    try {
      setFindings(await advisorAnalyze());
    } catch {
      setFindings([]);
    } finally {
      setBusy(false);
    }
  };

  const onCleaned = () => {
    void refreshSummary();
    void analyze();
  };

  if (!scanReady) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <h1 className="text-gradient text-2xl font-bold">No scan yet</h1>
          <p className="mt-2 text-sm text-ink-mute">The advisor reads scan results to find space sinks.</p>
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

  if (findings === null) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <div className="max-w-md text-center">
          <h1 className="text-gradient text-3xl font-bold tracking-tight">Cleanup Advisor</h1>
          <p className="mt-3 text-sm leading-relaxed text-ink-mute">
            A curated knowledge base of Windows space sinks — temp files, caches,
            update leftovers, stale build artifacts. Every finding is rated{" "}
            <span className="text-ok">Safe</span> / <span className="text-warn">Caution</span> /{" "}
            <span className="text-danger">Expert</span> and explained. Deletions go to
            the Recycle Bin, always.
          </p>
          <button
            onClick={() => void analyze()}
            disabled={busy}
            className="mt-6 rounded-xl bg-gradient-glow px-6 py-2.5 text-sm font-semibold text-void disabled:opacity-50"
          >
            {busy ? "Analyzing…" : "Analyze this scan"}
          </button>
        </div>
      </div>
    );
  }

  const reclaimable = findings
    .filter((f) => f.deletable)
    .reduce((sum, f) => sum + f.bytes, 0);
  const tiers: Tier[] = ["safe", "caution", "expert"];

  return (
    <div className="mx-auto max-w-3xl px-8 py-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">
            <span className="text-gradient">{formatBytes(reclaimable)}</span> reclaimable
          </h1>
          <p className="mt-1 text-sm text-ink-mute">
            {findings.length} findings in this scan · deletions always go to the Recycle Bin
          </p>
        </div>
        <button
          onClick={() => void analyze()}
          disabled={busy}
          className="rounded-xl border border-edge px-4 py-2 text-[13px] font-semibold text-ink-mute hover:text-ink disabled:opacity-50"
        >
          Re-analyze
        </button>
      </div>

      {findings.length === 0 ? (
        <div className="mt-16 text-center">
          <p className="text-lg font-semibold text-ok">Nothing to clean</p>
          <p className="mt-1 text-sm text-ink-mute">
            No known space sinks found in the scanned area.
          </p>
        </div>
      ) : (
        tiers.map((tier) => {
          const group = findings.filter((f) => f.tier === tier);
          if (group.length === 0) return null;
          return (
            <section key={tier} className="mt-8">
              <div className="mb-3 flex items-center gap-3">
                <TierBadge tier={tier} />
                <p className="text-[12px] text-ink-faint">{TIER_META[tier].blurb}</p>
              </div>
              <div className="space-y-3">
                {group.map((f) => (
                  <FindingCard key={f.ruleId} finding={f} onCleaned={onCleaned} />
                ))}
              </div>
            </section>
          );
        })
      )}
    </div>
  );
}
