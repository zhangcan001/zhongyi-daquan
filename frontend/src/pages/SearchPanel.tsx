import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type SearchResult = {
  itemId: number;
  itemType: string;
  code?: string | null;
  name: string;
  category?: string | null;
  summary?: string | null;
  dataStatus: string;
  matchedBy: string;
};

type SearchResponse = {
  total: number;
  durationMs: number;
  results: SearchResult[];
};

export function SearchPanel() {
  const [query, setQuery] = useState("足三里");
  const [response, setResponse] = useState<SearchResponse | null>(null);
  const [message, setMessage] = useState("");

  async function runSearch(nextQuery = query) {
    const keyword = nextQuery.trim();
    if (!keyword) return;
    setMessage("搜索中...");
    try {
      const result = await invoke<SearchResponse>("search_knowledge", {
        request: { query: keyword, itemType: null, page: 1, pageSize: 20 },
      });
      setResponse(result);
      setMessage(`命中 ${result.total} 条，用时 ${result.durationMs}ms。`);
    } catch (cause) {
      setMessage(String(cause));
    }
  }

  return (
    <div className="panel search-panel">
      <label htmlFor="global-search">全局搜索</label>
      <div className="inline-action-row">
        <input
          id="global-search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              runSearch();
            }
          }}
          placeholder="搜索足三里、ST36、黄芪、补中益气汤、胃经"
        />
        <button type="button" onClick={() => runSearch()}>
          搜索
        </button>
      </div>
      <div className="quick-searches">
        {["黄芪", "足三里", "ST36", "胃经", "补中益气汤"].map((keyword) => (
          <button
            key={keyword}
            type="button"
            onClick={() => {
              setQuery(keyword);
              runSearch(keyword);
            }}
          >
            {keyword}
          </button>
        ))}
      </div>
      {message ? <span>{message}</span> : null}
      {response?.results.length ? (
        <div className="compact-result-list">
          {response.results.map((item) => (
            <div key={item.itemId}>
              <strong>{item.name}</strong>
              <span>{[item.code, item.category, item.dataStatus, item.matchedBy].filter(Boolean).join(" / ")}</span>
              {item.summary ? <small>{item.summary}</small> : null}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
