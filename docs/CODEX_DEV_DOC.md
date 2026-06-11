# 《中医大全》v4-entry-refine-performance-ai-ready  
# Codex 专用开发文档与多线程开发指令

> 本文档用于直接交给 Codex / Codex CLI / Codex IDE 插件执行开发。  
> 项目目标是开发一个纯本地 Windows 桌面软件《中医大全》，第一阶段重点是：**中医资料录入、导入、清洗、校验、去重、合并、关系建议、搜索性能、备份恢复、AI 接口预留**。  
> 第一阶段不做真实 AI 调用、不做联网、不做在线问诊、不做自动诊断、不做自动开方。

---

## 0. 使用方式建议

建议不要让一个 Codex 会话一次性完成全部功能。  
本项目应采用**多线程 / 多分支 / 多工作区并行开发**。

推荐方式：

```bash
git checkout -b codex/main-base
```

然后按模块建立多个工作分支或工作树：

```bash
git worktree add ../zy-thread-db codex/thread-db-schema
git worktree add ../zy-thread-entry codex/thread-entry-ui
git worktree add ../zy-thread-import codex/thread-import-clean
git worktree add ../zy-thread-search codex/thread-search-performance
git worktree add ../zy-thread-relation codex/thread-dedup-relation
git worktree add ../zy-thread-backup codex/thread-backup-jobs
git worktree add ../zy-thread-ai codex/thread-ai-ready
git worktree add ../zy-thread-qa codex/thread-qa-regression
```

如果你不使用 `git worktree`，也可以每次开一个 Codex 线程，让它只修改自己负责的模块。  
**不要让多个线程同时大改同一个文件。**

---

# 1. 项目总目标

## 1.1 项目名称

```text
中医大全
```

## 1.2 当前版本目标

```text
v0.1-alpha-package
```

## 1.3 产品定位

开发一个纯本地运行的中医知识资产管理软件。

核心目标不是问诊，不是 AI 看病，而是：

```text
录入 → 导入 → 映射 → 清洗 → 校验 → 暂存 → 修正 → 去重 → 合并 → 关联 → 入库 → 搜索 → 备份
```

## 1.4 第一阶段必须完成

```text
1. Tauri + React + SQLite 桌面软件框架
2. 用户本地数据目录
3. 六类知识：中药、方剂、经络、穴位、证型、病症
4. 手动新增、编辑、删除
5. 表格录入
6. JSON / CSV 导入
7. 字段映射
8. 暂存区
9. 数据标准化
10. 数据校验
11. 重复检测
12. 重复合并
13. 关系建议
14. FTS5 搜索
15. search_terms 中文搜索词表
16. knowledge_list_view_cache 列表读缓存
17. relation_count_cache 关系数量缓存
18. background_jobs 后台任务
19. performance_logs 性能日志
20. 版本历史
21. 备份恢复
22. AI 设置页面和 AI 接口预留
```

---

# 2. 硬性边界

## 2.1 必须坚持

```text
纯本地运行
产品运行期默认不联网
不登录
不上传
不依赖服务器
数据保存在用户电脑
可备份
可恢复
可迁移
AI 可关闭
AI 可替换
AI 不参与 v0.1 主流程
```

开发过程说明：

```text
“不联网”指后期产品运行期与 v0.1 主流程不接入互联网、不上传数据、不依赖服务器。
开发过程允许联网安装依赖、查询构建资料、运行开发工具。
AI 默认关闭；配置 OpenAI-compatible API 后才真实调用，主流程不上传整库或原始资料。
```

## 2.2 v0.1 禁止开发

```text
真实 AI 调用
在线问诊
自动诊断
自动开方
药品交易
云同步
用户社区
医生入驻
资料可信度评分
来源级别 A/B/C/D/E
字段级证据
发布快照
复杂医学冲突检测
人体穴位图谱
健康档案
医案系统
```

## 2.3 文案禁区

软件内不得出现：

```text
AI医生
自动诊断
自动开方
治疗方案
自行针刺教程
按此服药
保证治愈
```

## 2.4 必须显示的安全文案

全局免责声明：

```text
本软件仅用于中医知识学习、资料整理与本地记录，不构成医疗诊断、治疗建议或处方依据。
```

针灸相关字段旁显示：

```text
仅供专业学习参考，请勿自行操作。
```

---

# 3. 技术架构

## 3.1 技术栈

```text
Tauri + React + SQLite
```

## 3.2 分层架构

