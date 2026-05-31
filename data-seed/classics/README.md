# 四部经典数据包说明

本目录用于放置经典数据导入规范和小体积示例，不存放完整精校数据包。

建议完整数据包范围：

- 《黄帝内经》
- 《伤寒论》
- 《金匮要略》
- 《神农本草经》

完整数据包建议作为本地导入包或 GitHub Release 附件交付。导入前应确认来源、版权、校注说明和字段映射。

示例文件：

- `classics.sample.json`：仅用于验证导入字段结构，不代表完整数据。

## Import Engine V2 兼容说明

推荐导入顺序：

1. `json/knowledge_items_import_curated.json`
2. `json/knowledge_items_import_full_clean.json`
3. `json/classic_passages_curated.json`
4. `json/classic_passages_full_clean.json`

标准 `knowledge_items_*` 文件会被识别为 `knowledge_items_v1`，不需要手工字段映射。系统会保留 `content` 原文、`source_note` 出处、`tags`、`detail`，并在确认入库后重建搜索索引、追加包内搜索词和生成质量报告。

标准 `classic_passages_*` 文件会被识别为 `classic_passages_v1`。由于 v0.1 尚未建立独立 classic 专表，条文会暂时映射为 `syndrome` 类型，分类为 `原典 / 书名`，正文写入 `content`。

如果后续数据包提供 `import_manifest.json`，ZIP 导入会优先按 manifest 中的 `files` 和 `import_order` 处理；如果只有普通 `manifest.json`，系统会自动查找已知经典数据文件。

## v0.3 manifest 包

推荐 Release 附件名：

- `zhongyi_classics_curated_v0_3_manifest.zip`

该包应在根目录包含 `import_manifest.json`，示例见：

- `import_manifest.example.json`

当前 v0.1 的 manifest 导入策略是先导入 `primary: true` 的主知识 JSON，即 `json/knowledge_items_import_curated.json`。`classic_passages` 和 `search_terms` 会在概览中展示；系统会先从主知识 JSON 的 `name/code/category/tags` 派生搜索词，独立 `search_terms_curated.json` 的完整入库后续由专门导入能力接入。

确认入库后建议打开“质量报告”，检查字段覆盖率、疑似重复指纹、包内搜索词数量和关键词抽检结果。如果误导入，可使用批次回滚删除本批次写入数据。

导入后建议搜索：

- `桂枝汤`
- `太阳病`
- `上古天真论`
- `神农本草经`
- `黄帝内经`
- `金匮要略`
