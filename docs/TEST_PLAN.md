# 测试计划

## 目标

线程 H 提供基础自动化回归和大数据性能验证能力，覆盖数据库初始化、搜索、导入暂存、备份恢复、AI 占位和性能阈值。

## 自动化测试

后端测试：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

当前覆盖：

- 数据库初始化：PRAGMA、核心表、索引、FTS5 可用。
- 搜索：`search_terms` 与 `knowledge_fts` 命中。
- 导入流程：批次、暂存行、校验问题写入。
- 备份恢复：复制数据库后仍可搜索。
- AI 占位：命令返回禁用状态，不真实调用 AI。
- Import Engine V2：
  - `knowledge_items_v1` 检测。
  - `classic_passages_v1` 检测。
  - `generic_csv` 字段评分映射。
  - `tags` 数组与字符串标准化。
  - `detail` 对象写入对应详情字段。
  - `content` 原文和 `source_note` 出处保留。
  - 确认入库后重建搜索索引，并可搜索 `上古天真论`、`太阳病` 等导入内容。
  - 真实四部经典包验收：识别 `knowledge_items_import_curated.json`、`classic_passages_curated.json`、`search_terms_curated.json`，并验证 `zhongyi_classics_curated_v0_3_manifest.zip` manifest 驱动导入。
- Import Quality V1：
  - 确认入库后记录 `confirmed_item_ids_json`。
  - 生成导入质量报告，覆盖字段覆盖率、重复指纹、包内搜索词数量。
  - 标准知识 JSON / manifest 主知识文件确认后追加 `imported_package` 搜索词。
  - 批次回滚后删除本批次知识条目，并重建搜索索引，原关键词不再命中。
- 已解压数据包文件夹导入：
  - 根目录存在 `import_manifest.json` 时按 manifest 识别。
  - `import_manifest.json` 带 UTF-8 BOM 时仍可解析。
  - manifest 可指向 `json/knowledge_items_import.json`。
  - manifest 指向文件不存在时返回明确错误。
  - 没有 manifest 但存在 `json/knowledge_items_import.json` 时可按 `knowledge_items_v1` 识别。
  - manifest 非主文件支持读取 `role`、`auto_stage`、`description`。
  - `primary: false` 的辅助文件不会自动暂存，并返回 `skip_reason`。
  - 多个可导入文件指向同一 `target` 时生成重复风险 warning。
  - 文件夹里只有 PDF 时拒绝直接导入，并提示先转换为标准 `import_manifest` 数据包。
  - 文件夹导入确认后会重建搜索索引，可搜索新导入条目。
- Smart Import Center V1：
  - 标准 manifest ZIP / 文件夹不进入字段映射，UI 只显示计划摘要和报告摘要。
  - `import_intent` 可由 manifest 或旧 `import_profile` 推断。
  - 可生成 `ImportPlan`，统计 create、skip、merge、attach annotation、needs review。
  - `annotation_enrichment` 对同名中药自动写入 `knowledge_annotations`。
  - `primary_seed` 对冲突重复项不覆盖已有内容。
  - `classic_text` 对同 source_note 条文自动跳过。
  - `knowledge_annotations` 内容进入搜索索引。
  - AI 未启用时返回本地规则处理提示，不阻塞导入计划。
  - `generic_csv` / `unknown` 仍进入字段映射。
  - `create_new`、`attach_annotation` 和 `merge_empty_fields` 会记录 `import_run_changes`。
  - `merge_empty_fields` 会记录 `before_json` / `after_json`。
  - `rollback_import_run` 可删除本次新增知识条目。
  - `rollback_import_run` 可删除本次新增注解。
  - `rollback_import_run` 可恢复本次补空字段。
  - 回滚后搜索索引重建。
  - `list_import_runs` 返回最近导入历史。
  - `get_import_run_report` 返回导入报告。

前端检查：

```powershell
npm --prefix frontend run check
npm --prefix frontend run build
```

## 测试数据生成器

```powershell
.\scripts\generate_regression_data.ps1
```

默认生成：

- `knowledge_items`：10,000 条。
- `knowledge_relations`：50,000 条。
- `data_import_rows`：10,000 条。
- `duplicate_candidates`：1,000 条。
- `relation_suggestions`：1,000 条。

生成器会同步写入 `knowledge_fts`、`search_terms`、`knowledge_list_view_cache` 和 `relation_count_cache`，便于立即执行搜索与分页性能检查。

当前性能验证关键词：

- `黄芪`
- `足三里`
- `ST36`
- `补中益气汤`
- `胃经`
- 经典数据包导入后补充验证：`桂枝汤`、`太阳病`、`上古天真论`、`神农本草经`
- v0.3 manifest 数据包导入后补充验证：`黄帝内经`、`金匮要略`
- 已解压文件夹数据包导入后补充验证：`文件夹桂枝汤`

## 性能阈值

生成器默认附带性能检查：

- 搜索 10,000 条知识：小于 500ms。
- 知识列表翻页：小于 300ms。
- 关系表 50,000 条详情首屏：小于 500ms。
- 上述五个关键词的单次搜索耗时应记录在回归输出中，作为发布收口报告依据。

这些阈值是本地开发机上的回归门槛。若 CI 或低性能机器偶发超时，应记录环境、数据库大小、查询耗时和是否冷启动。

## v0.1-alpha-package 实测结果

本地执行：

```powershell
.\scripts\generate_regression_data.ps1
```

结果：

| 项目 | 结果 |
| --- | --- |
| `knowledge_items` | 10,000 |
| `knowledge_relations` | 50,000 |
| `data_import_rows` | 10,000 |
| `duplicate_candidates` | 1,000 |
| `relation_suggestions` | 1,000 |
| 数据生成耗时 | 3,104ms |
| `performance_search_ms` | 5ms |
| `performance_list_page_ms` | 0ms |
| `performance_relation_first_page_ms` | 0ms |
| `黄芪` | 10ms |
| `足三里` | 9ms |
| `ST36` | 7ms |
| `补中益气汤` | 9ms |
| `胃经` | 9ms |

## 手工回归重点

- 可新增黄芪、足阳明胃经、足三里。
- 首页可进入“智能导入中心”，入口文案为“导入标准数据包，系统自动识别、去重、合并并生成导入报告。”
- 标准 ZIP / 已解压文件夹只显示四步导入流程，不默认展示字段映射、primary、auxiliary、target、manifest 路径或重复明细。
- 单个 JSON / CSV 仅作为高级入口；只有 `generic_csv` / `generic_json` / `unknown` 才需要字段映射确认。
- 导入报告显示成功、跳过、附加注解、失败、导入批次号、查看报告和一键回滚。
- 回滚后再次搜索本次新增条目，应不再命中；回滚本次新增注解后，注解来源关键词应不再通过该注解命中。
- PDF 原始资料文件夹不能直接导入，错误文案应明确说明先用外部工具转换为标准数据包。
- 可显示错误行和校验问题。
- 可检测重复候选。
- 可接受关系建议并写入正式关系。
- 备份恢复后搜索仍可用。
- AI 设置页面存在，默认关闭。
