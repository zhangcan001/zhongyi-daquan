# 经典数据导入说明

## 目标

`v0.1-alpha-package` 不把四部经典精校完整数据包直接放入 Git 仓库。仓库内只保留导入规范、字段说明和小体积 sample，完整数据包建议作为本地导入包或 GitHub Release 附件交付。

## 建议纳入的四部经典

- 《黄帝内经》：建议拆分为篇、章节、原文、校注、关键词。
- 《伤寒论》：建议拆分为条文编号、原文、方剂、证候、治法。
- 《金匮要略》：建议拆分为篇名、条文编号、病证、方剂、原文。
- 《神农本草经》：建议拆分为药名、品级、性味、主治、原文。

## 推荐字段

| 字段 | 说明 |
| --- | --- |
| `classic_name` | 经典名称 |
| `volume` | 卷、篇或部 |
| `section` | 章节、篇名或条文分组 |
| `entry_no` | 条文编号，可为空 |
| `title` | 条目标题 |
| `original_text` | 原文 |
| `annotation` | 校注或整理说明 |
| `keywords` | 关键词数组 |
| `related_items` | 关联中药、方剂、经络、穴位、证型、病症 |
| `source_note` | 来源与版权说明 |

## Import Engine V2 导入流程

推荐优先导入标准 JSON、ZIP 数据包或已解压的数据包文件夹：

1. `knowledge_items_import_curated.json`
2. `knowledge_items_import_full_clean.json`
3. `classic_passages_curated.json`
4. `classic_passages_full_clean.json`
5. `zhongyi_classics_curated_v0_3_manifest.zip`
6. 已解压的 `zhongyi_classics_curated_v0_3_manifest/` 文件夹
7. 如果暂时没有 v0.3 manifest 包，可导入 v0.2 包内的 `json/knowledge_items_import_curated.json`

导入步骤：

1. 在批量导入页面选择 JSON、CSV、ZIP，或点击“导入数据包文件夹”选择已解压文件夹。
2. 点击“识别文件”，查看 `detected_type`、置信度、记录数和原因。
3. 如果识别为 `knowledge_items_v1` 或 `classic_passages_v1`，页面会显示“可直接导入”，不需要人工映射。
4. 如果识别为 `generic_csv`，系统显示字段映射候选，高置信字段自动映射，中置信字段建议确认，低置信字段不自动映射。
5. 导入后先进入暂存区，不直接写入正式知识库。
6. 执行清洗校验，处理空字段、重复条文、异常编号和关联名称。
7. 人工验收后确认入库，系统会重建搜索索引，并生成导入质量报告。
8. 如发现导入批次错误，可在暂存页回滚该批次；系统会删除本批次写入条目并重建搜索索引。

标准经典数据包中：

- `content` 原文完整保留。
- `source_note` 出处完整保留，并尽量附加 `source_url/classic_id/page_title/section_title`。
- `tags` 支持数组和分隔字符串。
- `detail` 对象会按知识类型写入详情字段，无法识别的上下文保留到 `notes`。
- 空字段不会导致整批失败。
- 确认入库后会追加由 `name/code/category/tags` 派生的包内搜索词。

## v0.3 manifest 数据包

`zhongyi_classics_curated_v0_3_manifest.zip` 基于 v0.2 数据包增加根目录 `import_manifest.json`。也可以把 ZIP 解压为文件夹后直接选择文件夹导入。manifest 声明包名、schema、文件列表、主数据文件和导入顺序。当前 v0.1 会自动暂存 `primary: true` 的 `knowledge_items_import_curated.json` 或 `knowledge_items_import.json`，其他文件显示在数据包概览中但不自动混入同一批次。确认入库后可查看质量报告，重点检查 `content`、`source_note`、`tags` 覆盖率，以及关键词搜索抽检。

`primary: true` 表示主数据文件。标准数据包推荐只设置一个主知识文件，并设置 `role: "main_knowledge_items"`、`auto_stage: true`。非主文件默认作为辅助文件展示，不自动暂存，避免与主文件重复导入。

`role: "auxiliary_export"` 适合 `json/herb_items_import.json` 这类辅助导出文件。它通常是 `knowledge_items_import.json` 的中药子集或备用导出，系统会显示其 `description` 与跳过原因：非 primary 主数据文件默认不自动暂存，避免重复导入。如果多个 `knowledge_items_v1` 文件同时指向 `knowledge_items`，页面会提示重复风险。

标准文件夹结构示例：

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

如果文件夹没有 `import_manifest.json`，系统会尝试扫描常见标准路径，例如 `json/knowledge_items_import.json`、`json/knowledge_items_import_curated.json`、`json/knowledge_items_import_full_clean.json`、`csv/knowledge_items_import.csv`。推荐优先补齐 `import_manifest.json`，这样可以明确 package_name、import_profile、导入顺序和主数据文件。

主软件不解析 PDF、Word 或图片原始资料。包含《神农本草经》PDF 的资料文件夹需要先通过外部数据处理工具转换为标准 JSON/CSV 和 `import_manifest.json`，再导入本软件。

仓库内提供示例：

- `data-seed/classics/import_manifest.example.json`
- `data-seed/private-samples/shennong_bencao_manifest.example.json`

导入后搜索验收关键词：

- `桂枝汤`
- `太阳病`
- `上古天真论`
- `神农本草经`
- `黄帝内经`
- `金匮要略`

详见 `docs/IMPORT_ENGINE_V2.md` 和 `docs/IMPORT_QUALITY_V1.md`。

## 数据存放策略

- 仓库：只放 `data-seed/classics/README.md` 和小体积 `classics.sample.json`。
- 本地导入包：可放完整精校数据，路径由发布说明或内部交付文档记录。
- GitHub Release：如需对外交付完整数据，建议以附件形式上传，不随源码提交。

## 发布限制

- 不使用未经授权的现代整理本全文。
- 不把超大数据文件提交到 Git。
- 不在 v0.1 中提供在线同步或云端数据下载。
- 不把经典条文解释包装成诊断、处方或治疗建议。
