CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TYPE user_status AS ENUM (
  'active',
  'pending_verification',
  'suspended',
  'deleted'
);

CREATE TYPE mfa_factor_type AS ENUM ('totp', 'webauthn', 'recovery_code');
CREATE TYPE mfa_factor_status AS ENUM ('pending', 'active', 'revoked');
CREATE TYPE workspace_type AS ENUM ('personal', 'team');
CREATE TYPE workspace_member_role AS ENUM ('owner', 'admin', 'member', 'viewer');
CREATE TYPE workspace_member_status AS ENUM ('active', 'invited', 'suspended', 'removed');
CREATE TYPE api_key_status AS ENUM ('active', 'revoked', 'expired');
CREATE TYPE storage_object_type AS ENUM ('file', 'folder');
CREATE TYPE storage_object_status AS ENUM ('pending', 'active', 'trashed', 'deleted');
CREATE TYPE upload_session_status AS ENUM ('pending', 'completed', 'cancelled', 'expired');
CREATE TYPE share_link_permission AS ENUM ('download');

CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email TEXT NOT NULL UNIQUE,
  email_verified_at TIMESTAMPTZ,
  name TEXT NOT NULL,
  status user_status NOT NULL DEFAULT 'pending_verification',
  mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL,
  revoked_at TIMESTAMPTZ,
  ip INET,
  user_agent TEXT
);

CREATE TABLE mfa_factors (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  factor_type mfa_factor_type NOT NULL,
  status mfa_factor_status NOT NULL DEFAULT 'pending',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  last_used_at TIMESTAMPTZ
);

