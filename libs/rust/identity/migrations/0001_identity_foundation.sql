CREATE TABLE identity_principals (
    id UUID PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('human', 'service_account')),
    status TEXT NOT NULL CHECK (status IN ('pending_verification', 'active', 'suspended', 'closed')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE TABLE identity_login_identifiers (
    id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES identity_principals(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('email', 'oidc_subject', 'webauthn_user_handle')),
    normalized_value TEXT NOT NULL,
    verified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (kind, normalized_value)
);

CREATE TABLE identity_password_credentials (
    principal_id UUID PRIMARY KEY REFERENCES identity_principals(id) ON DELETE CASCADE,
    password_hash TEXT NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    compromised_at TIMESTAMPTZ
);

CREATE TABLE identity_audit_events (
    id UUID PRIMARY KEY,
    principal_id UUID REFERENCES identity_principals(id) ON DELETE SET NULL,
    actor_principal_id UUID REFERENCES identity_principals(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    correlation_id UUID NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    CHECK (jsonb_typeof(details) = 'object')
);

CREATE INDEX identity_audit_events_principal_time_idx
    ON identity_audit_events (principal_id, occurred_at DESC);

CREATE TABLE identity_outbox (
    id UUID PRIMARY KEY,
    event_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    published_at TIMESTAMPTZ,
    attempts SMALLINT NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    CHECK (jsonb_typeof(payload) = 'object')
);

CREATE INDEX identity_outbox_pending_idx
    ON identity_outbox (occurred_at)
    WHERE published_at IS NULL;
