# Codex 桌面版单项目 Worktree 使用说明

本项目现在采用 Codex 桌面版“单项目 + Worktree 线程”管理方式。

## 只添加一个项目

Codex 桌面版左侧只添加这个项目：

```text
C:\Users\ADMIN\Documents\zhongyi-daquan
```

不要再把下面这些目录添加为 Codex 项目：

```text
zy-thread-db
zy-thread-search
zy-thread-ai
zy-thread-entry
zy-thread-import
zy-thread-relation
zy-thread-backup
zy-thread-qa
其他 zy-thread-* 目录
```

这些旧外部 worktree 目录已废弃。后续由 Codex 桌面版在同一个 `zhongyi-daquan` 项目中创建 Worktree 线程。

## 新建线程方式

在 Codex 桌面版中新建线程时选择：

```text
模式：Worktree
Starting branch：当前主分支，例如 main/master/当前分支
```

不要再选择旧的 `codex/thread-*` 分支，也不要手动执行旧的 worktree 创建脚本。

## 第一批线程

建议先创建并运行：

```text
线程A-数据库与迁移
线程D-搜索与性能
线程G-AI接口预留
```

每个线程仍然先读取：

```text
docs/CODEX_DEV_DOC.md
docs/codex_threads/对应线程文件
```

## 第二批线程

第一批完成并由总控线程合并后，再创建：

```text
线程B-知识CRUD与表格录入
线程C-导入字段映射清洗校验
线程E-去重合并关系建议
线程F-后台任务备份恢复维护工具
线程H-测试文档回归
```

每个线程仍然先读取：

```text
docs/CODEX_DEV_DOC.md
docs/codex_threads/对应线程文件
```

## 旧脚本位置

旧的手动分支和外部 worktree 脚本已经移动到：

```text
scripts/legacy_worktree_scripts/
```

这些脚本仅保留作历史参考。现在由 Codex 桌面版创建 Worktree 线程，不再手动创建 `zy-thread-*` 外部工作区。