CREATE TABLE plans (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  code TEXT NOT NULL UNIQUE,
  included_storage_gb INTEGER NOT NULL CHECK (included_storage_gb >= 0),
  included_users INTEGER NOT NULL CHECK (included_users >= 0),
  retention_days INTEGER NOT NULL CHECK (retention_days >= 0),
  max_share_links INTEGER NOT NULL CHECK (max_share_links >= 0),
  audit_level TEXT NOT NULL,
  max_share_link_ttl_days INTEGER NOT NULL CHECK (max_share_link_ttl_days > 0),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE workspaces (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_type workspace_type NOT NULL,
  name TEXT NOT NULL,
  owner_user_id UUID NOT NULL REFERENCES users (id),
  plan_id UUID NOT NULL REFERENCES plans (id),
  trial_started_at TIMESTAMPTZ,
  trial_ends_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE workspace_policies (
  workspace_id UUID PRIMARY KEY REFERENCES workspaces (id) ON DELETE CASCADE,
  member_can_create_share_links BOOLEAN NOT NULL DEFAULT FALSE,
  require_admin_approval_for_member_share BOOLEAN NOT NULL DEFAULT TRUE,
  default_share_link_ttl_days INTEGER NOT NULL DEFAULT 7 CHECK (default_share_link_ttl_days > 0),
  max_share_link_ttl_days INTEGER NOT NULL CHECK (max_share_link_ttl_days > 0),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE workspace_members (
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  role workspace_member_role NOT NULL,
  status workspace_member_status NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (workspace_id, user_id)
);

CREATE TABLE api_keys (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  key_prefix TEXT NOT NULL,
  key_hash TEXT NOT NULL UNIQUE,
  scopes TEXT[] NOT NULL DEFAULT '{}',
  status api_key_status NOT NULL DEFAULT 'active',
  created_by UUID NOT NULL REFERENCES users (id),
  http_signature_public_key TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ,
  last_used_at TIMESTAMPTZ,
  last_used_ip INET
);

CREATE TABLE api_request_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  api_key_id UUID REFERENCES api_keys (id) ON DELETE SET NULL,
  request_id TEXT NOT NULL,
  method TEXT NOT NULL,
  path TEXT NOT NULL,
  status_code INTEGER NOT NULL,
  error_code TEXT,
  scopes_used TEXT[] NOT NULL DEFAULT '{}',
  ip INET,
  user_agent TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE usage_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  meter TEXT NOT NULL,
  quantity BIGINT NOT NULL,
  unit TEXT NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL,
  source TEXT NOT NULL,
  idempotency_key TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE storage_objects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  parent_id UUID REFERENCES storage_objects (id) ON DELETE SET NULL,
  object_type storage_object_type NOT NULL,
  name TEXT NOT NULL,
  size_bytes BIGINT NOT NULL DEFAULT 0 CHECK (size_bytes >= 0),
  mime_type TEXT,
  object_key TEXT,
  checksum TEXT,
  status storage_object_status NOT NULL DEFAULT 'pending',
  created_by UUID NOT NULL REFERENCES users (id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  trashed_at TIMESTAMPTZ,
  CHECK (
    (object_type = 'folder' AND object_key IS NULL AND mime_type IS NULL)
    OR object_type = 'file'
  )
);

CREATE TABLE upload_sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  storage_object_id UUID NOT NULL REFERENCES storage_objects (id) ON DELETE CASCADE,
  created_by UUID NOT NULL REFERENCES users (id),
  expected_size_bytes BIGINT NOT NULL CHECK (expected_size_bytes >= 0),
  expected_checksum TEXT,
  upload_offset_bytes BIGINT NOT NULL DEFAULT 0 CHECK (upload_offset_bytes >= 0),
  storage_multipart_upload_id TEXT,
  status upload_session_status NOT NULL DEFAULT 'pending',
  expires_at TIMESTAMPTZ NOT NULL,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE upload_parts (
  upload_session_id UUID NOT NULL REFERENCES upload_sessions (id) ON DELETE CASCADE,
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  part_number INTEGER NOT NULL CHECK (part_number > 0),
  offset_bytes BIGINT NOT NULL CHECK (offset_bytes >= 0),
  size_bytes BIGINT NOT NULL CHECK (size_bytes > 0),
  etag TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (upload_session_id, part_number)
);

CREATE TABLE share_links (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  storage_object_id UUID NOT NULL REFERENCES storage_objects (id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  permission share_link_permission NOT NULL DEFAULT 'download',
  expires_at TIMESTAMPTZ NOT NULL,
  max_downloads INTEGER CHECK (max_downloads IS NULL OR max_downloads > 0),
  download_count INTEGER NOT NULL DEFAULT 0 CHECK (download_count >= 0),
  created_by UUID NOT NULL REFERENCES users (id),
  revoked_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE quota_usage (
  workspace_id UUID PRIMARY KEY REFERENCES workspaces (id) ON DELETE CASCADE,
  used_storage_bytes BIGINT NOT NULL DEFAULT 0 CHECK (used_storage_bytes >= 0),
  file_count BIGINT NOT NULL DEFAULT 0 CHECK (file_count >= 0),
  bandwidth_out_bytes_month BIGINT NOT NULL DEFAULT 0 CHECK (bandwidth_out_bytes_month >= 0),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE audit_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  actor_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_id UUID,
  ip INET,
  user_agent TEXT,
  metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at);
CREATE INDEX idx_mfa_factors_user_id ON mfa_factors (user_id);
CREATE INDEX idx_workspaces_owner_user_id ON workspaces (owner_user_id);
CREATE INDEX idx_workspace_members_user_id ON workspace_members (user_id);
CREATE INDEX idx_api_keys_workspace_id ON api_keys (workspace_id);
CREATE INDEX idx_api_request_logs_workspace_id_created_at
  ON api_request_logs (workspace_id, created_at DESC);
CREATE INDEX idx_usage_events_workspace_meter_occurred_at
  ON usage_events (workspace_id, meter, occurred_at DESC);
CREATE UNIQUE INDEX idx_usage_events_idempotency_key
  ON usage_events (workspace_id, idempotency_key)
  WHERE idempotency_key IS NOT NULL;
CREATE INDEX idx_storage_objects_workspace_parent_name
  ON storage_objects (workspace_id, parent_id, name);
CREATE INDEX idx_storage_objects_workspace_status
  ON storage_objects (workspace_id, status);
CREATE UNIQUE INDEX idx_storage_objects_object_key
  ON storage_objects (object_key)
  WHERE object_key IS NOT NULL;
CREATE INDEX idx_upload_sessions_workspace_status_expires_at
  ON upload_sessions (workspace_id, status, expires_at);
CREATE INDEX idx_upload_parts_workspace_upload
  ON upload_parts (workspace_id, upload_session_id, part_number);
CREATE INDEX idx_share_links_workspace_id ON share_links (workspace_id);
CREATE INDEX idx_share_links_storage_object_id ON share_links (storage_object_id);
CREATE INDEX idx_share_links_expires_at ON share_links (expires_at);
CREATE INDEX idx_audit_events_workspace_created_at
  ON audit_events (workspace_id, created_at DESC);
CREATE INDEX idx_audit_events_action_created_at
  ON audit_events (action, created_at DESC);
INSERT INTO plans (
  code,
  included_storage_gb,
  included_users,
  retention_days,
  max_share_links,
  audit_level,
  max_share_link_ttl_days
)
VALUES
  ('solo_pro', 100, 1, 30, 100, 'standard', 30),
  ('team', 1000, 10, 90, 1000, 'standard', 90),
  ('team_plus', 5000, 25, 365, 5000, 'extended', 365)
ON CONFLICT (code) DO NOTHING;
ALTER TABLE users
ADD COLUMN password_hash TEXT NOT NULL DEFAULT '';

ALTER TABLE sessions
ADD COLUMN session_token_hash TEXT NOT NULL DEFAULT '';

ALTER TABLE audit_events
ALTER COLUMN workspace_id DROP NOT NULL;

CREATE UNIQUE INDEX idx_sessions_session_token_hash ON sessions (session_token_hash);

CREATE TABLE email_verification_tokens (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  expires_at TIMESTAMPTZ NOT NULL,
  consumed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE password_reset_tokens (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  expires_at TIMESTAMPTZ NOT NULL,
  consumed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_email_verification_tokens_user_id
  ON email_verification_tokens (user_id, expires_at DESC);
CREATE INDEX idx_password_reset_tokens_user_id
  ON password_reset_tokens (user_id, expires_at DESC);
INSERT INTO plans (
  code,
  included_storage_gb,
  included_users,
  retention_days,
  max_share_links,
  audit_level,
  max_share_link_ttl_days
)
VALUES
  ('trial', 5, 1, 30, 25, 'minimal', 7)
ON CONFLICT (code) DO NOTHING;

WITH trial_plan AS (
  SELECT id
  FROM plans
  WHERE code = 'trial'
  LIMIT 1
)
UPDATE workspaces
SET plan_id = trial_plan.id,
    updated_at = NOW()
FROM trial_plan
WHERE workspaces.trial_ends_at IS NOT NULL
  AND workspaces.trial_ends_at > NOW()
  AND workspaces.plan_id <> trial_plan.id;

CREATE TYPE workspace_invitation_status AS ENUM ('pending', 'accepted', 'revoked', 'expired');

CREATE TABLE workspace_invitations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  email TEXT NOT NULL,
  role workspace_member_role NOT NULL,
  invited_by UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  token_hash TEXT NOT NULL UNIQUE,
  status workspace_invitation_status NOT NULL DEFAULT 'pending',
  expires_at TIMESTAMPTZ NOT NULL,
  accepted_at TIMESTAMPTZ,
  revoked_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (lower(email) = email)
);

CREATE INDEX idx_workspace_invitations_workspace_id
  ON workspace_invitations (workspace_id, created_at DESC);
CREATE INDEX idx_workspace_invitations_email_status
  ON workspace_invitations (email, status, expires_at DESC);
ALTER TABLE audit_events
  ADD COLUMN previous_event_hash TEXT,
  ADD COLUMN event_hash TEXT;

WITH ordered_events AS (
  SELECT
    id,
    LAG(event_hash_seed) OVER (
      PARTITION BY workspace_id
      ORDER BY created_at ASC, id ASC
    ) AS previous_hash_seed,
    event_hash_seed
  FROM (
    SELECT
      id,
      workspace_id,
      created_at,
      encode(
        digest(
          concat_ws(
            '|',
            id::text,
            workspace_id::text,
            COALESCE(actor_user_id::text, ''),
            action,
            target_type,
            COALESCE(target_id::text, ''),
            COALESCE(ip::text, ''),
            COALESCE(user_agent, ''),
            metadata::text,
            created_at::text
          ),
          'sha256'
        ),
        'hex'
      ) AS event_hash_seed
    FROM audit_events
  ) seeds
)
UPDATE audit_events ae
SET previous_event_hash = ordered_events.previous_hash_seed,
    event_hash = ordered_events.event_hash_seed
FROM ordered_events
WHERE ae.id = ordered_events.id;

ALTER TABLE audit_events
  ALTER COLUMN event_hash SET NOT NULL;

CREATE UNIQUE INDEX idx_audit_events_event_hash
  ON audit_events (event_hash);

CREATE OR REPLACE FUNCTION audit_events_set_hash()
RETURNS TRIGGER AS $$
DECLARE
  previous_hash TEXT;
BEGIN
  SELECT event_hash
  INTO previous_hash
  FROM audit_events
  WHERE workspace_id = NEW.workspace_id
  ORDER BY created_at DESC, id DESC
  LIMIT 1;

  NEW.previous_event_hash := previous_hash;
  NEW.event_hash := encode(
    digest(
      concat_ws(
        '|',
        NEW.id::text,
        NEW.workspace_id::text,
        COALESCE(NEW.actor_user_id::text, ''),
        NEW.action,
        NEW.target_type,
        COALESCE(NEW.target_id::text, ''),
        COALESCE(NEW.ip::text, ''),
        COALESCE(NEW.user_agent, ''),
        NEW.metadata::text,
        NEW.created_at::text,
        COALESCE(previous_hash, '')
      ),
      'sha256'
    ),
    'hex'
  );

  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION audit_events_prevent_mutation()
RETURNS TRIGGER AS $$
BEGIN
  RAISE EXCEPTION 'audit_events is append-only';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_events_set_hash
BEFORE INSERT ON audit_events
FOR EACH ROW
EXECUTE FUNCTION audit_events_set_hash();

CREATE TRIGGER trg_audit_events_prevent_update
BEFORE UPDATE ON audit_events
FOR EACH ROW
EXECUTE FUNCTION audit_events_prevent_mutation();

CREATE TRIGGER trg_audit_events_prevent_delete
BEFORE DELETE ON audit_events
FOR EACH ROW
EXECUTE FUNCTION audit_events_prevent_mutation();

ALTER TABLE audit_events
  DROP CONSTRAINT audit_events_workspace_id_fkey,
  ADD CONSTRAINT audit_events_workspace_id_fkey
    FOREIGN KEY (workspace_id) REFERENCES workspaces (id) ON DELETE RESTRICT;

CREATE TYPE privacy_request_type AS ENUM (
  'account_export',
  'account_delete',
  'workspace_export',
  'workspace_delete'
);

CREATE TYPE privacy_request_status AS ENUM (
  'queued',
  'processing',
  'completed',
  'failed',
  'rejected'
);

CREATE TABLE privacy_requests (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  request_type privacy_request_type NOT NULL,
  status privacy_request_status NOT NULL DEFAULT 'queued',
  subject_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
  workspace_id UUID REFERENCES workspaces (id) ON DELETE RESTRICT,
  requested_by UUID REFERENCES users (id) ON DELETE SET NULL,
  worker_job_id UUID,
  identity_verified BOOLEAN NOT NULL DEFAULT TRUE,
  legal_hold BOOLEAN NOT NULL DEFAULT FALSE,
  rejection_reason TEXT,
  result JSONB,
  requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (
    (request_type IN ('account_export', 'account_delete') AND subject_user_id IS NOT NULL)
    OR
    (request_type IN ('workspace_export', 'workspace_delete') AND workspace_id IS NOT NULL)
  )
);

CREATE UNIQUE INDEX idx_privacy_requests_open_idempotency
  ON privacy_requests (request_type, (COALESCE(subject_user_id, workspace_id)))
  WHERE status IN ('queued', 'processing');

CREATE INDEX idx_privacy_requests_subject_user_id
  ON privacy_requests (subject_user_id, requested_at DESC);

CREATE INDEX idx_privacy_requests_workspace_id
  ON privacy_requests (workspace_id, requested_at DESC);

CREATE INDEX idx_privacy_requests_status_requested_at
  ON privacy_requests (status, requested_at ASC);

ALTER TABLE workspaces
  ADD COLUMN deleted_at TIMESTAMPTZ;

ALTER TABLE users
  ADD COLUMN deleted_at TIMESTAMPTZ;
ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS step_up_verified_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS step_up_expires_at TIMESTAMPTZ;
CREATE TABLE IF NOT EXISTS workspace_memberships (
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  role workspace_member_role NOT NULL,
  status workspace_member_status NOT NULL DEFAULT 'active',
  source TEXT NOT NULL DEFAULT 'manual',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (workspace_id, user_id)
);

INSERT INTO workspace_memberships (
  workspace_id,
  user_id,
  role,
  status,
  source,
  created_at,
  updated_at
)
SELECT
  workspace_id,
  user_id,
  role,
  status,
  'manual',
  created_at,
  updated_at
FROM workspace_members
ON CONFLICT (workspace_id, user_id) DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_workspace_memberships_user_id ON workspace_memberships (user_id);
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'tenant_status') THEN
    CREATE TYPE tenant_status AS ENUM ('active', 'suspended', 'deleted');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'organization_status') THEN
    CREATE TYPE organization_status AS ENUM ('active', 'suspended', 'deleted');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'membership_status') THEN
    CREATE TYPE membership_status AS ENUM ('active', 'invited', 'suspended', 'removed');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'membership_source') THEN
    CREATE TYPE membership_source AS ENUM ('manual', 'invitation', 'scim', 'jit', 'system');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'tenant_kind') THEN
    CREATE TYPE tenant_kind AS ENUM ('personal', 'team', 'enterprise', 'system');
  END IF;
END
$$;

CREATE TABLE IF NOT EXISTS tenants (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  kind tenant_kind NOT NULL DEFAULT 'personal',
  name TEXT NOT NULL,
  slug TEXT NOT NULL UNIQUE,
  status tenant_status NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS organizations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
  parent_organization_id UUID REFERENCES organizations (id) ON DELETE SET NULL,
  name TEXT NOT NULL,
  slug TEXT NOT NULL,
  status organization_status NOT NULL DEFAULT 'active',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (tenant_id, slug)
);

CREATE TABLE IF NOT EXISTS tenant_memberships (
  tenant_id UUID NOT NULL REFERENCES tenants (id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  status membership_status NOT NULL DEFAULT 'active',
  source membership_source NOT NULL DEFAULT 'manual',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (tenant_id, user_id)
);

CREATE TABLE IF NOT EXISTS organization_memberships (
  organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  status membership_status NOT NULL DEFAULT 'active',
  source membership_source NOT NULL DEFAULT 'manual',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (organization_id, user_id)
);

ALTER TABLE workspaces
  ADD COLUMN IF NOT EXISTS tenant_id UUID,
  ADD COLUMN IF NOT EXISTS organization_id UUID;

ALTER TABLE sessions
  ADD COLUMN IF NOT EXISTS tenant_id UUID,
  ADD COLUMN IF NOT EXISTS organization_id UUID;

CREATE INDEX IF NOT EXISTS idx_tenant_memberships_user_id ON tenant_memberships (user_id);
CREATE INDEX IF NOT EXISTS idx_organization_memberships_user_id ON organization_memberships (user_id);
CREATE INDEX IF NOT EXISTS idx_workspaces_tenant_id ON workspaces (tenant_id);
CREATE INDEX IF NOT EXISTS idx_workspaces_organization_id ON workspaces (organization_id);
CREATE INDEX IF NOT EXISTS idx_sessions_tenant_id ON sessions (tenant_id);
CREATE INDEX IF NOT EXISTS idx_sessions_organization_id ON sessions (organization_id);
ALTER TABLE sessions
    ADD COLUMN IF NOT EXISTS workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL;

ALTER TABLE workspace_policies
    ADD COLUMN IF NOT EXISTS required_acr TEXT NOT NULL DEFAULT 'aal1';

CREATE INDEX IF NOT EXISTS idx_sessions_workspace_id ON sessions(workspace_id);
ALTER TABLE mfa_factors
    ADD COLUMN IF NOT EXISTS label TEXT,
    ADD COLUMN IF NOT EXISTS totp_secret_base32 TEXT,
    ADD COLUMN IF NOT EXISTS totp_digits SMALLINT NOT NULL DEFAULT 6,
    ADD COLUMN IF NOT EXISTS totp_period_seconds INTEGER NOT NULL DEFAULT 30,
    ADD COLUMN IF NOT EXISTS totp_last_used_counter BIGINT,
    ADD COLUMN IF NOT EXISTS factor_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS confirmed_at TIMESTAMPTZ;

UPDATE mfa_factors
SET label = COALESCE(label, factor_type::text),
    factor_data = COALESCE(factor_data, '{}'::jsonb)
WHERE label IS NULL OR factor_data IS NULL;

CREATE INDEX IF NOT EXISTS idx_mfa_factors_user_status
    ON mfa_factors (user_id, status);

-- =============================================================================
-- Row Level Security - Multi-Tenant Isolation v1 (Cloud Service)
-- =============================================================================

-- Helper function to set GUC variables for RLS context
CREATE OR REPLACE FUNCTION nvbes_set_context(
    p_user_id UUID DEFAULT NULL,
    p_tenant_id UUID DEFAULT NULL,
    p_workspace_id UUID DEFAULT NULL
) RETURNS void AS $$
BEGIN
    IF p_user_id IS NOT NULL THEN
        PERFORM set_config('nvbes.user_id', p_user_id::text, true);
    END IF;
    IF p_tenant_id IS NOT NULL THEN
        PERFORM set_config('nvbes.tenant_id', p_tenant_id::text, true);
    END IF;
    IF p_workspace_id IS NOT NULL THEN
        PERFORM set_config('nvbes.workspace_id', p_workspace_id::text, true);
    END IF;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Create BYPASSRLS role for system operations
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'nvbes_system') THEN
        CREATE ROLE nvbes_system BYPASSRLS;
    END IF;
END
$$;

-- =============================================================================
-- Tenant-scoped tables: isolation by tenant_id
-- =============================================================================

ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON tenants
    FOR ALL
    USING (id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE organizations ENABLE ROW LEVEL SECURITY;
CREATE POLICY organization_isolation ON organizations
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE tenant_memberships ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_membership_isolation ON tenant_memberships
    FOR ALL
    USING (tenant_id = current_setting('nvbes.tenant_id')::uuid);

ALTER TABLE organization_memberships ENABLE ROW LEVEL SECURITY;
CREATE POLICY organization_membership_isolation ON organization_memberships
    FOR ALL
    USING (organization_id IN (
        SELECT id FROM organizations
        WHERE tenant_id = current_setting('nvbes.tenant_id')::uuid
    ));

-- =============================================================================
-- Workspace-scoped tables: isolation by workspace_id
-- =============================================================================

ALTER TABLE workspaces ENABLE ROW LEVEL SECURITY;

CREATE POLICY workspace_read_via_membership ON workspaces
    FOR SELECT
    USING (id IN (
        SELECT workspace_id FROM workspace_members
        WHERE user_id = current_setting('nvbes.user_id')::uuid
        AND status = 'active'
    ));

CREATE POLICY workspace_write ON workspaces
    FOR ALL
    USING (id = current_setting('nvbes.workspace_id')::uuid)
    WITH CHECK (id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE workspace_policies ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_policy_isolation ON workspace_policies
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE workspace_members ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_member_isolation ON workspace_members
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE workspace_invitations ENABLE ROW LEVEL SECURITY;
CREATE POLICY workspace_invitation_isolation ON workspace_invitations
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE quota_usage ENABLE ROW LEVEL SECURITY;
CREATE POLICY quota_usage_isolation ON quota_usage
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE usage_events ENABLE ROW LEVEL SECURITY;
CREATE POLICY usage_event_isolation ON usage_events
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
CREATE POLICY session_isolation ON sessions
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE api_keys ENABLE ROW LEVEL SECURITY;
CREATE POLICY api_key_isolation ON api_keys
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE api_request_logs ENABLE ROW LEVEL SECURITY;
CREATE POLICY api_request_log_isolation ON api_request_logs
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE storage_objects ENABLE ROW LEVEL SECURITY;
CREATE POLICY storage_object_isolation ON storage_objects
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE upload_sessions ENABLE ROW LEVEL SECURITY;
CREATE POLICY upload_session_isolation ON upload_sessions
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE upload_parts ENABLE ROW LEVEL SECURITY;
CREATE POLICY upload_part_isolation ON upload_parts
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE share_links ENABLE ROW LEVEL SECURITY;
CREATE POLICY share_link_isolation ON share_links
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

ALTER TABLE privacy_requests ENABLE ROW LEVEL SECURITY;
CREATE POLICY privacy_request_isolation ON privacy_requests
    FOR ALL
    USING (
        workspace_id = current_setting('nvbes.workspace_id')::uuid
        OR subject_user_id = current_setting('nvbes.user_id')::uuid
    );

-- Audit events: read isolation (append-only enforced by existing trigger)
ALTER TABLE audit_events ENABLE ROW LEVEL SECURITY;
CREATE POLICY audit_read ON audit_events
    FOR SELECT
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

-- =============================================================================
-- Tables WITHOUT RLS (global/system tables)
-- =============================================================================
-- users (global user accounts)
-- mfa_factors (user-scoped)
-- email_verification_tokens (user-scoped)
-- password_reset_tokens (user-scoped)
-- plans
-- oauth_scope_metadata (not present in cloud-service)
-- Migration: data_classification_v1
-- Description: Add data classification framework columns
-- Date: 2026-05-12

-- Classification enum
CREATE TYPE data_classification AS ENUM ('public', 'internal', 'confidential', 'restricted');

-- Add classification to storage_objects
ALTER TABLE storage_objects
  ADD COLUMN classification data_classification NOT NULL DEFAULT 'restricted';

ALTER TABLE storage_objects
  ADD COLUMN contains_pii BOOLEAN NOT NULL DEFAULT false;

-- Add classification to share_links (inherited from parent object)
ALTER TABLE share_links
  ADD COLUMN classification data_classification NOT NULL DEFAULT 'restricted';

-- Add classification to upload_sessions
ALTER TABLE upload_sessions
  ADD COLUMN classification data_classification NOT NULL DEFAULT 'restricted';

-- Index for classification-based queries on active objects
CREATE INDEX idx_storage_objects_classification
  ON storage_objects(classification)
  WHERE status = 'active';
-- Migration: file_scanning_v1
-- Description: Add file scanning and quarantine support
-- Date: 2026-05-13

-- Add 'quarantined' to storage_object_status enum
ALTER TYPE storage_object_status ADD VALUE IF NOT EXISTS 'quarantined';

-- Add scan columns to storage_objects
ALTER TABLE storage_objects
  ADD COLUMN scan_status TEXT NOT NULL DEFAULT 'pending',
  ADD COLUMN scanned_at TIMESTAMPTZ,
  ADD COLUMN scan_engine TEXT,
  ADD COLUMN quarantine_reason TEXT;

-- Index for quarantine cleanup worker
CREATE INDEX idx_storage_objects_quarantined
  ON storage_objects(scanned_at)
  WHERE status = 'quarantined';

-- Index for scan status queries
CREATE INDEX idx_storage_objects_scan_status
  ON storage_objects(scan_status)
  WHERE status = 'pending';
ALTER TABLE users
ADD COLUMN IF NOT EXISTS identity_subject TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_identity_subject
ON users (identity_subject);

-- Consolidated V0 migrations that used to live as separate files.

-- Migration: 0002_multi_region_compliance.sql
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'data_region') THEN
        CREATE TYPE data_region AS ENUM ('eu', 'us', 'ch', 'apac');
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'legal_jurisdiction') THEN
        CREATE TYPE legal_jurisdiction AS ENUM ('gdpr', 'ccpa', 'nfdap', 'global');
    END IF;
END $$;

ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS data_region data_region NOT NULL DEFAULT 'eu',
    ADD COLUMN IF NOT EXISTS jurisdiction legal_jurisdiction NOT NULL DEFAULT 'gdpr';

-- Migration: 0003_api_key_nonces.sql
CREATE TABLE IF NOT EXISTS api_key_nonces (
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  api_key_id UUID NOT NULL REFERENCES api_keys (id) ON DELETE CASCADE,
  nonce TEXT NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (api_key_id, nonce)
);

CREATE INDEX IF NOT EXISTS idx_api_key_nonces_created_at
  ON api_key_nonces (created_at);

CREATE INDEX IF NOT EXISTS idx_api_key_nonces_workspace_id_created_at
  ON api_key_nonces (workspace_id, created_at DESC);

ALTER TABLE api_key_nonces ENABLE ROW LEVEL SECURITY;
CREATE POLICY api_key_nonce_isolation ON api_key_nonces
    FOR ALL
    USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);

