-- Add append policy and sent folder hint to accounts
ALTER TABLE accounts ADD COLUMN append_policy TEXT DEFAULT 'auto';
ALTER TABLE accounts ADD COLUMN sent_folder_hint TEXT;

-- Message bodies cache table
CREATE TABLE IF NOT EXISTS message_bodies (
    account_id TEXT NOT NULL,
    folder TEXT NOT NULL,
    uid INTEGER NOT NULL,
    body TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    PRIMARY KEY (account_id, folder, uid)
);

-- Simple metrics snapshot table (optional future use)
CREATE TABLE IF NOT EXISTS metrics_snapshots (
    ts INTEGER PRIMARY KEY,
    emails_sent INTEGER,
    finalize_success INTEGER,
    finalize_pending INTEGER
);
