import { useEffect, useState } from "react";
import { getAppInfo } from "@/shared/ipc";
import { Logo } from "@/app/TitleBar";

export default function DashboardView() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    getAppInfo()
      .then((info) => setVersion(info.version))
      .catch(() => setVersion(null));
  }, []);

  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="max-w-lg text-center">
        <div className="mx-auto flex h-20 w-20 items-center justify-center rounded-3xl border border-edge bg-panel shadow-[0_0_60px_-12px_rgba(47,212,240,0.35)]">
          <Logo size={40} />
        </div>
        <h1 className="mt-8 text-4xl font-bold tracking-tight">
          Scan. <span className="text-gradient">See.</span> Reclaim.
        </h1>
        <p className="mx-auto mt-4 max-w-sm text-sm leading-relaxed text-ink-mute">
          Diskovery reads your drives at engine speed, draws every byte on a living
          treemap and tells you — with receipts — what is safe to clean.
        </p>
        <div className="mt-8 flex items-center justify-center gap-3">
          <button
            disabled
            className="cursor-not-allowed rounded-xl bg-gradient-glow px-6 py-2.5 text-sm font-semibold text-void opacity-50"
            title="Scan engine lands in Phase 1"
          >
            Choose what to scan
          </button>
        </div>
        {version && (
          <p className="mt-10 font-mono text-[11px] tracking-widest text-ink-faint">
            CORE v{version} · IPC LINK OK
          </p>
        )}
      </div>
    </div>
  );
}
