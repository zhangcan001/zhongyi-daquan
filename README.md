# 中医大全

《中医大全》是一个纯本地 Windows 桌面软件，当前发布收口版本为 `v0.1-alpha-package`，基于 `v0.1-alpha` 完成本地打包验证。当前阶段聚焦中医知识资产管理：录入、导入、映射、清洗、校验、去重、合并、关系建议、搜索、备份恢复和 AI 接口预留。

本项目不是问诊软件，不做自动诊断，不做自动开方。全局安全边界是：本软件仅用于中医知识学习、资料整理与本地记录，不构成医疗诊断、治疗建议或处方依据。

## 项目定位

- 技术栈：Tauri + React + SQLite。
- 运行形态：Windows 本地桌面应用。
- 数据位置：用户本机应用数据目录下的 `中医大全数据/`。
- 调用链路：React 前端通过 Tauri invoke 调用 Rust commands，后端按 `commands -> services -> repositories -> SQLite` 分层。
- AI 策略：默认不真实调用模型；本地 AI 知识库助手可基于 SQLite 资料生成来源支撑的方剂资料卡。

## 联网边界

- 开发期允许联网安装 Node、Rust、Tauri 依赖，也允许查询构建资料。
- 产品运行期默认不联网、不登录、不上传、不依赖服务器。
- v0.1 主流程不接入真实 AI，不发送本地知识库数据。

## AI 方剂资料卡

AI 知识库助手支持在本地资料范围内展示经方和方剂信息：

- 展示本地知识库中的原方组成。
- 展示古籍或讲义中的原文剂量。
- 展示同单位药材比例，不自动换算现代个人剂量。
- 展示原文煎服法。
- 展示本地资料中的方剂注解摘要。
- 方剂组成、剂量和煎服法必须带来源。

安全边界：

- 不生成自动处方。
- 不根据个人情况给“每味多少克、吃几天”的执行指令。
- 不替代医生处方。
- 不提供针灸操作指导。
- 本地资料没有组成时，会明确显示“本地资料中未检索到完整组成”，不会编造。

## v0.1-alpha

当前 `v0.1-alpha` 已完成源码构建、开发启动和 Windows 安装包验证。

已完成内容：

- 本地首页、知识库、表格录入、搜索面板和导入示例入口可用。
- AI 默认关闭，仅保留本地占位与配置入口。
- 启动后会创建本地数据库目录与 SQLite 数据库文件。
- 可从源码构建出 Windows 桌面程序并生成 NSIS 安装包。

当前限制：

- 不接入真实 AI。
- 不联网。
- 不做人体穴位图。
- 不做健康档案。
- 不做医案系统。
- 不包含 v0.2 功能。

发布声明：

本软件仅用于中医知识学习、资料整理与本地记录，不构成医疗诊断、治疗建议或处方依据。

## v0.1.2-ux-polish

`v0.1.2-test` 是 `v0.1.2` 的测试发布名，应用版本为 `0.1.2`。目标是把已导入数据变成更好查、更好看、更好用的学习资料库，重点优化搜索结果、知识详情、资料注解、来源页码、最近查看、收藏和个人备注。

使用方式：

- 在首页学习工作台顶部搜索框输入中药、方剂、穴位、经络、原典或注解关键词。
- 搜索结果会按中药、方剂、穴位、经络、原典章节、原典条文、注解资料和其他分组。
- 点击结果后查看详情，主信息、正文、类型专属字段和资料注解会分区展示。
- 注解区域会显示来源标题、PDF 页码或 `source_note`，支持折叠查看全文。
- 可收藏条目、复制当前条目，或添加本地个人备注。
- 首页显示最近查看、我的收藏和最近导入批次。
- 导入历史、报告和回滚仍在智能导入中心，回滚前需要二次确认。

边界：

- 不解析 PDF。
- 不真实调用 AI。
- 不做在线问诊。
- 不做自动诊断。
- 不做自动开方。
- 不做针灸操作指导。

## v0.1-alpha-package

`v0.1-alpha-package` 基于 `v0.1-alpha`，完成打包验证、启动验证和发布文档补齐。

验收结果：

- 全量检查通过。
- `npm run tauri:build` 通过。
- 已生成 Windows 可执行文件和 NSIS 安装包。

打包产物路径：

```text
src-tauri/target/release/zhongyi-daquan.exe
src-tauri/target/release/bundle/nsis/中医大全_0.1.2_x64-setup.exe
release-assets/zhongyi-daquan_0.1.2-test_x64-setup.exe
```

版本口径：

- 发布标签：`v0.1-alpha-package`。
- 基础验收标签：`v0.1-alpha`。
- `v0.1.2-test` 测试发布使用应用版本 `0.1.2`，对应 `package.json`、`frontend/package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json`。

## 启动

首次运行前需要安装 Node.js、npm、Rust 和 Tauri 所需系统依赖。

```powershell
npm install --prefix frontend
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri:dev
```

前端单独调试：

```powershell
npm --prefix frontend run dev
```

## 测试

前端检查：

```powershell
npm --prefix frontend run check
npm --prefix frontend run build
```

后端检查：

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

一键检查：

```powershell
.\scripts\check_all.ps1
```

生产打包：

```powershell
npm run tauri:build
```

生成线程 H 回归数据并执行性能阈值检查：

```powershell
.\scripts\generate_regression_data.ps1
```

默认生成到 `local-data/database/thread_h_regression.db`，包含 10,000 条 `knowledge_items`、50,000 条 `knowledge_relations`、10,000 条 `data_import_rows`、1,000 条 `duplicate_candidates` 和 1,000 条 `relation_suggestions`。

`v0.1-alpha-package` 收口实测中，`黄芪`、`足三里`、`ST36`、`补中益气汤`、`胃经` 五个关键词搜索均小于 500ms。

## 数据目录

应用启动后会在 Tauri 应用数据目录下创建：

```text
中医大全数据/
├─ database/zhongyi.db
├─ images/
├─ imports/
├─ exports/
├─ backups/
├─ logs/
├─ config/
└─ temp/
```

## 文档

- [数据库结构](docs/DATABASE_SCHEMA.md)
- [开发指南](docs/DEV_GUIDE.md)
- [测试计划](docs/TEST_PLAN.md)
- [AI 知识库助手 v0.2 方剂回答](docs/AI_ASSISTANT_V020.md)
- [v0.1.2 搜索与阅读体验优化](docs/UX_POLISH_V012.md)
- [打包说明](docs/PACKAGING.md)
- [发布检查清单](docs/RELEASE_CHECKLIST.md)
- [经典数据导入说明](docs/CLASSICS_DATA_IMPORT.md)
- [Codex 开发总文档](docs/CODEX_DEV_DOC.md)
