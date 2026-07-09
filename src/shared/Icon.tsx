const paths: Record<string, string> = {
  // 24×24 viewBox, 1.8 stroke
  pulse: "M3 12h4l2.5-7 5 14 2.5-7H21",
  mosaic: "M3 3h8v12H3zM13 3h8v6h-8zM13 11h8v10h-8zM3 17h8v4H3z",
  layers: "M12 3 2 8.5 12 14l10-5.5zM2 13.5 12 19l10-5.5",
  sparkle:
    "M12 2l1.8 5.7L19.5 9.5l-5.7 1.8L12 17l-1.8-5.7L4.5 9.5l5.7-1.8zM19 15l.9 2.6L22.5 18.5l-2.6.9L19 22l-.9-2.6-2.6-.9 2.6-.9z",
  broom:
    "M14 3l7 7-4.5 1.5L18 17c-3 3-8 4-13 4 0-5 1-10 4-13l5.5 1.5z",
  gear: "M12 8.5A3.5 3.5 0 1 0 12 15.5 3.5 3.5 0 0 0 12 8.5zM19.4 13.5l1.8 1-1.8 3.2-2-.6a7 7 0 0 1-1.7 1l-.3 2.1h-3.6l-.3-2.1a7 7 0 0 1-1.7-1l-2 .6-1.8-3.2 1.7-1.4a7 7 0 0 1 0-2l-1.7-1.4 1.8-3.2 2 .6a7 7 0 0 1 1.7-1l.3-2.1h3.6l.3 2.1a7 7 0 0 1 1.7 1l2-.6 1.8 3.2-1.8 1.4a7 7 0 0 1 0 2z",
  drive:
    "M4 6h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1zM3 13h18M7 16.5h.01M11 16.5h.01",
  folder:
    "M3 6a1 1 0 0 1 1-1h5l2 2.5h9a1 1 0 0 1 1 1V18a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1z",
};

export type IconName = keyof typeof paths;

export function Icon({
  name,
  size = 20,
  className,
  filled = false,
}: {
  name: IconName;
  size?: number;
  className?: string;
  filled?: boolean;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={filled ? "currentColor" : "none"}
      stroke="currentColor"
      strokeWidth={filled ? 0 : 1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden
    >
      <path d={paths[name] ?? ""} />
    </svg>
  );
}
