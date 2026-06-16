-- Reconciliation for the `users` table.
-- Previous migration 20241002120000_create_users.sql used 'email'.
-- New RBAC logic expects 'username' and 'role'.

-- Use a block that tries to rename the column if it exists.
-- SQLite doesn't have an easy "IF COLUMN EXISTS", so we rely on the migration runner's tolerance
-- or we use a pragmatic approach.

-- Rename email to username (this will fail gracefully if email doesn't exist due to our runner's logic)
ALTER TABLE users RENAME COLUMN email TO username;

-- Add role column (will fail gracefully if already exists)
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'Member';
