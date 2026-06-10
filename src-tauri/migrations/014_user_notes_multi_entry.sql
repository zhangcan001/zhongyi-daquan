CREATE TABLE IF NOT EXISTS user_notes_v2 (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL,
  note_text TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

INSERT INTO user_notes_v2 (id, item_id, note_text, created_at, updated_at)
SELECT id, item_id, note_text, created_at, updated_at
FROM user_notes;

DROP TABLE user_notes;

ALTER TABLE user_notes_v2 RENAME TO user_notes;

CREATE INDEX IF NOT EXISTS idx_user_notes_item_id ON user_notes(item_id);
CREATE INDEX IF NOT EXISTS idx_user_notes_updated_at ON user_notes(updated_at);
