CREATE TYPE developer_role AS ENUM (
  'developer_admin',
  'app_manager',
  'webhook_manager',
  'log_viewer',
  'integration_tester',
  'docs_viewer'
);

CREATE TYPE developer_marketplace_status AS ENUM ('pending', 'approved', 'rejected', 'suspended');
CREATE TYPE developer_scope_risk AS ENUM ('low', 'medium', 'high', 'restricted');
CREATE TYPE developer_scope_lifecycle AS ENUM ('proposed', 'active', 'deprecated', 'retired');
CREATE TYPE developer_webhook_status AS ENUM ('active', 'paused', 'revoked');
CREATE TYPE developer_delivery_status AS ENUM ('pending', 'delivered', 'failed', 'replayed');
CREATE TYPE developer_health_status AS ENUM ('passing', 'warning', 'failing', 'unknown');

CREATE TABLE developer_role_assignments (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  principal_id uuid NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
  role developer_role NOT NULL,
  assigned_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  UNIQUE (tenant_id, principal_id, role)
);

CREATE INDEX idx_developer_role_assignments_active
  ON developer_role_assignments (tenant_id, principal_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_marketplace_apps (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  client_id text NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  status developer_marketplace_status NOT NULL DEFAULT 'pending',
  submitted_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  reviewed_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  review_reason text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (tenant_id, client_id)
);

CREATE TABLE developer_consent_screens (
  client_id text PRIMARY KEY REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  product_name text NOT NULL,
  logo_url text,
  support_url text,
  privacy_url text,
  terms_url text,
  description text NOT NULL DEFAULT '',
  updated_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  updated_at timestamptz NOT NULL DEFAULT now()
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

CREATE TABLE developer_secret_rotations (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  client_id text NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  active_version_id uuid NOT NULL DEFAULT gen_random_uuid(),
  previous_version_expires_at timestamptz NOT NULL,
  overlap_ends_at timestamptz NOT NULL,
  rotated_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT developer_secret_rotation_overlap_check CHECK (overlap_ends_at = previous_version_expires_at)
);

CREATE TABLE developer_webhook_endpoints (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name text NOT NULL,
  url text NOT NULL,
  signing_secret_ciphertext text NOT NULL,
  signing_secret_last4 text NOT NULL,
  status developer_webhook_status NOT NULL DEFAULT 'active',
  created_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz
);

CREATE INDEX idx_developer_webhook_endpoints_active
  ON developer_webhook_endpoints (tenant_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_webhook_deliveries (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL REFERENCES developer_webhook_endpoints(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  event_id uuid NOT NULL,
  event_type text NOT NULL,
  status developer_delivery_status NOT NULL,
  attempt_count integer NOT NULL DEFAULT 0,
  response_status integer,
  error_message text,
  created_at timestamptz NOT NULL DEFAULT now(),
  delivered_at timestamptz,
  replayed_from_delivery_id uuid REFERENCES developer_webhook_deliveries(id) ON DELETE SET NULL
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
  actor_principal_id uuid NOT NULL REFERENCES principals(id) ON DELETE CASCADE,
  token_hash_prefix text NOT NULL,
  active boolean NOT NULL,
  access_decision text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
