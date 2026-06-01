# Smart Import Center V1

Smart Import Center V1 是《中医大全》的商业化导入中心。它把“识别文件、理解 manifest、判断主文件、处理重复、确认入库”收敛成一个面向用户的流程：选择标准数据包，查看导入计划，点击开始导入。

## 边界

- 主软件只导入处理好的标准数据包。
- PDF、Word、图片等原始资料由外部工具处理，主软件不解析 PDF。
- 标准数据包支持 ZIP 和已解压文件夹。
- 不做真实医疗 AI，不做在线问诊，不做自动诊断，不做自动开方。

## 用户体验

用户不需要理解 `primary`、`auxiliary`、重复项、注解包或验收关键词。软件会读取 `import_manifest.json`，自动推断导入意图，生成 `ImportPlan`，并把重复处理策略展示为摘要。

流程：

1. 用户选择 ZIP 数据包或已解压文件夹。
2. 系统分析 manifest 和主数据文件。
3. 系统生成 `ImportPlan`。
4. 用户查看摘要并点击“开始 Smart Import”。
5. 系统执行可自动处理的动作，跳过重复项，需要人工判断的内容保留为待确认。
6. 执行后重建搜索索引并生成导入报告。

## ImportPlan

`ImportPlan` 包含：

- `package_name`
- `import_intent`
- `duplicate_policy`
- `total_records`
- `create_count`
- `update_count`
- `attach_annotation_count`
- `skip_duplicate_count`
- `needs_review_count`
- `warnings`
- `actions`

动作类型：

- `create_new`：新增知识条目。
- `skip_duplicate`：跳过重复。
- `merge_empty_fields`：只补充现有条目的空字段，不覆盖已有内容。
- `attach_annotation`：把资料写入 `knowledge_annotations`。
- `needs_review`：保留待确认，不自动执行。
- `reject_invalid`：拒绝无效记录。

## Manifest 升级

新增可选字段：

- `import_intent`
- `duplicate_policy`
- `ai_assist`

`import_intent` 可选：

- `primary_seed`
- `classic_text`
- `annotation_enrichment`
- `relation_enrichment`
- `search_terms`
- `incremental_update`
- `backup_restore`

`duplicate_policy` 可选：

- `auto`
- `skip_existing`
- `attach_annotation`
- `merge_empty_fields`
- `ask_on_conflict`
- `create_duplicate`

旧 manifest 兼容：

- `classics_curated_v1` 推断为 `classic_text`。
- `pdf_herb_notes_private_v1` 推断为 `annotation_enrichment`。
- 普通 `knowledge_items_v1` 推断为 `primary_seed`。

## 重复处理策略

`annotation_enrichment`：

- 同名中药已存在：`attach_annotation`。
- 同名中药不存在：`create_new`。
- 内容高度相似：`skip_duplicate`。

`primary_seed`：

- 同名同类型且内容相似：`skip_duplicate`。
- 现有条目有空字段且新资料可补充：`merge_empty_fields`。
- 明显冲突：`needs_review`。

`classic_text`：

- 同 `source_note` 或内容已存在：`skip_duplicate`。
- 否则：`create_new`。

`search_terms`：

- 重复 term 自动去重。

## knowledge_annotations

`knowledge_annotations` 用于承载同名药物、方剂、经典条文的不同资料来源。注解不会创建重复主条目，而是附加到现有知识条目。

搜索索引覆盖：

- `knowledge_items.name`
- `alias`
- `category`
- `summary`
- `content`
- `source_note`
- `tags`
- `knowledge_annotations.content`
- `knowledge_annotations.source_note`
- `knowledge_annotations.tags_json`

## AI 辅助预留

AI 只用于低置信度资料整理问题的辅助任务：

- `duplicate_resolution`
- `field_classification`
- `summary`
- `anomaly_detection`

当前不真实调用 AI。如果 AI 未启用，返回：`AI 导入辅助当前未启用，系统使用本地规则处理。`
