import type { ViewId } from "@/app/store";
import DashboardView from "./DashboardView";
import ComingSoon from "./ComingSoon";

export const views: Record<ViewId, React.ReactNode> = {
  dashboard: <DashboardView />,
  treemap: (
    <ComingSoon
      phase={2}
      title="Living Treemap"
      description="Every file on your drive as a single luminous mosaic — cushion-shaded, GPU-drawn, zoomable to the last byte."
      features={[
        "Squarified layout computed in the Rust core",
        "Animated drill-down and hover inspector",
        "Category color system for instant reading",
        "Open, reveal and recycle straight from the map",
      ]}
    />
  ),
  duplicates: (
    <ComingSoon
      phase={3}
      title="Duplicate Lab"
      description="Three-stage BLAKE3 pipeline finds byte-identical files with zero false positives — and remembers hashes between sessions."
      features={[
        "Size groups → 2 KB prehash → full BLAKE3",
        "Persistent hash cache for instant re-scans",
        "Keep-strategies: newest, oldest, by folder",
        "Recycle Bin first; hardlink replacement for experts",
      ]}
    />
  ),
  advisor: (
    <ComingSoon
      phase={4}
      title="Cleanup Advisor"
      description="A curated knowledge base of Windows space sinks, each rated Safe / Caution / Expert — with reclaimable sizes and receipts."
      features={[
        "40+ detectors: temp, caches, Windows.old, node_modules…",
        "One-click cleanup for the Safe tier",
        "Every action reversible by default",
      ]}
    />
  ),
  ai: (
    <ComingSoon
      phase={5}
      title="AI Insights"
      description="Gemini reads an anonymized statistical digest of your scan — never file names, never contents — and writes you an expert cleanup plan."
      features={[
        "Data passport: see the exact payload before it's sent",
        "Strict opt-in, fully functional offline",
        "≤ 4K tokens per scan, cached digests",
      ]}
    />
  ),
  settings: (
    <ComingSoon
      phase={5}
      title="Settings"
      description="Theme, language, AI key management and scan preferences arrive together with the features they configure."
      features={["Gemini API key vault", "EN / RU interface", "Scan exclusions"]}
    />
  ),
};
