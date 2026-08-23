-- Machine-to-machine OAuth clients

ALTER TABLE oauth_clients
    ADD COLUMN IF NOT EXISTS last_used_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_oauth_clients_last_used_at
    ON oauth_clients (last_used_at DESC)
    WHERE last_used_at IS NOT NULL;
