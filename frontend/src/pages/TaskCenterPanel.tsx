import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  BackgroundJob,
  BackupReport,
  MaintenanceReport,
  RestoreReport,
} from "../modules/runtime/types";

const jobTypeLabels: Record<string, string> = {
  import_batch: "批量导入",
  clean_batch: "批量清洗/维护",
  dedup_batch: "批量去重",
  relation_suggest_batch: "关系建议",
  rebuild_search_index: "重建搜索索引",
  backup: "备份",
  restore: "恢复",
  ai_task: "AI 占位任务",
  clear_database_content: "清空数据库内容",
};

const statusLabels: Record<string, string> = {
  pending: "等待中",
  running: "运行中",
  success: "成功",
  failed: "失败",
};

export function TaskCenterPanel() {
  const [jobs, setJobs] = useState<BackgroundJob[]>([]);
  const [message, setMessage] = useState<string>("任务中心准备就绪。");
  const [restoreDir, setRestoreDir] = useState<string>("");
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [restoreReport, setRestoreReport] = useState<RestoreReport | null>(null);

  const refreshJobs = async () => {
    const nextJobs = await invoke<BackgroundJob[]>("list_jobs", {
      request: { limit: 50 },
    });
    setJobs(nextJobs);
  };

  useEffect(() => {
    refreshJobs().catch((cause) => setMessage(String(cause)));
  }, []);

  const runAction = async <T,>(action: string, command: string, args?: Record<string, unknown>) => {
    setBusyAction(action);
    setMessage("任务执行中，请稍候。");
    try {
      const report = await invoke<T>(command, args);
      setMessage(formatReport(report));
      if (command === "create_backup" && isBackupReport(report)) {
        setRestoreDir(report.backupDir);
      }
      if (command === "restore_backup") {
        setRestoreReport(report as RestoreReport);
      }
      await refreshJobs();
    } catch (cause) {
      setMessage(String(cause));
    } finally {
      setBusyAction(null);
    }
  };

  const clearDatabaseContent = async () => {
    const firstConfirm = window.confirm(
      "此操作会清空知识库、导入记录、搜索索引、关系、日志等业务数据，且不能直接撤销。建议先执行备份。确定继续？",
    );
    if (!firstConfirm) return;
    const typed = window.prompt("请再输入“清空数据库”确认执行。");
    if (typed !== "清空数据库") {
      setMessage("已取消清空数据库。");
      return;
    }
    await runAction<MaintenanceReport>("clear-db", "clear_database_content");
  };

  return (
    <section className="section-band task-center">
      <div className="section-heading">
        <div>
          <h2>任务中心</h2>
          <p>后台任务、备份恢复和数据库维护。</p>
        </div>
        <button type="button" onClick={() => refreshJobs()} disabled={busyAction !== null}>
          刷新
        </button>
      </div>

      <div className="maintenance-grid">
        <button
          type="button"
          onClick={() => runAction<BackupReport>("backup", "create_backup")}
          disabled={busyAction !== null}
        >
          执行备份
        </button>
        <button
          type="button"
          onClick={() => runAction<MaintenanceReport>("rebuild", "run_rebuild_search_index_job")}
          disabled={busyAction !== null}
        >
          重建索引
        </button>
        <button
          type="button"
          onClick={() => runAction<MaintenanceReport>("optimize", "optimize_database")}
          disabled={busyAction !== null}
        >
          优化数据库
        </button>
        <button
          type="button"
          onClick={() => runAction<MaintenanceReport>("clean-temp", "clean_temp_imports")}
          disabled={busyAction !== null}
        >
          清理临时导入
        </button>
        <button
          type="button"
          onClick={() =>
            runAction<MaintenanceReport>("clean-logs", "clean_old_performance_logs", {
              request: { keepDays: 30 },
            })
          }
          disabled={busyAction !== null}
        >
          清理性能日志
        </button>
        <button
          type="button"
          onClick={() => runAction<MaintenanceReport>("export-report", "export_performance_report")}
          disabled={busyAction !== null}
        >
          导出性能报告
        </button>
        <button
          className="danger-button"
          type="button"
          onClick={clearDatabaseContent}
          disabled={busyAction !== null}
        >
          一键清空数据库内容
        </button>
      </div>

      <div className="restore-row">
        <input
          value={restoreDir}
          onChange={(event) => setRestoreDir(event.target.value)}
          placeholder="输入备份目录路径，例如 ...\\backups\\backup-20260529120000-manual"
        />
        <button
          type="button"
          onClick={() =>
            runAction<RestoreReport>("restore", "restore_backup", {
              request: { backupDir: restoreDir },
            })
          }
          disabled={busyAction !== null || restoreDir.trim().length === 0}
        >
          恢复备份
        </button>
      </div>

      <p className="task-message">{message}</p>
      {restoreReport ? (
        <div className="restore-report">
          <strong>恢复报告</strong>
          <span>来源：{restoreReport.restoredFrom}</span>
          <span>恢复前备份：{restoreReport.safetyBackupDir}</span>
          <span>{restoreReport.rebuildSearchIndexNote}</span>
        </div>
      ) : null}

      <div className="job-table-wrap">
        <table className="job-table">
          <thead>
            <tr>
              <th>任务类型</th>
              <th>状态</th>
              <th>进度</th>
              <th>错误信息</th>
              <th>更新时间</th>
            </tr>
          </thead>
          <tbody>
            {jobs.length === 0 ? (
              <tr>
                <td colSpan={5}>暂无后台任务。</td>
              </tr>
            ) : (
              jobs.map((job) => (
                <tr key={job.id}>
                  <td>{jobTypeLabels[job.jobType] ?? job.jobType}</td>
                  <td>{statusLabels[job.status] ?? job.status}</td>
                  <td>
                    <div className="progress-cell">
                      <progress value={job.progress} max={100} />
                      <span>{Math.round(job.progress)}%</span>
                    </div>
                  </td>
                  <td>{job.errorMessage ?? ""}</td>
                  <td>{job.updatedAt}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function formatReport(report: unknown): string {
  if (isBackupReport(report)) {
    return `备份完成：${report.backupDir}`;
  }
  if (isMaintenanceReport(report)) {
    return report.message;
  }
  if (isRestoreReport(report)) {
    return `恢复完成：${report.rebuildSearchIndexNote}`;
  }
  return "任务已完成。";
}

function isBackupReport(report: unknown): report is BackupReport {
  return Boolean(report && typeof report === "object" && "backupDir" in report);
}

function isMaintenanceReport(report: unknown): report is MaintenanceReport {
  return Boolean(report && typeof report === "object" && "message" in report);
}

function isRestoreReport(report: unknown): report is RestoreReport {
  return Boolean(report && typeof report === "object" && "restoredFrom" in report);
}
