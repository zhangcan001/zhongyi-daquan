# 开发指南

## 环境

需要 Node.js、npm、Rust、Cargo 和 Tauri 构建依赖。开发期允许联网安装依赖；产品运行期默认不联网。

```powershell
npm install --prefix frontend
cargo check --manifest-path src-tauri/Cargo.toml
```

## 运行

```powershell
npm run tauri:dev
```

前端单独运行：

```powershell
npm --prefix frontend run dev
```

## 分层约定

- 前端不直接访问 SQLite。
- 前端通过 Tauri invoke 调用 `src-tauri/src/commands`。
- commands 只做参数接收和返回。
- services 编排业务流程。
- repositories 负责 SQLite 和文件系统访问。
- 大数据导入编排、索引重建、备份恢复、数据库维护应记录到 `background_jobs`；耗时入口优先使用异步启动命令并由任务中心轮询。

## 线程 H 边界

线程 H 只维护测试、文档、回归工具：

- 可以新增测试数据生成器、测试用例、性能检查和文档。
- 可以新增 `scripts/check_*.ps1`。
- 不修复业务功能问题，除非问题位于测试脚本或文档。
- 不重构数据库迁移。
- 不接入真实 AI。

## 常用检查

```powershell
.\scripts\check_frontend.ps1
.\scripts\check_backend.ps1
.\scripts\check_all.ps1
```

生成回归数据库：

```powershell
.\scripts\generate_regression_data.ps1
```

也可以指定输出路径：

```powershell
.\scripts\generate_regression_data.ps1 local-data/database/custom-thread-h.db
```

## AI 助手规则

AI 助手可在用户配置后调用外部模型和联网检索；联网失败时必须自动降级到本地知识库回答，并把外部来源与本地来源分开标注。AI 输出默认只作为草稿、建议或回答展示，不得绕过导入/确认流程直接写入正式知识库。
