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

type MergeDuplicateResponse = {
  candidateId: number;
  status: string;
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
  const [duplicateStatus, setDuplicateStatus] = useState("pending");
  const [relationStatus, setRelationStatus] = useState("pending");
  const [busyAction, setBusyAction] = useState<string | null>(null);

  async function refresh() {
    const [nextDuplicates, nextRelations] = await Promise.all([
      invoke<DuplicateListResponse>("list_duplicate_candidates", {
        request: { status: duplicateStatus || null, page: 1, pageSize: 20 },
      }),
      invoke<RelationListResponse>("list_relation_suggestions", {
        request: { status: relationStatus || null, page: 1, pageSize: 20 },
      }),
    ]);
    setDuplicates(nextDuplicates);
    setRelations(nextRelations);
  }

  useEffect(() => {
    refresh().catch((cause) => setMessage(String(cause)));
  }, [duplicateStatus, relationStatus]);

  async function runDuplicates() {
    setBusyAction("duplicates");
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
    } finally {
      setBusyAction(null);
    }
  }

  async function runRelations() {
    setBusyAction("relations");
    setMessage("关系建议生成中...");
    try {
      const result = await invoke<{ suggestionsCreated: number }>("generate_relation_suggestions", {
        request: { itemType: null, sourceItemId: null },
      });
      await refresh();
      setMessage(`关系建议完成：新增 ${result.suggestionsCreated} 条。`);
    } catch (cause) {
      setMessage(String(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function acceptSuggestion(id: number) {
    setBusyAction(`accept-${id}`);
    try {
      await invoke("accept_relation_suggestion", { suggestionId: id });
      await refresh();
      setMessage("已接受关系建议并写入正式关系。");
    } catch (cause) {
      setMessage(String(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function rejectSuggestion(id: number) {
    setBusyAction(`reject-${id}`);
    try {
      await invoke("reject_relation_suggestion", { suggestionId: id });
      await refresh();
      setMessage("已驳回关系建议。");
    } catch (cause) {
      setMessage(String(cause));
    } finally {
      setBusyAction(null);
    }
  }

  async function mergeDuplicate(id: number, strategy: "merge_tags" | "keep_existing") {
    setBusyAction(`${strategy}-${id}`);
    try {
      const result = await invoke<MergeDuplicateResponse>("merge_duplicate_candidate", {
        request: { candidateId: id, strategy },
      });
      await refresh();
      setMessage(result.status === "kept" ? "已保留现有条目。" : "已合并标签与别名，并刷新搜索索引。");
    } catch (cause) {
      setMessage(String(cause));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <section className="section-band review-panel">
      <div className="section-heading">
        <div>
          <h2>重复检测与关系建议</h2>
          <p>用于验收重复 ST36、方剂组成关系和待处理建议。</p>
        </div>
        <button type="button" onClick={() => refresh()} disabled={busyAction !== null}>
          刷新
        </button>
      </div>

      <div className="import-actions">
        <button type="button" onClick={runDuplicates} disabled={busyAction !== null}>
          运行重复检测
        </button>
        <button type="button" onClick={runRelations} disabled={busyAction !== null}>
          生成关系建议
        </button>
      </div>
      <p className="ai-message">{message}</p>

      <div className="review-grid">
        <div>
          <div className="review-list-heading">
            <h3>重复候选：{duplicates?.total ?? 0}</h3>
            <select value={duplicateStatus} onChange={(event) => setDuplicateStatus(event.target.value)}>
              <option value="pending">待处理</option>
              <option value="merged">已合并</option>
              <option value="kept">已保留</option>
              <option value="">全部</option>
            </select>
          </div>
          <div className="compact-result-list">
            {duplicates?.candidates.length ? (
              duplicates.candidates.map((item) => (
                <div key={item.id}>
                  <strong>{item.existingName ?? item.importedName ?? "未命名候选"}</strong>
                  <span>{[item.duplicateName, item.matchType, item.status].filter(Boolean).join(" / ")}</span>
                  {item.matchScore !== null && item.matchScore !== undefined ? (
                    <small>匹配度：{Math.round(item.matchScore * 100)}%</small>
                  ) : null}
                  {item.reason ? <small>{item.reason}</small> : null}
                  {item.status === "pending" ? (
                    <span className="inline-action-row compact-actions">
                      <button
                        type="button"
                        disabled={busyAction !== null}
                        onClick={() => mergeDuplicate(item.id, "merge_tags")}
                      >
                        合并标签
                      </button>
                      <button
                        type="button"
                        disabled={busyAction !== null}
                        onClick={() => mergeDuplicate(item.id, "keep_existing")}
                      >
                        保留现有
                      </button>
                    </span>
                  ) : null}
                </div>
              ))
            ) : (
              <p className="empty-text">当前状态下暂无重复候选。</p>
            )}
          </div>
        </div>
        <div>
          <div className="review-list-heading">
            <h3>关系建议：{relations?.total ?? 0}</h3>
            <select value={relationStatus} onChange={(event) => setRelationStatus(event.target.value)}>
              <option value="pending">待处理</option>
              <option value="accepted">已接受</option>
              <option value="rejected">已驳回</option>
              <option value="">全部</option>
            </select>
          </div>
          <div className="compact-result-list">
            {relations?.suggestions.length ? (
              relations.suggestions.map((item) => (
                <div key={item.id}>
                  <strong>{[item.sourceName, item.targetName].filter(Boolean).join(" -> ") || "待关联条目"}</strong>
                  <span>{[item.relationType, item.status].filter(Boolean).join(" / ")}</span>
                  {item.confidence !== null && item.confidence !== undefined ? (
                    <small>置信度：{Math.round(item.confidence * 100)}%</small>
                  ) : null}
                  {item.reason ? <small>{item.reason}</small> : null}
                  {item.status === "pending" ? (
                    <span className="inline-action-row compact-actions">
                      <button type="button" disabled={busyAction !== null} onClick={() => acceptSuggestion(item.id)}>
                        接受
                      </button>
                      <button type="button" disabled={busyAction !== null} onClick={() => rejectSuggestion(item.id)}>
                        驳回
                      </button>
                    </span>
                  ) : null}
                </div>
              ))
            ) : (
              <p className="empty-text">当前状态下暂无关系建议。</p>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
