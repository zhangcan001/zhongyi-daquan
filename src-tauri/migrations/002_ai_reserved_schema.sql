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

CREATE INDEX IF NOT EXISTS idx_ai_tasks_status ON ai_tasks(status);
CREATE INDEX IF NOT EXISTS idx_ai_tasks_type ON ai_tasks(task_type);
CREATE INDEX IF NOT EXISTS idx_ai_drafts_status ON ai_drafts(status);
CREATE INDEX IF NOT EXISTS idx_ai_call_logs_created_at ON ai_call_logs(created_at);
