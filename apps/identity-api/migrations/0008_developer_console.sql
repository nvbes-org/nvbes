DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM pg_constraint
    WHERE conrelid = 'principals'::regclass
      AND conname = 'principals_tenant_id_id_unique'
  ) THEN
    ALTER TABLE principals
      ADD CONSTRAINT principals_tenant_id_id_unique UNIQUE (tenant_id, id);
  END IF;

  IF NOT EXISTS (
    SELECT 1
    FROM pg_constraint
    WHERE conrelid = 'oauth_clients'::regclass
      AND conname = 'oauth_clients_tenant_id_client_id_unique'
  ) THEN
    ALTER TABLE oauth_clients
      ADD CONSTRAINT oauth_clients_tenant_id_client_id_unique UNIQUE (tenant_id, client_id);
  END IF;
END $$;

DO $$
BEGIN
  CREATE TYPE developer_role AS ENUM (
    'developer_admin',
    'app_manager',
    'webhook_manager',
    'log_viewer',
    'integration_tester',
    'docs_viewer'
  );
EXCEPTION
  WHEN duplicate_object THEN NULL;
END $$;

CREATE TYPE developer_marketplace_status AS ENUM ('pending', 'approved', 'rejected', 'suspended');
CREATE TYPE developer_scope_risk AS ENUM ('low', 'medium', 'high', 'restricted');
CREATE TYPE developer_scope_lifecycle AS ENUM ('proposed', 'active', 'deprecated', 'retired');
CREATE TYPE developer_client_secret_version_status AS ENUM ('active', 'overlap', 'expired', 'revoked');
CREATE TYPE developer_webhook_status AS ENUM ('active', 'paused', 'revoked');
CREATE TYPE developer_delivery_status AS ENUM ('pending', 'delivered', 'failed', 'replayed');
CREATE TYPE developer_health_status AS ENUM ('passing', 'warning', 'failing', 'unknown');

