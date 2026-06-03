import { ClipboardEvent, useMemo, useState } from "react";
import { saveGridDirtyRows } from "../modules/knowledge/api";
import { emptyKnowledgeInput } from "../modules/knowledge/schema";
import {
  completenessOptions,
  dataStatusOptions,
  knowledgeTypeOptions,
  type KnowledgeInput,
  type KnowledgeType,
} from "../modules/knowledge/types";

const supportedTypes: KnowledgeType[] = ["acupoint", "herb", "formula"];

type GridColumn = {
  key: string;
  label: string;
  width: string;
  options?: Array<{ value: string; label: string }>;
};

const columnsByType: Partial<Record<KnowledgeType, GridColumn[]>> = {
  herb: [
    { key: "name", label: "名称", width: "150px" },
    { key: "code", label: "编号", width: "110px" },
    { key: "category", label: "分类", width: "120px" },
    { key: "dataStatus", label: "状态", width: "120px", options: dataStatusOptions },
    { key: "completenessStatus", label: "完整度", width: "110px", options: completenessOptions },
    { key: "natureFlavor", label: "性味", width: "150px" },
    { key: "fourQi", label: "四气", width: "120px" },
    { key: "fiveFlavors", label: "五味", width: "140px" },
    { key: "meridians", label: "归经", width: "150px" },
    { key: "channelTropism", label: "归经/脏腑", width: "150px" },
    { key: "toxicity", label: "毒性", width: "120px" },
    { key: "origin", label: "产地/来源", width: "220px" },
    { key: "effects", label: "功效", width: "220px" },
    { key: "indications", label: "主治", width: "220px" },
    { key: "classicApplications", label: "经方应用", width: "220px" },
  ],
  formula: [
    { key: "name", label: "名称", width: "160px" },
    { key: "code", label: "编号", width: "110px" },
    { key: "category", label: "分类", width: "120px" },
    { key: "dataStatus", label: "状态", width: "120px", options: dataStatusOptions },
    { key: "completenessStatus", label: "完整度", width: "110px", options: completenessOptions },
    { key: "sourceText", label: "出处", width: "160px" },
    { key: "composition", label: "组成", width: "260px" },
    { key: "effects", label: "功效", width: "220px" },
    { key: "indications", label: "主治", width: "220px" },
  ],
  acupoint: [
    { key: "name", label: "名称", width: "140px" },
    { key: "code", label: "编号", width: "110px" },
    { key: "category", label: "分类", width: "120px" },
    { key: "dataStatus", label: "状态", width: "120px", options: dataStatusOptions },
    { key: "completenessStatus", label: "完整度", width: "110px", options: completenessOptions },
    { key: "acupointCode", label: "穴位编号", width: "130px" },
    { key: "meridianItemId", label: "经络ID", width: "100px" },
    { key: "bodyRegion", label: "部位", width: "120px" },
    { key: "standardLocation", label: "标准定位", width: "260px" },
    { key: "functions", label: "功能", width: "220px" },
    { key: "indications", label: "关联症候", width: "220px" },
  ],
  meridian: [],
  syndrome: [],
  disease: [],
};

