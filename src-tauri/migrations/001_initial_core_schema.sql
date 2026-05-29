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

CREATE TABLE IF NOT EXISTS disease_details (
  item_id INTEGER PRIMARY KEY,
  symptoms TEXT,
  common_syndromes TEXT,
  care_advice TEXT,
  medical_warning TEXT,
  notes TEXT,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

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

CREATE TABLE IF NOT EXISTS field_mapping_templates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  target_type TEXT NOT NULL,
  source_headers_json TEXT NOT NULL,
  mapping_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS standard_terms (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  term_type TEXT NOT NULL,
  standard_name TEXT NOT NULL,
  aliases TEXT,
  code TEXT,
  notes TEXT
);

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

CREATE TABLE IF NOT EXISTS duplicate_candidates (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  batch_id INTEGER,
  existing_item_id INTEGER,
  imported_row_id INTEGER,
  match_type TEXT NOT NULL,
  match_score REAL,
  reason TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL,
  FOREIGN KEY(batch_id) REFERENCES data_import_batches(id) ON DELETE CASCADE,
  FOREIGN KEY(existing_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE,
  FOREIGN KEY(imported_row_id) REFERENCES data_import_rows(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS merge_records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  existing_item_id INTEGER NOT NULL,
  imported_row_id INTEGER,
  merge_strategy TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(existing_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE,
  FOREIGN KEY(imported_row_id) REFERENCES data_import_rows(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS knowledge_fingerprints (
  item_id INTEGER PRIMARY KEY,
  type TEXT NOT NULL,
  code_norm TEXT,
  name_norm TEXT,
  pinyin_norm TEXT,
  alias_norm TEXT,
  fingerprint TEXT NOT NULL,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS relation_suggestions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_item_id INTEGER,
  target_item_id INTEGER,
  relation_type TEXT NOT NULL,
  confidence REAL,
  reason TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at TEXT NOT NULL,
  FOREIGN KEY(source_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE,
  FOREIGN KEY(target_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_relations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_item_id INTEGER NOT NULL,
  target_item_id INTEGER NOT NULL,
  relation_type TEXT NOT NULL,
  note TEXT,
  FOREIGN KEY(source_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE,
  FOREIGN KEY(target_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS relation_count_cache (
  item_id INTEGER NOT NULL,
  relation_type TEXT NOT NULL,
  count INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(item_id, relation_type),
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
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
  weight INTEGER NOT NULL DEFAULT 10,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

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
  updated_at TEXT NOT NULL,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS knowledge_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL,
  version_no INTEGER NOT NULL,
  snapshot_json TEXT NOT NULL,
  change_summary TEXT,
  changed_at TEXT NOT NULL,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
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

CREATE TABLE IF NOT EXISTS audit_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  action TEXT NOT NULL,
  target_type TEXT,
  target_id INTEGER,
  before_json TEXT,
  after_json TEXT,
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
  updated_at TEXT NOT NULL,
  FOREIGN KEY(related_batch_id) REFERENCES data_import_batches(id) ON DELETE SET NULL,
  FOREIGN KEY(related_row_id) REFERENCES data_import_rows(id) ON DELETE SET NULL,
  FOREIGN KEY(related_item_id) REFERENCES knowledge_items(id) ON DELETE SET NULL
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
  updated_at TEXT NOT NULL,
  FOREIGN KEY(task_id) REFERENCES ai_tasks(id) ON DELETE SET NULL
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

CREATE INDEX IF NOT EXISTS idx_herb_details_item ON herb_details(item_id);
CREATE INDEX IF NOT EXISTS idx_formula_details_item ON formula_details(item_id);
CREATE INDEX IF NOT EXISTS idx_meridian_details_item ON meridian_details(item_id);
CREATE INDEX IF NOT EXISTS idx_acupoint_details_item ON acupoint_details(item_id);
CREATE INDEX IF NOT EXISTS idx_acupoint_details_meridian ON acupoint_details(meridian_item_id);
CREATE INDEX IF NOT EXISTS idx_syndrome_details_item ON syndrome_details(item_id);
CREATE INDEX IF NOT EXISTS idx_disease_details_item ON disease_details(item_id);
CREATE INDEX IF NOT EXISTS idx_import_batches_status ON data_import_batches(status);
CREATE INDEX IF NOT EXISTS idx_import_batches_target_type ON data_import_batches(target_type);
CREATE INDEX IF NOT EXISTS idx_validation_rules_target_field ON validation_rules(target_type, field_name);
CREATE INDEX IF NOT EXISTS idx_standard_terms_type_name ON standard_terms(term_type, standard_name);
CREATE INDEX IF NOT EXISTS idx_transform_steps_batch ON data_transform_steps(batch_id);
CREATE INDEX IF NOT EXISTS idx_transform_row_changes_step ON data_transform_row_changes(step_id);
CREATE INDEX IF NOT EXISTS idx_duplicate_existing_item ON duplicate_candidates(existing_item_id);
CREATE INDEX IF NOT EXISTS idx_duplicate_imported_row ON duplicate_candidates(imported_row_id);
CREATE INDEX IF NOT EXISTS idx_merge_existing_item ON merge_records(existing_item_id);
CREATE INDEX IF NOT EXISTS idx_fingerprints_type_name ON knowledge_fingerprints(type, name_norm);
CREATE INDEX IF NOT EXISTS idx_fingerprints_fingerprint ON knowledge_fingerprints(fingerprint);
CREATE INDEX IF NOT EXISTS idx_relation_suggestions_status ON relation_suggestions(status);
CREATE INDEX IF NOT EXISTS idx_relation_suggestions_source ON relation_suggestions(source_item_id);
CREATE INDEX IF NOT EXISTS idx_relation_suggestions_target ON relation_suggestions(target_item_id);
CREATE INDEX IF NOT EXISTS idx_relation_count_item ON relation_count_cache(item_id);
CREATE INDEX IF NOT EXISTS idx_list_cache_type_status ON knowledge_list_view_cache(type, data_status);
CREATE INDEX IF NOT EXISTS idx_versions_item_version ON knowledge_versions(item_id, version_no);
CREATE INDEX IF NOT EXISTS idx_jobs_status_type ON background_jobs(status, job_type);
CREATE INDEX IF NOT EXISTS idx_performance_action_created ON performance_logs(action, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_target ON audit_logs(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_ai_provider_enabled ON ai_provider_settings(enabled);
CREATE INDEX IF NOT EXISTS idx_ai_templates_task_enabled ON ai_prompt_templates(task_type, enabled);
CREATE INDEX IF NOT EXISTS idx_ai_tasks_status_type ON ai_tasks(status, task_type);
CREATE INDEX IF NOT EXISTS idx_ai_drafts_status ON ai_drafts(status);
CREATE INDEX IF NOT EXISTS idx_ai_call_logs_task_created ON ai_call_logs(task_type, created_at);
