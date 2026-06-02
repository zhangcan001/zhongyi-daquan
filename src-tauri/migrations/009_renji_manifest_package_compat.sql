ALTER TABLE knowledge_items ADD COLUMN detail TEXT DEFAULT '{}';
ALTER TABLE knowledge_items ADD COLUMN import_batch_id TEXT;
ALTER TABLE knowledge_items ADD COLUMN source_package TEXT DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_knowledge_items_code ON knowledge_items(code);
CREATE INDEX IF NOT EXISTS idx_knowledge_items_type ON knowledge_items(type);
CREATE INDEX IF NOT EXISTS idx_knowledge_items_category ON knowledge_items(category);
CREATE INDEX IF NOT EXISTS idx_knowledge_items_name ON knowledge_items(name);
CREATE INDEX IF NOT EXISTS idx_knowledge_items_import_batch_id ON knowledge_items(import_batch_id);

DROP TABLE IF EXISTS knowledge_fts;
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
  name,
  code,
  alias,
  pinyin,
  category,
  summary,
  content,
  source_note,
  tags,
  detail_text
);
