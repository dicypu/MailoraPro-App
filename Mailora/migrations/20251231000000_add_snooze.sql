ALTER TABLE messages ADD COLUMN snoozed_until DATETIME;
CREATE INDEX idx_messages_snoozed_until ON messages(snoozed_until);
