CREATE TABLE IF NOT EXISTS internal_admin_compliance_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  action_kind TEXT NOT NULL CHECK (
    action_kind IN ('revoke_consent', 'request_erasure', 'review_suppression')
  ),
  principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  consent_id UUID REFERENCES user_consents(id) ON DELETE SET NULL,
  email TEXT,
  status TEXT NOT NULL DEFAULT 'applied' CHECK (status IN ('applied', 'requested', 'reviewed')),
  reason TEXT NOT NULL,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (
    (action_kind = 'revoke_consent' AND consent_id IS NOT NULL)
    OR (action_kind = 'request_erasure' AND principal_id IS NOT NULL)
    OR (action_kind = 'review_suppression' AND email IS NOT NULL)
  )
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_compliance_actions_tenant_created
  ON internal_admin_compliance_actions (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_internal_admin_compliance_actions_workspace_created
  ON internal_admin_compliance_actions (workspace_id, created_at DESC);