export function GridEntryPage() {
  const [itemType, setItemType] = useState<KnowledgeType>("acupoint");
  const [rows, setRows] = useState<KnowledgeInput[]>(() => makeRows("acupoint", 12));
  const [dirtyRows, setDirtyRows] = useState<Set<number>>(() => new Set());
  const [selectedRow, setSelectedRow] = useState(0);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [message, setMessage] = useState("");

  const columns = useMemo(() => columnsByType[itemType] ?? columnsByType.herb ?? [], [itemType]);
  const visibleRows = rows;

  function resetType(nextType: KnowledgeType) {
    setItemType(nextType);
    setRows(makeRows(nextType, 12));
    setDirtyRows(new Set());
    setErrors({});
    setSelectedRow(0);
    setMessage("");
  }

  function updateCell(rowIndex: number, key: string, value: string) {
    setRows((current) =>
      current.map((row, index) => (index === rowIndex ? writeCell(row, key, value) : row)),
    );
    setDirtyRows((current) => new Set(current).add(rowIndex));
    validateCellOnChange(rowIndex, key, value);
  }

  function addRow() {
    setRows((current) => [...current, emptyKnowledgeInput(itemType)]);
  }

  function copyRow() {
    const source = rows[selectedRow] ?? emptyKnowledgeInput(itemType);
    setRows((current) => [...current, structuredClone(source)]);
    setDirtyRows((current) => new Set(current).add(rows.length));
  }

  function handlePaste(event: ClipboardEvent<HTMLDivElement>) {
    const text = event.clipboardData.getData("text");
    if (!text.includes("\t") && !text.includes("\n")) return;
    event.preventDefault();
    const pastedRows = text
      .trimEnd()
      .split(/\r?\n/)
      .map((line) => line.split("\t"));
    setRows((current) => {
      const next = [...current];
      pastedRows.forEach((cells, offset) => {
        const rowIndex = selectedRow + offset;
        let row = next[rowIndex] ?? emptyKnowledgeInput(itemType);
        columns.forEach((column, columnIndex) => {
          if (cells[columnIndex] !== undefined) {
            row = writeCell(row, column.key, cells[columnIndex]);
          }
        });
        next[rowIndex] = row;
      });
      return next;
    });
    setDirtyRows((current) => {
      const next = new Set(current);
      pastedRows.forEach((_, offset) => next.add(selectedRow + offset));
      return next;
    });
  }

  function validateDirtyRows() {
    const nextErrors: Record<string, string> = {};
    dirtyRows.forEach((rowIndex) => {
      const row = rows[rowIndex];
      if (!row) return;

      if (!row.name.trim()) {
        nextErrors[`${rowIndex}:name`] = "名称不能为空";
      }

      if (row.code && !validateCode(row.code)) {
        nextErrors[`${rowIndex}:code`] = "编号格式错误，应为大写字母开头，如 ST36";
      }

      if (itemType === "acupoint") {
        const acupointCode = readCell(row, "acupoint_code") || readCell(row, "acupointCode");
        if (acupointCode && !validateAcupointCode(String(acupointCode))) {
          nextErrors[`${rowIndex}:acupointCode`] = "穴位编号格式错误，应为 ST36 格式";
        }
      }

      if (row.itemType !== itemType) {
        nextErrors[`${rowIndex}:itemType`] = "行类型与表格类型不一致";
      }
    });
    setErrors(nextErrors);
    return Object.keys(nextErrors).length === 0;
  }

  function validateCode(code: string): boolean {
    if (!code.trim()) return true;
    const trimmed = code.trim();
    return /^[A-Z][A-Z0-9_-]*$/.test(trimmed);
  }

  function validateAcupointCode(code: string): boolean {
    if (!code.trim()) return true;
    return /^[A-Z]{2}\d+$/.test(code.trim());
  }

  function validateCellOnChange(rowIndex: number, key: string, value: string) {
    const row = rows[rowIndex];
    if (!row) return;

    const errorKey = `${rowIndex}:${key}`;

    if (key === "name" && !value.trim()) {
      setErrors((current) => ({ ...current, [errorKey]: "名称不能为空" }));
      return;
    }

    if (key === "code" && value && !validateCode(value)) {
      setErrors((current) => ({ ...current, [errorKey]: "编号格式错误" }));
      return;
    }

    if ((key === "acupointCode" || key === "acupoint_code") && value && !validateAcupointCode(value)) {
      setErrors((current) => ({ ...current, [errorKey]: "穴位编号格式错误" }));
      return;
    }

    setErrors((current) => {
      const next = { ...current };
      delete next[errorKey];
      return next;
    });
  }

  function saveRows() {
    if (!validateDirtyRows()) {
      setMessage("请先修正高亮单元格");
      return;
    }
    const payload = Array.from(dirtyRows).map((index) => rows[index]).filter(Boolean);
    saveGridDirtyRows(itemType, payload)
      .then((response) => {
        const nextErrors: Record<string, string> = {};
        response.errors.forEach((error) => {
          const originalRow = Array.from(dirtyRows)[error.rowIndex];
          nextErrors[`${originalRow}:${error.fieldName}`] = error.message;
        });
        setErrors(nextErrors);
        if (response.errors.length === 0) {
          setDirtyRows(new Set());
        }
        setMessage(`已保存 ${response.savedCount} 行，错误 ${response.errors.length} 行`);
      })
      .catch((error) => setMessage(String(error)));
  }

  return (
    <section className="section-band">
      <div className="section-heading">
        <div>
          <h2>表格录入</h2>
          <p>支持新增行、复制行、批量粘贴、下拉选择、错误高亮和 dirty_rows 批量保存。</p>
        </div>
        <div className="grid-actions">
          <select value={itemType} onChange={(event) => resetType(event.target.value as KnowledgeType)}>
            {supportedTypes.map((type) => (
              <option key={type} value={type}>
                {knowledgeTypeOptions.find((option) => option.value === type)?.label}
              </option>
            ))}
          </select>
          <button type="button" onClick={addRow}>
            新增行
          </button>
          <button type="button" onClick={copyRow}>
            复制行
          </button>
          <button type="button" onClick={saveRows}>
            保存 dirty_rows
          </button>
        </div>
      </div>

      <div className="grid-entry-shell" onPaste={handlePaste}>
        <div className="grid-entry-table" style={{ gridTemplateColumns: columns.map((col) => col.width).join(" ") }}>
          {columns.map((column) => (
            <div className="grid-head" key={column.key}>
              {column.label}
            </div>
          ))}
          {visibleRows.map((row, rowIndex) =>
            columns.map((column) => {
              const error = errors[`${rowIndex}:${column.key}`];
              return (
                <div
                  className={error ? "grid-cell error-cell" : "grid-cell"}
                  key={`${rowIndex}:${column.key}`}
                  title={error}
                  onFocus={() => setSelectedRow(rowIndex)}
                >
                  {column.options ? (
                    <select
                      value={String(readCell(row, column.key) ?? "")}
                      onChange={(event) => updateCell(rowIndex, column.key, event.target.value)}
                    >
                      {column.options.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      value={String(readCell(row, column.key) ?? "")}
                      onChange={(event) => updateCell(rowIndex, column.key, event.target.value)}
                    />
                  )}
                </div>
              );
            }),
          )}
        </div>
      </div>
      <div className="grid-footer">
        <span>当前行数：{rows.length}</span>
        <span>dirty_rows：{dirtyRows.size}</span>
        <span>虚拟滚动结构：表格容器已固定高度，可替换为窗口化渲染。</span>
      </div>
      {message ? <p className="ai-message">{message}</p> : null}
    </section>
  );
}

function makeRows(itemType: KnowledgeType, count: number) {
  return Array.from({ length: count }, () => emptyKnowledgeInput(itemType));
}

function readCell(row: KnowledgeInput, key: string) {
  if (key in row && key !== "detail") {
    return row[key as keyof KnowledgeInput];
  }
  return row.detail[key] ?? "";
}

function writeCell(row: KnowledgeInput, key: string, value: string): KnowledgeInput {
  if (key in row && key !== "detail") {
    return { ...row, [key]: value };
  }
  return {
    ...row,
    detail: {
      ...row.detail,
      [key]: key.endsWith("ItemId") && value.trim() ? Number(value) : value,
    },
  };
}
