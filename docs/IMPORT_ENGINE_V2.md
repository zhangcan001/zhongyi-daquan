# Import Engine V2

Import Engine V2 用于解决 v0.1-alpha-package 中“先手工映射所有字段”的导入体验问题。新的流程先识别数据类型，再决定是否直接适配，只有普通 CSV 或未知 JSON 才进入字段评分映射。

## 支持的数据类型

| 类型 | 识别条件 | 处理方式 |
| --- | --- | --- |
| `knowledge_items_v1` | 顶层数组，元素包含 `type/name`，或包含 `content/source_note/detail`，或文件名为 `knowledge_items_import_*` | 直接适配为知识草稿，不要求人工映射 |
| `classic_passages_v1` | 包含 `work_title/original_text/section_title`，或 `classic_id/page_title/original_text`，或文件名为 `classic_passages_*` | 转为 `syndrome` 类型原典条目 |
| `search_terms_v1` | 包含 `term/term_type/weight`，或 `item_name/term` | 当前仅识别并提示，v0.1 不直接导入搜索词表 |
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

## 常见错误

- `manifest 指向的文件不存在`：检查 ZIP 内路径是否与 `import_manifest.json` 一致。
- `JSON 行必须是对象`：标准 JSON 应为对象数组，或包含 `rows` 数组。
- `CSV 引号未闭合`：检查 CSV 中英文引号是否成对。
- 搜索不到新导入数据：确认已执行“确认入库”，系统会在确认后重建搜索索引。
