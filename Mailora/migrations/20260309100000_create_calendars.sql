-- MIGRATION: 20260309100000_create_calendars.sql
-- v1.7 Calendar (CalDAV) tables

-- 1. Takvimler (Calendars)
CREATE TABLE calendars (
    id              TEXT PRIMARY KEY,
    account_id      TEXT NOT NULL,
    url             TEXT NOT NULL,
    display_name    TEXT,
    color           TEXT,
    description     TEXT,
    ctag            TEXT,
    sync_token      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE(account_id, url)
);
CREATE INDEX idx_calendars_account ON calendars(account_id);

-- 2. Etkinlikler (Events)
CREATE TABLE calendar_events (
    id              TEXT PRIMARY KEY,
    calendar_id     TEXT NOT NULL,
    uid             TEXT NOT NULL, -- iCalendar UID
    href            TEXT NOT NULL, -- URL to the .ics resource on server
    etag            TEXT,
    raw_ical        TEXT NOT NULL, -- The complete raw iCalendar data
    
    summary         TEXT,
    description     TEXT,
    location        TEXT,
    
    dtstart         TEXT, -- Stored as UTC ISO8601 string or Date (e.g., "2026-03-15T10:00:00Z" or "2026-03-15")
    dtend           TEXT,
    is_all_day      INTEGER NOT NULL DEFAULT 0,
    timezone        TEXT, -- Optional exact timezone if not UTC
    
    rrule           TEXT, -- Repetition rule
    status          TEXT, -- TENTATIVE, CONFIRMED, CANCELLED
    
    sync_status     TEXT NOT NULL DEFAULT 'local', -- 'local', 'synced', 'conflict', 'pending_delete', 'pending_update', 'pending_create'
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE,
    UNIQUE(calendar_id, href)
);
CREATE INDEX idx_calendar_events_calendar ON calendar_events(calendar_id);
CREATE INDEX idx_calendar_events_dates ON calendar_events(dtstart, dtend);

-- 3. Katılımcılar (Attendees)
CREATE TABLE event_attendees (
    id              TEXT PRIMARY KEY,
    event_id        TEXT NOT NULL,
    email           TEXT NOT NULL,
    cn              TEXT, -- Common Name
    partstat        TEXT NOT NULL DEFAULT 'NEEDS-ACTION', -- ACCEPTED, DECLINED, TENTATIVE, DELEGATED
    is_organizer    INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (event_id) REFERENCES calendar_events(id) ON DELETE CASCADE
);
CREATE INDEX idx_event_attendees_event ON event_attendees(event_id);

-- 4. Alarmlar (Alarms/Reminders)
CREATE TABLE event_alarms (
    id              TEXT PRIMARY KEY,
    event_id        TEXT NOT NULL,
    action          TEXT NOT NULL DEFAULT 'DISPLAY', -- DISPLAY, AUDIO, EMAIL
    trigger_text    TEXT NOT NULL, -- e.g., "-PT15M" for 15 minutes before
    description     TEXT,
    FOREIGN KEY (event_id) REFERENCES calendar_events(id) ON DELETE CASCADE
);

-- 5. CalDAV Senkronizasyon Durumu 
CREATE TABLE caldav_sync_state (
    account_id      TEXT PRIMARY KEY,
    last_synced_at  TEXT,
    error_msg       TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

-- Full Text Search Setup for Events
CREATE VIRTUAL TABLE calendar_events_fts USING fts5(
    event_id UNINDEXED,
    summary,
    description,
    location,
    content='calendar_events',
    content_rowid='rowid',
    tokenize='unicode61'
);

CREATE TRIGGER events_ai AFTER INSERT ON calendar_events BEGIN
    INSERT INTO calendar_events_fts(rowid, event_id, summary, description, location)
    VALUES (new.rowid, new.id, new.summary, new.description, new.location);
END;

CREATE TRIGGER events_ad AFTER DELETE ON calendar_events BEGIN
    INSERT INTO calendar_events_fts(calendar_events_fts, rowid, event_id, summary, description, location)
    VALUES ('delete', old.rowid, old.id, old.summary, old.description, old.location);
END;

CREATE TRIGGER events_au AFTER UPDATE ON calendar_events BEGIN
    INSERT INTO calendar_events_fts(calendar_events_fts, rowid, event_id, summary, description, location)
    VALUES ('delete', old.rowid, old.id, old.summary, old.description, old.location);
    INSERT INTO calendar_events_fts(rowid, event_id, summary, description, location)
    VALUES (new.rowid, new.id, new.summary, new.description, new.location);
END;
