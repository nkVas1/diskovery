export default function ComingSoon({
  phase,
  title,
  description,
  features,
}: {
  phase: number;
  title: string;
  description: string;
  features: string[];
}) {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="max-w-md">
        <span className="rounded-full border border-edge px-3 py-1 font-mono text-[11px] tracking-[0.2em] text-ink-faint uppercase">
          Phase {phase}
        </span>
        <h1 className="text-gradient mt-5 text-3xl font-bold tracking-tight">{title}</h1>
        <p className="mt-3 text-sm leading-relaxed text-ink-mute">{description}</p>
        <ul className="mt-6 space-y-2.5">
          {features.map((f) => (
            <li key={f} className="flex items-start gap-2.5 text-sm text-ink-faint">
              <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-gradient-glow" />
              {f}
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
