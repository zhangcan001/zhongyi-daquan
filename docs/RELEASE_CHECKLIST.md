# 发布检查清单

## v0.1.2-test

基于：`v0.1.2-test`

源码包版本：`0.1.2`

内容：

- 打包验证。
- 启动验证。
- 发布文档补齐。

限制：

- AI 默认关闭；配置后才真实调用用户提供的 OpenAI-compatible API。
- 不在线问诊。
- 不自动诊断。
- 不自动开方。

## 代码检查

- [x] `powershell -ExecutionPolicy Bypass -File .\scripts\check_all.ps1`
- [x] `npm --prefix frontend run check`
- [x] `npm --prefix frontend run build`
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] `cargo check --manifest-path src-tauri/Cargo.toml`

## 性能验证

- [x] `.\scripts\generate_regression_data.ps1`
- [x] 10,000 条知识、50,000 条关系、10,000 条导入暂存行生成成功
- [x] `黄芪` 搜索耗时 10ms
- [x] `足三里` 搜索耗时 9ms
- [x] `ST36` 搜索耗时 7ms
- [x] `补中益气汤` 搜索耗时 9ms
- [x] `胃经` 搜索耗时 9ms

## 开发启动

- [x] `npm run tauri:dev`
- [x] Vite 启动成功并监听 `http://127.0.0.1:1420`
- [x] Tauri 编译成功
- [x] `zhongyi-daquan.exe` 成功拉起
- [x] 首页可打开
- [x] AI 默认关闭
- [x] 搜索面板可见
- [x] 导入示例入口可见

## 生产打包

- [x] `npm run tauri:build`
- [x] `src-tauri/target/release/zhongyi-daquan.exe`
- [x] `src-tauri/target/release/bundle/nsis/中医大全_0.1.2_x64-setup.exe`
- [x] 安装包可启动
- [x] 首次启动可创建本地数据目录
- [x] 启动后首页正常
- [ ] `src-tauri/target/release/bundle/msi/`
- [x] 正式发布图标资源，`src-tauri/icons/` 已补齐 `.ico` 与多尺寸 PNG，`bundle.icon` 已配置。

## 发布边界

- [x] 产品运行期默认不联网
- [x] 不要求登录
- [x] 不上传本地数据
- [x] 不出现“AI医生”“自动诊断”“自动开方”等禁用文案
- [x] AI 默认关闭
- [x] AI 未配置时返回友好禁用提示，不调用网络
- [x] AI 配置后任务写入 `ai_tasks`，可查询状态并保留调用日志

## 备注

- 当前已验证 NSIS 安装包链路。
- MSI 产物仍依赖 WiX 工具链可用性，当前环境下全量打包会在 WiX 下载阶段额外等待，属于发布前环境项，不属于业务功能问题。
- 经典精校完整数据包不进入 Git 仓库，建议作为本地导入包或 GitHub Release 附件交付。
