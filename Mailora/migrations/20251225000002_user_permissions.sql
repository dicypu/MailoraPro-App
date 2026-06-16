-- Link users to accounts for granular visibility
CREATE TABLE IF NOT EXISTS user_accounts (
    user_id INTEGER NOT NULL,
    account_id TEXT NOT NULL,
    PRIMARY KEY (user_id, account_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Ensure event_logs is ready (previously defined in create_rbac_tables but making sure)
CREATE TABLE IF NOT EXISTS event_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER,
    account_id TEXT,
    action TEXT NOT NULL,
    details TEXT,
    ip_address TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for performance
CREATE INDEX IF NOT EXISTS idx_user_accounts_uid ON user_accounts(user_id);
CREATE INDEX IF NOT EXISTS idx_event_logs_uid ON event_logs(user_id);
