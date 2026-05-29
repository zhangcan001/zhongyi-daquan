DROP TABLE IF EXISTS knowledge_fts;

CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
  name,
  code,
  alias,
  pinyin,
  category,
  summary,
  content,
  tags
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
  PRIMARY KEY(item_id, relation_type)
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
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_knowledge_pinyin ON knowledge_items(pinyin);
CREATE INDEX IF NOT EXISTS idx_knowledge_category ON knowledge_items(category);
CREATE INDEX IF NOT EXISTS idx_knowledge_updated_at ON knowledge_items(updated_at);
CREATE INDEX IF NOT EXISTS idx_rel_source ON knowledge_relations(source_item_id);
CREATE INDEX IF NOT EXISTS idx_rel_target ON knowledge_relations(target_item_id);
CREATE INDEX IF NOT EXISTS idx_rel_type ON knowledge_relations(relation_type);
CREATE INDEX IF NOT EXISTS idx_rel_source_type ON knowledge_relations(source_item_id, relation_type);
CREATE INDEX IF NOT EXISTS idx_search_terms_type ON search_terms(term_type);
CREATE INDEX IF NOT EXISTS idx_search_terms_term_type ON search_terms(term, term_type);
CREATE INDEX IF NOT EXISTS idx_list_cache_type_status ON knowledge_list_view_cache(type, data_status);
CREATE INDEX IF NOT EXISTS idx_list_cache_updated_at ON knowledge_list_view_cache(updated_at);
