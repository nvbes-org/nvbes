-- Refresh tokens table for OAuth 2.1
CREATE TABLE IF NOT EXISTS identity_refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    family_id UUID NOT NULL,
    principal_id UUID NOT NULL,
    token_hash BYTEA NOT NULL,
    client_id VARCHAR(255) NOT NULL,
    scope TEXT NOT NULL,
    issued_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    revoked_at TIMESTAMP WITH TIME ZONE,
    last_used_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT fk_refresh_tokens_principal FOREIGN KEY (principal_id) REFERENCES identity_principals(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_hash ON identity_refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family ON identity_refresh_tokens(family_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_principal ON identity_refresh_tokens(principal_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at ON identity_refresh_tokens(expires_at);