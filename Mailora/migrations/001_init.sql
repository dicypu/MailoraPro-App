CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'Member',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    email TEXT NOT NULL,
    display_name TEXT,
    color TEXT DEFAULT '#3b82f6',
    imap_host TEXT NOT NULL,
    imap_port INTEGER NOT NULL DEFAULT 993,
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL DEFAULT 587,
    password_enc TEXT NOT NULL,
    enabled BOOLEAN DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id),
    uid INTEGER,
    folder TEXT NOT NULL DEFAULT 'Inbox',
    from_addr TEXT,
    subject TEXT,
    preview TEXT,
    body TEXT,
    date DATETIME,
    read BOOLEAN DEFAULT 0,
    pinned BOOLEAN DEFAULT 0,
    snoozed BOOLEAN DEFAULT 0,
    snooze_until DATETIME,
    important BOOLEAN DEFAULT 0,
    has_attachment BOOLEAN DEFAULT 0,
    is_newsletter BOOLEAN DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id),
    filename TEXT NOT NULL,
    mime_type TEXT,
    size_bytes INTEGER,
    content BLOB,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