CREATE TABLE IF NOT EXISTS developer_role_assignments (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  principal_id uuid NOT NULL,
  role developer_role NOT NULL,
  assigned_by uuid,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  FOREIGN KEY (tenant_id, principal_id)
    REFERENCES tenant_memberships(tenant_id, principal_id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id, assigned_by)
    REFERENCES principals(tenant_id, id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_developer_role_assignments_active_unique
  ON developer_role_assignments (tenant_id, principal_id, role)
  WHERE revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_developer_role_assignments_active
  ON developer_role_assignments (tenant_id, principal_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_marketplace_apps (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  client_id varchar(100) NOT NULL,
  status developer_marketplace_status NOT NULL DEFAULT 'pending',
  submitted_by uuid,
  reviewed_by uuid,
  review_reason text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, client_id),
  FOREIGN KEY (tenant_id, client_id)
    REFERENCES oauth_clients(tenant_id, client_id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id, submitted_by)
    REFERENCES principals(tenant_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (tenant_id, reviewed_by)
    REFERENCES principals(tenant_id, id) ON DELETE RESTRICT
);

CREATE TABLE developer_consent_screens (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  client_id varchar(100) NOT NULL,
  product_name text NOT NULL,
  logo_url text,
  support_url text,
  privacy_url text,
  terms_url text,
  description text NOT NULL DEFAULT '',
  updated_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, client_id),
  FOREIGN KEY (tenant_id, client_id)
    REFERENCES oauth_clients(tenant_id, client_id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id, updated_by)
    REFERENCES principals(tenant_id, id) ON DELETE RESTRICT
);

CREATE TABLE developer_scope_registry (
  scope_key text PRIMARY KEY,
  display_name text NOT NULL,
  description text NOT NULL,
  risk developer_scope_risk NOT NULL,
  owner_team text NOT NULL,
  lifecycle developer_scope_lifecycle NOT NULL DEFAULT 'proposed',
  allowed_audiences text[] NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE developer_client_secret_versions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  client_id varchar(100) NOT NULL,
  status developer_client_secret_version_status NOT NULL,
  client_secret_hash text NOT NULL,
  secret_last4 text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz,
  revoked_at timestamptz,
  UNIQUE (tenant_id, id),
  UNIQUE (tenant_id, client_id, id),
  FOREIGN KEY (tenant_id, client_id)
    REFERENCES oauth_clients(tenant_id, client_id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_developer_client_secret_versions_one_active
  ON developer_client_secret_versions (tenant_id, client_id)
  WHERE status = 'active' AND revoked_at IS NULL;

CREATE TABLE developer_secret_rotations (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  client_id varchar(100) NOT NULL,
  previous_version_id uuid NOT NULL,
  active_version_id uuid NOT NULL,
  previous_version_expires_at timestamptz NOT NULL,
  overlap_ends_at timestamptz NOT NULL,
  rotated_by uuid,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT developer_secret_rotation_overlap_check CHECK (overlap_ends_at = previous_version_expires_at),
  FOREIGN KEY (tenant_id, client_id)
    REFERENCES oauth_clients(tenant_id, client_id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id, rotated_by)
    REFERENCES principals(tenant_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (tenant_id, client_id, previous_version_id)
    REFERENCES developer_client_secret_versions(tenant_id, client_id, id) ON DELETE RESTRICT,
  FOREIGN KEY (tenant_id, client_id, active_version_id)
    REFERENCES developer_client_secret_versions(tenant_id, client_id, id) ON DELETE RESTRICT
);

CREATE UNIQUE INDEX idx_developer_secret_rotations_one_active
  ON developer_secret_rotations (tenant_id, client_id)
  WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS developer_webhook_endpoints (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name text NOT NULL,
  url text NOT NULL,
  signing_secret_ciphertext text NOT NULL,
  signing_secret_last4 text NOT NULL,
  status developer_webhook_status NOT NULL DEFAULT 'active',
  created_by uuid,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  UNIQUE (tenant_id, id),
  FOREIGN KEY (tenant_id, created_by)
    REFERENCES principals(tenant_id, id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_developer_webhook_endpoints_active
  ON developer_webhook_endpoints (tenant_id)
  WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS developer_webhook_deliveries (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL,
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  event_id uuid NOT NULL,
  event_type text NOT NULL,
  status developer_delivery_status NOT NULL,
  attempt_count integer NOT NULL DEFAULT 0,
  response_status integer,
  error_message text,
  created_at timestamptz NOT NULL DEFAULT now(),
  delivered_at timestamptz,
  replayed_from_delivery_id uuid,
  UNIQUE (tenant_id, id),
  FOREIGN KEY (tenant_id, endpoint_id)
    REFERENCES developer_webhook_endpoints(tenant_id, id) ON DELETE CASCADE,
  FOREIGN KEY (tenant_id, replayed_from_delivery_id)
    REFERENCES developer_webhook_deliveries(tenant_id, id) ON DELETE RESTRICT
);

CREATE TABLE developer_sandbox_tenants (
  tenant_id uuid PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
  sandbox_tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  status text NOT NULL DEFAULT 'ready',
  data_profile text NOT NULL DEFAULT 'minimal',
  reset_requested_at timestamptz,
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE developer_health_checks (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  target_type text NOT NULL,
  target_id text NOT NULL,
  check_kind text NOT NULL,
  status developer_health_status NOT NULL,
  summary text NOT NULL,
  checked_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_developer_health_checks_latest
  ON developer_health_checks (tenant_id, target_type, target_id, check_kind, checked_at DESC);

CREATE TABLE developer_token_debug_sessions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  actor_principal_id uuid NOT NULL,
  token_hash_prefix text NOT NULL,
  active boolean NOT NULL,
  access_decision text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  FOREIGN KEY (tenant_id, actor_principal_id)
    REFERENCES principals(tenant_id, id) ON DELETE CASCADE
);

ALTER TABLE developer_role_assignments ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_marketplace_apps ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_consent_screens ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_client_secret_versions ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_secret_rotations ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_webhook_endpoints ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_webhook_deliveries ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_sandbox_tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_health_checks ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_token_debug_sessions ENABLE ROW LEVEL SECURITY;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename = 'developer_role_assignments'
      AND policyname = 'developer_role_assignment_isolation'
  ) THEN
    CREATE POLICY developer_role_assignment_isolation ON developer_role_assignments
      USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
  END IF;
END $$;

CREATE POLICY developer_marketplace_app_isolation ON developer_marketplace_apps
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY developer_consent_screen_isolation ON developer_consent_screens
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY developer_client_secret_version_isolation ON developer_client_secret_versions
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY developer_secret_rotation_isolation ON developer_secret_rotations
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename = 'developer_webhook_endpoints'
      AND policyname = 'developer_webhook_endpoint_isolation'
  ) THEN
    CREATE POLICY developer_webhook_endpoint_isolation ON developer_webhook_endpoints
      USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename = 'developer_webhook_deliveries'
      AND policyname = 'developer_webhook_delivery_isolation'
  ) THEN
    CREATE POLICY developer_webhook_delivery_isolation ON developer_webhook_deliveries
      USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
  END IF;
END $$;

CREATE POLICY developer_sandbox_tenant_isolation ON developer_sandbox_tenants
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY developer_health_check_isolation ON developer_health_checks
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);

CREATE POLICY developer_token_debug_session_isolation ON developer_token_debug_sessions
  USING (tenant_id = current_setting('nvbes.tenant_id', true)::uuid);
