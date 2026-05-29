# 中医大全

纯本地 Windows 桌面软件骨架，目标版本为 `v0.1-entry-core-performance-ai-ready`。

## 当前状态

- Tauri + React + SQLite 基础项目结构已建立。
- 前端通过 `@tauri-apps/api` 的 `invoke` 调用 Rust commands。
- Rust 后端按 `commands -> services -> repositories -> SQLite` 分层。
- 数据库启动时会创建本地数据目录、执行 PRAGMA，并运行迁移。
- v0.1 AI 默认关闭，目前只提供占位返回，不真实调用 AI。

## 启动

首次运行前需要本机已有 Node.js、npm、Rust 和 Tauri 构建依赖。开发过程允许联网安装依赖；产品运行期默认不联网，v0.1 不真实调用 AI。

```powershell
npm install --prefix frontend
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri:dev
```

前端单独调试：

```powershell
npm install --prefix frontend
npm run dev
```

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

## 开发边界

- 产品运行期默认不联网、不登录、不上传。
- 开发过程允许联网安装依赖、查询构建资料和运行开发工具。
- 前端不得直接访问 SQLite。
- 复杂写入后续必须通过 service 使用事务。
- 大数据任务后续必须进入 `background_jobs`。
- 搜索后续使用 FTS5 + `search_terms`。
