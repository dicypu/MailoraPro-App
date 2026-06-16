-- Add OAuth2 fields to accounts table
ALTER TABLE accounts ADD COLUMN auth_method TEXT DEFAULT 'password' CHECK(auth_method IN ('password', 'oauth2'));
ALTER TABLE accounts ADD COLUMN oauth_access_token TEXT;
ALTER TABLE accounts ADD COLUMN oauth_refresh_token TEXT;
ALTER TABLE accounts ADD COLUMN oauth_expires_at INTEGER;
ALTER TABLE accounts ADD COLUMN oauth_token_type TEXT DEFAULT 'Bearer';

-- Index for faster OAuth token lookups
CREATE INDEX IF NOT EXISTS idx_accounts_oauth ON accounts(oauth_access_token) WHERE auth_method = 'oauth2';
