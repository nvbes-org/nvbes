-- Enforce enterprise authorization constraints and privileged access grants.
ALTER TABLE tenant_policies
    ADD COLUMN IF NOT EXISTS authorization_constraints JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE organization_policies
    ADD COLUMN IF NOT EXISTS authorization_constraints JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE workspace_policies
    ADD COLUMN IF NOT EXISTS authorization_constraints JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE tenant_policies
    ADD CONSTRAINT tenant_authorization_constraints_object
    CHECK (jsonb_typeof(authorization_constraints) = 'object');

ALTER TABLE organization_policies
    ADD CONSTRAINT organization_authorization_constraints_object
    CHECK (jsonb_typeof(authorization_constraints) = 'object');

ALTER TABLE workspace_policies
    ADD CONSTRAINT workspace_authorization_constraints_object
    CHECK (jsonb_typeof(authorization_constraints) = 'object');

CREATE TABLE privileged_access_grants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    granted_by UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    role identity_role NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'expired', 'revoked')),
    break_glass BOOLEAN NOT NULL DEFAULT FALSE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    revoked_by UUID REFERENCES principals(id) ON DELETE SET NULL,
    revocation_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (expires_at > created_at),
    CHECK (
        (status = 'revoked' AND revoked_at IS NOT NULL AND revocation_reason IS NOT NULL)
        OR (status <> 'revoked' AND revoked_at IS NULL)
    )
);

CREATE UNIQUE INDEX privileged_access_grants_one_active_role
    ON privileged_access_grants (tenant_id, principal_id, role)
    WHERE status = 'active' AND revoked_at IS NULL;

CREATE INDEX privileged_access_grants_expiry
    ON privileged_access_grants (expires_at)
    WHERE status = 'active';

CREATE TABLE privileged_action_approvals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    requested_by UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    approved_by UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
    reason TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (requested_by <> approved_by),
    CHECK (expires_at > created_at)
);

CREATE UNIQUE INDEX privileged_action_approval_active
    ON privileged_action_approvals (tenant_id, action, resource, requested_by)
    WHERE consumed_at IS NULL;

CREATE INDEX privileged_action_approval_expiry
    ON privileged_action_approvals (expires_at)
    WHERE consumed_at IS NULL;

CREATE UNIQUE INDEX tenant_domains_verified_global
    ON tenant_domains (lower(domain))
    WHERE verified_at IS NOT NULL;

ALTER TABLE service_accounts
    ADD COLUMN IF NOT EXISTS credential_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS mtls_certificate_sha256 TEXT,
    ADD COLUMN IF NOT EXISTS workload_identity_uri TEXT,
    ADD COLUMN IF NOT EXISTS disabled_at TIMESTAMPTZ;

ALTER TABLE service_accounts
    ADD CONSTRAINT service_account_short_lived_credential
    CHECK (
        credential_expires_at IS NULL
        OR last_rotated_at IS NULL
        OR credential_expires_at <= last_rotated_at + INTERVAL '90 days'
    );

CREATE UNIQUE INDEX service_accounts_workload_identity
    ON service_accounts (tenant_id, workload_identity_uri)
    WHERE workload_identity_uri IS NOT NULL AND disabled_at IS NULL;

CREATE UNIQUE INDEX service_accounts_mtls_certificate
    ON service_accounts (tenant_id, mtls_certificate_sha256)
    WHERE mtls_certificate_sha256 IS NOT NULL AND disabled_at IS NULL;

ALTER TABLE scim_provisioning_connectors
    ADD COLUMN IF NOT EXISTS credential_expires_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_rotated_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_sync_at TIMESTAMPTZ;

CREATE TABLE shared_security_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    subject_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    event_payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMPTZ,
    delivery_attempts INTEGER NOT NULL DEFAULT 0,
    CHECK (jsonb_typeof(event_payload) = 'object')
);

CREATE INDEX shared_security_events_delivery
    ON shared_security_events (occurred_at)
    WHERE delivered_at IS NULL;

ALTER TABLE privileged_access_grants ENABLE ROW LEVEL SECURITY;
ALTER TABLE privileged_action_approvals ENABLE ROW LEVEL SECURITY;
ALTER TABLE shared_security_events ENABLE ROW LEVEL SECURITY;

CREATE POLICY privileged_access_grant_isolation ON privileged_access_grants
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY privileged_action_approval_isolation ON privileged_action_approvals
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY shared_security_event_isolation ON shared_security_events
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid)
    WITH CHECK (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