```text
React 前端界面
↓
Tauri invoke
↓
Rust Commands
↓
Services 业务服务层
↓
Repositories 数据访问层
↓
SQLite / FTS5 / 本地文件系统
```

## 3.3 总目录结构

```text
zhongyi-daquan/
├─ src-tauri/
│  ├─ src/
│  │  ├─ commands/
│  │  ├─ services/
│  │  ├─ repositories/
│  │  ├─ models/
│  │  ├─ db/
│  │  ├─ errors/
│  │  └─ main.rs
│  └─ tauri.conf.json
│
├─ frontend/
│  ├─ src/
│  │  ├─ pages/
│  │  ├─ modules/
│  │  ├─ components/
│  │  ├─ hooks/
│  │  ├─ stores/
│  │  └─ routes/
│
├─ data-seed/
├─ docs/
└─ README.md
```

## 3.4 用户本地数据目录

```text
中医大全数据/
├─ database/
│  └─ zhongyi.db
├─ images/
├─ imports/
├─ exports/
├─ backups/
├─ logs/
├─ config/
└─ temp/
```

---

# 4. Rust 后端模块划分

## 4.1 commands

```text
knowledge_commands.rs
entry_commands.rs
import_commands.rs
clean_commands.rs
search_commands.rs
relation_commands.rs
backup_commands.rs
job_commands.rs
performance_commands.rs
ai_commands.rs
settings_commands.rs
```

## 4.2 services

```text
entry_service.rs
grid_edit_service.rs
import_project_service.rs
field_mapping_service.rs
normalize_service.rs
validation_service.rs
dedup_service.rs
relation_suggest_service.rs
transform_history_service.rs
search_index_service.rs
background_job_service.rs
backup_service.rs
version_service.rs
performance_service.rs
audit_log_service.rs
ai_placeholder_service.rs
settings_service.rs
```

## 4.3 repositories

```text
knowledge_repository.rs
detail_repository.rs
import_repository.rs
mapping_repository.rs
standard_term_repository.rs
validation_repository.rs
dedup_repository.rs
relation_repository.rs
search_repository.rs
version_repository.rs
job_repository.rs
backup_repository.rs
performance_repository.rs
audit_repository.rs
ai_repository.rs
settings_repository.rs
```

---

# 5. 数据库设计

## 5.1 SQLite 初始化 PRAGMA

数据库初始化时执行：

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -64000;
```

---

## 5.2 主知识表

```sql
CREATE TABLE IF NOT EXISTS knowledge_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  type TEXT NOT NULL,
  code TEXT,
  name TEXT NOT NULL,
  alias TEXT,
  pinyin TEXT,
  category TEXT,
  summary TEXT,
  content TEXT,
  source_note TEXT,
  tags TEXT,
  data_status TEXT NOT NULL DEFAULT 'draft',
  completeness_status TEXT NOT NULL DEFAULT 'partial',
  content_version INTEGER NOT NULL DEFAULT 1,
  is_favorite INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

### data_status

```text
draft        草稿
imported     已导入
needs_fix    需修正
validated    已校验
ready        可使用
archived     已归档
```

### completeness_status

```text
empty
partial
complete
```

默认搜索只显示：

```text
validated
ready
```

---

## 5.3 类型详情表

### 中药详情

```sql
CREATE TABLE IF NOT EXISTS herb_details (
  item_id INTEGER PRIMARY KEY,
  nature_flavor TEXT,
  meridians TEXT,
  effects TEXT,
  indications TEXT,
  dosage TEXT,
  contraindications TEXT,
  compatibility TEXT,
  notes TEXT,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);
```

### 方剂详情

```sql
CREATE TABLE IF NOT EXISTS formula_details (
  item_id INTEGER PRIMARY KEY,
  source_text TEXT,
  composition TEXT,
  usage TEXT,
  effects TEXT,
  indications TEXT,
  explanation TEXT,
  modifications TEXT,
  contraindications TEXT,
  notes TEXT,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);
```

### 经络详情

```sql
CREATE TABLE IF NOT EXISTS meridian_details (
  item_id INTEGER PRIMARY KEY,
  meridian_code TEXT,
  category TEXT,
  yin_yang TEXT,
  hand_foot TEXT,
  organ_relation TEXT,
  paired_meridian TEXT,
  pathway_text TEXT,
  main_indications TEXT,
  notes TEXT,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);
```

### 穴位详情

