# 中医大全

《中医大全》是一个纯本地 Windows 桌面软件，目标版本为 `v0.1-entry-core-performance-ai-ready`。当前阶段聚焦中医知识资产管理：录入、导入、映射、清洗、校验、去重、合并、关系建议、搜索、备份恢复和 AI 接口预留。

本项目不是问诊软件，不做自动诊断，不做自动开方。全局安全边界是：本软件仅用于中医知识学习、资料整理与本地记录，不构成医疗诊断、治疗建议或处方依据。

## 项目定位

- 技术栈：Tauri + React + SQLite。
- 运行形态：Windows 本地桌面应用。
- 数据位置：用户本机应用数据目录下的 `中医大全数据/`。
- 调用链路：React 前端通过 Tauri invoke 调用 Rust commands，后端按 `commands -> services -> repositories -> SQLite` 分层。
- AI 策略：v0.1 只做接口和数据表预留，默认关闭，不真实调用模型。

## 联网边界

- 开发期允许联网安装 Node、Rust、Tauri 依赖，也允许查询构建资料。
- 产品运行期默认不联网、不登录、不上传、不依赖服务器。
- v0.1 主流程不接入真实 AI，不发送本地知识库数据。

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

## v0.1-alpha-package

`v0.1-alpha-package` 基于 `v0.1-alpha`，完成打包验证、启动验证和发布文档补齐。

验收结果：

- 全量检查通过。
- `npm run tauri:build` 通过。
- 已生成 Windows 可执行文件和 NSIS 安装包。

打包产物路径：

```text
src-tauri/target/release/zhongyi-daquan.exe
src-tauri/target/release/bundle/nsis/中医大全_0.1.0_x64-setup.exe
```

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
- [打包说明](docs/PACKAGING.md)
- [发布检查清单](docs/RELEASE_CHECKLIST.md)
- [Codex 开发总文档](docs/CODEX_DEV_DOC.md)
