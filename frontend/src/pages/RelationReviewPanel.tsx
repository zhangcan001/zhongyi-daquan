import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type DuplicateCandidate = {
  id: number;
  existingName?: string | null;
  duplicateName?: string | null;
  importedName?: string | null;
  matchType: string;
  matchScore?: number | null;
  reason?: string | null;
  status: string;
};

type DuplicateListResponse = {
  total: number;
  candidates: DuplicateCandidate[];
};

type RelationSuggestion = {
  id: number;
  sourceName?: string | null;
  targetName?: string | null;
  relationType: string;
  confidence?: number | null;
  reason?: string | null;
  status: string;
};

type RelationListResponse = {
  total: number;
  suggestions: RelationSuggestion[];
};

export function RelationReviewPanel() {
  const [duplicates, setDuplicates] = useState<DuplicateListResponse | null>(null);
  const [relations, setRelations] = useState<RelationListResponse | null>(null);
  const [message, setMessage] = useState("可触发重复检测与关系建议。");

  async function refresh() {
    const [nextDuplicates, nextRelations] = await Promise.all([
      invoke<DuplicateListResponse>("list_duplicate_candidates", {
        request: { status: "pending", page: 1, pageSize: 20 },
      }),
      invoke<RelationListResponse>("list_relation_suggestions", {
        request: { status: "pending", page: 1, pageSize: 20 },
      }),
    ]);
    setDuplicates(nextDuplicates);
    setRelations(nextRelations);
  }

  useEffect(() => {
    refresh().catch((cause) => setMessage(String(cause)));
  }, []);

  async function runDuplicates() {
    setMessage("重复检测中...");
    try {
      const result = await invoke<{ fingerprintsUpserted: number; candidatesCreated: number }>(
        "run_duplicate_detection",
        { request: { batchId: null, itemType: null } },
      );
      await refresh();
      setMessage(`重复检测完成：更新指纹 ${result.fingerprintsUpserted} 条，新增候选 ${result.candidatesCreated} 条。`);
    } catch (cause) {
      setMessage(String(cause));
    }
  }

  async function runRelations() {
    setMessage("关系建议生成中...");
    try {
      const result = await invoke<{ suggestionsCreated: number }>("generate_relation_suggestions", {
        request: { itemType: null, sourceItemId: null },
      });
      await refresh();
      setMessage(`关系建议完成：新增 ${result.suggestionsCreated} 条。`);
    } catch (cause) {
      setMessage(String(cause));
    }
  }

  async function acceptSuggestion(id: number) {
    try {
      await invoke("accept_relation_suggestion", { suggestionId: id });
      await refresh();
      setMessage("已接受关系建议并写入正式关系。");
    } catch (cause) {
      setMessage(String(cause));
    }
  }

  return (
    <section className="section-band review-panel">
      <div className="section-heading">
        <div>
          <h2>重复检测与关系建议</h2>
          <p>用于验收重复 ST36、方剂组成关系和待处理建议。</p>
        </div>
        <button type="button" onClick={() => refresh()}>
          刷新
        </button>
      </div>

      <div className="import-actions">
        <button type="button" onClick={runDuplicates}>
          运行重复检测
        </button>
        <button type="button" onClick={runRelations}>
          生成关系建议
        </button>
      </div>
      <p className="ai-message">{message}</p>

      <div className="review-grid">
        <div>
          <h3>重复候选：{duplicates?.total ?? 0}</h3>
          <div className="compact-result-list">
            {duplicates?.candidates.map((item) => (
              <div key={item.id}>
                <strong>{item.existingName ?? item.importedName ?? "未命名候选"}</strong>
                <span>{[item.duplicateName, item.matchType, item.status].filter(Boolean).join(" / ")}</span>
                {item.reason ? <small>{item.reason}</small> : null}
              </div>
            ))}
          </div>
        </div>
        <div>
          <h3>关系建议：{relations?.total ?? 0}</h3>
          <div className="compact-result-list">
            {relations?.suggestions.map((item) => (
              <div key={item.id}>
                <strong>{[item.sourceName, item.targetName].filter(Boolean).join(" -> ") || "待关联条目"}</strong>
                <span>{[item.relationType, item.status].filter(Boolean).join(" / ")}</span>
                {item.reason ? <small>{item.reason}</small> : null}
                <button type="button" onClick={() => acceptSuggestion(item.id)}>
                  接受
                </button>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
