CREATE TABLE IF NOT EXISTS internal_admin_region_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  target_workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  action_kind TEXT NOT NULL CHECK (action_kind IN ('flag_residency', 'record_exception')),
  previous_data_region TEXT,
  next_data_region TEXT,
  previous_jurisdiction TEXT,
  next_jurisdiction TEXT,
  exception_kind TEXT,
  status TEXT NOT NULL DEFAULT 'applied' CHECK (status IN ('applied', 'recorded')),
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_region_actions_tenant_created
  ON internal_admin_region_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_region_actions_workspace_created
  ON internal_admin_region_actions (workspace_id, created_at DESC);
