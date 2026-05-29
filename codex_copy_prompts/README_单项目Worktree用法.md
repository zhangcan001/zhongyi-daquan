# 单项目 Worktree 用法

在 Codex 桌面版中只打开一个项目：

```text
C:\Users\ADMIN\Documents\zhongyi-daquan
```

不要再打开 `zy-thread-*` 文件夹。

## 新建线程

新建线程时选择：

```text
模式：Worktree
Starting branch：主分支，例如 master/main/当前主分支
```

不要选择旧的 `codex/thread-*` 分支。

## 第一批线程复制指令

```text
线程 A：复制 codex_copy_prompts/01_线程A_数据库.txt
线程 D：复制 codex_copy_prompts/04_线程D_搜索性能.txt
线程 G：复制 codex_copy_prompts/07_线程G_AI预留.txt
```

## 第二批线程复制指令

```text
线程 B：复制 codex_copy_prompts/02_线程B_知识CRUD表格录入.txt
线程 C：复制 codex_copy_prompts/03_线程C_导入清洗校验.txt
线程 E：复制 codex_copy_prompts/05_线程E_去重关系.txt
线程 F：复制 codex_copy_prompts/06_线程F_后台备份.txt
线程 H：复制 codex_copy_prompts/08_线程H_测试文档.txt
```

每个线程仍然先读取：

```text
docs/CODEX_DEV_DOC.md
docs/codex_threads/对应线程文件
```
