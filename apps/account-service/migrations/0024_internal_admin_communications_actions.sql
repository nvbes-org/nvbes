CREATE TABLE IF NOT EXISTS internal_admin_communications_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN ('replay_email', 'replay_webhook', 'suppress_email', 'unsuppress_email')
  ),
  email TEXT,
  email_message_id UUID,
  email_event_id UUID,
  status TEXT NOT NULL DEFAULT 'applied' CHECK (status IN ('queued', 'applied', 'removed')),
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (
    (action_kind = 'replay_email' AND email_message_id IS NOT NULL)
    OR (action_kind = 'replay_webhook' AND email_event_id IS NOT NULL)
    OR (action_kind IN ('suppress_email', 'unsuppress_email') AND email IS NOT NULL)
  )
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_communications_actions_tenant_created
  ON internal_admin_communications_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_communications_actions_workspace_created
  ON internal_admin_communications_actions (workspace_id, created_at DESC);
