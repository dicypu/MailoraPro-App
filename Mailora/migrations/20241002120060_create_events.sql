-- Event log for IN/OUT mail events (RBAC visibility)
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    direction TEXT NOT NULL CHECK(direction IN ('IN','OUT')),
    mailbox TEXT NOT NULL,
    actor TEXT,
    peer TEXT,
    subject TEXT,
    ts INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts DESC);
CREATE INDEX IF NOT EXISTS idx_events_mailbox ON events(mailbox);
