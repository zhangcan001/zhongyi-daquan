import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DashboardPage } from "./pages/DashboardPage";
import type { AppStatus } from "./modules/app/types";
import { KnowledgeWorkspace } from "./pages/KnowledgeWorkspace";
import { GridEntryPage } from "./pages/GridEntryPage";

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeView, setActiveView] = useState<"dashboard" | "knowledge" | "grid">("dashboard");

  useEffect(() => {
    invoke<AppStatus>("get_app_status")
      .then(setStatus)
      .catch((cause) => setError(String(cause)));
  }, []);

  return (
    <main className="app-shell">
      <DashboardPage
        status={status}
        error={error}
        activeView={activeView}
        onNavigate={setActiveView}
      />
      {activeView === "knowledge" ? <KnowledgeWorkspace /> : null}
      {activeView === "grid" ? <GridEntryPage /> : null}
    </main>
  );
}
