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

CREATE TABLE IF NOT EXISTS performance_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  action TEXT NOT NULL,
  duration_ms INTEGER NOT NULL,
  row_count INTEGER,
  query_type TEXT,
  created_at TEXT NOT NULL
);

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

CREATE TABLE IF NOT EXISTS search_terms (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL,
  term TEXT NOT NULL,
  term_type TEXT NOT NULL,
  weight INTEGER NOT NULL DEFAULT 10
);

CREATE INDEX IF NOT EXISTS idx_knowledge_type ON knowledge_items(type);
CREATE INDEX IF NOT EXISTS idx_knowledge_status ON knowledge_items(data_status);
CREATE INDEX IF NOT EXISTS idx_knowledge_type_status ON knowledge_items(type, data_status);
CREATE INDEX IF NOT EXISTS idx_knowledge_code ON knowledge_items(code);
CREATE INDEX IF NOT EXISTS idx_knowledge_name ON knowledge_items(name);
CREATE INDEX IF NOT EXISTS idx_search_terms_term ON search_terms(term);
CREATE INDEX IF NOT EXISTS idx_search_terms_item ON search_terms(item_id);
