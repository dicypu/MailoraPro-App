-- Add carddav_url column to accounts table for PIM v1.6
ALTER TABLE accounts ADD COLUMN carddav_url TEXT;
ALTER TABLE accounts ADD COLUMN caldav_url TEXT;
