# Import Quality V1

Import Quality V1 是 Import Engine V2 之后的收口增强，目标是让真实数据包导入后可检查、可追踪、可回滚。

## 已增强能力

- 确认入库时记录本批次写入的 `knowledge_items` ID。
- 确认入库时写入 `knowledge_fingerprints`，便于后续去重与合并建议。
- 确认入库后生成导入质量报告。
- 标准知识 JSON 与经典段落导入后补充包内搜索词。
- 支持按导入批次回滚本批次写入的知识条目，并重建搜索索引。

## 质量报告

质量报告包含：

- 识别类型、总行数、可导入行、警告行、错误行。
- `type/code/name/category/summary/content/source_note/tags` 覆盖率。
- 空字段统计。
- 批内疑似重复指纹数。
- 导入搜索词数量。
- 关键词搜索抽检结果。
- 修复建议。

当前关键词抽检包含：

- `桂枝汤`
- `太阳病`
- `上古天真论`
- `神农本草经`

## 批次回滚

回滚依据是 `data_import_batches.confirmed_item_ids_json`，只删除该批次确认入库时写入的知识条目。

回滚会同时处理：

- 删除本批次知识条目。
- 删除对应搜索词。
- 删除对应知识指纹。
- 将批次与行状态标记为 `rolled_back`。
- 重建搜索索引和列表缓存。

回滚不做跨批次合并拆分。如果用户在导入后手动编辑了本批次写入的知识条目，回滚仍会删除这些条目。

## 搜索词处理

确认入库后会先重建搜索索引，再追加包内导入搜索词，避免追加词被索引重建清空。

当前 v0.1 追加搜索词来源：

- `name`
- `code`
- `category`
- `tags`

这保证经典数据包中的标题、编号、原典分类、标签能稳定参与搜索。独立 `search_terms_curated.json` 的完整入库仍建议作为下一轮数据处理增强。

## 验收方式

后端测试：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml import_project_service::tests -- --nocapture
```

重点覆盖：

- manifest ZIP 主知识文件导入。
- 质量报告生成。
- `confirmed_item_ids_json` 写入。
- `imported_package` 搜索词追加。
- 批次回滚后搜索不再命中。

