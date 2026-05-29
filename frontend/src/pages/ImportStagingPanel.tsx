import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FieldMappingTemplate, ImportBatchSummary, Mapping, StagingPage } from "../modules/importPipeline/types";

const sampleJson = JSON.stringify(
  [
    { code: " st36 ", name: " 足三里 ", type: "穴位", meridians: "胃经", tags: "常用，保健" },
    { code: "bad code", name: "", type: "穴位", meridians: "未知经" },
  ],
  null,
  2,
);

const knowledgeTypes = ["中药", "方剂", "经络", "穴位", "证型", "病症"];

export function ImportStagingPanel() {
  const [importType, setImportType] = useState<"json" | "csv">("json");
  const [targetType, setTargetType] = useState("穴位");
  const [fileName, setFileName] = useState("manual-import.json");
  const [content, setContent] = useState(sampleJson);
  const [mappingText, setMappingText] = useState("");
  const [templateName, setTemplateName] = useState("默认导入映射");
  const [templates, setTemplates] = useState<FieldMappingTemplate[]>([]);
  const [selectedTemplateId, setSelectedTemplateId] = useState("");
  const [summary, setSummary] = useState<ImportBatchSummary | null>(null);
  const [staging, setStaging] = useState<StagingPage | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  const batchId = summary?.batch.id ?? staging?.summary.batch.id;
  const sourceHeaders = useMemo(() => readSourceHeaders(content, importType), [content, importType]);

  useEffect(() => {
    invoke<FieldMappingTemplate[]>("list_field_mapping_templates", { targetType })
      .then(setTemplates)
      .catch((cause) => setError(String(cause)));
  }, [targetType, summary?.batch.id]);

  async function loadFile(file: File | null) {
    if (!file) return;
    setFileName(file.name);
    setImportType(file.name.toLowerCase().endsWith(".csv") ? "csv" : "json");
    setContent(await file.text());
  }

  async function importToStaging() {
    setError("");
    setMessage("");
    try {
      const mapping = mappingText.trim() ? (JSON.parse(mappingText) as Mapping) : undefined;
      const command = importType === "json" ? "import_json_to_staging" : "import_csv_to_staging";
      const result = await invoke<ImportBatchSummary>(command, {
        request: {
          fileName,
          targetType,
          content,
          mapping,
          templateId: selectedTemplateId ? Number(selectedTemplateId) : null,
        },
      });
      setSummary(result);
      await loadStaging(result.batch.id);
      setMessage("已导入暂存区，未写入正式知识库。");
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function loadStaging(id = batchId) {
    if (!id) return;
    const result = await invoke<StagingPage>("get_import_staging_page", {
      batchId: id,
      page: 1,
      pageSize: 50,
    });
    setStaging(result);
    setSummary(result.summary);
  }

  async function saveTemplate() {
    setError("");
    try {
      const mapping = mappingText.trim() ? (JSON.parse(mappingText) as Mapping) : Object.fromEntries(sourceHeaders.map((header) => [header, header]));
      await invoke("save_field_mapping_template", {
        request: { name: templateName, targetType, sourceHeaders, mapping },
      });
      setMessage("字段映射模板已保存。");
      setTemplates(await invoke<FieldMappingTemplate[]>("list_field_mapping_templates", { targetType }));
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function runClean(stepType: string) {
    if (!batchId) return;
    setError("");
    try {
      const result = await invoke<{ affectedRows: number }>("apply_import_clean_step", {
        request: { batchId, stepType, params: null },
      });
      await loadStaging(batchId);
      setMessage(`清洗完成，影响 ${result.affectedRows} 行。`);
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function undoClean() {
    if (!batchId) return;
    const result = await invoke<{ affectedRows: number }>("undo_last_import_clean_step", { batchId });
    await loadStaging(batchId);
    setMessage(`已撤销上一步清洗，影响 ${result.affectedRows} 行。`);
  }

  async function confirmImport() {
    if (!batchId) return;
    const result = await invoke<{ importedCount: number; skippedCount: number }>("confirm_import_batch", { batchId });
    await loadStaging(batchId);
    setMessage(`确认入库完成：导入 ${result.importedCount} 行，跳过 ${result.skippedCount} 行。`);
  }

  return (
    <section className="section-band import-panel">
      <div className="section-heading">
        <div>
          <h2>批量导入与暂存区</h2>
          <p>JSON / CSV 先进入暂存区，完成映射、清洗和校验后再确认入库。</p>
        </div>
        <div className="status-pill muted">{batchId ? `批次 #${batchId}` : "未创建批次"}</div>
      </div>

      <div className="import-grid">
        <label>
          文件
          <input type="file" accept=".json,.csv,application/json,text/csv" onChange={(event) => loadFile(event.target.files?.[0] ?? null)} />
        </label>
        <label>
          格式
          <select value={importType} onChange={(event) => setImportType(event.target.value as "json" | "csv")}>
            <option value="json">JSON</option>
            <option value="csv">CSV</option>
          </select>
        </label>
        <label>
          知识类型
          <select value={targetType} onChange={(event) => setTargetType(event.target.value)}>
            {knowledgeTypes.map((type) => (
              <option key={type} value={type}>
                {type}
              </option>
            ))}
          </select>
        </label>
        <label>
          套用模板
          <select value={selectedTemplateId} onChange={(event) => setSelectedTemplateId(event.target.value)}>
            <option value="">自动映射</option>
            {templates.map((template) => (
              <option key={template.id} value={template.id}>
                {template.name}
              </option>
            ))}
          </select>
        </label>
      </div>

      <label className="stacked-field">
        导入内容
        <textarea value={content} onChange={(event) => setContent(event.target.value)} rows={7} />
      </label>

      <div className="mapping-row">
        <label>
          模板名称
          <input value={templateName} onChange={(event) => setTemplateName(event.target.value)} />
        </label>
        <label>
          人工映射 JSON
          <input
            placeholder='{"穴位编号":"code","穴名":"name","归经":"meridians"}'
            value={mappingText}
            onChange={(event) => setMappingText(event.target.value)}
          />
        </label>
      </div>

      <div className="import-actions">
        <button type="button" onClick={importToStaging}>导入暂存区</button>
        <button type="button" onClick={saveTemplate}>保存映射模板</button>
        <button type="button" disabled={!batchId} onClick={() => runClean("normalize_all")}>标准化清洗</button>
        <button type="button" disabled={!batchId} onClick={undoClean}>撤销清洗</button>
        <button type="button" disabled={!batchId} onClick={confirmImport}>确认入库</button>
      </div>

      {summary ? (
        <div className="summary-grid">
          <Metric label="总行数" value={summary.totalRows} />
          <Metric label="可导入数" value={summary.importableRows} />
          <Metric label="警告数" value={summary.warningRows} />
          <Metric label="错误数" value={summary.errorRows} />
        </div>
      ) : null}

      {message ? <p className="ai-message">{message}</p> : null}
      {error ? <p className="error-text">{error}</p> : null}

      {staging?.rows.length ? (
        <div className="staging-table-wrap">
          <table className="staging-table">
            <thead>
              <tr>
                <th>行</th>
                <th>状态</th>
                <th>名称</th>
                <th>编号</th>
                <th>错误原因</th>
                <th>修正建议</th>
              </tr>
            </thead>
            <tbody>
              {staging.rows.map((row) => (
                <tr key={row.id}>
                  <td>{row.rowIndex}</td>
                  <td>{row.status}</td>
                  <td>{String(row.normalized.name ?? "")}</td>
                  <td>{String(row.normalized.code ?? "")}</td>
                  <td>{row.issues.map((issue) => issue.message).join("；")}</td>
                  <td>{row.issues.map((issue) => issue.suggestion).filter(Boolean).join("；")}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
    </section>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function readSourceHeaders(content: string, importType: "json" | "csv") {
  try {
    if (importType === "csv") {
      return content.split(/\r?\n/)[0]?.split(",").map((header) => header.trim()).filter(Boolean) ?? [];
    }
    const parsed = JSON.parse(content) as unknown;
    const first = Array.isArray(parsed) ? parsed[0] : parsed && typeof parsed === "object" && "rows" in parsed ? (parsed as { rows?: unknown[] }).rows?.[0] : parsed;
    return first && typeof first === "object" ? Object.keys(first) : [];
  } catch {
    return [];
  }
}
