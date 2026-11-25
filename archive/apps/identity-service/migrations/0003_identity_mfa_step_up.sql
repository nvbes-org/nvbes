CREATE TABLE identity_auth_factors (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('totp')),
    state TEXT NOT NULL CHECK (state IN ('pending', 'active', 'revoked')),
    secret_ciphertext BYTEA NOT NULL,
    secret_nonce BYTEA NOT NULL CHECK (octet_length(secret_nonce) = 12),
    key_version SMALLINT NOT NULL CHECK (key_version > 0),
    last_accepted_counter BIGINT CHECK (last_accepted_counter >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    revoked_at TIMESTAMPTZ,
    UNIQUE (principal_id, kind)
);

ALTER TABLE identity_sessions
    ADD COLUMN step_up_expires_at TIMESTAMPTZ;
