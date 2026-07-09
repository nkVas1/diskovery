import type { ViewId } from "@/app/store";
import DashboardView from "./DashboardView";
import TreemapView from "./TreemapView";
import DuplicatesView from "./DuplicatesView";
import AdvisorView from "./AdvisorView";
import AiView from "./AiView";
import SettingsView from "./SettingsView";

export const views: Record<ViewId, React.ReactNode> = {
  dashboard: <DashboardView />,
  treemap: <TreemapView />,
  duplicates: <DuplicatesView />,
  advisor: <AdvisorView />,
  ai: <AiView />,
  settings: <SettingsView />,
};
