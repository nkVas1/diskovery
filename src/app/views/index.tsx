import type { ViewId } from "@/app/store";
import DashboardView from "./DashboardView";
import TreemapView from "./TreemapView";
import DuplicatesView from "./DuplicatesView";
import ComingSoon from "./ComingSoon";

export const views: Record<ViewId, React.ReactNode> = {
  dashboard: <DashboardView />,
  treemap: <TreemapView />,
  duplicates: <DuplicatesView />,
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
