# Import Engine V2

Import Engine V2 用于解决 v0.1-alpha-package 中“先手工映射所有字段”的导入体验问题。新的流程先识别数据类型，再决定是否直接适配，只有普通 CSV 或未知 JSON 才进入字段评分映射。

Smart Import Center V1 在 Import Engine V2 之上增加导入计划编排：标准 ZIP / 已解压文件夹会先生成 `ImportPlan`，自动处理重复、补空字段和注解附加；只有 `generic_csv` / `generic_json` / `unknown` 继续进入高级字段映射。普通用户界面默认不显示字段映射、manifest 技术路径和重复明细。详见 `docs/SMART_IMPORT_CENTER.md`。

## 支持的数据类型

| 类型 | 识别条件 | 处理方式 |
| --- | --- | --- |
| `knowledge_items_v1` | 顶层数组，元素包含 `type/name`，或包含 `content/source_note/detail`，或文件名为 `knowledge_items_import_*` | 直接适配为知识草稿，不要求人工映射 |
| `classic_passages_v1` | 包含 `work_title/original_text/section_title`，或 `classic_id/page_title/original_text`，或文件名为 `classic_passages_*` | 转为 `syndrome` 类型原典条目 |
| `search_terms_v1` | 包含 `term/term_type/weight`，或 `item_name/term`，也支持 `item_code/term` | 直接写入 `search_terms`，按 `item_id/item_code/item_name` 解析知识条目 |
| `standard_terms_v1` | 包含 `term_type/standard_name/aliases` | 直接写入或更新 `standard_terms` |
| `relation_suggestions_v1` | 包含 `source_name/target_name/relation_type`、`source_code/target_code/relation_type`、`source_item_id/target_item_id/relation_type`，或 `source_type/target_type` | 直接写入 `relation_suggestions`，保持 `pending` 待确认 |
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
- Smart Import 执行后会生成导入批次和变更日志，可一键回滚本次新增条目、附加注解和补空字段，并重建搜索索引。

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

## ZIP / 已解压文件夹与 manifest

标准数据包可以是 ZIP，也可以是已解压后的文件夹。两种入口共用同一套 ImportPackageReader 识别流程，并优先查找根目录 `import_manifest.json`。

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
      "primary": true,
      "role": "main_knowledge_items",
      "auto_stage": true,
      "description": "主知识条目文件，系统将自动暂存并导入该文件。"
    },
    {
      "path": "json/herb_items_import.json",
      "type": "knowledge_items_v1",
      "target": "knowledge_items",
      "primary": false,
      "required": false,
      "role": "auxiliary_export",
      "auto_stage": false,
      "description": "中药条目辅助导出文件，通常已包含在主知识文件中，默认不自动导入，避免重复。"
    }
  ],
  "import_order": ["knowledge_items", "search_terms"]
}
```

`primary` 表示本次导入的主数据文件。推荐每个标准数据包只设置一个 `primary: true` 的主知识文件；主文件默认 `auto_stage: true`，系统会自动暂存。`primary: false` 的文件默认 `auto_stage: false`，会显示为辅助文件，不自动暂存。

Smart Import manifest 可选声明 `import_intent`、`duplicate_policy`、`ai_assist`。旧 manifest 会按 `import_profile` 和主数据类型推断：`classics_curated_v1` 为 `classic_text`，`pdf_herb_notes_private_v1` 为 `annotation_enrichment`，普通 `knowledge_items_v1` 为 `primary_seed`。

`role` 与 `description` 用于解释文件用途，不影响识别；未知 `role` 不会报错。常见 `auxiliary_export` 表示辅助导出文件，例如 `herb_items_import.json` 可能是主知识文件中的中药子集或备用导出。为避免与 `knowledge_items_import.json` 重复入库，系统会保留展示但默认不导入。即使非主文件声明 `auto_stage: true`，当前版本也先显示为“可手动选择”，不会自动暂存。

如果 manifest 中多个可直接导入文件的 `type` 与 `target` 相同，但只有一个 `primary`，系统会提示：`检测到多个可导入文件指向同一目标表，系统仅自动暂存 primary 主文件，其余文件作为辅助文件保留，避免重复导入。`

如果 ZIP 或文件夹只有普通 `manifest.json`，系统会把它当作包说明，再自动查找 `json/knowledge_items_import.json`、`json/knowledge_items_import_curated.json`、`json/classic_passages_curated.json` 等已知文件。

v0.3 manifest 数据包建议命名为 `zhongyi_classics_curated_v0_3_manifest.zip`，或解压为同名文件夹后通过“导入数据包文件夹”入口选择。当前 v0.1 导入策略为：自动暂存 `primary: true` 的主文件，其他文件会显示在概览中，但不会混入同一导入批次。主文件可以是知识条目，也可以是 `search_terms_v1`、`standard_terms_v1` 或 `relation_suggestions_v1` 维护数据。确认知识主文件入库后仍会从 `name/code/category/tags` 追加包内搜索词。

推荐的已解压文件夹结构：

```text
shennong_bencao_ni_notes_private_import/
import_manifest.json
json/
knowledge_items_import.json
csv/
knowledge_items_import.csv
docs/
README_导入说明.md
```

文件夹导入会显示：

- `package_name`
- `import_profile`
- 是否找到 `import_manifest.json`
- 主数据文件
- 记录数
- 是否可直接导入

主软件不解析 PDF、Word 或图片原始资料。如果文件夹只包含 `3人纪-神农本草经.pdf` 这类原始文件，会返回：`PDF 原始资料不能直接导入，请先使用外部数据处理工具转换为标准 import_manifest 数据包。`

## 导入质量与回滚

确认入库后可在导入暂存页查看“质量报告”。报告会显示：

- 字段覆盖率，重点关注 `content`、`source_note`、`tags`。
- 批内疑似重复指纹数。
- 导入搜索词数量。
- 经典关键词抽检命中情况。
- 可执行修复建议。

如果 Smart Import 执行明显错误，可点击“一键回滚本次导入”。系统通过 `import_run_changes` 只撤销本批次创建或修改过的实体：新增知识条目、附加注解和补空字段。已跳过、待确认、失败项不处理；导入后被用户手动修改过的字段会提示风险并跳过。详见 `docs/IMPORT_ROLLBACK.md`。

## 常见错误

- `manifest 指向的文件不存在`：检查 ZIP 内路径是否与 `import_manifest.json` 一致。
- `找不到 manifest`：如果没有 `import_manifest.json`，系统会按内置经典包规则查找主 JSON；若 ZIP 结构被多包一层目录，也支持自动匹配结尾路径。
- `PDF 原始资料不能直接导入`：当前文件夹不是标准数据包，请先用外部工具整理为 JSON/CSV 与 `import_manifest.json`。
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
