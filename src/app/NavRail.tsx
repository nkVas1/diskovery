import { Icon, type IconName } from "@/shared/Icon";
import { useApp, type ViewId } from "@/app/store";

const items: { id: ViewId; icon: IconName; label: string }[] = [
  { id: "dashboard", icon: "pulse", label: "Scan" },
  { id: "treemap", icon: "mosaic", label: "Treemap" },
  { id: "duplicates", icon: "layers", label: "Dupes" },
  { id: "advisor", icon: "broom", label: "Advisor" },
  { id: "ai", icon: "sparkle", label: "Insights" },
];

function RailButton({ id, icon, label }: { id: ViewId; icon: IconName; label: string }) {
  const { view, setView } = useApp();
  const active = view === id;
  return (
    <button
      onClick={() => setView(id)}
      aria-current={active ? "page" : undefined}
      className={`group relative flex w-full flex-col items-center gap-1 rounded-xl py-2.5 transition-colors ${
        active ? "text-ink" : "text-ink-faint hover:text-ink-mute"
      }`}
    >
      {active && (
        <span className="absolute top-1/2 -left-3 h-7 w-[3px] -translate-y-1/2 rounded-r-full bg-gradient-glow" />
      )}
      <span
        className={`flex h-9 w-9 items-center justify-center rounded-lg transition-colors ${
          active ? "bg-overlay" : "group-hover:bg-panel"
        }`}
      >
        <Icon name={icon} />
      </span>
      <span className="text-[10px] font-medium tracking-wide">{label}</span>
    </button>
  );
}

export default function NavRail() {
  return (
    <nav className="flex w-[76px] shrink-0 flex-col items-stretch gap-1 border-r border-edge-soft bg-panel px-3 py-4">
      {items.map((it) => (
        <RailButton key={it.id} {...it} />
      ))}
      <div className="flex-1" />
      <RailButton id="settings" icon="gear" label="Settings" />
    </nav>
  );
}
