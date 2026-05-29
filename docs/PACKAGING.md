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

## 开发构建

```powershell
npm run tauri:dev
```

## 生产打包

```powershell
npm run tauri:build
```

构建产物由 Tauri 写入 `src-tauri/target/release/bundle/` 下的对应 Windows 安装包目录。

## 数据与升级

用户数据不应放入安装目录。运行期数据保存在 Tauri 应用数据目录下的 `中医大全数据/`，其中数据库位于 `database/zhongyi.db`。

升级包不得覆盖用户数据目录。涉及数据库结构变化时，应通过迁移脚本增量处理。

## 离线运行要求

发布包默认离线可用：

- 不要求登录。
- 不上传数据库。
- 不依赖远程服务。
- AI 设置可存在，但默认关闭且 v0.1 不真实调用模型。