```sql
CREATE TABLE IF NOT EXISTS acupoint_details (
  item_id INTEGER PRIMARY KEY,
  acupoint_code TEXT,
  meridian_item_id INTEGER,
  body_region TEXT,
  body_subregion TEXT,
  side_type TEXT,
  standard_location TEXT,
  locating_method TEXT,
  bone_cun TEXT,
  anatomy TEXT,
  functions TEXT,
  indications TEXT,
  needling_summary TEXT,
  moxibustion_summary TEXT,
  massage_summary TEXT,
  contraindications TEXT,
  precautions TEXT,
  risk_level TEXT,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE,
  FOREIGN KEY(meridian_item_id) REFERENCES knowledge_items(id)
);
```

### 证型详情

```sql
CREATE TABLE IF NOT EXISTS syndrome_details (
  item_id INTEGER PRIMARY KEY,
  symptoms TEXT,
  tongue TEXT,
  pulse TEXT,
  pathogenesis TEXT,
  treatment_principle TEXT,
  notes TEXT,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);
```

### 病症详情

```sql
CREATE TABLE IF NOT EXISTS disease_details (
  item_id INTEGER PRIMARY KEY,
  symptoms TEXT,
  common_syndromes TEXT,
  care_advice TEXT,
  medical_warning TEXT,
  notes TEXT,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);
```

---

## 5.4 导入相关表

```sql
CREATE TABLE IF NOT EXISTS data_import_batches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  file_name TEXT NOT NULL,
  import_type TEXT NOT NULL,
  target_type TEXT NOT NULL,
  status TEXT NOT NULL,
  total_count INTEGER DEFAULT 0,
  parsed_count INTEGER DEFAULT 0,
  valid_count INTEGER DEFAULT 0,
  warning_count INTEGER DEFAULT 0,
  error_count INTEGER DEFAULT 0,
  created_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS data_import_rows (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id INTEGER NOT NULL,
  row_index INTEGER NOT NULL,
  raw_json TEXT,
  mapped_json TEXT,
  normalized_json TEXT,
  status TEXT NOT NULL,
  error_message TEXT,
  warning_message TEXT,
  FOREIGN KEY(batch_id) REFERENCES data_import_batches(id) ON DELETE CASCADE
);
```

```sql
CREATE TABLE IF NOT EXISTS data_validation_issues (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id INTEGER NOT NULL,
  row_id INTEGER,
  severity TEXT NOT NULL,
  issue_code TEXT NOT NULL,
  field_name TEXT,
  message TEXT NOT NULL,
  suggestion TEXT,
  FOREIGN KEY(batch_id) REFERENCES data_import_batches(id) ON DELETE CASCADE,
  FOREIGN KEY(row_id) REFERENCES data_import_rows(id) ON DELETE CASCADE
);
```

---

## 5.5 字段映射、词典、校验

```sql
CREATE TABLE IF NOT EXISTS field_mapping_templates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  target_type TEXT NOT NULL,
  source_headers_json TEXT NOT NULL,
  mapping_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS standard_terms (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  term_type TEXT NOT NULL,
  standard_name TEXT NOT NULL,
  aliases TEXT,
  code TEXT,
  notes TEXT
);
```

```sql
CREATE TABLE IF NOT EXISTS validation_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  target_type TEXT NOT NULL,
  field_name TEXT NOT NULL,
  rule_type TEXT NOT NULL,
  rule_params_json TEXT,
  severity TEXT NOT NULL,
  message TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1
);
```

---

## 5.6 清洗历史

```sql
CREATE TABLE IF NOT EXISTS data_transform_steps (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id INTEGER NOT NULL,
  step_order INTEGER NOT NULL,
  step_type TEXT NOT NULL,
  params_json TEXT,
  affected_rows INTEGER DEFAULT 0,
  before_summary TEXT,
  after_summary TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(batch_id) REFERENCES data_import_batches(id) ON DELETE CASCADE
);
```

```sql
CREATE TABLE IF NOT EXISTS data_transform_row_changes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  step_id INTEGER NOT NULL,
  row_id INTEGER NOT NULL,
  field_name TEXT NOT NULL,
  old_value TEXT,
  new_value TEXT,
  FOREIGN KEY(step_id) REFERENCES data_transform_steps(id) ON DELETE CASCADE,
  FOREIGN KEY(row_id) REFERENCES data_import_rows(id) ON DELETE CASCADE
);
```

---

## 5.7 去重与合并

