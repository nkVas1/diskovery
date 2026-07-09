import { AnimatePresence, motion } from "motion/react";
import TitleBar from "@/app/TitleBar";
import NavRail from "@/app/NavRail";
import { useApp } from "@/app/store";
import { views } from "@/app/views";

export default function App() {
  const view = useApp((s) => s.view);

  return (
    <div className="flex h-full flex-col">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <NavRail />
        <main className="relative min-w-0 flex-1 overflow-hidden">
          <AnimatePresence mode="wait">
            <motion.div
              key={view}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.16, ease: "easeOut" }}
              className="h-full overflow-auto"
            >
              {views[view]}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>
    </div>
  );
}
