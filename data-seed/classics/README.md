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

标准 `knowledge_items_*` 文件会被识别为 `knowledge_items_v1`，不需要手工字段映射。系统会保留 `content` 原文、`source_note` 出处、`tags`、`detail`，并在确认入库后重建搜索索引。

标准 `classic_passages_*` 文件会被识别为 `classic_passages_v1`。由于 v0.1 尚未建立独立 classic 专表，条文会暂时映射为 `syndrome` 类型，分类为 `原典 / 书名`，正文写入 `content`。

如果后续数据包提供 `import_manifest.json`，ZIP 导入会优先按 manifest 中的 `files` 和 `import_order` 处理；如果只有普通 `manifest.json`，系统会自动查找已知经典数据文件。
