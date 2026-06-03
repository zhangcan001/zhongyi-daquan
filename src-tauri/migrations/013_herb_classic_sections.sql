ALTER TABLE herb_details ADD COLUMN origin TEXT;
ALTER TABLE herb_details ADD COLUMN processing TEXT;
ALTER TABLE herb_details ADD COLUMN classic_applications TEXT;

CREATE INDEX IF NOT EXISTS idx_herb_details_origin ON herb_details(origin);
