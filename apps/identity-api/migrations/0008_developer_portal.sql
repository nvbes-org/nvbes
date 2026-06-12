CREATE TYPE developer_role AS ENUM (
  'developer_admin',
  'app_manager',
  'webhook_manager',
  'log_viewer',
  'integration_tester',
  'docs_viewer'
);

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

CREATE INDEX idx_developer_role_assignments_tenant_principal
  ON developer_role_assignments (tenant_id, principal_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_webhook_endpoints (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name text NOT NULL,
  url text NOT NULL,
  signing_secret_ciphertext text NOT NULL,
  signing_secret_last4 text NOT NULL,
  status text NOT NULL DEFAULT 'active',
  created_by uuid REFERENCES principals(id) ON DELETE SET NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT developer_webhook_endpoints_status_check
    CHECK (status IN ('active', 'paused', 'revoked'))
);

CREATE INDEX idx_developer_webhook_endpoints_tenant
  ON developer_webhook_endpoints (tenant_id)
  WHERE revoked_at IS NULL;

CREATE TABLE developer_webhook_subscriptions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL REFERENCES developer_webhook_endpoints(id) ON DELETE CASCADE,
  event_type text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (endpoint_id, event_type),
  CONSTRAINT developer_webhook_subscriptions_event_type_check
    CHECK (event_type IN ('user.created', 'login.failed', 'session.revoked', 'client.created'))
);

CREATE TABLE developer_webhook_deliveries (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL REFERENCES developer_webhook_endpoints(id) ON DELETE CASCADE,
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  event_type text NOT NULL,
  event_id uuid NOT NULL,
  status text NOT NULL,
  attempt_count integer NOT NULL DEFAULT 0,
  response_status integer,
  error_message text,
  created_at timestamptz NOT NULL DEFAULT now(),
  delivered_at timestamptz,
  CONSTRAINT developer_webhook_deliveries_status_check
    CHECK (status IN ('pending', 'delivered', 'failed'))
);

CREATE INDEX idx_developer_webhook_deliveries_endpoint_created
  ON developer_webhook_deliveries (endpoint_id, created_at DESC);

CREATE INDEX idx_developer_webhook_deliveries_tenant_event
  ON developer_webhook_deliveries (tenant_id, event_type, created_at DESC);
