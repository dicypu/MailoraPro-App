-- Extend message_bodies cache with metadata columns
ALTER TABLE message_bodies ADD COLUMN html_body TEXT;
ALTER TABLE message_bodies ADD COLUMN subject TEXT;
ALTER TABLE message_bodies ADD COLUMN from_addr TEXT;
ALTER TABLE message_bodies ADD COLUMN date TEXT;
ALTER TABLE message_bodies ADD COLUMN flags TEXT;
