CREATE TABLE IF NOT EXISTS internal_admin_developer_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN ('revoke_client', 'rotate_secret', 'approve_marketplace_app')
  ),
  client_id TEXT,
  marketplace_app_id UUID,
  target_id UUID,
  status TEXT NOT NULL DEFAULT 'applied' CHECK (status IN ('applied', 'approved')),
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (
    (action_kind IN ('revoke_client', 'rotate_secret') AND client_id IS NOT NULL)
    OR (action_kind = 'approve_marketplace_app' AND marketplace_app_id IS NOT NULL)
  )
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_developer_actions_tenant_created
  ON internal_admin_developer_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_developer_actions_workspace_created
  ON internal_admin_developer_actions (workspace_id, created_at DESC);
