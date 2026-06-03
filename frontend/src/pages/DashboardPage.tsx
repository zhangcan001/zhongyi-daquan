import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppStatus } from "../modules/app/types";
import { getDashboardStats, listFavorites, listRecentViews } from "../modules/knowledge/api";
import type { DashboardStats, FavoriteItem, RecentView } from "../modules/knowledge/types";
import { AiSettingsPanel } from "./AiSettingsPanel";
import { RelationReviewPanel } from "./RelationReviewPanel";
import { SearchPanel } from "./SearchPanel";
import { TaskCenterPanel } from "./TaskCenterPanel";

type AppView = "dashboard" | "knowledge" | "grid" | "import";

type DashboardPageProps = {
  status: AppStatus | null;
  error: string | null;
  activeView: AppView;
  onNavigate: (view: AppView) => void;
};

type ImportRunSummary = {
  id: number;
  packageName?: string | null;
  importIntent: string;
  status: string;
  totalRecords: number;
  createCount: number;
  attachAnnotationCount: number;
  createdAt: string;
  rolledBackAt?: string | null;
};

const quickEntries: Array<{ label: string; hint: string; view: AppView }> = [
  { label: "中药", hint: "按药名、别名、本经原文检索", view: "knowledge" },
  { label: "方剂", hint: "查组成、主治、出处与注解", view: "knowledge" },
  { label: "穴位", hint: "查定位、所属经络和学习资料", view: "knowledge" },
  { label: "经络", hint: "查循行、脏腑关联和相关穴位", view: "knowledge" },
  { label: "原典", hint: "查章节、条文和讲解", view: "knowledge" },
  { label: "人纪讲义", hint: "查看已导入注解资料", view: "knowledge" },
  { label: "智能导入中心", hint: "导入标准数据包", view: "import" },
  { label: "导入历史", hint: "查看报告与回滚入口", view: "import" },
];

export function DashboardPage({ status, error, activeView, onNavigate }: DashboardPageProps) {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [recentViews, setRecentViews] = useState<RecentView[]>([]);
  const [favorites, setFavorites] = useState<FavoriteItem[]>([]);
  const [importRuns, setImportRuns] = useState<ImportRunSummary[]>([]);

  useEffect(() => {
    if (activeView !== "dashboard") return;
    getDashboardStats().then(setStats).catch(() => undefined);
    listRecentViews(8).then(setRecentViews).catch(() => undefined);
    listFavorites().then(setFavorites).catch(() => undefined);
    invoke<ImportRunSummary[]>("list_import_runs").then((runs) => setImportRuns(runs.slice(0, 5))).catch(() => undefined);
  }, [activeView]);

  return (
    <>
      <section className="topbar">
        <div>
          <h1>中医大全学习工作台</h1>
          <p>集中检索中药、方剂、穴位、经络与原典条文，整理本地资料和学习笔记。</p>
        </div>
        <div className="status-pill">{status?.databaseReady ? "数据库就绪" : "初始化中"}</div>
      </section>

      <nav className="main-tabs">
        <button className={activeView === "dashboard" ? "active" : ""} type="button" onClick={() => onNavigate("dashboard")}>
          学习工作台
        </button>
        <button className={activeView === "knowledge" ? "active" : ""} type="button" onClick={() => onNavigate("knowledge")}>
          知识库
        </button>
        <button className={activeView === "grid" ? "active" : ""} type="button" onClick={() => onNavigate("grid")}>
          表格录入
        </button>
        <button className={activeView === "import" ? "active" : ""} type="button" onClick={() => onNavigate("import")}>
          智能导入中心
        </button>
      </nav>

      {activeView === "dashboard" ? (
        <>
          <AiSettingsPanel />
          <SearchPanel />

          <section className="section-band">
            <div className="section-heading">
              <div>
                <h2>快捷入口</h2>
                <p>按学习场景进入资料库，搜索仍是主要入口。</p>
              </div>
            </div>
            <div className="action-grid">
              {quickEntries.map((entry) => (
                <button key={entry.label} type="button" onClick={() => onNavigate(entry.view)}>
                  {entry.label}
                  <small>{entry.hint}</small>
                </button>
              ))}
            </div>
          </section>

          <section className="section-band">
            <h2>数据概览</h2>
            {error ? <p className="error-text">{error}</p> : null}
            <div className="summary-grid">
              <Metric label="知识条目" value={stats?.knowledgeCount} />
              <Metric label="注解资料" value={stats?.annotationCount} />
              <Metric label="最近导入批次" value={importRuns[0]?.packageName || importRuns[0]?.id || "无"} />
              <Metric label="收藏数量" value={stats?.favoriteCount} />
              <Metric label="最近查看" value={stats?.recentViewCount} />
              <Metric label="版本" value={status?.version ?? "读取中"} />
            </div>
          </section>

          <section className="workspace-grid">
            <div className="panel">
              <h2>最近查看</h2>
              <SimpleList
                empty="暂无最近查看"
                rows={recentViews.map((item) => ({
                  key: item.id,
                  title: item.itemName,
                  meta: [typeLabel(item.itemType), item.category, item.viewedAt].filter(Boolean).join(" / "),
                }))}
              />
            </div>
            <div className="panel">
              <h2>我的收藏</h2>
              <SimpleList
                empty="暂无收藏"
                rows={favorites.slice(0, 8).map((item) => ({
                  key: item.id,
                  title: item.itemName,
                  meta: [typeLabel(item.itemType), item.category, item.createdAt].filter(Boolean).join(" / "),
                }))}
              />
            </div>
          </section>

          <section className="section-band">
            <div className="section-heading">
              <div>
                <h2>导入历史</h2>
                <p>查看最近导入批次；报告与回滚在智能导入中心处理，回滚前会二次确认。</p>
              </div>
              <button type="button" onClick={() => onNavigate("import")}>
                进入导入中心
              </button>
            </div>
            <SimpleList
              empty="暂无导入批次"
              rows={importRuns.map((run) => ({
                key: run.id,
                title: run.packageName || `导入批次 #${run.id}`,
                meta: [
                  run.importIntent,
                  run.status,
                  `主条目 ${run.createCount}`,
                  `注解 ${run.attachAnnotationCount}`,
                  run.rolledBackAt ? "已回滚" : "可查看报告",
                ].join(" / "),
              }))}
            />
          </section>

          <RelationReviewPanel />
          <TaskCenterPanel />
        </>
      ) : null}
    </>
  );
}

function Metric({ label, value }: { label: string; value: string | number | null | undefined }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value ?? "读取中"}</strong>
    </div>
  );
}

function SimpleList({ rows, empty }: { rows: Array<{ key: string | number; title: string; meta: string }>; empty: string }) {
  if (!rows.length) return <p className="empty-text">{empty}</p>;
  return (
    <div className="compact-result-list">
      {rows.map((row) => (
        <div key={row.key}>
          <strong>{row.title}</strong>
          <span>{row.meta}</span>
        </div>
      ))}
    </div>
  );
}

function typeLabel(type: string) {
  return (
    {
      herb: "中药",
      formula: "方剂",
      acupuncture: "针灸",
      acupoint: "穴位",
      meridian: "经络",
      syndrome: "原典条文",
      theory: "原典章节",
      note: "注解",
    }[type] ?? type
  );
}
