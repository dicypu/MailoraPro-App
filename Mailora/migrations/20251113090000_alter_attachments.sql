-- Add missing columns to support content id, inline flag and optional storage
-- Note: SQLite doesn't support IF NOT EXISTS for ADD COLUMN. Duplicate column errors are benign in our runner.
ALTER TABLE attachments ADD COLUMN content_id TEXT;
ALTER TABLE attachments ADD COLUMN is_inline INTEGER DEFAULT 0;
ALTER TABLE attachments ADD COLUMN data BLOB;
ALTER TABLE attachments ADD COLUMN file_path TEXT;

-- Helpful index
CREATE INDEX IF NOT EXISTS idx_attachments_message_id ON attachments(message_id);