```sql
CREATE TABLE IF NOT EXISTS duplicate_candidates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id INTEGER,
  existing_item_id INTEGER,
  imported_row_id INTEGER,
  match_type TEXT NOT NULL,
  match_score REAL,
  reason TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS merge_records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  existing_item_id INTEGER NOT NULL,
  imported_row_id INTEGER,
  merge_strategy TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT,
  created_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS knowledge_fingerprints (
  item_id INTEGER PRIMARY KEY,
  type TEXT NOT NULL,
  code_norm TEXT,
  name_norm TEXT,
  pinyin_norm TEXT,
  alias_norm TEXT,
  fingerprint TEXT NOT NULL
);
```

---

## 5.8 关系

```sql
CREATE TABLE IF NOT EXISTS relation_suggestions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_item_id INTEGER,
  target_item_id INTEGER,
  relation_type TEXT NOT NULL,
  confidence REAL,
  reason TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS knowledge_relations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_item_id INTEGER NOT NULL,
  target_item_id INTEGER NOT NULL,
  relation_type TEXT NOT NULL,
  note TEXT,
  FOREIGN KEY(source_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE,
  FOREIGN KEY(target_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);
```

```sql
CREATE TABLE IF NOT EXISTS relation_count_cache (
  item_id INTEGER NOT NULL,
  relation_type TEXT NOT NULL,
  count INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(item_id, relation_type)
);
```

---

## 5.9 搜索与缓存

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
  name,
  code,
  alias,
  pinyin,
  category,
  summary,
  content,
  tags,
  content=''
);
```

```sql
CREATE TABLE IF NOT EXISTS search_terms (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL,
  term TEXT NOT NULL,
  term_type TEXT NOT NULL,
  weight INTEGER NOT NULL DEFAULT 10
);
```

```sql
CREATE TABLE IF NOT EXISTS knowledge_list_view_cache (
  item_id INTEGER PRIMARY KEY,
  type TEXT NOT NULL,
  code TEXT,
  name TEXT NOT NULL,
  pinyin TEXT,
  category TEXT,
  summary TEXT,
  tags TEXT,
  data_status TEXT NOT NULL,
  is_favorite INTEGER NOT NULL DEFAULT 0,
  relation_count INTEGER DEFAULT 0,
  updated_at TEXT NOT NULL
);
```

---

## 5.10 版本、任务、性能、审计

```sql
CREATE TABLE IF NOT EXISTS knowledge_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL,
  version_no INTEGER NOT NULL,
  snapshot_json TEXT NOT NULL,
  change_summary TEXT,
  changed_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS background_jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_type TEXT NOT NULL,
  status TEXT NOT NULL,
  progress REAL DEFAULT 0,
  params_json TEXT,
  result_json TEXT,
  error_message TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS performance_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  action TEXT NOT NULL,
  duration_ms INTEGER NOT NULL,
  row_count INTEGER,
  query_type TEXT,
  created_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS audit_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  action TEXT NOT NULL,
  target_type TEXT,
  target_id INTEGER,
  before_json TEXT,
  after_json TEXT,
  created_at TEXT NOT NULL
);
```

---

## 5.11 AI 表

```sql
CREATE TABLE IF NOT EXISTS ai_provider_settings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_type TEXT NOT NULL,
  provider_name TEXT,
  base_url TEXT,
  api_key_encrypted TEXT,
  model_name TEXT,
  timeout_seconds INTEGER,
  max_tokens INTEGER,
  temperature REAL,
  enabled INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS ai_prompt_templates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_type TEXT NOT NULL,
  name TEXT NOT NULL,
  system_prompt TEXT,
  user_prompt_template TEXT,
  output_schema_json TEXT,
  safety_rules TEXT,
  version_no INTEGER NOT NULL DEFAULT 1,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS ai_tasks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_type TEXT NOT NULL,
  status TEXT NOT NULL,
  input_json TEXT,
  output_json TEXT,
  error_message TEXT,
  related_batch_id INTEGER,
  related_row_id INTEGER,
  related_item_id INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS ai_drafts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  task_id INTEGER,
  draft_type TEXT NOT NULL,
  draft_json TEXT NOT NULL,
  target_type TEXT,
  status TEXT NOT NULL DEFAULT 'pending_review',
  review_note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