-- Migration: 0004_audit_actor_principal_id.sql
ALTER TABLE audit_events
  ADD COLUMN IF NOT EXISTS actor_principal_id UUID;

UPDATE audit_events
SET actor_principal_id = COALESCE(actor_principal_id, actor_user_id)
WHERE actor_principal_id IS NULL
  AND actor_user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_audit_events_actor_principal_id
  ON audit_events (actor_principal_id, created_at DESC);

-- Migration: 0005_created_by_principal_id.sql
ALTER TABLE storage_objects
  ADD COLUMN IF NOT EXISTS created_by_principal_id UUID;

ALTER TABLE upload_sessions
  ADD COLUMN IF NOT EXISTS created_by_principal_id UUID;

ALTER TABLE share_links
  ADD COLUMN IF NOT EXISTS created_by_principal_id UUID;

UPDATE storage_objects so
SET created_by_principal_id = COALESCE(
  CASE
    WHEN u.identity_subject ~ '^[0-9a-fA-F-]{36}$' THEN u.identity_subject::uuid
    ELSE NULL
  END,
  so.created_by
)
FROM users u
WHERE u.id = so.created_by
  AND so.created_by_principal_id IS NULL;

UPDATE upload_sessions us
SET created_by_principal_id = COALESCE(
  CASE
    WHEN u.identity_subject ~ '^[0-9a-fA-F-]{36}$' THEN u.identity_subject::uuid
    ELSE NULL
  END,
  us.created_by
)
FROM users u
WHERE u.id = us.created_by
  AND us.created_by_principal_id IS NULL;

