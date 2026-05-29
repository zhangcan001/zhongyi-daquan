# 发布检查清单

## 代码检查

- [ ] `npm --prefix frontend run check`
- [ ] `npm --prefix frontend run build`
- [ ] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml`

## 回归数据

- [ ] `.\scripts\generate_regression_data.ps1`
- [ ] 确认生成 10,000 条 `knowledge_items`。
- [ ] 确认生成 50,000 条 `knowledge_relations`。
- [ ] 确认生成 10,000 条 `data_import_rows`。
- [ ] 确认生成 1,000 条 `duplicate_candidates`。
- [ ] 确认生成 1,000 条 `relation_suggestions`。

## 性能

- [ ] 搜索 10,000 条知识 < 500ms。
- [ ] 知识列表翻页 < 300ms。
- [ ] 关系表 50,000 条详情首屏 < 500ms。

## 产品边界

- [ ] 产品运行期默认不联网。
- [ ] 不要求登录。
- [ ] 不上传本地数据。
- [ ] 不出现“AI医生”“自动诊断”“自动开方”等禁用文案。
- [ ] AI 默认关闭。
- [ ] AI 占位命令返回当前版本未启用 AI 调用。

## 打包

- [ ] `npm run tauri:build`
- [ ] 安装包可启动。
- [ ] 首次启动可创建本地数据目录。
- [ ] 升级安装不覆盖用户数据目录。
