ALTER TABLE duplicate_candidates ADD COLUMN duplicate_item_id INTEGER REFERENCES knowledge_items(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_duplicate_duplicate_item ON duplicate_candidates(duplicate_item_id);
CREATE INDEX IF NOT EXISTS idx_fingerprints_type_code ON knowledge_fingerprints(type, code_norm);
CREATE INDEX IF NOT EXISTS idx_fingerprints_type_pinyin_category ON knowledge_fingerprints(type, pinyin_norm);
CREATE INDEX IF NOT EXISTS idx_relation_suggestions_unique_pending
  ON relation_suggestions(source_item_id, target_item_id, relation_type, status);
