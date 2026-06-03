ALTER TABLE ai_provider_settings ADD COLUMN max_context_items INTEGER DEFAULT 6;
ALTER TABLE ai_provider_settings ADD COLUMN max_context_chars INTEGER DEFAULT 6000;
ALTER TABLE ai_provider_settings ADD COLUMN only_use_local_context INTEGER NOT NULL DEFAULT 1;
ALTER TABLE ai_provider_settings ADD COLUMN safety_mode TEXT NOT NULL DEFAULT 'strict';
