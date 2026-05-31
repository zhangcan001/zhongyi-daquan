# Import Engine V2

Import Engine V2 用于解决 v0.1-alpha-package 中“先手工映射所有字段”的导入体验问题。新的流程先识别数据类型，再决定是否直接适配，只有普通 CSV 或未知 JSON 才进入字段评分映射。

## 支持的数据类型

| 类型 | 识别条件 | 处理方式 |
| --- | --- | --- |
| `knowledge_items_v1` | 顶层数组，元素包含 `type/name`，或包含 `content/source_note/detail`，或文件名为 `knowledge_items_import_*` | 直接适配为知识草稿，不要求人工映射 |
| `classic_passages_v1` | 包含 `work_title/original_text/section_title`，或 `classic_id/page_title/original_text`，或文件名为 `classic_passages_*` | 转为 `syndrome` 类型原典条目 |
| `search_terms_v1` | 包含 `term/term_type/weight`，或 `item_name/term` | 当前可识别并展示；确认导入知识主文件后会追加由名称、编号、分类、标签派生的包内搜索词 |
| `standard_terms_v1` | 包含 `term_type/standard_name/aliases` | 当前仅识别并提示，后续由维护工具接入 |
| `relation_suggestions_v1` | 包含 `source_name/target_name/relation_type`，或 `source_type/target_type` | 当前仅识别并提示，后续由关系工具接入 |
| `generic_csv` | 未命中特定结构的 CSV | 使用 MappingScorer 输出映射候选 |
| `generic_json` | 未命中特定结构的 JSON | 使用 MappingScorer 输出映射候选 |
| `unknown` | 无法解析或字段过少 | 提示用户检查文件 |

识别结果会返回 `detected_type`、`confidence`、`reason`、`sample_fields` 和 `record_count`。

## 标准数据包不需要人工映射

`knowledge_items_v1` 与 `classic_passages_v1` 字段结构已经稳定，系统会直接走 Adapter：

- `type/name/code/category/summary/content/source_note/tags` 会原样保留。
- `tags` 支持数组，也支持逗号、顿号、分号分隔的字符串。
- `detail` 对象会按知识类型写入对应详情字段。
- `source_url/classic_id/page_title/section_title` 会尽量并入 `source_note` 或 `notes`。
- 空字段不会导致整批失败。
- 确认入库后会重建搜索索引。
- 确认入库后会生成导入质量报告，可查看字段覆盖率、批内重复指纹、搜索抽检结果。
- 已确认入库的批次可按批次回滚，回滚会删除本批次写入条目并重建搜索索引。

## ClassicPassagesAdapter

`classic_passages_v1` 当前没有独立 classic 专表，因此先映射为知识条目：

| 输出字段 | 规则 |
| --- | --- |
| `type` | `syndrome` |
| `name` | `work_title + section_title` |
| `category` | `原典 / work_title` |
| `content` | `original_text` |
| `source_note` | 原 `source_note`，或 `work_title/page_title/section_title` |
| `tags` | `原典, work_title, section_title` |
| `detail/notes` | 保留 `classic_id/page_title/section_title/original_text/source_url` 等上下文 |

## MappingScorer

普通 CSV / unknown JSON 使用评分制：

| 因素 | 分值 |
| --- | --- |
| 字段名匹配 | 40 |
| 字段值模式 | 35 |
| 上下文字段 | 15 |
| 知识类型先验 | 10 |

置信度规则：

- `>= 0.85`：自动映射。
- `0.55 - 0.85`：需要用户确认。
- `< 0.55`：不自动映射。

## ZIP 与 manifest

ZIP 数据包如果包含 `import_manifest.json`，优先按 manifest 导入。

```json
{
  "package_name": "zhongyi_classics_curated_v0_3",
  "schema_version": "1.0",
  "import_profile": "classics_curated_v1",
  "files": [
    {
      "path": "json/knowledge_items_import_curated.json",
      "type": "knowledge_items_v1",
      "target": "knowledge_items",
      "primary": true
    }
  ],
  "import_order": ["knowledge_items", "search_terms"]
}
```

如果 ZIP 只有普通 `manifest.json`，系统会把它当作包说明，再自动查找 `json/knowledge_items_import_curated.json`、`json/classic_passages_curated.json` 等已知文件。

v0.3 manifest 数据包建议命名为 `zhongyi_classics_curated_v0_3_manifest.zip`。当前 v0.1 导入策略为：自动暂存 `primary: true` 的主知识文件，其他文件会显示在概览中，但不会混入同一导入批次。确认入库后会从主知识文件的 `name/code/category/tags` 追加包内搜索词；独立 `search_terms_curated.json` 的完整字段级入库后续由专门工具接入。

## 导入质量与回滚

确认入库后可在导入暂存页查看“质量报告”。报告会显示：

- 字段覆盖率，重点关注 `content`、`source_note`、`tags`。
- 批内疑似重复指纹数。
- 导入搜索词数量。
- 经典关键词抽检命中情况。
- 可执行修复建议。

如果导入批次明显错误，可点击“回滚批次”。系统只回滚该批次确认入库时创建的知识条目，并同步清理搜索词、知识指纹和搜索索引。详见 `docs/IMPORT_QUALITY_V1.md`。

## 常见错误

- `manifest 指向的文件不存在`：检查 ZIP 内路径是否与 `import_manifest.json` 一致。
- `找不到 manifest`：如果没有 `import_manifest.json`，系统会按内置经典包规则查找主 JSON；若 ZIP 结构被多包一层目录，也支持自动匹配结尾路径。
- `缺少主 JSON`：确认 ZIP 内存在 `json/knowledge_items_import_curated.json`。
- `detail 字段异常`：标准 `detail` 应为对象；无法识别字段会尽量保留到 `notes`。
- `tags 格式异常`：建议使用数组，或使用逗号、顿号、分号分隔字符串。
- `JSON 行必须是对象`：标准 JSON 应为对象数组，或包含 `rows` 数组。
- `CSV 引号未闭合`：检查 CSV 中英文引号是否成对。
- 搜索不到新导入数据：确认已执行“确认入库”，系统会在确认后重建搜索索引。

导入后建议搜索验收：

- `桂枝汤`
- `太阳病`
- `上古天真论`
- `神农本草经`
- `黄帝内经`
- `金匮要略`
