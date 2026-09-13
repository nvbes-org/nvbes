CREATE TABLE IF NOT EXISTS rate_limit_windows (
    action TEXT NOT NULL,
    key_hash BYTEA NOT NULL CHECK (octet_length(key_hash) = 32),
    hits BIGINT NOT NULL CHECK (hits > 0),
    expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (action, key_hash)
);
CREATE INDEX IF NOT EXISTS rate_limit_windows_expiry ON rate_limit_windows (expires_at);
