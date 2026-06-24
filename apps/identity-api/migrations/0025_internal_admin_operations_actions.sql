CREATE TABLE IF NOT EXISTS internal_admin_operations_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  action_kind TEXT NOT NULL,
  provider_event_id UUID REFERENCES billing_provider_events(id) ON DELETE SET NULL,
  export_run_id UUID REFERENCES billing_export_runs(id) ON DELETE SET NULL,
  reconciliation_difference_id UUID REFERENCES billing_reconciliation_differences(id) ON DELETE SET NULL,
  previous_state TEXT,
  next_state TEXT NOT NULL,
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_operations_actions_tenant_created
  ON internal_admin_operations_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_operations_actions_workspace_created
  ON internal_admin_operations_actions (workspace_id, created_at DESC);

