-- Create messages table for storing synced emails
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id TEXT NOT NULL,
    folder TEXT NOT NULL,
    uid INTEGER NOT NULL,
    message_id TEXT,
    
    -- Headers
    subject TEXT,
    from_addr TEXT,
    to_addr TEXT,
    cc TEXT,
    bcc TEXT,
    reply_to TEXT,
    date TEXT,
    
    -- Body content
    body_plain TEXT,
    body_html TEXT,
    
    -- Metadata
    flags TEXT, -- JSON array: ["\\Seen", "\\Flagged"]
    size INTEGER,
    has_attachments BOOLEAN DEFAULT 0,
    
    -- Timestamps
    synced_at TEXT NOT NULL DEFAULT (datetime('now')),
    internal_date TEXT,
    
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE(account_id, folder, uid)
);

-- Indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_messages_account_folder ON messages(account_id, folder);
CREATE INDEX IF NOT EXISTS idx_messages_from ON messages(from_addr);
CREATE INDEX IF NOT EXISTS idx_messages_subject ON messages(subject);

-- Create attachments table
CREATE TABLE IF NOT EXISTS attachments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id INTEGER NOT NULL,
    
    -- Attachment info
    filename TEXT,
    content_type TEXT,
    size INTEGER,
    content_id TEXT,
    
    -- Storage
    is_inline BOOLEAN DEFAULT 0,
    data BLOB, -- Store small attachments, or NULL for large ones
    file_path TEXT, -- Path to large attachment on disk
    
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_attachments_message_id ON attachments(message_id);

-- Full-text search table for messages
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    subject,
    from_addr,
    to_addr,
    body_plain,
    content='messages',
    content_rowid='id'
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, subject, from_addr, to_addr, body_plain)
    VALUES (new.id, new.subject, new.from_addr, new.to_addr, new.body_plain);
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
    DELETE FROM messages_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
    UPDATE messages_fts SET
        subject = new.subject,
        from_addr = new.from_addr,
        to_addr = new.to_addr,
        body_plain = new.body_plain
    WHERE rowid = new.id;
END;
