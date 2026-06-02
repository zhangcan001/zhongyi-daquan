import { useEffect, useMemo, useState } from "react";
import {
  deleteUserNote,
  getKnowledgeDetail,
  recordRecentView,
  saveUserNote,
  toggleFavorite,
} from "./api";
import { detailFields } from "./schema";
import type { KnowledgeAnnotation, KnowledgeDetailResponse, KnowledgeType, UserNote } from "./types";

type Props = {
  itemId: number | null;
  query?: string;
  onBack?: () => void;
  onChanged?: () => void;
};

const typeTitles: Record<string, string> = {
  herb: "中药",
  formula: "方剂",
  acupuncture: "针灸",
  acupoint: "穴位",
  meridian: "经络",
  syndrome: "原典条文",
  theory: "原典章节",
  note: "注解资料",
  disease: "病症",
};

export function KnowledgeDetailReader({ itemId, query = "", onBack, onChanged }: Props) {
  const [detail, setDetail] = useState<KnowledgeDetailResponse | null>(null);
  const [message, setMessage] = useState("");
  const [noteText, setNoteText] = useState("");

  useEffect(() => {
    if (!itemId) {
      setDetail(null);
      return;
    }
    getKnowledgeDetail(itemId)
      .then((response) => {
        setDetail(response);
        setNoteText(response.notes[0]?.noteText ?? "");
        return recordRecentView(itemId);
      })
      .catch((error) => setMessage(String(error)));
  }, [itemId]);

  const importantFields = useMemo(() => {
    if (!detail) return [];
    const fields = detailFields[detail.item.itemType as KnowledgeType] ?? [];
    return fields
      .map((field) => ({
        label: field.label,
        value: stringValue(detail.detail?.[field.key]),
        safetyNote: field.safetyNote,
      }))
      .filter((field) => field.value);
  }, [detail]);

  if (!itemId) {
    return <p className="empty-text">从左侧搜索结果中选择一个条目查看详情。</p>;
  }
  if (!detail) {
    return <p className="empty-text">正在读取详情...</p>;
  }

  const item = detail.item;
  const annotations = detail.annotations ?? [];
  const notes = detail.notes ?? [];
  const sourceLine = [item.sourcePackage, item.sourceNote].filter(Boolean).join(" | ");

  async function copyCurrent() {
    const text = [
      item.name,
      typeTitles[item.itemType] ?? item.itemType,
      item.category,
      item.summary,
      item.content,
      annotations.map((annotation) => `${annotationSource(annotation)}\n${annotation.content}`).join("\n\n"),
    ]
      .filter(Boolean)
      .join("\n\n");
    await navigator.clipboard.writeText(text);
    setMessage("已复制当前条目");
  }

  function toggleCurrentFavorite() {
    if (!item.id) return;
    toggleFavorite(item.id)
      .then((response) => {
        setDetail(response);
        setMessage(response.item.isFavorite ? "已收藏" : "已取消收藏");
        onChanged?.();
      })
      .catch((error) => setMessage(String(error)));
  }

  function saveNote() {
    if (!item.id) return;
    saveUserNote(item.id, noteText)
      .then((note) => {
        setDetail((current) => current && { ...current, notes: [note] });
        setMessage("个人备注已保存");
        onChanged?.();
      })
      .catch((error) => setMessage(String(error)));
  }

  function removeNote(note: UserNote) {
    deleteUserNote(note.id)
      .then(() => {
        setDetail((current) => current && { ...current, notes: [] });
        setNoteText("");
        setMessage("个人备注已删除");
        onChanged?.();
      })
      .catch((error) => setMessage(String(error)));
  }

  return (
    <article className="reader-shell">
      <div className="reader-title-row">
        <div>
          <span className="type-badge">{typeTitles[item.itemType] ?? item.itemType}</span>
          <h2>{highlight(item.name, query)}</h2>
          <p>{[item.category, item.alias ? `别名：${item.alias}` : "", item.pinyin].filter(Boolean).join(" / ")}</p>
        </div>
        <div className="reader-actions">
          {onBack ? (
            <button type="button" onClick={onBack}>
              返回
            </button>
          ) : null}
          <button type="button" onClick={copyCurrent}>
            复制
          </button>
          <button type="button" onClick={toggleCurrentFavorite}>
            {item.isFavorite ? "已收藏" : "收藏"}
          </button>
        </div>
      </div>

      <section className="reader-section">
        <h3>主信息</h3>
        <dl className="meta-list">
          <dt>来源</dt>
          <dd>{sourceLine || "未记录"}</dd>
          <dt>标签</dt>
          <dd>{item.tags || "未记录"}</dd>
          <dt>导入批次</dt>
          <dd>{item.importBatchId ? `#${item.importBatchId}` : item.sourcePackage || "未记录"}</dd>
        </dl>
      </section>

      <section className="reader-section">
        <h3>正文</h3>
        {item.summary ? <p className="reader-summary">{highlight(item.summary, query)}</p> : null}
        {item.content ? <div className="reader-content">{highlight(item.content, query)}</div> : null}
      </section>

      {importantFields.length ? (
        <section className="reader-section">
          <h3>类型专属信息</h3>
          <div className="field-list">
            {importantFields.map((field) => (
              <div key={field.label}>
                <strong>{field.label}</strong>
                <p>{highlight(field.value, query)}</p>
                {field.safetyNote ? <small>{field.safetyNote}</small> : null}
              </div>
            ))}
          </div>
        </section>
      ) : null}

      <section className="reader-section">
        <h3>资料注解</h3>
        {annotations.length ? (
          <div className="annotation-list">
            {annotations.map((annotation, index) => (
              <details
                key={annotation.id}
                className="annotation-block"
                open={index < 2 || Boolean(query.trim() && annotation.content.includes(query))}
              >
                <summary>
                  <span>{annotationSource(annotation)}</span>
                  <small>{annotation.tags || "无标签"}</small>
                </summary>
                <p>{highlight(annotation.content, query)}</p>
              </details>
            ))}
          </div>
        ) : (
          <p className="empty-text">暂无注解资料。</p>
        )}
      </section>

      <section className="reader-section">
        <h3>个人备注</h3>
        <textarea
          className="note-editor"
          value={noteText}
          onChange={(event) => setNoteText(event.target.value)}
          placeholder="记录自己的学习备注，不参与医疗建议。"
        />
        <div className="detail-actions">
          <button type="button" onClick={saveNote}>
            {notes.length ? "保存备注" : "添加备注"}
          </button>
          <button type="button" disabled={!notes.length} onClick={() => notes[0] && removeNote(notes[0])}>
            删除备注
          </button>
        </div>
      </section>
      {message ? <p className="ai-message">{message}</p> : null}
    </article>
  );
}

function annotationSource(annotation: KnowledgeAnnotation) {
  return [annotation.sourceTitle, annotation.sourceNote].filter(Boolean).join(" | ") || "未记录来源";
}

function stringValue(value: unknown) {
  if (value === null || value === undefined) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number") return String(value);
  if (typeof value === "boolean") return value ? "是" : "否";
  return JSON.stringify(value);
}

function highlight(value: string, query: string) {
  if (!query.trim()) return value;
  const parts = value.split(query);
  if (parts.length === 1) return value;
  return parts.map((part, index) => (
    <span key={`${part}-${index}`}>
      {part}
      {index < parts.length - 1 ? <mark>{query}</mark> : null}
    </span>
  ));
}