```sql
CREATE TABLE IF NOT EXISTS ai_call_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_type TEXT,
  model_name TEXT,
  task_type TEXT,
  input_hash TEXT,
  prompt_version INTEGER,
  request_summary TEXT,
  response_summary TEXT,
  duration_ms INTEGER,
  token_usage_json TEXT,
  status TEXT,
  error_message TEXT,
  created_at TEXT NOT NULL
);
```

---

# 6. 必须创建的索引

```sql
CREATE INDEX IF NOT EXISTS idx_knowledge_type ON knowledge_items(type);
CREATE INDEX IF NOT EXISTS idx_knowledge_status ON knowledge_items(data_status);
CREATE INDEX IF NOT EXISTS idx_knowledge_type_status ON knowledge_items(type, data_status);
CREATE INDEX IF NOT EXISTS idx_knowledge_code ON knowledge_items(code);
CREATE INDEX IF NOT EXISTS idx_knowledge_name ON knowledge_items(name);
CREATE INDEX IF NOT EXISTS idx_knowledge_pinyin ON knowledge_items(pinyin);
CREATE INDEX IF NOT EXISTS idx_knowledge_category ON knowledge_items(category);
CREATE INDEX IF NOT EXISTS idx_knowledge_updated_at ON knowledge_items(updated_at);

CREATE INDEX IF NOT EXISTS idx_rel_source ON knowledge_relations(source_item_id);
CREATE INDEX IF NOT EXISTS idx_rel_target ON knowledge_relations(target_item_id);
CREATE INDEX IF NOT EXISTS idx_rel_type ON knowledge_relations(relation_type);
CREATE INDEX IF NOT EXISTS idx_rel_source_type ON knowledge_relations(source_item_id, relation_type);

CREATE INDEX IF NOT EXISTS idx_import_rows_batch ON data_import_rows(batch_id);
CREATE INDEX IF NOT EXISTS idx_import_rows_status ON data_import_rows(status);
CREATE INDEX IF NOT EXISTS idx_import_issues_batch ON data_validation_issues(batch_id);
CREATE INDEX IF NOT EXISTS idx_duplicate_batch ON duplicate_candidates(batch_id);

CREATE INDEX IF NOT EXISTS idx_search_terms_term ON search_terms(term);
CREATE INDEX IF NOT EXISTS idx_search_terms_item ON search_terms(item_id);
CREATE INDEX IF NOT EXISTS idx_search_terms_type ON search_terms(term_type);
```

---

# 7. 前端页面

## 7.1 首页工作台

显示：

```text
大搜索框
收藏条目
最近浏览
待处理导入批次
待修正错误数
重复候选数
关系建议数
后台任务状态
```

## 7.2 知识库页面

六类：

```text
中药
方剂
经络
穴位
证型
病症
```

支持：

```text
列表
筛选
搜索
详情
收藏
编辑
删除
查看关系
查看版本历史
```

## 7.3 数据录入中心

入口：

```text
快速新增
表格录入
批量导入
字段映射
暂存区
数据清洗
重复合并
关系建议
导入历史
任务中心
```

## 7.4 表格录入

要求：

```text
类 Excel 编辑
复制粘贴
批量填充
下拉选择
自动补全
错误单元格高亮
虚拟滚动
dirty_rows 保存
```

## 7.5 导入向导

步骤：

```text
选择文件
选择知识类型
字段映射
预览前 100 行
后台解析全部
数据清洗
数据校验
重复检测
进入暂存区
确认入库
```

## 7.6 暂存区

显示：

```text
总行数
可导入数
警告数
错误数
错误行
错误原因
修正建议
```

操作：

```text
编辑本行
跳过本行
批量确认
导出错误报告
确认入库
```

## 7.7 数据清洗

支持：

```text
去空格
全角半角统一
编号大写
拼音自动生成
标签拆分
经络别名标准化
批量替换
批量设置分类
批量设置状态
撤销上一步
```

## 7.8 重复合并

支持：

```text
保留已有
用新数据覆盖
仅补充空字段
合并标签
另存为新条目
```

## 7.9 关系建议

支持：

```text
接受
拒绝
批量接受
查看原因
```

---

# 8. 性能要求

## 8.1 后端

```text
所有列表分页
默认 page_size = 50
最大 page_size = 200
导入每批 500-1000 行
大任务走 background_jobs
搜索使用 FTS5 + search_terms
正式入库后批量更新索引
详情页关联关系懒加载
```

## 8.2 前端

