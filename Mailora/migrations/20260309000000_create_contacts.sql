-- Contacts: Ana tablo
CREATE TABLE contacts (
    id          TEXT PRIMARY KEY,
    account_id  TEXT NOT NULL,
    vcard_uid   TEXT,
    etag        TEXT,
    href        TEXT,
    full_name   TEXT NOT NULL,
    first_name  TEXT,
    last_name   TEXT,
    middle_name TEXT,
    prefix      TEXT,
    suffix      TEXT,
    company     TEXT,
    department  TEXT,
    title       TEXT,
    note        TEXT,
    birthday    TEXT,
    photo_data  TEXT,
    website_url TEXT,
    gender      TEXT,
    language    TEXT,
    timezone    TEXT,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    sync_status TEXT NOT NULL DEFAULT 'local',
    raw_vcard   TEXT,
    synced_at   TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);
CREATE INDEX idx_contacts_account ON contacts(account_id);
CREATE INDEX idx_contacts_name ON contacts(last_name, first_name);

-- E-postalar (bire-çok)
CREATE TABLE contact_emails (
    id          TEXT PRIMARY KEY,
    contact_id  TEXT NOT NULL,
    email       TEXT NOT NULL,
    label       TEXT NOT NULL DEFAULT 'other',
    is_primary  INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
);
CREATE INDEX idx_contact_emails_email ON contact_emails(email);
CREATE INDEX idx_contact_emails_contact ON contact_emails(contact_id);

-- Telefon numaraları (bire-çok)
CREATE TABLE contact_phones (
    id          TEXT PRIMARY KEY,
    contact_id  TEXT NOT NULL,
    phone       TEXT NOT NULL,
    label       TEXT NOT NULL DEFAULT 'other',
    is_primary  INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
);
CREATE INDEX idx_contact_phones_contact ON contact_phones(contact_id);

-- Adresler (bire-çok)
CREATE TABLE contact_addresses (
    id          TEXT PRIMARY KEY,
    contact_id  TEXT NOT NULL,
    label       TEXT NOT NULL DEFAULT 'other',
    street      TEXT,
    city        TEXT,
    region      TEXT,
    postal_code TEXT,
    country     TEXT,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
);

-- Sosyal medya linkleri
CREATE TABLE contact_social (
    id          TEXT PRIMARY KEY,
    contact_id  TEXT NOT NULL,
    service     TEXT NOT NULL,
    url         TEXT NOT NULL,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
);

-- Kişi grupları
CREATE TABLE contact_groups (
    id          TEXT PRIMARY KEY,
    account_id  TEXT NOT NULL,
    name        TEXT NOT NULL,
    color       TEXT,
    vcard_kind  TEXT,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE,
    UNIQUE (account_id, name)
);

-- Kişi ↔ Grup ilişkisi
CREATE TABLE contact_group_members (
    contact_id  TEXT NOT NULL,
    group_id    TEXT NOT NULL,
    PRIMARY KEY (contact_id, group_id),
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE,
    FOREIGN KEY (group_id) REFERENCES contact_groups(id) ON DELETE CASCADE
);

-- CardDAV sync durumu
CREATE TABLE carddav_sync_state (
    account_id          TEXT NOT NULL,
    addressbook_url     TEXT NOT NULL,
    display_name        TEXT,
    sync_token          TEXT,
    ctag                TEXT,
    last_synced_at      TEXT,
    PRIMARY KEY (account_id, addressbook_url)
);

-- Conflict log
CREATE TABLE contact_conflicts (
    id              TEXT PRIMARY KEY,
    contact_id      TEXT NOT NULL,
    local_data      TEXT NOT NULL,
    remote_data     TEXT NOT NULL,
    detected_at     TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at     TEXT,
    resolution      TEXT
);

-- FTS5 tam metin arama
CREATE VIRTUAL TABLE contacts_fts USING fts5(
    contact_id UNINDEXED,
    full_name,
    company,
    note,
    content='contacts',
    content_rowid='rowid',
    tokenize='unicode61'
);

-- FTS trigger'ları
CREATE TRIGGER contacts_ai AFTER INSERT ON contacts BEGIN
    INSERT INTO contacts_fts(rowid, contact_id, full_name, company, note)
    VALUES (new.rowid, new.id, new.full_name, new.company, new.note);
END;
CREATE TRIGGER contacts_ad AFTER DELETE ON contacts BEGIN
    INSERT INTO contacts_fts(contacts_fts, rowid, contact_id, full_name, company, note)
    VALUES ('delete', old.rowid, old.id, old.full_name, old.company, old.note);
END;
CREATE TRIGGER contacts_au AFTER UPDATE ON contacts BEGIN
    INSERT INTO contacts_fts(contacts_fts, rowid, contact_id, full_name, company, note)
    VALUES ('delete', old.rowid, old.id, old.full_name, old.company, old.note);
    INSERT INTO contacts_fts(rowid, contact_id, full_name, company, note)
    VALUES (new.rowid, new.id, new.full_name, new.company, new.note);
END;
