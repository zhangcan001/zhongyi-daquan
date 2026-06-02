# v0.1.2 视觉验收与打包前检查

## 验收时间

2026-06-02 23:48:33 +08:00

## 验收范围

- 首页学习工作台
- 全局搜索入口
- 搜索筛选入口
- 快捷入口
- 数据概览
- 最近查看
- 我的收藏
- 导入历史摘要
- 智能导入中心入口
- 知识详情与注解展示能力
- 收藏、备注、最近查看后端能力
- Tauri dev 启动
- NSIS 打包

## 启动结果

执行 `npm run tauri:dev` 后，Tauri 桌面窗口成功启动，窗口标题为“中医大全”。首页可访问性树能读取到“中医大全学习工作台”、全局搜索、筛选按钮、快捷入口、数据概览、最近查看、我的收藏和导入历史摘要。

## 搜索抽检关键词

真实运行数据库路径：

```text
C:\Users\ADMIN\AppData\Roaming\com.zhongyi.daquan\中医大全数据\database\zhongyi.db
```

抽检关键词：

- 人参
- 甘草
- 黄耆
- 黄芪
- 倪注
- 神农本草经
- 桂枝汤
- 太阳病
- 上古天真论
- 足三里
- 足阳明胃经
- 理中丸
- 金匮要略

结果：以上 13 个关键词均能在真实库中命中样本记录。命中记录包含类型、分类、摘要、`source_note`、导入批次等信息；来源字段可见 PDF 名称、章节和 PDF 页码。

说明：当前真实库 `knowledge_annotations` 数量为 0，本次无法用真实全量库证明“详情页显示 knowledge_annotations 注解”。该能力已由自动化测试覆盖：`detail_recent_favorite_and_notes_work`、`enhanced_search_hits_renji_reading_keywords`、`renji_sample_directory_imports_annotations_and_rolls_back`。

## 视觉问题清单

- 首页学习工作台：未发现阻塞视觉问题。搜索框、筛选、快捷入口、数据概览和导入历史层级清楚。
- 搜索入口：搜索框宽度、按钮和筛选标签在当前桌面窗口尺寸下未见遮挡。
- 快捷入口：按钮文字可读，无明显溢出。
- 数据概览：统计卡片可读；当前真实库显示 `注解资料 = 0`，与数据库状态一致。
- 最近查看/收藏：空状态文案可见，无布局异常。
- 导入历史摘要：最近 5 个导入批次可见，首页未展示 manifest 或字段映射等技术细节。

## 已修复问题

本轮未发现需要修改代码的阻塞 UI 问题，因此未做 CSS 或业务逻辑修复。

## 截图结果

已保存首页工作台截图：

```text
docs/screenshots/v0.1.2/home-workbench.png
```

未保存搜索页和详情页截图。原因是 Windows Computer Use 对当前 Tauri WebView 的截图状态捕获失败，返回 `SetIsBorderRequired failed: 不支持此接口 (0x80004002)`；元素输入也依赖该截图状态，无法稳定操作搜索结果页。已按替代方案使用真实 Tauri 启动、可访问性树、真实库抽检、前端构建和自动化测试完成验收。

## Playwright 状态

Playwright CLI 浏览器安装仍未完成。此前 `chrome-for-testing` 安装超时，本轮按要求不继续卡在 Playwright，也未大幅修改本机环境。

## 打包结果

执行：

```powershell
npm run tauri:build
```

结果：成功。

安装包路径：

```text
src-tauri\target\release\bundle\nsis\中医大全_0.1.0_x64-setup.exe
```

打包产物未提交到仓库。

## 全量检查结果

以下命令均通过：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\check_all.ps1
npm --prefix frontend run check
npm --prefix frontend run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Rust 测试结果：40 个库内测试通过，5 个回归测试通过。

## 发布建议

建议进入 v0.1.2 测试版发布。

发布理由：

- Tauri dev 可启动。
- 首页学习工作台可见且结构清楚。
- 真实库 13 个关键词均可命中样本记录。
- 收藏、备注、最近查看和注解能力已有自动化测试覆盖。
- NSIS 安装包可生成。
- 全量检查通过。

发布注意：

- Playwright 截图级验证未完成。
- 当前真实全量库 `knowledge_annotations = 0`，注解区视觉需要后续用 annotation_enrichment 数据包或样例包再次人工截图确认。
