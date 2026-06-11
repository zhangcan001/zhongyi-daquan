# 数据库结构

数据库使用 SQLite，应用初始化时创建 `中医大全数据/database/zhongyi.db`，并执行以下 PRAGMA：

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -64000;
```

## 迁移文件

- `src-tauri/migrations/001_initial_core_schema.sql`：核心知识、导入、清洗、去重、关系、搜索、任务、备份审计和 AI 相关表。
- `src-tauri/migrations/002_ai_reserved_schema.sql`：AI 表兼容迁移。
- `src-tauri/migrations/003_thread_d_search_performance.sql`：搜索性能相关 FTS、缓存和索引调整。

## 核心知识

- `knowledge_items`：主知识表，统一存放中药、方剂、经络、穴位、证型、病症。
- `herb_details`、`formula_details`、`meridian_details`、`acupoint_details`、`syndrome_details`、`disease_details`：类型详情表，通过 `item_id` 关联 `knowledge_items`。
- `knowledge_versions`：知识版本快照。

`data_status` 当前约定：`draft`、`imported`、`needs_fix`、`validated`、`ready`、`archived`。默认搜索只返回 `validated` 和 `ready`。

## 导入与清洗

- `data_import_batches`：一次导入任务的批次信息。
- `data_import_rows`：导入暂存区行数据，保留 raw、mapped、normalized 三阶段 JSON。
- `data_validation_issues`：校验问题。
- `field_mapping_templates`：字段映射模板。
- `standard_terms`：标准词表。
- `validation_rules`：校验规则。
- `data_transform_steps`：清洗步骤。
- `data_transform_row_changes`：清洗行级变更。

## 去重与关系

- `duplicate_candidates`：重复候选。
- `merge_records`：合并记录。
- `knowledge_fingerprints`：去重指纹。
- `relation_suggestions`：关系建议。
- `knowledge_relations`：正式关系。
- `relation_count_cache`：关系数量缓存。

## 搜索与缓存

- `knowledge_fts`：FTS5 全文搜索表。
- `search_terms`：中文、拼音、编号、别名等搜索词表。
- `knowledge_list_view_cache`：列表分页缓存，避免大列表反复联表。
- `performance_logs`：搜索、分页、重建索引等性能记录。

## 后台、审计与 AI

- `background_jobs`：后台任务。
- `audit_logs`：审计日志。
- `ai_provider_settings`、`ai_prompt_templates`、`ai_tasks`、`ai_drafts`、`ai_call_logs`：AI 设置、任务、草稿和调用日志。AI 默认关闭，用户配置 OpenAI-compatible API 后才发起请求。

## 关键索引

知识表按 `type`、`data_status`、`type + data_status`、`code`、`name`、`pinyin`、`category`、`updated_at` 建索引。

关系表按 `source_item_id`、`target_item_id`、`relation_type`、`source_item_id + relation_type` 建索引。

导入、重复、搜索词、列表缓存、AI 任务和性能日志均有面向回归场景的索引。完整定义以迁移 SQL 为准。
