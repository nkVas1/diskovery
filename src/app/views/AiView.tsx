import { useEffect, useState } from "react";
import { useScan } from "@/app/scanStore";
import { useApp } from "@/app/store";
import { formatBytes, formatInt, middleTruncate } from "@/shared/format";
import {
  aiAnalyze,
  aiDigestPreview,
  getSettings,
  type AiAnalysis,
  type AiAction,
  type DigestPreview,
  type SettingsDto,
  type Tier,
} from "@/shared/ipc";

const HEALTH_META = {
  good: { label: "Healthy", className: "bg-ok/15 text-ok" },
  attention: { label: "Needs attention", className: "bg-warn/15 text-warn" },
  critical: { label: "Critical", className: "bg-danger/15 text-danger" },
} as const;

const RISK_META: Record<Tier, string> = {
  safe: "bg-ok/15 text-ok",
  caution: "bg-warn/15 text-warn",
  expert: "bg-danger/15 text-danger",
};

function ActionCard({ action }: { action: AiAction }) {
  const setView = useApp((s) => s.setView);
  return (
    <div className="rounded-2xl border border-edge-soft bg-panel p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="text-[14px] font-semibold">{action.title}</h3>
            <span
              className={`rounded-md px-2 py-0.5 text-[10px] font-bold tracking-[0.12em] uppercase ${RISK_META[action.risk] ?? RISK_META.caution}`}
            >
              {action.risk}
            </span>
          </div>
          <p className="mt-1.5 text-[12px] leading-relaxed text-ink-mute">{action.detail}</p>
          {action.resolvedTarget && (
            <p
              className="mt-1 truncate font-mono text-[11px] text-ink-faint"
              title={action.resolvedTarget}
            >
              {middleTruncate(action.resolvedTarget, 80)}
            </p>
          )}
        </div>
        {action.estimatedBytes != null && action.estimatedBytes > 0 && (
          <p className="shrink-0 font-mono text-lg font-semibold tabular-nums">
            {formatBytes(action.estimatedBytes)}
          </p>
        )}
      </div>
      <div className="mt-2.5 flex gap-1.5">
        {action.kind === "advisor" && (
          <ActionButton onClick={() => setView("advisor")}>Open Advisor</ActionButton>
        )}
        {action.kind === "duplicates" && (
          <ActionButton onClick={() => setView("duplicates")}>Open Duplicate Lab</ActionButton>
        )}
        {action.kind === "manual" && action.resolvedTarget && (
          <ActionButton
            onClick={() => {
              // resolvedTarget is a real local path; reveal needs a node id we
              // don't have here — fall back to treemap navigation.
              setView("treemap");
            }}
          >
            Explore in treemap
          </ActionButton>
        )}
      </div>
    </div>
  );
}

function ActionButton({ children, onClick }: { children: React.ReactNode; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      className="rounded-lg border border-edge px-3 py-1 text-[12px] font-semibold text-ink-mute transition-colors hover:border-ink-faint hover:text-ink"
    >
      {children}
    </button>
  );
}