```text
表格虚拟滚动
暂存区虚拟滚动
搜索输入 300ms 防抖
搜索结果分组展示
每组默认 20 条
点击查看更多分页加载
大数据不进全局 store
只提交 dirty_rows
```

## 8.3 性能验收

测试数据：

```text
knowledge_items：10,000 条
knowledge_relations：50,000 条
data_import_rows：10,000 行
duplicate_candidates：1,000 条
relation_suggestions：1,000 条
```

目标：

```text
启动到首页可交互 < 2 秒
首页加载 < 1 秒
知识列表翻页 < 300ms
全局搜索 < 500ms
详情页基础信息 < 300ms
关联知识首屏 < 500ms
暂存区 10,000 行分页切换 < 500ms
表格录入 5,000 行滚动流畅
导入 10,000 行 CSV 不阻塞界面
重建搜索索引后台执行
```

---

# 9. AI 接口预留要求

## 9.1 AI 当前状态

```text
创建 AI 表
创建 AI 设置页面
创建 ai_commands.rs
创建 AI 服务接口与 OpenAI-compatible 调用路径
默认关闭 AI
没有 AI 配置时软件完整可用
```

## 9.2 可接入提供方

```text
Ollama
OpenAI 兼容接口
DeepSeek
本地 HTTP 模型服务
自定义模型服务
本地 RAG
```

## 9.3 AI 输出规则

```text
AI 输出只能进入 ai_drafts
用户确认后才能入库
AI 不直接修改正式知识库
AI 不诊断
AI 不开方
AI 不生成针刺治疗方案
AI 不要求用户服药
```

---

# 10. 多线程开发分工

## 总控线程：架构与集成

分支建议：

```text
codex/main-base
```

任务：

```text
1. 初始化项目
2. 建立目录结构
3. 确定统一错误类型
4. 确定命令返回格式
5. 建立数据库迁移框架
6. 合并其他线程成果
7. 跑全量测试
```

---

## 线程 A：数据库与迁移

分支建议：

```text
codex/thread-db-schema
```

任务：

```text
1. 建立 SQLite 初始化
2. 执行 PRAGMA
3. 创建全部核心表
4. 创建 FTS5
5. 创建索引
6. 创建种子数据
7. 创建迁移脚本
8. 提供数据库文档
```

验收：

```text
数据库首次启动可自动创建
重复启动不会重复建表报错
FTS5 可用
索引存在
外键启用
```

---

## 线程 B：知识库 CRUD 与表格录入

分支建议：

```text
codex/thread-entry-ui
```

任务：

```text
1. 知识列表
2. 知识详情
3. 手动新增
4. 编辑
5. 删除
6. 收藏
7. 表格录入页面
8. 虚拟滚动
9. dirty_rows 保存
```

验收：

```text
可新增黄芪
可新增足阳明胃经
可新增足三里并绑定胃经
可表格录入多条穴位
可复制粘贴多行数据
```

---

## 线程 C：导入、字段映射、清洗、校验

分支建议：

```text
codex/thread-import-clean
```

任务：

```text
1. JSON 导入
2. CSV 导入
3. 字段映射
4. 字段映射模板
5. 暂存区
6. 标准化处理
7. 校验规则
8. 错误报告
9. 清洗步骤记录
10. 撤销上一步清洗
```

验收：

```text
导入后先进暂存区
能显示错误行
能保存字段映射模板
能把 st36 标准化为 ST36
能把 胃经 标准化为 足阳明胃经
```

---

## 线程 D：搜索与性能

分支建议：

```text
codex/thread-search-performance
```

任务：

```text
1. FTS5 搜索
2. search_terms 词表
3. 搜索权重排序
4. 搜索结果分组
5. knowledge_list_view_cache
6. relation_count_cache
7. rebuild_search_index
8. 性能日志
9. 性能测试数据生成器
```

验收：

```text
搜索足三里、ST36、黄芪、补中益气汤、胃经均可命中
10,000 条知识搜索 < 500ms
列表翻页 < 300ms
```

---

## 线程 E：去重、合并、关系建议

分支建议：

```text
codex/thread-dedup-relation
```

任务：

```text
1. knowledge_fingerprints
2. duplicate_candidates
3. 重复检测规则
4. 合并页面
5. merge_records
6. relation_suggestions
7. 方剂组成识别中药
8. 穴位绑定经络
9. 用户接受关系建议后写入 knowledge_relations
```

验收：

```text
能检测重复 ST36
能合并重复数据
方剂组成能生成中药关系建议
接受后写入 knowledge_relations
```

