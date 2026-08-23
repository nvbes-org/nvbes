CREATE TABLE registration_enrollment_tokens (
    token_hash TEXT PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    CONSTRAINT registration_enrollment_tokens_expiry_check CHECK (expires_at > created_at)
);

CREATE INDEX registration_enrollment_tokens_principal_active_idx
    ON registration_enrollment_tokens (principal_id, expires_at)
    WHERE consumed_at IS NULL;