UPDATE share_links sl
SET created_by_principal_id = COALESCE(
  CASE
    WHEN u.identity_subject ~ '^[0-9a-fA-F-]{36}$' THEN u.identity_subject::uuid
    ELSE NULL
  END,
  sl.created_by
)
FROM users u
WHERE u.id = sl.created_by
  AND sl.created_by_principal_id IS NULL;

ALTER TABLE storage_objects
  ALTER COLUMN created_by_principal_id SET NOT NULL;

ALTER TABLE upload_sessions
  ALTER COLUMN created_by_principal_id SET NOT NULL;

ALTER TABLE share_links
  ALTER COLUMN created_by_principal_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_storage_objects_created_by_principal_id
  ON storage_objects (workspace_id, created_by_principal_id);

CREATE INDEX IF NOT EXISTS idx_upload_sessions_created_by_principal_id
  ON upload_sessions (workspace_id, created_by_principal_id);

CREATE INDEX IF NOT EXISTS idx_share_links_created_by_principal_id
  ON share_links (workspace_id, created_by_principal_id);

-- Migration: 0006_api_keys_principals.sql
ALTER TABLE api_keys
  ADD COLUMN IF NOT EXISTS created_by_principal_id UUID;

UPDATE api_keys
SET created_by_principal_id = COALESCE(created_by_principal_id, created_by)
WHERE created_by_principal_id IS NULL;

