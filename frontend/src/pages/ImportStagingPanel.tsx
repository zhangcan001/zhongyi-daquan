import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  FieldMappingSuggestion,
  FieldMappingTemplate,
  ImportBatchSummary,
  ImportParsedPreview,
  Mapping,
  StagingPage,
} from "../modules/importPipeline/types";

const sampleJson = JSON.stringify(
  [
    { code: " st36 ", name: " 足三里 ", type: "穴位", meridians: "胃经", tags: "常用，保健" },
    { code: "bad code", name: "", type: "穴位", meridians: "未知经" },
  ],
  null,
  2,
);
const sampleCsv = "code,name,type,meridians,tags\r\n st36 , 足三里 ,穴位,胃经,常用;保健\r\nbad code,,穴位,未知经,\r\n";

const knowledgeTypes = [
  { value: "mixed", label: "自动识别 / 混合类型" },
  { value: "中药", label: "中药" },
  { value: "方剂", label: "方剂" },
  { value: "经络", label: "经络" },
  { value: "穴位", label: "穴位" },
  { value: "证型", label: "证型" },
  { value: "病症", label: "病症" },
];

export function ImportStagingPanel() {
  const [importType, setImportType] = useState<"json" | "csv" | "zip">("json");
  const [targetType, setTargetType] = useState("穴位");
  const [fileName, setFileName] = useState("manual-import.json");
  const [content, setContent] = useState(sampleJson);
  const [binaryContent, setBinaryContent] = useState<number[] | null>(null);
  const [mappingText, setMappingText] = useState("");
  const [templateName, setTemplateName] = useState("默认导入映射");
  const [templates, setTemplates] = useState<FieldMappingTemplate[]>([]);
  const [selectedTemplateId, setSelectedTemplateId] = useState("");
  const [summary, setSummary] = useState<ImportBatchSummary | null>(null);
  const [staging, setStaging] = useState<StagingPage | null>(null);
  const [preview, setPreview] = useState<ImportParsedPreview | null>(null);
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
    const lowerName = file.name.toLowerCase();
    const nextType = lowerName.endsWith(".zip") ? "zip" : lowerName.endsWith(".csv") ? "csv" : "json";
    setImportType(nextType);
    if (nextType === "zip") {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      setBinaryContent(bytes);
      setContent("ZIP 数据包已读取，内容不在文本框中展开。");
      await previewZip(file.name, bytes);
    } else {
      setBinaryContent(null);
      const text = await file.text();
      setContent(text);
      await previewTextImport(nextType, text);
    }
    if (file.name.includes("knowledge_items_import") || file.name.includes("curated")) {
      setTargetType("mixed");
    }
  }

  async function previewTextImport(nextType = importType, nextContent = content) {
    if (nextType === "zip") return;
    try {
      const command = nextType === "json" ? "preview_json_import" : "preview_csv_import";
      setPreview(await invoke<ImportParsedPreview>(command, { content: nextContent }));
    } catch (cause) {
      setPreview(null);
      setError(String(cause));
    }
  }

  async function previewZip(nextFileName = fileName, bytes = binaryContent) {
    if (!bytes) return;
    try {
      setPreview(await invoke<ImportParsedPreview>("preview_zip_import", { fileName: nextFileName, content: bytes }));
    } catch (cause) {
      setPreview(null);
      setError(String(cause));
    }
  }

  async function importToStaging() {
    setError("");
    setMessage("");
    try {
      const mapping = mappingText.trim() ? (JSON.parse(mappingText) as Mapping) : undefined;
      const request = {
        fileName,
        targetType: preview?.directImportReady ? "mixed" : targetType,
        content,
        mapping,
        templateId: selectedTemplateId ? Number(selectedTemplateId) : null,
      };
      const result = importType === "zip"
        ? await invoke<ImportBatchSummary>("import_zip_to_staging", { request, content: binaryContent ?? [] })
        : await invoke<ImportBatchSummary>(importType === "json" ? "import_json_to_staging" : "import_csv_to_staging", { request });
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
          <input type="file" accept=".json,.csv,.zip,application/json,text/csv,application/zip" onChange={(event) => loadFile(event.target.files?.[0] ?? null)} />
        </label>
        <label>
          格式
          <select
            value={importType}
            onChange={(event) => {
              const nextType = event.target.value as "json" | "csv" | "zip";
              setImportType(nextType);
              setBinaryContent(null);
              setPreview(null);
              setFileName(nextType === "json" ? "manual-import.json" : nextType === "csv" ? "manual-import.csv" : "manual-import.zip");
              setContent(nextType === "json" ? sampleJson : nextType === "csv" ? sampleCsv : "请选择 ZIP 数据包。");
            }}
          >
            <option value="json">JSON</option>
            <option value="csv">CSV</option>
            <option value="zip">ZIP 数据包</option>
          </select>
        </label>
        <label>
          知识类型
          <select value={targetType} onChange={(event) => setTargetType(event.target.value)}>
            {knowledgeTypes.map((type) => (
              <option key={type.value} value={type.value}>
                {type.label}
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
        <textarea
          value={content}
          onChange={(event) => {
            setContent(event.target.value);
            setPreview(null);
          }}
          rows={7}
          disabled={importType === "zip"}
        />
      </label>

      <div className="import-actions">
        <button type="button" onClick={() => importType === "zip" ? previewZip() : previewTextImport()}>识别文件</button>
      </div>

      {preview ? (
        <div className="preview-panel">
          <div className="summary-grid">
            <Metric label="识别类型" value={preview.detection.detectedType} />
            <Metric label="置信度" value={`${Math.round(preview.detection.confidence * 100)}%`} />
            <Metric label="记录数" value={preview.detection.recordCount} />
            <Metric label="导入方式" value={preview.directImportReady ? "可直接导入" : "需映射确认"} />
          </div>
          <p className="ai-message">{preview.detection.reason}</p>
          {preview.warnings.length ? <p className="error-text">{preview.warnings.join("；")}</p> : null}
          {!preview.directImportReady && preview.mappingSuggestions.length ? <MappingSuggestionTable suggestions={preview.mappingSuggestions} /> : null}
        </div>
      ) : null}

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
        <button
          type="button"
          onClick={() => {
            setImportType("json");
            setFileName("manual-import.json");
            setContent(sampleJson);
            setBinaryContent(null);
            setPreview(null);
          }}
        >
          载入 JSON 示例
        </button>
        <button
          type="button"
          onClick={() => {
            setImportType("csv");
            setFileName("manual-import.csv");
            setContent(sampleCsv);
            setBinaryContent(null);
            setPreview(null);
          }}
        >
          载入 CSV 示例
        </button>
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
                <th>类型</th>
                <th>编号</th>
                <th>摘要 / 内容</th>
                <th>原始字段</th>
                <th>映射后字段</th>
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
                  <td>{String(row.normalized.type ?? "")}</td>
                  <td>{String(row.normalized.code ?? "")}</td>
                  <td>{previewText(row.normalized.summary ?? row.normalized.content)}</td>
                  <td>{Object.keys(row.raw ?? {}).slice(0, 8).join("，")}</td>
                  <td>{Object.keys(row.mapped ?? {}).slice(0, 8).join("，")}</td>
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

function previewText(value: unknown) {
  const text = typeof value === "string" ? value : value == null ? "" : JSON.stringify(value);
  return text.length > 80 ? `${text.slice(0, 80)}...` : text;
}

function Metric({ label, value }: { label: string; value: number | string }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function MappingSuggestionTable({ suggestions }: { suggestions: FieldMappingSuggestion[] }) {
  return (
    <div className="staging-table-wrap compact">
      <table className="staging-table">
        <thead>
          <tr>
            <th>原始字段</th>
            <th>建议目标</th>
            <th>置信度</th>
            <th>处理</th>
            <th>原因</th>
          </tr>
        </thead>
        <tbody>
          {suggestions.map((suggestion) => (
            <tr key={suggestion.sourceField}>
              <td>{suggestion.sourceField}</td>
              <td>{suggestion.targetField ?? "不自动映射"}</td>
              <td>{Math.round(suggestion.confidence * 100)}%</td>
              <td>{suggestion.decision === "auto" ? "自动勾选" : suggestion.decision === "confirm" ? "需确认" : "忽略"}</td>
              <td>{suggestion.reason}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function readSourceHeaders(content: string, importType: "json" | "csv" | "zip") {
  try {
    if (importType === "zip") return [];
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
