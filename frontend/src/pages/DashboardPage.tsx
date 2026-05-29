import type { AppStatus } from "../modules/app/types";
import { AiSettingsPanel } from "./AiSettingsPanel";
import { TaskCenterPanel } from "./TaskCenterPanel";

type DashboardPageProps = {
  status: AppStatus | null;
  error: string | null;
};

const knowledgeTypes = ["中药", "方剂", "经络", "穴位", "证型", "病症"];
const entryActions = ["快速新增", "表格录入", "批量导入", "字段映射", "暂存区", "数据清洗"];

export function DashboardPage({ status, error }: DashboardPageProps) {
  return (
    <main className="app-shell">
      <section className="topbar">
        <div>
          <h1>中医大全</h1>
          <p>本软件仅用于中医知识学习、资料整理与本地记录，不构成医疗诊断、治疗建议或处方依据。</p>
        </div>
        <div className="status-pill">{status?.databaseReady ? "数据库就绪" : "初始化中"}</div>
      </section>

      <section className="workspace-grid">
        <div className="panel search-panel">
          <label htmlFor="global-search">全局搜索</label>
          <input id="global-search" placeholder="搜索足三里、ST36、黄芪、补中益气汤、胃经" disabled />
          <span>搜索将在后续线程接入 FTS5 与 search_terms。</span>
        </div>

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
          {entryActions.map((action) => (
            <button key={action} type="button" disabled>
              {action}
            </button>
          ))}
        </div>
      </section>

      <TaskCenterPanel />

      <AiSettingsPanel />
    </main>
  );
}
