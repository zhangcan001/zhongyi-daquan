import type { AppStatus } from "../modules/app/types";
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

const knowledgeTypes = ["中药", "方剂", "经络", "穴位", "证型", "病症"];
const entryActions = ["快速新增", "表格录入", "批量导入", "字段映射", "暂存区", "数据清洗"];

export function DashboardPage({ status, error, activeView, onNavigate }: DashboardPageProps) {
  return (
    <>
      <section className="topbar">
        <div>
          <h1>中医大全</h1>
          <p>本软件仅用于中医知识学习、资料整理与本地记录，不构成医疗诊断、治疗建议或处方依据。</p>
        </div>
        <div className="status-pill">{status?.databaseReady ? "数据库就绪" : "初始化中"}</div>
      </section>

      <nav className="main-tabs">
        <button
          className={activeView === "knowledge" ? "active" : ""}
          type="button"
          onClick={() => onNavigate("knowledge")}
        >
          知识库
        </button>
        <button
          className={activeView === "grid" ? "active" : ""}
          type="button"
          onClick={() => onNavigate("grid")}
        >
          表格录入
        </button>
        <button
          className={activeView === "import" ? "active" : ""}
          type="button"
          onClick={() => onNavigate("import")}
        >
          智能导入中心
        </button>
        <button
          className={activeView === "dashboard" ? "active" : ""}
          type="button"
          onClick={() => onNavigate("dashboard")}
        >
          状态
        </button>
      </nav>

      {activeView === "dashboard" ? (
        <>
          <section className="workspace-grid">
            <SearchPanel />

            <div className="panel">
              <h2>应用状态</h2>
              {error ? <p className="error-text">{error}</p> : null}
              <dl>
                <dt>版本</dt>
                <dd>{status?.version ?? "读取中"}</dd>
                <dt>AI</dt>
                <dd>{status?.aiEnabled ? "已启用" : "默认关闭"}</dd>
                <dt>本地数据目录</dt>
                <dd>{status?.dataDir ?? "准备中"}</dd>
              </dl>
            </div>
          </section>

          <section className="section-band">
            <h2>知识库</h2>
            <div className="type-grid">
              {knowledgeTypes.map((type) => (
                <button key={type} type="button" disabled>
                  {type}
                </button>
              ))}
            </div>
          </section>

          <section className="section-band">
            <h2>数据录入中心</h2>
            <div className="action-grid">
              <button type="button" onClick={() => onNavigate("import")}>
                智能导入中心
                <small>导入标准数据包，系统自动识别、去重、合并并生成导入报告。</small>
              </button>
              {entryActions.map((action) => (
                <button key={action} type="button" disabled>
                  {action}
                </button>
              ))}
            </div>
          </section>

          <RelationReviewPanel />
          <TaskCenterPanel />
          <AiSettingsPanel />
        </>
      ) : null}

      {activeView !== "dashboard" ? (
        <section className="summary-strip">
          <span>知识类型：{knowledgeTypes.join(" / ")}</span>
          <span>入口：{entryActions.join(" / ")}</span>
        </section>
      ) : null}
    </>
  );
}