---

## 线程 F：后台任务、备份恢复、维护工具

分支建议：

```text
codex/thread-backup-jobs
```

任务：

```text
1. background_jobs
2. 任务中心 UI
3. 导入任务进度
4. 重建索引任务
5. 备份数据库
6. 恢复数据库
7. 恢复后重建索引
8. 数据库维护页面
```

验收：

```text
导入 10,000 行 CSV 不阻塞界面
重建索引可后台执行
备份恢复后仍可搜索
任务中心显示进度
```

---

## 线程 G：AI 接口预留

分支建议：

```text
codex/thread-ai-ready
```

任务：

```text
1. AI 表
2. AI 设置页面
3. ai_commands.rs
4. AiProviderService 接口
5. PromptTemplateService 接口
6. AiDraftService 接口
7. AiSafetyService 接口
8. 禁用态返回“AI 未启用或尚未配置 API Key”
```

验收：

```text
AI 设置页面存在
AI 默认关闭
可以保存 provider_type、base_url、model_name
无 AI 配置时软件完整可用
AI 禁用态返回明确提示
```

---

## 线程 H：测试、文档、回归

分支建议：

```text
codex/thread-qa-regression
```

任务：

```text
1. 编写测试数据生成器
2. 编写基础单元测试
3. 编写导入流程测试
4. 编写搜索性能测试
5. 编写备份恢复测试
6. 编写 README
7. 编写 DATABASE_SCHEMA.md
8. 编写 DEV_GUIDE.md
9. 编写 PACKAGING.md
10. 编写 TEST_PLAN.md
```

验收：

```text
能生成 10,000 条知识
能生成 50,000 条关系
能测试搜索性能
能测试备份恢复
文档齐全
```

---

# 11. 多线程开发顺序

建议顺序：

```text
第一轮：
A 数据库与迁移
B 基础知识 CRUD
D 搜索基础

第二轮：
C 导入与暂存
E 去重与关系建议
F 后台任务与备份

第三轮：
G AI 接口预留
H 测试与文档

第四轮：
总控线程集成所有分支
跑全量测试
修复冲突
准备 v0.1-release
```

---

# 12. 每个线程提交时必须输出

每个 Codex 线程完成后必须输出：

```text
1. 完成内容
2. 修改文件列表
3. 新增表 / 新增接口 / 新增页面
4. 如何启动
5. 如何测试
6. 已通过测试
7. 未完成事项
8. 风险点
9. 下一步建议
```

---

# 13. 最终验收标准

```text
1. 双击 exe 启动独立窗口。
2. 无联网请求。
3. 可手动新增黄芪。
4. 可手动新增足阳明胃经。
5. 可手动新增足三里并绑定足阳明胃经。
6. 可表格录入多条穴位。
7. 可复制 Excel 多行数据粘贴到表格录入页。
8. 可导入 JSON。
9. 可导入 CSV。
10. 导入后先进入暂存区。
11. 暂存区显示错误行和错误原因。
12. 可保存字段映射模板。
13. 可自动把 st36 标准化为 ST36。
14. 可自动把 胃经 标准化为 足阳明胃经。
15. 可检测重复 ST36。
16. 可合并重复数据。
17. 清洗操作写入 data_transform_steps。
18. 可撤销上一步清洗。
19. 方剂组成能生成中药关系建议。
20. 接受关系建议后写入 knowledge_relations。
21. 正式入库后可全局搜索。
22. 搜索足三里、ST36、黄芪、补中益气汤、胃经均可命中。
23. 搜索 10,000 条知识 < 500ms。
24. 知识列表翻页 < 300ms。
25. 详情页基础信息 < 300ms。
26. 关系表 50,000 条时详情首屏 < 500ms。
27. 表格录入 5,000 行滚动流畅。
28. 导入 10,000 行 CSV 不阻塞界面。
29. 后台任务有进度显示。
30. 重建搜索索引可后台执行。
31. 编辑知识生成版本历史。
32. 恢复版本后搜索索引同步更新。
33. 可备份恢复。
34. 备份恢复后数据仍然可搜索。
35. AI 设置页面存在。
36. AI 默认关闭。
37. 没有 AI 配置时软件完整可用。
38. AI 禁用态返回“AI 未启用或尚未配置 API Key”。
```

---

# 14. 给 Codex 的总控指令

下面这段可以直接复制给总控线程 Codex：

