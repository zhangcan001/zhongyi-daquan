# 《中医大全》Codex 可直接复制使用项目包

这个文件夹已经按项目结构整理好了。

你解压后，把整个 `zhongyi-daquan-ready` 文件夹复制到：

```text
C:\Users\ADMIN\Documents\
```

然后建议改名为：

```text
zhongyi-daquan
```

最终路径应该是：

```text
C:\Users\ADMIN\Documents\zhongyi-daquan
```

## 你应该先做什么

打开 PowerShell，执行：

```powershell
cd "$env:USERPROFILE\Documents\zhongyi-daquan"
.\scripts\01_初始化Git并提交文档.ps1
```

然后打开 VS Code：

```powershell
code "$env:USERPROFILE\Documents\zhongyi-daquan"
```

先开一个 Codex 总控线程，复制：

```text
codex_copy_prompts\00_先发给总控线程.txt
```

等总控线程完成项目骨架并提交后，再执行：

```powershell
.\scripts\02_创建多线程worktree.ps1
```

## 目录说明

```text
docs/CODEX_DEV_DOC.md                 完整开发文档
docs/codex_threads/                   A-H 多线程指令
codex_copy_prompts/                   可以直接复制给 Codex 的简短指令
scripts/                              PowerShell 操作脚本
src-tauri/src/                        Rust/Tauri 后端目录骨架
frontend/src/                         React 前端目录骨架
data-seed/                            种子数据目录
local-data/                           本地数据目录示例
docs/architecture/                    架构图
```

## 新手最稳顺序

```text
1. 总控线程：创建项目骨架
2. 线程 A：数据库
3. 线程 D：搜索性能
4. 线程 G：AI 预留
5. 合并 A/D/G
6. 线程 B：知识 CRUD
7. 线程 C：导入清洗
8. 线程 E：去重关系
9. 线程 F：后台备份
10. 线程 H：测试文档
11. 总控线程最终集成
```
