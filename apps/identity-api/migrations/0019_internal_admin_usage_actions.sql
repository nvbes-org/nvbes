CREATE TABLE IF NOT EXISTS internal_admin_usage_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  action_kind TEXT NOT NULL CHECK (action_kind IN ('correct_usage', 'freeze_meter', 'replay_rollup')),
  meter_code TEXT NOT NULL,
  target_id UUID,
  quantity_delta BIGINT,
  status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'applied', 'replayed')),
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_usage_actions_tenant_created
  ON internal_admin_usage_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_usage_actions_workspace_created
  ON internal_admin_usage_actions (workspace_id, created_at DESC);
