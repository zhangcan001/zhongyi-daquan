CREATE TABLE IF NOT EXISTS import_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  package_name TEXT,
  import_intent TEXT NOT NULL,
  package_path TEXT,
  status TEXT NOT NULL DEFAULT 'running',
  total_records INTEGER NOT NULL DEFAULT 0,
  create_count INTEGER NOT NULL DEFAULT 0,
  update_count INTEGER NOT NULL DEFAULT 0,
  attach_annotation_count INTEGER NOT NULL DEFAULT 0,
  skip_duplicate_count INTEGER NOT NULL DEFAULT 0,
  failed_count INTEGER NOT NULL DEFAULT 0,
  report_json TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT,
  rolled_back_at TEXT
);

CREATE TABLE IF NOT EXISTS import_run_changes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  import_run_id INTEGER NOT NULL,
  action_type TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id INTEGER,
  target_existing_id INTEGER,
  before_json TEXT,
  after_json TEXT,
  rollback_action TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'applied',
  created_at TEXT NOT NULL,
  rolled_back_at TEXT,
  FOREIGN KEY(import_run_id) REFERENCES import_runs(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS import_reports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  import_run_id INTEGER NOT NULL,
  summary_json TEXT NOT NULL,
  warnings_json TEXT NOT NULL,
  errors_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY(import_run_id) REFERENCES import_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_import_runs_created_at
  ON import_runs(created_at);

CREATE INDEX IF NOT EXISTS idx_import_run_changes_run
  ON import_run_changes(import_run_id);

CREATE INDEX IF NOT EXISTS idx_import_reports_run
  ON import_reports(import_run_id);