```text
你是本项目的总控开发线程。项目名称为《中医大全》，当前发布收口版本为 v0.1-alpha-package，基于 v0.1-alpha。

请严格按照 docs/CODEX_DEV_DOC.md 中的架构执行开发。

本项目是纯本地 Windows 桌面软件，技术栈为 Tauri + React + SQLite。
当前版本重点是中医资料录入、批量导入、字段映射、数据清洗、校验、去重、合并、关系建议、搜索性能、后台任务、备份恢复和 AI 接口预留。

硬性要求：
1. 产品运行期默认不联网；开发过程允许联网安装依赖、查询资料和运行开发工具。
2. 不登录。
3. 不上传。
4. 不在线问诊。
5. 不自动诊断。
6. 不自动开方。
7. AI 默认关闭；配置 OpenAI-compatible API 后才真实调用。
8. 前端不得直接访问 SQLite。
9. 前端通过 Tauri invoke 调用 Rust commands。
10. commands 调用 services。
11. services 调用 repositories。
12. 复杂写入必须使用事务。
13. 大数据任务必须走 background_jobs。
14. 表格和暂存区必须使用虚拟滚动。
15. 所有列表必须分页。
16. 搜索必须使用 FTS5 + search_terms。
17. AI 默认关闭，配置后走受控 OpenAI-compatible 调用。

请先检查当前仓库结构，然后执行：
1. 创建推荐目录结构。
2. 创建 docs/CODEX_DEV_DOC.md。
3. 创建数据库迁移基础。
4. 拆分多线程任务边界。
5. 不要一次性实现全部功能，先完成可运行骨架。
6. 每次修改后运行可用测试。
7. 输出修改文件、测试结果和下一步建议。
```

---

# 15. 给各线程的最短指令模板

## 线程 A 指令

```text
你负责《中医大全》线程 A：数据库与迁移。
只修改数据库、模型、迁移、基础 repository，不做 UI。
目标：完成 SQLite 初始化、PRAGMA、所有核心表、索引、FTS5、种子数据和数据库文档。
完成后输出修改文件、表结构、测试结果。
```

## 线程 B 指令

```text
你负责《中医大全》线程 B：知识 CRUD 与表格录入。
只做知识库页面、手动新增、编辑、删除、收藏、表格录入和 dirty_rows 保存。
不要改导入流水线，不要改搜索底层。
完成后确保可新增黄芪、足阳明胃经、足三里。
```

## 线程 C 指令

```text
你负责《中医大全》线程 C：导入、字段映射、清洗、校验。
只做 JSON/CSV 导入、字段映射、暂存区、标准化、校验规则、错误报告、清洗历史和撤销。
不要做真实 AI，不要做人体图谱。
```

## 线程 D 指令

```text
你负责《中医大全》线程 D：搜索与性能。
只做 FTS5、search_terms、搜索权重、列表缓存、关系数量缓存、性能日志、测试数据生成器和搜索性能测试。
确保 10,000 条知识搜索小于 500ms。
```

## 线程 E 指令

```text
你负责《中医大全》线程 E：去重、合并、关系建议。
只做 fingerprints、duplicate_candidates、merge_records、relation_suggestions、knowledge_relations 相关功能。
确保可以检测重复 ST36，并能生成方剂-中药关系建议。
```

## 线程 F 指令

```text
你负责《中医大全》线程 F：后台任务、备份恢复、维护工具。
只做 background_jobs、任务中心、备份、恢复、恢复后重建索引、数据库维护页。
确保导入和重建索引不会卡 UI。
```

## 线程 G 指令

```text
你负责《中医大全》线程 G：AI 接口。
维护 AI 表、AI 设置页、ai_commands、AI 服务接口、禁用态提示和 OpenAI-compatible 调用路径。
AI 默认关闭，配置后才真实调用。
AI 默认关闭，无配置时主功能完整可用。
```

## 线程 H 指令

```text
你负责《中医大全》线程 H：测试、文档、回归。
只做测试数据生成器、单元测试、集成测试、性能测试、README、DATABASE_SCHEMA.md、DEV_GUIDE.md、PACKAGING.md、TEST_PLAN.md。
确保所有线程合并后可回归验证。
```

---

# 16. 最终开发提醒

不要一次性做大而全。  
第一版只追求：

```text
能录入
能导入
能清洗
能校验
能去重
能关联
能搜索
不卡顿
可备份
AI 可预留
```

其余功能全部后置。
