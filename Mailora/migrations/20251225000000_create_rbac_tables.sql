-- Create users table for RBAC
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'Member', -- 'Admin' or 'Member'
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Basic event logs table (if not exists or needs expansion)
CREATE TABLE IF NOT EXISTS event_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER, -- nullable for system events
    account_id TEXT, -- related email account if applicable
    action TEXT NOT NULL, -- 'LOGIN', 'FETCH', 'SEND', etc.
    details TEXT,
    ip_address TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
