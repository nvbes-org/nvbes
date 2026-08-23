CREATE TABLE IF NOT EXISTS internal_admin_entitlement_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN ('grant_feature', 'revoke_feature', 'override_quota', 'publish_changes')
  ),
  feature_code TEXT,
  quota_code TEXT,
  quantity BIGINT,
  status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'published')),
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  published_at TIMESTAMPTZ,
  CHECK (
    (action_kind IN ('grant_feature', 'revoke_feature') AND feature_code IS NOT NULL)
    OR (action_kind = 'override_quota' AND quota_code IS NOT NULL AND quantity IS NOT NULL)
    OR action_kind = 'publish_changes'
  )
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_entitlement_actions_tenant_created
  ON internal_admin_entitlement_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_entitlement_actions_workspace_created
  ON internal_admin_entitlement_actions (workspace_id, created_at DESC);
