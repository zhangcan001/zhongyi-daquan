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
- 可导入 CSV/JSON 并进入暂存区。
- 可显示错误行和校验问题。
- 可检测重复候选。
- 可接受关系建议并写入正式关系。
- 备份恢复后搜索仍可用。
- AI 设置页面存在，默认关闭。
