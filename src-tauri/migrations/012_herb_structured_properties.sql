ALTER TABLE herb_details ADD COLUMN four_qi TEXT;
ALTER TABLE herb_details ADD COLUMN five_flavors TEXT;
ALTER TABLE herb_details ADD COLUMN channel_tropism TEXT;
ALTER TABLE herb_details ADD COLUMN toxicity TEXT;
ALTER TABLE herb_details ADD COLUMN property_notes TEXT;

CREATE INDEX IF NOT EXISTS idx_herb_details_four_qi ON herb_details(four_qi);
CREATE INDEX IF NOT EXISTS idx_herb_details_channel_tropism ON herb_details(channel_tropism);
