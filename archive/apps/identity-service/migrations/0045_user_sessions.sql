CREATE TABLE IF NOT EXISTS user_sessions (
    session_id UUID PRIMARY KEY,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    browser_session_token_hash TEXT,
    tenant_id UUID,
    organization_id UUID,
    workspace_id UUID,
    workspace_region TEXT,
    client_id TEXT,
    acr TEXT,
    amr JSONB NOT NULL DEFAULT '[]'::jsonb,
    auth_time TIMESTAMPTZ,
    idle_timeout_seconds BIGINT,
    idle_expires_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,
    step_up_verified_at TIMESTAMPTZ,
    step_up_expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    ip TEXT,
    user_agent TEXT,
    cookie_theft_risk_score DOUBLE PRECISION,
    cookie_theft_detected_at TIMESTAMPTZ,
    account_device_id UUID,
    device_trust_level TEXT,
    device_trust_score SMALLINT,
    risk_score DOUBLE PRECISION,
    risk_decision TEXT,
    activity_window_started_at TIMESTAMPTZ,
    activity_request_count INTEGER NOT NULL DEFAULT 0,
    last_activity_risk_event_at TIMESTAMPTZ,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_sessions_principal
    ON user_sessions (principal_id, expires_at DESC)
    WHERE revoked_at IS NULL;

CREATE INDEX idx_user_sessions_lookup
    ON user_sessions (session_id)
    WHERE revoked_at IS NULL;
