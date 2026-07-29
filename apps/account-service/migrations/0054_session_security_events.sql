-- Add session risk confirmation and outbound security-event delivery.
ALTER TABLE user_sessions
    ADD COLUMN IF NOT EXISTS risk_confirmed_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS risk_confirmed_score DOUBLE PRECISION;

ALTER TABLE oauth_clients
    ADD COLUMN IF NOT EXISTS backchannel_logout_uri TEXT,
    ADD COLUMN IF NOT EXISTS backchannel_logout_session_required BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS security_event_receiver_uri TEXT;

ALTER TABLE oauth_clients
    ADD CONSTRAINT oauth_clients_backchannel_logout_uri_no_fragment
    CHECK (backchannel_logout_uri IS NULL OR position('#' IN backchannel_logout_uri) = 0),
    ADD CONSTRAINT oauth_clients_security_event_receiver_uri_no_fragment
    CHECK (security_event_receiver_uri IS NULL OR position('#' IN security_event_receiver_uri) = 0);

CREATE TABLE security_event_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    oauth_client_id UUID NOT NULL REFERENCES oauth_clients(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    session_id UUID,
    event_kind TEXT NOT NULL,
    endpoint_uri TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (oauth_client_id, session_id, event_kind),
    CONSTRAINT security_event_deliveries_kind
        CHECK (
            event_kind IN (
                'oidc_backchannel_logout',
                'caep_session_revoked',
                'risc_credential_compromise'
            )
        )
);

CREATE INDEX security_event_deliveries_pending
    ON security_event_deliveries (next_attempt_at, created_at)
    WHERE delivered_at IS NULL;
