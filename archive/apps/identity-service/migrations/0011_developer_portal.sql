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

CREATE UNIQUE INDEX idx_principals_id_tenant_id_unique
  ON principals (id, tenant_id);

CREATE TABLE IF NOT EXISTS developer_role_assignments (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  principal_id uuid NOT NULL,
  role developer_role NOT NULL,
  assigned_by uuid,
  created_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT developer_role_assignments_principal_tenant_fk
    FOREIGN KEY (principal_id, tenant_id) REFERENCES principals(id, tenant_id) ON DELETE CASCADE,
  CONSTRAINT developer_role_assignments_assigned_by_tenant_fk
    FOREIGN KEY (assigned_by, tenant_id) REFERENCES principals(id, tenant_id) ON DELETE SET NULL (assigned_by)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_developer_role_assignments_active_unique
  ON developer_role_assignments (tenant_id, principal_id, role)
  WHERE revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_developer_role_assignments_tenant_principal
  ON developer_role_assignments (tenant_id, principal_id)
  WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS developer_webhook_endpoints (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name text NOT NULL,
  url text NOT NULL,
  signing_secret_ciphertext text NOT NULL,
  signing_secret_last4 text NOT NULL,
  status text NOT NULL DEFAULT 'active',
  created_by uuid,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now(),
  revoked_at timestamptz,
  CONSTRAINT developer_webhook_endpoints_created_by_tenant_fk
    FOREIGN KEY (created_by, tenant_id) REFERENCES principals(id, tenant_id) ON DELETE SET NULL (created_by),
  CONSTRAINT developer_webhook_endpoints_status_check
    CHECK (status IN ('active', 'paused', 'revoked')),
  CONSTRAINT developer_webhook_endpoints_revoked_status_check
    CHECK ((status = 'revoked') = (revoked_at IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_developer_webhook_endpoints_tenant
  ON developer_webhook_endpoints (tenant_id)
  WHERE revoked_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_developer_webhook_endpoints_id_tenant_unique
  ON developer_webhook_endpoints (id, tenant_id);

CREATE TABLE IF NOT EXISTS developer_webhook_subscriptions (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL REFERENCES developer_webhook_endpoints(id) ON DELETE CASCADE,
  event_type text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (endpoint_id, event_type),
  CONSTRAINT developer_webhook_subscriptions_event_type_check
    CHECK (event_type IN ('user.created', 'login.failed', 'session.revoked', 'client.created'))
);

CREATE TABLE IF NOT EXISTS developer_webhook_deliveries (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  endpoint_id uuid NOT NULL,
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  event_type text NOT NULL,
  event_id uuid NOT NULL,
  status text NOT NULL,
  attempt_count integer NOT NULL DEFAULT 0,
  response_status integer,
  error_message text,
  created_at timestamptz NOT NULL DEFAULT now(),
  delivered_at timestamptz,
  CONSTRAINT developer_webhook_deliveries_endpoint_tenant_fk
    FOREIGN KEY (endpoint_id, tenant_id) REFERENCES developer_webhook_endpoints(id, tenant_id) ON DELETE CASCADE,
  CONSTRAINT developer_webhook_deliveries_attempt_count_check
    CHECK (attempt_count >= 0),
  CONSTRAINT developer_webhook_deliveries_status_check
    CHECK (status IN ('pending', 'delivered', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_developer_webhook_deliveries_endpoint_created
  ON developer_webhook_deliveries (endpoint_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_developer_webhook_deliveries_tenant_event
  ON developer_webhook_deliveries (tenant_id, event_type, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS idx_developer_webhook_deliveries_replay_once
  ON developer_webhook_deliveries (tenant_id, replayed_from_delivery_id)
  WHERE replayed_from_delivery_id IS NOT NULL;

ALTER TABLE developer_role_assignments ENABLE ROW LEVEL SECURITY;
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
      FOR ALL
      USING (tenant_id = current_setting('nvbes.tenant_id')::uuid)
      WITH CHECK (tenant_id = current_setting('nvbes.tenant_id')::uuid);
  END IF;
END $$;

ALTER TABLE developer_webhook_endpoints ENABLE ROW LEVEL SECURITY;
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
      FOR ALL
      USING (tenant_id = current_setting('nvbes.tenant_id')::uuid)
      WITH CHECK (tenant_id = current_setting('nvbes.tenant_id')::uuid);
  END IF;
END $$;

ALTER TABLE developer_webhook_subscriptions ENABLE ROW LEVEL SECURITY;
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
    FROM pg_policies
    WHERE schemaname = 'public'
      AND tablename = 'developer_webhook_subscriptions'
      AND policyname = 'developer_webhook_subscription_isolation'
  ) THEN
    CREATE POLICY developer_webhook_subscription_isolation ON developer_webhook_subscriptions
      FOR ALL
      USING (endpoint_id IN (
        SELECT id FROM developer_webhook_endpoints
        WHERE tenant_id = current_setting('nvbes.tenant_id')::uuid
      ))
      WITH CHECK (endpoint_id IN (
        SELECT id FROM developer_webhook_endpoints
        WHERE tenant_id = current_setting('nvbes.tenant_id')::uuid
      ));
  END IF;
END $$;

ALTER TABLE developer_webhook_deliveries ENABLE ROW LEVEL SECURITY;
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
      FOR ALL
      USING (tenant_id = current_setting('nvbes.tenant_id')::uuid)
      WITH CHECK (tenant_id = current_setting('nvbes.tenant_id')::uuid);
  END IF;
END $$;
