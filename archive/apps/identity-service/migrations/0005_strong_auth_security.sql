-- Strong authentication and brute-force controls.

ALTER TABLE tenants
    ADD COLUMN IF NOT EXISTS mfa_policy TEXT NOT NULL DEFAULT 'optional'
        CHECK (mfa_policy IN ('optional', 'required_admins', 'required_all'));

ALTER TABLE workspace_policies
    ADD COLUMN IF NOT EXISTS mfa_policy TEXT NOT NULL DEFAULT 'optional'
        CHECK (mfa_policy IN ('optional', 'required_admins', 'required_all'));

CREATE INDEX IF NOT EXISTS idx_risk_events_ip_created_at
    ON risk_events (ip_address, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_risk_events_principal_type_created_at
    ON risk_events (principal_id, event_type, created_at DESC);
