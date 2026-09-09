CREATE TABLE identity_webauthn_credentials (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    credential_id BYTEA NOT NULL UNIQUE CHECK (octet_length(credential_id) BETWEEN 16 AND 1024),
    public_key JSONB NOT NULL CHECK (jsonb_typeof(public_key) = 'object'),
    sign_count BIGINT NOT NULL DEFAULT 0 CHECK (sign_count >= 0),
    backed_up BOOLEAN NOT NULL DEFAULT FALSE,
    discoverable BOOLEAN NOT NULL DEFAULT TRUE,
    label TEXT NOT NULL CHECK (char_length(label) BETWEEN 1 AND 128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX identity_webauthn_principal_idx
    ON identity_webauthn_credentials (principal_id, created_at);

CREATE TABLE identity_webauthn_challenges (
    id UUID PRIMARY KEY,
    principal_id UUID REFERENCES identity_principals(id) ON DELETE CASCADE,
    challenge BYTEA NOT NULL UNIQUE CHECK (octet_length(challenge) = 32),
    purpose TEXT NOT NULL CHECK (purpose IN ('registration', 'authentication', 'step_up')),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX identity_webauthn_challenges_expiry_idx
    ON identity_webauthn_challenges (expires_at)
    WHERE consumed_at IS NULL;
