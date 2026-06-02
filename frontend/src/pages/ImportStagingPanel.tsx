import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ExecuteImportPlanResult,
  ImportParsedPreview,
  ImportPlan,
  ImportRunReport,
  ImportRunSummary,
  RollbackImportRunResult,
} from "../modules/importPipeline/types";

type ImportStep = "pick" | "analyze" | "plan" | "report";

const sampleJson = JSON.stringify(
  [{ type: "herb", name: "黄耆", content: "黄耆测试资料", tags: "中药" }],
  null,
  2,
);
const sampleCsv = "type,name,content,tags\r\nherb,黄耆,黄耆测试资料,中药\r\n";

export function ImportStagingPanel() {
  const [step, setStep] = useState<ImportStep>("pick");
  const [packagePath, setPackagePath] = useState("");
  const [fileName, setFileName] = useState("");
  const [plan, setPlan] = useState<ImportPlan | null>(null);
  const [report, setReport] = useState<ExecuteImportPlanResult | null>(null);
  const [runReport, setRunReport] = useState<ImportRunReport | null>(null);
  const [runs, setRuns] = useState<ImportRunSummary[]>([]);
  const [advancedContent, setAdvancedContent] = useState(sampleJson);
  const [advancedKind, setAdvancedKind] = useState<"json" | "csv">("json");
  const [advancedPreview, setAdvancedPreview] = useState<ImportParsedPreview | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  const packageTitle = useMemo(() => {
    if (plan?.packageName?.includes("shennong_bencao") || plan?.packageName?.includes("ni_notes")) {
      return "本草注解增强包";
    }
    return readableIntent(plan?.importIntent);
  }, [plan?.importIntent, plan?.packageName]);

  useEffect(() => {
    refreshRuns();
  }, []);

  async function refreshRuns() {
    try {
      setRuns(await invoke<ImportRunSummary[]>("list_import_runs"));
    } catch {
      setRuns([]);
    }
  }

  async function choosePackage(kind: "zip" | "folder") {
    setError("");
    setMessage("");
    setReport(null);
    setRunReport(null);
    setStep("pick");
    const selected = await open({
      directory: kind === "folder",
      multiple: false,
      title: kind === "folder" ? "选择已解压数据包文件夹" : "选择 ZIP 数据包",
      filters: kind === "zip" ? [{ name: "标准数据包", extensions: ["zip"] }] : undefined,
    });
    if (!selected || Array.isArray(selected)) return;
    await analyzePackage(selected);
  }

  async function chooseAdvancedFile(file: File | null) {
    if (!file) return;
    setError("");
    setMessage("");
    setFileName(file.name);
    const kind = file.name.toLowerCase().endsWith(".csv") ? "csv" : "json";
    const content = await file.text();
    setAdvancedKind(kind);
    setAdvancedContent(content);
    await previewAdvancedContent(kind, content);
  }

  async function previewAdvancedContent(kind = advancedKind, content = advancedContent) {
    try {
      const command = kind === "csv" ? "preview_csv_import" : "preview_json_import";
      const preview = await invoke<ImportParsedPreview>(command, { content });
      setAdvancedPreview(preview);
      setMessage(
        preview.directImportReady
          ? "已自动识别标准单文件，可继续使用 Smart Import 数据包方式导入。"
          : "该文件需要高级导入确认，字段映射详情已保持折叠。",
      );
    } catch (cause) {
      setAdvancedPreview(null);
      setError(String(cause));
    }
  }

  async function analyzePackage(selectedPath: string) {
    setPackagePath(selectedPath);
    setFileName(selectedPath.split(/[\\/]/).filter(Boolean).pop() ?? "标准数据包");
    setStep("analyze");
    try {
      const nextPlan = await invoke<ImportPlan>("preview_import_plan", { packagePath: selectedPath });
      setPlan(nextPlan);
      setStep(nextPlan.actions.some((action) => action.actionType === "needs_review") ? "plan" : "plan");
      setMessage("分析完成，系统已生成导入计划。");
    } catch (cause) {
      setPlan(null);
      setError(String(cause));
      setStep("pick");
    }
  }

  async function executePlan() {
    if (!plan) return;
    setError("");
    setMessage("");
    try {
      const result = await invoke<ExecuteImportPlanResult>("execute_import_plan", { plan });
      setReport(result);
      setStep("report");
      setMessage("导入完成，搜索索引已重建。");
      await refreshRuns();
      if (result.importRunId) {
        setRunReport(await invoke<ImportRunReport>("get_import_run_report", { importRunId: result.importRunId }));
      }
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function rollbackCurrentRun() {
    if (!report?.importRunId) return;
    if (!window.confirm("确认回滚本次导入？系统只撤销本批次记录的新增、注解和补空字段。")) return;
    setError("");
    try {
      const result = await invoke<RollbackImportRunResult>("rollback_import_run", {
        importRunId: report.importRunId,
      });
      setMessage(
        `回滚完成：撤销 ${result.rolledBackChanges} 项，跳过 ${result.skippedChanges} 项。搜索索引已重建。`,
      );
      if (result.warnings.length) {
        setError(result.warnings.join("；"));
      }
      await refreshRuns();
      setRunReport(await invoke<ImportRunReport>("get_import_run_report", { importRunId: report.importRunId }));
    } catch (cause) {
      setError(String(cause));
    }
  }

  function cancelImport() {
    setStep("pick");
    setPlan(null);
    setReport(null);
    setRunReport(null);
    setPackagePath("");
    setFileName("");
    setMessage("");
    setError("");
  }

  return (
    <section className="section-band import-panel">
      <div className="section-heading">
        <div>
          <h2>智能导入中心</h2>
          <p>选择标准数据包，系统会自动识别、去重、合并和生成导入计划。</p>
        </div>
        <div className="status-pill muted">{stepLabel(step)}</div>
      </div>

      <div className="summary-strip">
        <span>1. 选择数据包</span>
        <span>2. 自动分析</span>
        <span>3. 确认导入计划</span>
        <span>4. 查看报告 / 一键回滚</span>
      </div>

      <div className="import-actions">
        <button type="button" onClick={() => choosePackage("zip")}>选择 ZIP 数据包</button>
        <button type="button" onClick={() => choosePackage("folder")}>选择已解压数据包文件夹</button>
        <label className="file-button">
          选择单个 JSON / CSV 文件，高级入口
          <input type="file" accept=".json,.csv,application/json,text/csv" onChange={(event) => chooseAdvancedFile(event.target.files?.[0] ?? null)} />
        </label>
      </div>

      {plan ? (
        <div className="preview-panel">
          <div className="summary-grid">
            <Metric label="数据包名称" value={plan.packageName ?? (fileName || "未命名数据包")} />
            <Metric label="数据包类型" value={packageTitle} />
            <Metric label="导入意图" value={readableIntent(plan.importIntent)} />
            <Metric label="新增条目" value={plan.createCount} />
            <Metric label="附加注解" value={plan.attachAnnotationCount} />
            <Metric label="跳过重复数" value={plan.skipDuplicateCount} />
            <Metric label="待确认数" value={plan.needsReviewCount} />
            <Metric label="错误数" value={plan.rejectInvalidCount} />
            <Metric label="是否可回滚" value="导入后可回滚" />
          </div>
          {packageTitle === "本草注解增强包" ? (
            <p className="ai-message">系统会把已存在条目的内容作为注解资料附加，避免创建重复主条目。</p>
          ) : null}
          {plan.needsReviewCount > 0 ? <p className="error-text">存在待确认项，本次不会自动执行这些记录。</p> : null}
        </div>
      ) : null}

      <div className="import-actions primary-actions">
        <button type="button" disabled={!plan || Boolean(report)} onClick={executePlan}>开始导入</button>
        <button type="button" onClick={cancelImport}>取消</button>
      </div>

      {report ? (
        <div className="preview-panel">
          <h3>导入报告</h3>
          <div className="summary-grid">
            <Metric label="导入成功数量" value={report.createdCount + report.mergedCount + report.attachedAnnotationCount} />
            <Metric label="跳过数量" value={report.skippedCount} />
            <Metric label="附加注解数量" value={report.attachedAnnotationCount} />
            <Metric label="失败数量" value={readNumber(report.reportJson.failed_count) + report.rejectedCount} />
            <Metric label="导入批次号" value={report.importRunId ? `#${report.importRunId}` : "未记录"} />
            <Metric label="搜索索引" value={report.searchIndexRebuilt ? "已重建" : "未重建"} />
          </div>
          <div className="import-actions">
            <button type="button" disabled={!report.importRunId} onClick={rollbackCurrentRun}>一键回滚本次导入</button>
          </div>
          {runReport ? (
            <details className="advanced-details">
              <summary>查看报告</summary>
              <pre>{JSON.stringify(runReport.summary, null, 2)}</pre>
            </details>
          ) : null}
        </div>
      ) : null}

      <details className="advanced-details">
        <summary>高级详情</summary>
        <p>字段映射、manifest 路径、动作明细和技术字段只在这里查看。标准数据包默认不会进入字段映射。</p>
        <dl>
          <dt>当前路径</dt>
          <dd>{packagePath || "未选择"}</dd>
          <dt>计划编号</dt>
          <dd>{plan?.planId ?? "未生成"}</dd>
          <dt>重复策略</dt>
          <dd>{plan?.duplicatePolicy ?? "未生成"}</dd>
          <dt>记录总数</dt>
          <dd>{plan?.totalRecords ?? "未生成"}</dd>
          <dt>合并补全数</dt>
          <dd>{plan?.updateCount ?? "未生成"}</dd>
        </dl>
        {plan ? (
          <>
            <div className="summary-grid">
              <Metric label="类型分布" value={formatCounts(plan.typeCounts)} />
              <Metric label="分类分布" value={formatCounts(plan.categoryCounts)} />
              <Metric label="缺失字段" value={formatCounts(plan.missingFieldCounts) || "无"} />
              <Metric label="重复 code" value={plan.duplicateCodes.length ? plan.duplicateCodes.join("，") : "无"} />
            </div>
            <KeywordChecks checks={plan.keywordChecks} />
          </>
        ) : null}
        {plan?.warnings.length ? <p className="error-text">{plan.warnings.join("；")}</p> : null}
        {plan?.actions.length ? (
          <div className="staging-table-wrap compact">
            <table className="staging-table">
              <thead>
                <tr>
                  <th>行</th>
                  <th>动作</th>
                  <th>名称</th>
                  <th>原因</th>
                </tr>
              </thead>
              <tbody>
                {plan.actions.slice(0, 40).map((action) => (
                  <tr key={`${action.rowIndex}-${action.actionType}-${action.name ?? ""}`}>
                    <td>{action.rowIndex}</td>
                    <td>{action.actionType}</td>
                    <td>{action.name ?? "-"}</td>
                    <td>{action.reason}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : null}
      </details>

      <details className="advanced-details">
        <summary>高级导入（JSON / CSV）</summary>
        <p>仅 generic_csv、generic_json 或 unknown 数据需要进入这里。系统会先自动识别，只有失败时才需要处理字段映射。</p>
        <label className="stacked-field">
          文件内容
          <textarea
            value={advancedContent}
            rows={6}
            onChange={(event) => {
              setAdvancedContent(event.target.value);
              setAdvancedPreview(null);
            }}
          />
        </label>
        <div className="import-actions">
          <button
            type="button"
            onClick={() => {
              setAdvancedKind("json");
              setAdvancedContent(sampleJson);
              setAdvancedPreview(null);
            }}
          >
            JSON 示例
          </button>
          <button
            type="button"
            onClick={() => {
              setAdvancedKind("csv");
              setAdvancedContent(sampleCsv);
              setAdvancedPreview(null);
            }}
          >
            CSV 示例
          </button>
          <button type="button" onClick={() => previewAdvancedContent()}>预览当前内容</button>
        </div>
        {advancedPreview ? (
          <div className="summary-grid">
            <Metric label="识别类型" value={advancedPreview.detection.detectedType} />
            <Metric label="记录数" value={advancedPreview.detection.recordCount} />
            <Metric label="导入方式" value={advancedPreview.directImportReady ? "标准导入" : "高级确认"} />
            <Metric label="格式" value={advancedKind.toUpperCase()} />
          </div>
        ) : null}
      </details>

      {runs.length ? (
        <details className="advanced-details">
          <summary>最近导入批次</summary>
          <div className="staging-table-wrap compact">
            <table className="staging-table">
              <thead>
                <tr>
                  <th>批次</th>
                  <th>数据包</th>
                  <th>类型</th>
                  <th>状态</th>
                  <th>时间</th>
                </tr>
              </thead>
              <tbody>
                {runs.map((run) => (
                  <tr key={run.id}>
                    <td>#{run.id}</td>
                    <td>{run.packageName ?? "未命名"}</td>
                    <td>{readableIntent(run.importIntent)}</td>
                    <td>{run.rolledBackAt ? "已回滚" : run.status}</td>
                    <td>{run.completedAt ?? run.createdAt}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </details>
      ) : null}

      {message ? <p className="ai-message">{message}</p> : null}
      {error ? <p className="error-text">{error}</p> : null}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: number | string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function readableIntent(intent?: string | null) {
  switch (intent) {
    case "primary_seed":
      return "初始知识数据包";
    case "classic_text":
      return "原典文本数据包";
    case "annotation_enrichment":
      return "注解增强数据包";
    case "relation_enrichment":
      return "关系增强数据包";
    case "search_terms":
      return "搜索词增强数据包";
    case "incremental_update":
      return "增量更新数据包";
    case "backup_restore":
      return "备份恢复数据包";
    default:
      return "标准数据包";
  }
}

function stepLabel(step: ImportStep) {
  switch (step) {
    case "pick":
      return "选择数据包";
    case "analyze":
      return "正在分析";
    case "plan":
      return "确认导入计划";
    case "report":
      return "查看导入报告";
  }
}

function readNumber(value: unknown) {
  return typeof value === "number" ? value : 0;
}

function formatCounts(counts?: Record<string, number>) {
  if (!counts) return "";
  return Object.entries(counts)
    .slice(0, 8)
    .map(([key, value]) => `${key}: ${value}`)
    .join("，");
}

function KeywordChecks({ checks }: { checks?: Record<string, boolean> }) {
  const entries = Object.entries(checks ?? {});
  if (!entries.length) return null;
  return (
    <div className="keyword-checks">
      {entries.map(([keyword, hit]) => (
        <span key={keyword} className={hit ? "status-pill" : "status-pill muted"}>
          {keyword} {hit ? "命中" : "未见"}
        </span>
      ))}
    </div>
  );
}
