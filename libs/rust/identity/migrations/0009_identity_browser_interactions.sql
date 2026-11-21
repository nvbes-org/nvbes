-- Browser state never shares the OAuth client state/nonce or the login cookie.
ALTER TABLE identity_oauth_requests
    ADD COLUMN browser_hash BYTEA,
    ADD COLUMN csrf_hash BYTEA,
    ADD COLUMN bound_session_id UUID REFERENCES identity_sessions(id),
    ADD CONSTRAINT identity_oauth_browser_binding CHECK (
        (browser_hash IS NULL AND csrf_hash IS NULL AND bound_session_id IS NULL)
        OR (kind = 'authorization' AND octet_length(browser_hash) = 32
            AND browser_hash IS NOT NULL AND octet_length(csrf_hash) = 32
            AND csrf_hash IS NOT NULL)
    );

CREATE TABLE identity_oauth_consents (
    principal_id UUID NOT NULL REFERENCES identity_principals(id),
    client_id TEXT NOT NULL CHECK (length(client_id) BETWEEN 3 AND 128),
    policy_hash BYTEA NOT NULL CHECK (octet_length(policy_hash) = 32),
    resource TEXT NOT NULL,
    audience TEXT NOT NULL,
    scope TEXT NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp() + interval '30 days',
    PRIMARY KEY(principal_id, client_id, policy_hash),
    CHECK (expires_at > granted_at)
);
CREATE INDEX identity_oauth_consents_expiry_idx ON identity_oauth_consents(expires_at);

-- Fixed hash slots cap cardinality even when callers vary identifiers or IPs.
-- Collisions may throttle unrelated callers, but can never bypass a limit.
CREATE TABLE identity_rate_buckets (
    category TEXT NOT NULL CHECK (category IN ('login_account', 'login_source', 'protocol_source')),
    slot INTEGER NOT NULL CHECK (slot BETWEEN 0 AND 4095),
    attempts INTEGER NOT NULL CHECK (attempts BETWEEN 1 AND 120),
    reset_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(category, slot)
);
