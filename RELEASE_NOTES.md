# Release Notes

## v0.1-alpha

发布时间准备版，完成源码构建与 Windows 桌面安装包验证。

## v0.1-alpha-package

基于 `v0.1-alpha`，完成本地打包验证、启动验证和发布文档补齐。

### 已完成

- 全量检查通过：`check_all.ps1`、前端检查、前端构建、Rust fmt、cargo check、cargo test。
- 生产打包命令通过：`npm run tauri:build`。
- 已确认 Windows 可执行文件与 NSIS 安装包产物路径。
- 已补齐 README、打包说明和发布检查清单。

### 启动命令

```powershell
npm run tauri:dev
```

### 打包命令

```powershell
npm run tauri:build
```

### 打包产物

```text
src-tauri/target/release/zhongyi-daquan.exe
src-tauri/target/release/bundle/nsis/中医大全_0.1.0_x64-setup.exe
```

### 当前限制

- 不真实调用 AI。
- 不在线问诊。
- 不自动诊断。
- 不自动开方。
- 不创建 GitHub Release。
- 不上传安装包。

### 已完成

- 本地首页、知识库、表格录入、搜索面板、导入示例入口。
- AI 默认关闭，仅保留本地占位与设置入口。
- 本地数据库目录与 SQLite 文件首次启动可自动创建。
- 自动检查通过：前端检查、前端构建、Rust fmt、cargo check、cargo test。
- 已验证 Windows NSIS 安装包生成与启动。

### 当前限制

- 不接入真实 AI。
- 不联网。
- 不做人体穴位图。
- 不做健康档案。
- 不做医案系统。
- 不包含 v0.2 功能。

### 发布声明

本软件仅用于中医知识学习、资料整理与本地记录，不构成医疗诊断、治疗建议或处方依据。
