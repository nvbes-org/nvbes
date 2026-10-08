CREATE TABLE identity_mfa_recovery_codes (
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    code_hash BYTEA NOT NULL CHECK (octet_length(code_hash)=32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    consumed_at TIMESTAMPTZ,
    PRIMARY KEY(principal_id, code_hash)
);

-- This token is never stored in identity_sessions and cannot authorize OAuth.
CREATE TABLE identity_mfa_recovery_sessions (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL UNIQUE REFERENCES identity_principals(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash)=32),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

ALTER TABLE identity_webauthn_challenges
    ADD COLUMN recovery_session_id UUID REFERENCES identity_mfa_recovery_sessions(id) ON DELETE CASCADE;
CREATE UNIQUE INDEX identity_webauthn_recovery_registration_idx
    ON identity_webauthn_challenges(recovery_session_id)
    WHERE recovery_session_id IS NOT NULL;
