# 导入回滚

Smart Import Center V1 会为每次执行的导入计划创建一个 `import_runs` 批次，并把每个实际动作写入 `import_run_changes`。因此用户导入后可以在报告页执行“一键回滚本次导入”。

## 可回滚内容

- `create_new`：删除本次新建的 `knowledge_items`，并通过外键同步清理详情、注解等关联数据。
- `attach_annotation`：删除本次新增的 `knowledge_annotations`。
- `merge_empty_fields`：使用 `before_json` 恢复导入前为空、且本次补全过的字段。
- 回滚完成后会重建搜索索引。

## 不会回滚内容

- `skip_duplicate`：导入时没有写入数据，不处理。
- `needs_review`：未自动执行，不处理。
- `reject_invalid` / `failed`：没有成功写入，不处理。
- 导入后被用户手动修改过的补空字段：系统会提示风险并跳过，避免静默覆盖用户后续编辑。

## 表结构

- `import_runs`：导入批次、摘要计数、状态、报告 JSON、完成时间和回滚时间。
- `import_run_changes`：每个动作的实体、目标、`before_json`、`after_json` 和回滚动作。
- `import_reports`：导入报告和回滚报告。

## 前端展示

普通用户只看到导入报告摘要、导入批次号、查看报告和“一键回滚本次导入”。动作明细、技术路径和策略字段放在“高级详情”折叠区，默认隐藏。