ALTER TABLE api_keys
  ALTER COLUMN created_by_principal_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_api_keys_created_by_principal_id
  ON api_keys (workspace_id, created_by_principal_id);

-- Migration: 0007_workspace_owner_principal_id.sql
ALTER TABLE workspaces
  ADD COLUMN IF NOT EXISTS owner_principal_id UUID;

UPDATE workspaces
SET owner_principal_id = COALESCE(owner_principal_id, owner_user_id)
WHERE owner_principal_id IS NULL;

ALTER TABLE workspaces
  ALTER COLUMN owner_principal_id SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_workspaces_owner_principal_id
  ON workspaces (owner_principal_id);

-- Migration: 0008_privacy_requested_by_principal_id.sql
ALTER TABLE privacy_requests
  ADD COLUMN IF NOT EXISTS requested_by_principal_id UUID;

UPDATE privacy_requests
SET requested_by_principal_id = COALESCE(requested_by_principal_id, requested_by)
WHERE requested_by_principal_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_privacy_requests_requested_by_principal_id
  ON privacy_requests (requested_by_principal_id, requested_at DESC);

-- Migration: 0009_api_request_log_actor_principal_id.sql
ALTER TABLE api_request_logs
  ADD COLUMN IF NOT EXISTS actor_principal_id UUID;

