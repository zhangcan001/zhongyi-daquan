CREATE TABLE IF NOT EXISTS knowledge_annotations (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  knowledge_item_id INTEGER NOT NULL,
  annotation_type TEXT NOT NULL,
  source_title TEXT,
  source_note TEXT,
  content TEXT,
  detail_json TEXT,
  tags_json TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(knowledge_item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_knowledge_annotations_item
  ON knowledge_annotations(knowledge_item_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_annotations_source
  ON knowledge_annotations(source_title, source_note);
