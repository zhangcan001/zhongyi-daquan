# 发布检查清单

## 代码检查

- [x] `powershell -ExecutionPolicy Bypass -File .\scripts\check_all.ps1`
- [x] `npm --prefix frontend run check`
- [x] `npm --prefix frontend run build`
- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] `cargo check --manifest-path src-tauri/Cargo.toml`

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

- [x] `npm run tauri:build -- --bundles nsis`
- [x] `src-tauri/target/release/zhongyi-daquan.exe`
- [x] `src-tauri/target/release/bundle/nsis/中医大全_0.1.0_x64-setup.exe`
- [x] 安装包可启动
- [x] 首次启动可创建本地数据目录
- [x] 启动后首页正常
- [ ] `npm run tauri:build`
- [ ] `src-tauri/target/release/bundle/msi/`

## 发布边界

- [x] 产品运行期默认不联网
- [x] 不要求登录
- [x] 不上传本地数据
- [x] 不出现“AI医生”“自动诊断”“自动开方”等禁用文案
- [x] AI 默认关闭
- [x] AI 占位命令返回当前版本未启用 AI 调用

## 备注

- 当前已验证 NSIS 安装包链路。
- MSI 产物仍依赖 WiX 工具链可用性，当前环境下全量打包会在 WiX 下载阶段额外等待，属于发布前环境项，不属于业务功能问题。