UPDATE api_request_logs arl
SET actor_principal_id = COALESCE(arl.actor_principal_id, ak.created_by_principal_id)
FROM api_keys ak
WHERE arl.api_key_id = ak.id
  AND arl.actor_principal_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_api_request_logs_actor_principal_id_created_at
  ON api_request_logs (actor_principal_id, created_at DESC);

-- Migration: 0010_billing_entitlement_projection.sql
CREATE TABLE IF NOT EXISTS billing_entitlement_snapshots (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
  source_event_id TEXT,
  status TEXT NOT NULL,
  features JSONB NOT NULL DEFAULT '{}'::jsonb,
  quotas JSONB NOT NULL DEFAULT '{}'::jsonb,
  billing_locked BOOLEAN NOT NULL DEFAULT FALSE,
  effective_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_billing_entitlement_snapshots_workspace_effective
  ON billing_entitlement_snapshots (workspace_id, effective_at DESC, created_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_entitlement_snapshots_source_event
  ON billing_entitlement_snapshots (workspace_id, source_event_id)
  WHERE source_event_id IS NOT NULL;

ALTER TABLE billing_entitlement_snapshots ENABLE ROW LEVEL SECURITY;
CREATE POLICY billing_entitlement_snapshot_isolation ON billing_entitlement_snapshots
  USING (workspace_id = current_setting('nvbes.workspace_id')::uuid);
