import { useCallback, useEffect, useMemo, useState } from "react";
import {
  createKnowledgeItem,
  deleteKnowledgeItem,
  getKnowledgeDetail,
  listKnowledgeItems,
  setKnowledgeFavorite,
  updateKnowledgeItem,
} from "../modules/knowledge/api";
import { KnowledgeDetailReader } from "../modules/knowledge/KnowledgeDetailReader";
import { herbNatureClass, herbNatureFromItem, meridianElementClass, meridianElementFromItem } from "../modules/knowledge/natureColor";
import { detailFields, emptyKnowledgeInput } from "../modules/knowledge/schema";
import {
  completenessOptions,
  dataStatusOptions,
  knowledgeTypeOptions,
  type KnowledgeInput,
  type KnowledgeItem,
  type KnowledgeType,
} from "../modules/knowledge/types";

type Mode = "create" | "edit";

export function KnowledgeWorkspace() {
  const [typeFilter, setTypeFilter] = useState<KnowledgeType | "">("herb");
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const [items, setItems] = useState<KnowledgeItem[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [mode, setMode] = useState<Mode>("create");
  const [form, setForm] = useState<KnowledgeInput>(() => emptyKnowledgeInput("herb"));
  const [versions, setVersions] = useState(0);
  const [message, setMessage] = useState("");
  const [loading, setLoading] = useState(false);

  const selectedType = form.itemType;
  const pageCount = Math.max(1, Math.ceil(total / 50));

  const loadList = useCallback(() => {
    setLoading(true);
    listKnowledgeItems({
      itemType: typeFilter,
      query,
      dataStatus: statusFilter,
      page,
      pageSize: 50,
    })
      .then((response) => {
        setItems(response.items);
        setTotal(response.total);
      })
      .catch((error) => setMessage(String(error)))
      .finally(() => setLoading(false));
  }, [page, query, statusFilter, typeFilter]);

  useEffect(() => {
    loadList();
  }, [loadList]);

  function startCreate(itemType: KnowledgeType = (typeFilter || "herb") as KnowledgeType) {
    setMode("create");
    setSelectedId(null);
    setVersions(0);
    setForm(emptyKnowledgeInput(itemType));
    setMessage("");
  }

  function loadDetail(itemId: number) {
    getKnowledgeDetail(itemId)
      .then((response) => {
        setSelectedId(itemId);
        setMode("edit");
        setVersions(response.versions.length);
        setForm({
          itemType: response.item.itemType,
          code: response.item.code ?? "",
          name: response.item.name,
          alias: response.item.alias ?? "",
          pinyin: response.item.pinyin ?? "",
          category: response.item.category ?? "",
          summary: response.item.summary ?? "",
          content: response.item.content ?? "",
          sourceNote: response.item.sourceNote ?? "",
          tags: response.item.tags ?? "",
          dataStatus: response.item.dataStatus,
          completenessStatus: response.item.completenessStatus,
          isFavorite: response.item.isFavorite,
          importBatchId: response.item.importBatchId ?? "",
          sourcePackage: response.item.sourcePackage ?? "",
          detail: response.detail ?? {},
        });
      })
      .catch((error) => setMessage(String(error)));
  }

  function updateForm<K extends keyof KnowledgeInput>(key: K, value: KnowledgeInput[K]) {
    setForm((current) => ({ ...current, [key]: value }));
  }

  function updateDetail(key: string, value: string) {
    setForm((current) => ({
      ...current,
      detail: {
        ...current.detail,
        [key]: key.endsWith("ItemId") && value.trim() ? Number(value) : value,
      },
    }));
  }

  function saveForm() {
    const action =
      mode === "edit" && selectedId
        ? updateKnowledgeItem(selectedId, form)
        : createKnowledgeItem(form);
    action
      .then((response) => {
        setSelectedId(response.item.id ?? null);
        setMode("edit");
        setVersions(response.versions.length);
        setMessage("已保存知识条目");
        loadList();
      })
      .catch((error) => setMessage(String(error)));
  }

  function removeSelected() {
    if (!selectedId) return;
    const ok = window.confirm("确定删除当前知识条目？删除前会保存版本快照。");
    if (!ok) return;
    deleteKnowledgeItem(selectedId)
      .then(() => {
        setMessage("已删除知识条目");
        startCreate();
        loadList();
      })
      .catch((error) => setMessage(String(error)));
  }

  function toggleFavorite() {
    if (!selectedId) {
      updateForm("isFavorite", !form.isFavorite);
      return;
    }
    setKnowledgeFavorite(selectedId, !form.isFavorite)
      .then((response) => {
        updateForm("isFavorite", response.item.isFavorite);
        setMessage(response.item.isFavorite ? "已收藏" : "已取消收藏");
        loadList();
      })
      .catch((error) => setMessage(String(error)));
  }

  const detailInputs = useMemo(() => detailFields[selectedType], [selectedType]);

  return (
    <section className="workspace-split">
      <div className="list-pane">
        <div className="section-heading">
          <div>
            <h2>知识库</h2>
            <p>六类知识统一分页列表，支持筛选、收藏和详情编辑。</p>
          </div>
          <button type="button" onClick={() => startCreate()}>
            新增
          </button>
        </div>

        <div className="filter-row">
          <select
            value={typeFilter}
            onChange={(event) => {
              setTypeFilter(event.target.value as KnowledgeType | "");
              setPage(1);
            }}
          >
            <option value="">全部类型</option>
            {knowledgeTypeOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <select
            value={statusFilter}
            onChange={(event) => {
              setStatusFilter(event.target.value);
              setPage(1);
            }}
          >
            <option value="">全部状态</option>
            {dataStatusOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <input
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setPage(1);
            }}
            placeholder="名称、编号、拼音、标签"
          />
        </div>

        <div className="knowledge-list" aria-busy={loading}>
          {items.map((item) => (
            <KnowledgeListRow
              item={item}
              key={item.id}
              active={item.id === selectedId}
              onOpen={() => item.id && loadDetail(item.id)}
            />
          ))}
          {items.length === 0 ? <p className="empty-text">暂无知识条目</p> : null}
        </div>

        <div className="pager">
          <button type="button" disabled={page <= 1} onClick={() => setPage((value) => value - 1)}>
            上一页
          </button>
          <span>
            {page} / {pageCount}，共 {total} 条
          </span>
          <button
            type="button"
            disabled={page >= pageCount}
            onClick={() => setPage((value) => value + 1)}
          >
            下一页
          </button>
        </div>
      </div>

      <div className="detail-pane">
        <div className="section-heading">
          <div>
            <h2>{mode === "create" ? "新增知识" : "知识详情"}</h2>
            <p>先阅读资料、来源和注解；需要整理时再编辑字段。</p>
          </div>
          <button type="button" onClick={toggleFavorite}>
            {form.isFavorite ? "取消收藏" : "收藏"}
          </button>
        </div>

        {mode === "edit" && selectedId ? (
          <KnowledgeDetailReader itemId={selectedId} query={query} onChanged={loadList} />
        ) : null}

        <details className="advanced-details edit-details" open={mode === "create"}>
          <summary>{mode === "create" ? "填写新知识条目" : "整理与编辑字段"}</summary>

        <div className="form-grid">
          <label>
            类型
            <select
              value={form.itemType}
              onChange={(event) => updateForm("itemType", event.target.value as KnowledgeType)}
            >
              {knowledgeTypeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            名称
            <input value={form.name} onChange={(event) => updateForm("name", event.target.value)} />
          </label>
          <label>
            编号
            <input value={form.code ?? ""} onChange={(event) => updateForm("code", event.target.value)} />
          </label>
          <label>
            别名
            <input value={form.alias ?? ""} onChange={(event) => updateForm("alias", event.target.value)} />
          </label>
          <label>
            拼音
            <input value={form.pinyin ?? ""} onChange={(event) => updateForm("pinyin", event.target.value)} />
          </label>
          <label>
            分类
            <input value={form.category ?? ""} onChange={(event) => updateForm("category", event.target.value)} />
          </label>
          <label>
            数据状态
            <select
              value={form.dataStatus}
              onChange={(event) => updateForm("dataStatus", event.target.value as KnowledgeInput["dataStatus"])}
            >
              {dataStatusOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            完整度
            <select
              value={form.completenessStatus}
              onChange={(event) =>
                updateForm("completenessStatus", event.target.value as KnowledgeInput["completenessStatus"])
              }
            >
              {completenessOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>

        <label className="wide-field">
          摘要
          <textarea value={form.summary ?? ""} onChange={(event) => updateForm("summary", event.target.value)} />
        </label>
        <label className="wide-field">
          内容
          <textarea value={form.content ?? ""} onChange={(event) => updateForm("content", event.target.value)} />
        </label>
        <div className="form-grid">
          <label>
            出处
            <input value={form.sourceNote ?? ""} onChange={(event) => updateForm("sourceNote", event.target.value)} />
          </label>
          <label>
            标签
            <input value={form.tags ?? ""} onChange={(event) => updateForm("tags", event.target.value)} />
          </label>
        </div>
        {mode === "edit" ? (
          <div className="summary-grid compact">
            <div>
              <span>导入批次</span>
              <strong>{form.importBatchId ? `#${form.importBatchId}` : "手工/旧数据"}</strong>
            </div>
            <div>
              <span>来源包</span>
              <strong>{form.sourcePackage || "未记录"}</strong>
            </div>
          </div>
        ) : null}

        <h3>类型详情</h3>
        <div className="form-grid">
          {detailInputs.map((field) => (
            <label key={field.key} className={field.kind === "textarea" ? "span-2" : undefined}>
              {field.label}
              {field.kind === "textarea" ? (
                <textarea
                  value={String(form.detail[field.key] ?? "")}
                  onChange={(event) => updateDetail(field.key, event.target.value)}
                />
              ) : (
                <input
                  type={field.kind === "number" ? "number" : "text"}
                  value={String(form.detail[field.key] ?? "")}
                  onChange={(event) => updateDetail(field.key, event.target.value)}
                />
              )}
            </label>
          ))}
        </div>

        <details className="advanced-details">
          <summary>detail JSON</summary>
          <pre>{JSON.stringify(form.detail ?? {}, null, 2)}</pre>
        </details>

        <div className="detail-actions">
          <button type="button" onClick={saveForm}>
            保存
          </button>
          <button type="button" disabled={!selectedId} onClick={removeSelected}>
            删除
          </button>
          <span>版本历史：{versions} 条</span>
        </div>
        {message ? <p className="ai-message">{message}</p> : null}
        </details>
      </div>
    </section>
  );
}

function KnowledgeListRow({
  item,
  active,
  onOpen,
}: {
  item: KnowledgeItem;
  active: boolean;
  onOpen: () => void;
}) {
  const nature = herbNatureFromItem(item);
  const element = meridianElementFromItem(item);
  return (
    <button className={active ? "knowledge-row active" : "knowledge-row"} type="button" onClick={onOpen}>
      <span>
        <strong className={herbNatureClass(nature) ?? meridianElementClass(element)}>{item.name}</strong>
        <small>{[item.code, item.category, item.sourceNote].filter(Boolean).join(" / ") || "未填写编号"}</small>
      </span>
      <span className="row-meta">
        {nature ? <em className={`nature-chip herb-nature-${nature.tone}`}>{nature.label}</em> : null}
        {element ? <em className={`element-chip meridian-element-${element.tone}`}>{element.organ} / {element.label}</em> : null}
        {item.isFavorite ? "收藏" : ""}
        {item.dataStatus}
      </span>
    </button>
  );
}
