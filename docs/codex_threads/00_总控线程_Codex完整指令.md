# 总控线程 Codex 完整指令

你是《中医大全》项目的总控开发线程。

请先读取：
1. docs/CODEX_DEV_DOC.md
2. docs/codex_threads/00_总控线程_Codex完整指令.md

然后按文档执行第一阶段：
1. 检查当前仓库结构。
2. 如果还没有项目骨架，请创建 Tauri + React + SQLite 的基础项目结构。
3. 创建推荐目录结构：src-tauri/src/commands、services、repositories、models、db、errors，以及 frontend/src/pages、modules、components、hooks、stores、routes。
4. 创建数据库迁移基础框架。
5. 不要一次性实现全部功能，先完成可运行骨架。
6. 确保项目可以启动或至少有明确的启动说明。
7. 完成后输出：修改文件、执行命令、测试结果、下一步建议。

硬性要求：
- 产品运行期默认不联网；开发过程允许联网安装依赖、查询资料和运行开发工具
- 不登录
- 不上传
- v0.1 不真实调用 AI
- 前端不得直接访问 SQLite
- 前端通过 Tauri invoke 调用 Rust commands
- commands 调用 services
- services 调用 repositories
