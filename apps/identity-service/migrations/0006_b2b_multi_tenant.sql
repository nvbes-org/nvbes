DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'identity_role') THEN
        CREATE TYPE identity_role AS ENUM (
            'owner',
            'admin',
            'security_admin',
            'billing_admin',
            'member'
        );
    END IF;
END $$;

ALTER TYPE workspace_member_role ADD VALUE IF NOT EXISTS 'security_admin';
ALTER TYPE workspace_member_role ADD VALUE IF NOT EXISTS 'billing_admin';

ALTER TABLE tenant_memberships
    ADD COLUMN IF NOT EXISTS role identity_role NOT NULL DEFAULT 'member';

ALTER TABLE organization_memberships
    ADD COLUMN IF NOT EXISTS role identity_role NOT NULL DEFAULT 'member';

UPDATE tenant_memberships tm
SET role = 'owner'
WHERE role = 'member'
  AND EXISTS (
      SELECT 1
      FROM workspace_memberships wm
      INNER JOIN workspaces w ON w.id = wm.workspace_id
      WHERE w.tenant_id = tm.tenant_id
        AND wm.principal_id = tm.principal_id
        AND wm.status = 'active'
        AND wm.role = 'owner'
  );

UPDATE tenant_memberships tm
SET role = 'admin'
WHERE role = 'member'
  AND EXISTS (
      SELECT 1
      FROM workspace_memberships wm
      INNER JOIN workspaces w ON w.id = wm.workspace_id
      WHERE w.tenant_id = tm.tenant_id
        AND wm.principal_id = tm.principal_id
        AND wm.status = 'active'
        AND wm.role = 'admin'
  );

UPDATE organization_memberships om
SET role = 'owner'
WHERE role = 'member'
  AND EXISTS (
      SELECT 1
      FROM workspace_memberships wm
      INNER JOIN workspaces w ON w.id = wm.workspace_id
      WHERE w.organization_id = om.organization_id
        AND wm.principal_id = om.principal_id
        AND wm.status = 'active'
        AND wm.role = 'owner'
  );

UPDATE organization_memberships om
SET role = 'admin'
WHERE role = 'member'
  AND EXISTS (
      SELECT 1
      FROM workspace_memberships wm
      INNER JOIN workspaces w ON w.id = wm.workspace_id
      WHERE w.organization_id = om.organization_id
        AND wm.principal_id = om.principal_id
        AND wm.status = 'active'
        AND wm.role = 'admin'
  );

ALTER TABLE workspace_invitations
    ADD COLUMN IF NOT EXISTS invited_by UUID REFERENCES principals(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE TABLE IF NOT EXISTS tenant_invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role identity_role NOT NULL DEFAULT 'member',
    status invitation_status NOT NULL DEFAULT 'pending',
    invited_by UUID REFERENCES principals(id) ON DELETE SET NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS organization_invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role identity_role NOT NULL DEFAULT 'member',
    status invitation_status NOT NULL DEFAULT 'pending',
    invited_by UUID REFERENCES principals(id) ON DELETE SET NULL,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenant_invitations_pending_email
    ON tenant_invitations (tenant_id, lower(email))
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_organization_invitations_pending_email
    ON organization_invitations (organization_id, lower(email))
    WHERE status = 'pending';

CREATE TABLE IF NOT EXISTS system_policies (
    id BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    member_can_create_share_links BOOLEAN NOT NULL DEFAULT FALSE,
    require_admin_approval_for_member_share BOOLEAN NOT NULL DEFAULT TRUE,
    default_share_link_ttl_days INTEGER NOT NULL DEFAULT 7,
    max_share_link_ttl_days INTEGER NOT NULL DEFAULT 30,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO system_policies (id)
VALUES (TRUE)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS tenant_policies (
    tenant_id UUID PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
    member_can_create_share_links BOOLEAN,
    require_admin_approval_for_member_share BOOLEAN,
    default_share_link_ttl_days INTEGER,
    max_share_link_ttl_days INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS organization_policies (
    organization_id UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    member_can_create_share_links BOOLEAN,
    require_admin_approval_for_member_share BOOLEAN,
    default_share_link_ttl_days INTEGER,
    max_share_link_ttl_days INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE tenant_invitations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_invitations ENABLE ROW LEVEL SECURITY;
ALTER TABLE system_policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE tenant_policies ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_policies ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS tenant_invitation_isolation ON tenant_invitations;
CREATE POLICY tenant_invitation_isolation ON tenant_invitations
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

DROP POLICY IF EXISTS organization_invitation_isolation ON organization_invitations;
CREATE POLICY organization_invitation_isolation ON organization_invitations
    USING (
        organization_id IN (
            SELECT id FROM organizations
            WHERE tenant_id = current_setting('nvbes.tenant_id', true)::uuid
        )
    );

DROP POLICY IF EXISTS system_policy_read ON system_policies;
CREATE POLICY system_policy_read ON system_policies
    USING (TRUE);

DROP POLICY IF EXISTS tenant_policy_isolation ON tenant_policies;
CREATE POLICY tenant_policy_isolation ON tenant_policies
    USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

DROP POLICY IF EXISTS organization_policy_isolation ON organization_policies;
CREATE POLICY organization_policy_isolation ON organization_policies
    USING (
        organization_id IN (
            SELECT id FROM organizations
            WHERE tenant_id = current_setting('nvbes.tenant_id', true)::uuid
        )
    );
