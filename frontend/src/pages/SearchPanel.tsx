import { useState } from "react";
import { searchKnowledgeEnhanced } from "../modules/knowledge/api";
import { KnowledgeDetailReader } from "../modules/knowledge/KnowledgeDetailReader";
import type { EnhancedSearchResponse, EnhancedSearchResult } from "../modules/knowledge/types";

const filters = ["全部", "中药", "方剂", "针灸", "原典", "注解"];
const quickSearches = ["人参", "甘草", "黄耆", "黄芪", "倪注", "桂枝汤", "太阳病", "上古天真论", "足三里", "理中丸"];

export function SearchPanel() {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("全部");
  const [response, setResponse] = useState<EnhancedSearchResponse | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [message, setMessage] = useState("");

  async function runSearch(nextQuery = query, nextFilter = filter) {
    const keyword = nextQuery.trim();
    if (!keyword) {
      setMessage("请输入搜索关键词");
      return;
    }
    setMessage("搜索中...");
    try {
      const result = await searchKnowledgeEnhanced({
        query: keyword,
        filter: nextFilter,
        page: 1,
        pageSize: 60,
      });
      setResponse(result);
      setSelectedId(firstResultId(result));
      setMessage(`命中 ${result.total} 条，用时 ${result.durationMs}ms。`);
    } catch (cause) {
      setMessage(String(cause));
    }
  }

  function chooseFilter(nextFilter: string) {
    setFilter(nextFilter);
    if (query.trim()) {
      runSearch(query, nextFilter);
    }
  }

  return (
    <div className="panel search-panel">
      <label htmlFor="global-search">全局搜索</label>
      <div className="inline-action-row search-bar-large">
        <input
          id="global-search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") runSearch();
          }}
          placeholder="搜索中药、方剂、穴位、经络、原典、注解……"
        />
        <button type="button" onClick={() => runSearch()}>
          搜索
        </button>
      </div>

      <div className="filter-pills">
        {filters.map((item) => (
          <button key={item} className={item === filter ? "active" : ""} type="button" onClick={() => chooseFilter(item)}>
            {item}
          </button>
        ))}
      </div>

      <div className="quick-searches">
        {quickSearches.map((keyword) => (
          <button
            key={keyword}
            type="button"
            onClick={() => {
              setQuery(keyword);
              runSearch(keyword, filter);
            }}
          >
            {keyword}
          </button>
        ))}
      </div>

      {message ? <span>{message}</span> : null}

      {response && response.total === 0 ? (
        <div className="empty-suggestions">
          <strong>没有找到结果</strong>
          <span>换一个关键词试试</span>
          <span>如搜索黄芪未命中，可试试黄耆</span>
          <span>确认是否已导入相关数据包</span>
        </div>
      ) : null}

      {response?.groups.length ? (
        <div className="search-layout">
          <div className="grouped-results">
            {response.groups.map((group) => (
              <section key={group.groupName} className="result-group">
                <h3>{group.groupName}</h3>
                {group.results.map((item) => (
                  <SearchCard
                    key={item.itemId}
                    item={item}
                    query={response.query}
                    active={item.itemId === selectedId}
                    onOpen={() => setSelectedId(item.itemId)}
                  />
                ))}
              </section>
            ))}
          </div>
          <div className="search-detail">
            <KnowledgeDetailReader itemId={selectedId} query={response.query} />
          </div>
        </div>
      ) : null}
    </div>
  );
}

function SearchCard({
  item,
  query,
  active,
  onOpen,
}: {
  item: EnhancedSearchResult;
  query: string;
  active: boolean;
  onOpen: () => void;
}) {
  const source = [item.sourceTitle, item.sourceNote].filter(Boolean).join(" | ");
  return (
    <button className={active ? "result-card active" : "result-card"} type="button" onClick={onOpen}>
      <span className="result-title-row">
        <strong>{highlight(item.name, query)}</strong>
        <em>{item.typeLabel}</em>
      </span>
      <span>{[item.category, item.code].filter(Boolean).join(" / ") || "未记录分类"}</span>
      {item.summary ? <small>{highlight(item.summary, query)}</small> : null}
      {item.contentSnippet ? <small>{highlight(item.contentSnippet, query)}</small> : null}
      {item.annotationSnippet ? <small className="annotation-hit">{highlight(item.annotationSnippet, query)}</small> : null}
      <span className="source-line">{source || item.sourcePackage || "未记录来源"}</span>
      <span className="result-flags">
        {item.hasAnnotations ? `注解 ${item.annotationCount}` : "无注解"}
        {item.importBatchId ? `来源批次 #${item.importBatchId}` : item.sourcePackage ? `来源包 ${item.sourcePackage}` : ""}
      </span>
    </button>
  );
}

function firstResultId(response: EnhancedSearchResponse) {
  return response.groups[0]?.results[0]?.itemId ?? null;
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
