CREATE TABLE IF NOT EXISTS recent_views (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL UNIQUE,
  item_name TEXT NOT NULL,
  item_type TEXT NOT NULL,
  category TEXT,
  viewed_at TEXT NOT NULL,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_favorites (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL UNIQUE,
  item_name TEXT NOT NULL,
  item_type TEXT NOT NULL,
  category TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS user_notes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  item_id INTEGER NOT NULL UNIQUE,
  note_text TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(item_id) REFERENCES knowledge_items(id) ON DELETE CASCADE
);

INSERT OR IGNORE INTO user_favorites (item_id, item_name, item_type, category, created_at)
SELECT id, name, type, category, datetime('now')
FROM knowledge_items
WHERE is_favorite = 1;

CREATE INDEX IF NOT EXISTS idx_recent_views_viewed_at ON recent_views(viewed_at);
CREATE INDEX IF NOT EXISTS idx_user_favorites_created_at ON user_favorites(created_at);
CREATE INDEX IF NOT EXISTS idx_user_notes_updated_at ON user_notes(updated_at);
