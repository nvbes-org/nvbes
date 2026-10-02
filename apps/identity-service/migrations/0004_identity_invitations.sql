CREATE TABLE identity_invitations (
    id UUID PRIMARY KEY,
    invited_by_principal_id UUID NOT NULL REFERENCES identity_principals(id),
    email_hash BYTEA NOT NULL CHECK (octet_length(email_hash) = 32),
    code_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(code_hash) = 32),
    state TEXT NOT NULL CHECK (state IN ('pending', 'accepted', 'revoked', 'expired')),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    accepted_at TIMESTAMPTZ,
    accepted_principal_id UUID REFERENCES identity_principals(id),
    CHECK (expires_at > created_at),
    CHECK ((state = 'accepted') = (accepted_at IS NOT NULL AND accepted_principal_id IS NOT NULL))
);

CREATE UNIQUE INDEX identity_invitations_pending_email_idx
    ON identity_invitations(email_hash) WHERE state = 'pending';
CREATE INDEX identity_invitations_inviter_time_idx
    ON identity_invitations(invited_by_principal_id, created_at DESC);
