import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

function ControlButton({
  label,
  onClick,
  danger = false,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      aria-label={label}
      onClick={onClick}
      className={`flex h-10 w-12 items-center justify-center text-ink-faint transition-colors ${
        danger ? "hover:bg-danger hover:text-void" : "hover:bg-overlay hover:text-ink"
      }`}
    >
      {children}
    </button>
  );
}

export function Logo({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden>
      <defs>
        <linearGradient id="dsk-g" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="var(--color-glow-cyan)" />
          <stop offset="1" stopColor="var(--color-glow-violet)" />
        </linearGradient>
      </defs>
      <path
        d="M12 2.2 21.8 12 12 21.8 2.2 12Z"
        fill="none"
        stroke="url(#dsk-g)"
        strokeWidth="1.9"
        strokeLinejoin="round"
      />
      <path d="M12 7.4 16.6 12 12 16.6 7.4 12Z" fill="url(#dsk-g)" />
    </svg>
  );
}

export default function TitleBar() {
  return (
    <header
      data-tauri-drag-region
      className="flex h-10 shrink-0 items-center border-b border-edge-soft bg-panel"
    >
      <div data-tauri-drag-region className="flex items-center gap-2.5 pl-4">
        <Logo />
        <span data-tauri-drag-region className="text-[13px] font-semibold tracking-[0.18em] uppercase">
          Diskovery
        </span>
      </div>
      <div data-tauri-drag-region className="flex-1" />
      <div className="flex">
        <ControlButton label="Minimize" onClick={() => void win.minimize()}>
          <svg width="11" height="11" viewBox="0 0 11 11" aria-hidden>
            <line x1="0.5" y1="5.5" x2="10.5" y2="5.5" stroke="currentColor" />
          </svg>
        </ControlButton>
        <ControlButton label="Maximize" onClick={() => void win.toggleMaximize()}>
          <svg width="11" height="11" viewBox="0 0 11 11" aria-hidden>
            <rect x="0.5" y="0.5" width="10" height="10" fill="none" stroke="currentColor" />
          </svg>
        </ControlButton>
        <ControlButton label="Close" danger onClick={() => void win.close()}>
          <svg width="11" height="11" viewBox="0 0 11 11" aria-hidden>
            <path d="M0.5 0.5l10 10M10.5 0.5l-10 10" stroke="currentColor" />
          </svg>
        </ControlButton>
      </div>
    </header>
  );
}
