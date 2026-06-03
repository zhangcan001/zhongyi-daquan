import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DashboardPage } from "./pages/DashboardPage";
import type { AppStatus } from "./modules/app/types";
import { KnowledgeWorkspace } from "./pages/KnowledgeWorkspace";
import { GridEntryPage } from "./pages/GridEntryPage";
import { ImportStagingPanel } from "./pages/ImportStagingPanel";
import { RelationReviewPanel } from "./pages/RelationReviewPanel";
import { TaskCenterPanel } from "./pages/TaskCenterPanel";

export type AppView = "dashboard" | "knowledge" | "tools";

export function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeView, setActiveView] = useState<AppView>("dashboard");

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
      {activeView === "tools" ? (
        <section className="tools-console">
          <section className="section-band">
            <div className="section-heading">
              <div>
                <h2>高级工具</h2>
                <p>导入、批量录入、关系审查和维护任务集中放在这里，日常学习检索不需要进入。</p>
              </div>
            </div>
          </section>
          <details className="advanced-details tool-panel" open>
            <summary>智能导入中心</summary>
            <ImportStagingPanel />
          </details>
          <details className="advanced-details tool-panel">
            <summary>表格录入</summary>
            <GridEntryPage />
          </details>
          <details className="advanced-details tool-panel">
            <summary>关系审查</summary>
            <RelationReviewPanel />
          </details>
          <details className="advanced-details tool-panel">
            <summary>任务与维护</summary>
            <TaskCenterPanel />
          </details>
        </section>
      ) : null}
    </main>
  );
}
