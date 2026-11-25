CREATE TABLE identity_sessions (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);

CREATE INDEX identity_sessions_active_principal_idx
    ON identity_sessions (principal_id, expires_at DESC)
    WHERE revoked_at IS NULL;

CREATE TABLE identity_recovery_challenges (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    CHECK (expires_at > created_at),
    CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX identity_recovery_active_principal_idx
    ON identity_recovery_challenges (principal_id, expires_at DESC)
    WHERE consumed_at IS NULL;