export default function AiView() {
  const scanReady = useScan((s) => s.summary !== null);
  const setView = useApp((s) => s.setView);

  const [settings, setSettingsState] = useState<SettingsDto | null>(null);
  const [passport, setPassport] = useState<DigestPreview | null>(null);
  const [showPassport, setShowPassport] = useState(false);
  const [analysis, setAnalysis] = useState<AiAnalysis | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getSettings().then(setSettingsState).catch(() => setSettingsState(null));
  }, []);

  const loadPassport = async () => {
    try {
      setPassport(await aiDigestPreview());
      setShowPassport(true);
    } catch (err) {
      setError(String(err));
    }
  };

  const run = async (force: boolean) => {
    setBusy(true);
    setError(null);
    try {
      setAnalysis(await aiAnalyze(force));
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  if (!scanReady) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center">
          <h1 className="text-gradient text-2xl font-bold">No scan yet</h1>
          <p className="mt-2 text-sm text-ink-mute">AI insights are built from scan statistics.</p>
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

  if (settings && !settings.hasGeminiKey) {
    return (
      <div className="flex h-full items-center justify-center p-8">
        <div className="max-w-md text-center">
          <h1 className="text-gradient text-3xl font-bold tracking-tight">AI Insights</h1>
          <p className="mt-3 text-sm leading-relaxed text-ink-mute">
            Gemini reads an anonymized statistical digest of your scan — never file
            names, never contents — and writes an expert cleanup plan. To enable it,
            add a Gemini API key in Settings.
          </p>
          <button
            onClick={() => setView("settings")}
            className="mt-6 rounded-xl bg-gradient-glow px-6 py-2.5 text-sm font-semibold text-void"
          >
            Open Settings
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-3xl px-8 py-10">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">AI Insights</h1>
          <p className="mt-1 text-sm text-ink-mute">
            Anonymized digest → Gemini → prioritized plan. Nothing is sent until you click.
          </p>
        </div>
        <div className="flex gap-2.5">
          <button
            onClick={() => void loadPassport()}
            className="rounded-xl border border-edge px-4 py-2 text-[13px] font-semibold text-ink-mute hover:text-ink"
          >
            Data passport
          </button>
          <button
            onClick={() => void run(analysis !== null)}
            disabled={busy}
            className="rounded-xl bg-gradient-glow px-5 py-2 text-[13px] font-semibold text-void disabled:opacity-50"
          >
            {busy ? "Analyzing…" : analysis ? "Regenerate" : "Analyze with Gemini"}
          </button>
        </div>
      </div>

      {showPassport && passport && (
        <div className="mt-6 rounded-2xl border border-edge-soft bg-panel p-4">
          <div className="flex items-center justify-between">
            <h2 className="text-[12px] font-semibold tracking-[0.14em] text-ink-faint uppercase">
              Exactly what will be sent (~{formatInt(passport.approxTokens)} tokens)
            </h2>
            <button
              onClick={() => setShowPassport(false)}
              className="text-[12px] text-ink-faint hover:text-ink"
            >
              Close
            </button>
          </div>
          <pre className="mt-3 max-h-80 overflow-auto rounded-lg bg-void p-3 font-mono text-[11px] leading-relaxed text-ink-mute select-text">
            {passport.json}
          </pre>
          <p className="mt-2 text-[11px] text-ink-faint">
            Tokens like &lt;dir3&gt; replace your folder names; the mapping back to real
            paths stays on this device.
          </p>
        </div>
      )}

      {error && (
        <div className="mt-6 rounded-2xl border border-danger/30 bg-danger/10 p-4">
          <p className="text-[13px] wrap-break-word text-danger">{error}</p>
        </div>
      )}

      {busy && !analysis && (
        <div className="mt-10 space-y-3">
          {[0, 1, 2].map((i) => (
            <div key={i} className="h-20 animate-pulse rounded-2xl bg-panel" />
          ))}
        </div>
      )}

      {analysis && (
        <div className="mt-8">
          <div className="flex flex-wrap items-center gap-3">
            <span
              className={`rounded-md px-2.5 py-1 text-[11px] font-bold tracking-[0.12em] uppercase ${HEALTH_META[analysis.health]?.className ?? HEALTH_META.attention.className}`}
            >
              {HEALTH_META[analysis.health]?.label ?? analysis.health}
            </span>
            {analysis.cached && (
              <span className="rounded-md border border-edge px-2 py-0.5 text-[10px] text-ink-faint uppercase">
                cached
              </span>
            )}
          </div>
          <h2 className="text-gradient mt-3 text-xl font-bold">{analysis.headline}</h2>
          <p className="mt-2 text-sm leading-relaxed text-ink-mute select-text">{analysis.summary}</p>

          {analysis.actions.length > 0 && (
            <section className="mt-7">
              <h3 className="mb-3 text-[11px] font-semibold tracking-[0.16em] text-ink-faint uppercase">
                Prioritized plan
              </h3>
              <div className="space-y-3">
                {analysis.actions.map((a, i) => (
                  <ActionCard key={i} action={a} />
                ))}
              </div>
            </section>
          )}

          {analysis.observations.length > 0 && (
            <section className="mt-7">
              <h3 className="mb-3 text-[11px] font-semibold tracking-[0.16em] text-ink-faint uppercase">
                Observations
              </h3>
              <ul className="space-y-2">
                {analysis.observations.map((o) => (
                  <li key={o} className="flex items-start gap-2.5 text-[13px] text-ink-mute select-text">
                    <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-gradient-glow" />
                    {o}
                  </li>
                ))}
              </ul>
            </section>
          )}

          <p className="mt-8 font-mono text-[11px] tracking-wide text-ink-faint">
            {analysis.model} · ~{formatInt(analysis.approxTokens)} tokens sent · digest cached per scan
          </p>
        </div>
      )}
    </div>
  );
}
