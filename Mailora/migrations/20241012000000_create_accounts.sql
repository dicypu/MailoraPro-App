-- External email accounts (Gmail, Outlook, Yahoo, etc.)
CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL CHECK(provider IN ('gmail', 'outlook', 'yahoo', 'icloud', 'custom')),
    display_name TEXT,
    imap_host TEXT NOT NULL,
    imap_port INTEGER NOT NULL,
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL,
    -- Credentials (encrypted in production, base64 for now)
    credentials_encrypted TEXT NOT NULL,
    -- Sync settings
    enabled BOOLEAN NOT NULL DEFAULT 1,
    sync_frequency_secs INTEGER DEFAULT 300,
    last_sync_ts INTEGER,
    -- Metadata
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_accounts_email ON accounts(email);
CREATE INDEX IF NOT EXISTS idx_accounts_provider ON accounts(provider);
CREATE INDEX IF NOT EXISTS idx_accounts_enabled ON accounts(enabled);
