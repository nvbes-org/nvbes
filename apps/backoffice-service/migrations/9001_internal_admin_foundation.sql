CREATE TABLE IF NOT EXISTS idempotency_responses (
  idempotency_key TEXT NOT NULL,
  scope TEXT NOT NULL,
  request_hash TEXT NOT NULL,
  response_status INT NOT NULL,
  response_body BYTEA NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '24 hours',
  PRIMARY KEY (idempotency_key, scope)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_responses_expires_at
  ON idempotency_responses (expires_at);

CREATE TABLE IF NOT EXISTS internal_admin_operator_grants (
  principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  granted_by_principal_id UUID REFERENCES principals(id) ON DELETE SET NULL,
  granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  revoked_at TIMESTAMPTZ,
  reason TEXT,
  PRIMARY KEY (principal_id, role),
  CONSTRAINT internal_admin_operator_grants_role_check CHECK (
    role IN (
      'compliance_admin',
      'developer_admin',
      'finance_admin',
      'operations_admin',
      'platform_admin',
      'product_admin',
      'security_admin',
      'support_agent',
      'viewer'
    )
  ),
  CONSTRAINT internal_admin_operator_grants_status_check CHECK (
    status IN ('active', 'revoked')
  ),
  CONSTRAINT internal_admin_operator_grants_revoked_at_check CHECK (
    (status = 'revoked' AND revoked_at IS NOT NULL)
    OR (status = 'active' AND revoked_at IS NULL)
  )
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_operator_grants_active_role
  ON internal_admin_operator_grants (role, principal_id)
  WHERE status = 'active';

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

CREATE TABLE IF NOT EXISTS internal_admin_incidents (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  severity TEXT NOT NULL DEFAULT 'medium',
  status TEXT NOT NULL DEFAULT 'open',
  details JSONB NOT NULL DEFAULT '{}'::jsonb,
  opened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  resolved_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_incidents_workspace_status
  ON internal_admin_incidents (workspace_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS internal_admin_maintenance_windows (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'scheduled',
  scheduled_start_at TIMESTAMPTZ NOT NULL,
  scheduled_end_at TIMESTAMPTZ NOT NULL,
  reason TEXT NOT NULL,
  created_by_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (scheduled_end_at > scheduled_start_at)
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_maintenance_windows_workspace_status
  ON internal_admin_maintenance_windows (workspace_id, status, scheduled_start_at);

CREATE TABLE IF NOT EXISTS internal_admin_job_runs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  job_kind TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'queued',
  payload JSONB NOT NULL DEFAULT '{}'::jsonb,
  last_error TEXT,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_internal_admin_job_runs_workspace_status
  ON internal_admin_job_runs (workspace_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS internal_admin_operations_actions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  actor_principal_id UUID NOT NULL REFERENCES principals(id) ON DELETE RESTRICT,
  action_kind TEXT NOT NULL,
  provider_event_id UUID,
  export_run_id UUID,
  reconciliation_difference_id UUID,
  incident_id UUID REFERENCES internal_admin_incidents(id) ON DELETE SET NULL,
  maintenance_window_id UUID REFERENCES internal_admin_maintenance_windows(id) ON DELETE SET NULL,
  job_run_id UUID REFERENCES internal_admin_job_runs(id) ON DELETE SET NULL,
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
