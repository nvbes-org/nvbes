-- Short-lived protocol state lives in the existing Identity database.
CREATE TABLE identity_oauth_requests (
    handle_hash BYTEA PRIMARY KEY CHECK (octet_length(handle_hash) = 32),
    kind TEXT NOT NULL CHECK (kind IN ('authorization', 'par')),
    client_id TEXT NOT NULL CHECK (length(client_id) BETWEEN 3 AND 128),
    parameters JSONB NOT NULL CHECK (jsonb_typeof(parameters) = 'object'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp() + interval '5 minutes',
    CHECK (expires_at > created_at)
);
CREATE INDEX identity_oauth_requests_expiry_idx ON identity_oauth_requests(expires_at);

CREATE TABLE identity_oauth_codes (
    code_hash BYTEA PRIMARY KEY CHECK (octet_length(code_hash) = 32),
    session_id UUID NOT NULL REFERENCES identity_sessions(id),
    principal_id UUID NOT NULL REFERENCES identity_principals(id),
    client_id TEXT NOT NULL,
    parameters JSONB NOT NULL CHECK (jsonb_typeof(parameters) = 'object'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp() + interval '60 seconds',
    consumed_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);
CREATE INDEX identity_oauth_codes_expiry_idx ON identity_oauth_codes(expires_at);

-- A grant binds later access/refresh tokens to one authorization and session.
CREATE TABLE identity_oauth_grants (
    id UUID PRIMARY KEY,
    code_hash BYTEA NOT NULL UNIQUE REFERENCES identity_oauth_codes(code_hash),
    session_id UUID NOT NULL REFERENCES identity_sessions(id),
    principal_id UUID NOT NULL REFERENCES identity_principals(id),
    client_id TEXT NOT NULL,
    parameters JSONB NOT NULL CHECK (jsonb_typeof(parameters) = 'object'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    revoked_at TIMESTAMPTZ
);
CREATE INDEX identity_oauth_grants_active_session_idx
    ON identity_oauth_grants(session_id) WHERE revoked_at IS NULL;
