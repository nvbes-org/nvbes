CREATE TABLE IF NOT EXISTS tenant_break_glass_accounts (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
    procedure_reference TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    revoked_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
    revoked_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    PRIMARY KEY (tenant_id, principal_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_tenant_break_glass_active_principal
    ON tenant_break_glass_accounts (tenant_id, principal_id)
    WHERE revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_tenant_break_glass_active_tenant
    ON tenant_break_glass_accounts (tenant_id, created_at DESC)
    WHERE revoked_at IS NULL;

ALTER TABLE tenant_break_glass_accounts ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_break_glass_account_isolation ON tenant_break_glass_accounts;
CREATE POLICY tenant_break_glass_account_isolation ON tenant_break_glass_accounts
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
