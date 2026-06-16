-- Align messages schema to current code expectations
-- Add has_attachments if missing
ALTER TABLE messages ADD COLUMN has_attachments BOOLEAN DEFAULT 0;

-- Add convenient index for listing by date per account/folder
CREATE INDEX IF NOT EXISTS idx_messages_acc_folder_date ON messages(account_id, folder, date DESC);

-- Optional: flags index for unread filtering (keeps LIKE scans cheaper)
CREATE INDEX IF NOT EXISTS idx_messages_flags ON messages(flags);
