ALTER TABLE data_import_batches ADD COLUMN confirmed_item_ids_json TEXT;
ALTER TABLE data_import_batches ADD COLUMN quality_report_json TEXT;
ALTER TABLE data_import_batches ADD COLUMN search_terms_imported_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE data_import_batches ADD COLUMN rolled_back_at TEXT;

CREATE INDEX IF NOT EXISTS idx_import_batches_status ON data_import_batches(status);
