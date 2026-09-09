-- Refresh secrets are opaque, single-use and bound to one authorization grant.
CREATE TABLE identity_oauth_refresh_tokens (
    token_hash BYTEA PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    family_id UUID NOT NULL,
    grant_id UUID NOT NULL REFERENCES identity_oauth_grants(id),
    principal_id UUID NOT NULL REFERENCES identity_principals(id),
    client_id TEXT NOT NULL CHECK (length(client_id) BETWEEN 3 AND 128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    rotated_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ
);
CREATE INDEX identity_oauth_refresh_family_idx
    ON identity_oauth_refresh_tokens(family_id);
CREATE INDEX identity_oauth_refresh_grant_idx
    ON identity_oauth_refresh_tokens(grant_id);
