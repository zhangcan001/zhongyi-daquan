# 打包说明

## 前置检查

```powershell
.\scripts\check_all.ps1
```

确认以下事项：

- 前端 `tsc` 通过。
- 前端 Vite build 通过。
- Rust fmt、test、check 通过。
- AI 默认关闭。
- 产品运行期无联网依赖。
- `v0.1-alpha` 已验证可从源码构建为 Windows 桌面程序。

## 开发构建

```powershell
npm run tauri:dev
```

## 生产打包

## v0.1-alpha-package

`v0.1-alpha-package` 基于 `v0.1-alpha`，用于确认本地 Windows 打包链路、启动链路和发布文档完整性。

已验证的 Windows 安装包构建命令：

```powershell
npm run tauri:build
```

等价底层命令：

```powershell
.\frontend\node_modules\.bin\tauri.cmd build --config src-tauri/tauri.conf.json --bundles nsis --ci
```

构建产物由 Tauri 写入 `src-tauri/target/release/`，其中：

- 可执行文件：`src-tauri/target/release/zhongyi-daquan.exe`
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/中医大全_0.1.0_x64-setup.exe`
- MSI 安装包：`src-tauri/target/release/bundle/msi/`

说明：

- `npm run tauri:build` 已在当前环境验证通过。
- 默认脚本生成 NSIS 安装包，避免 `targets = all` 在本机缺少 WiX/MSI 工具链时卡住。
- 如果需要完整 MSI 产物，先确保 WiX 资源已可用，再执行全量打包。
- 当前 `src-tauri/tauri.conf.json` 的 `bundle.icon` 仍为空。TODO：正式发布前补齐 `.ico` 与对应尺寸图标资源；本次不临时生成占位图标。

## 版本口径

- 当前发布收口：`v0.1-alpha-package`。
- 基础验收版本：`v0.1-alpha`。
- Tauri/npm/Cargo 源码包版本：`0.1.0`。

## 数据与升级

用户数据不应放入安装目录。运行期数据保存在 Tauri 应用数据目录下的 `中医大全数据/`，其中数据库位于 `database/zhongyi.db`。

升级包不得覆盖用户数据目录。涉及数据库结构变化时，应通过迁移脚本增量处理。

## 离线运行要求

发布包默认离线可用：

- 不要求登录。
- 不上传数据库。
- 不依赖远程服务。
- AI 设置可存在，但默认关闭且 v0.1 不真实调用模型。
