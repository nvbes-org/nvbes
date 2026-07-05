-- Billing runtime foundation.
--
-- Identity remains the source of truth for users, tenants and workspaces. Billing keeps
-- local projections with the legacy table names currently used by the Billing runtime.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'tenant_kind') THEN
    CREATE TYPE tenant_kind AS ENUM ('personal', 'team', 'enterprise', 'system');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'tenant_status') THEN
    CREATE TYPE tenant_status AS ENUM ('active', 'suspended', 'deleted');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workspace_type') THEN
    CREATE TYPE workspace_type AS ENUM ('personal', 'team');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workspace_member_role') THEN
    CREATE TYPE workspace_member_role AS ENUM ('owner', 'admin', 'member', 'viewer');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'workspace_member_status') THEN
    CREATE TYPE workspace_member_status AS ENUM ('active', 'invited', 'suspended', 'removed');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'membership_source') THEN
    CREATE TYPE membership_source AS ENUM ('manual', 'invitation', 'scim', 'jit', 'system');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'step_up_level') THEN
    CREATE TYPE step_up_level AS ENUM ('aal1', 'aal2', 'aal3');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'billing_provider') THEN
    CREATE TYPE billing_provider AS ENUM ('stripe', 'mollie');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'subscription_status') THEN
    CREATE TYPE subscription_status AS ENUM ('trialing', 'active', 'past_due', 'canceled', 'incomplete', 'suspended');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'customer_type') THEN
    CREATE TYPE customer_type AS ENUM ('b2b', 'b2c');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'billing_adjustment_type') THEN
    CREATE TYPE billing_adjustment_type AS ENUM ('credit', 'refund', 'manual_adjustment');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'billing_webhook_status') THEN
    CREATE TYPE billing_webhook_status AS ENUM ('received', 'processed', 'rejected', 'failed');
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'email_event_type') THEN
    CREATE TYPE email_event_type AS ENUM (
      'email_delivered',
      'email_bounced',
      'email_complained',
      'email_dropped',
      'email_blacklisted',
      'email_unsubscribed',
      'email_opened',
      'email_clicked',
      'email_sent'
    );
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'email_message_status') THEN
    CREATE TYPE email_message_status AS ENUM ('queued', 'sent', 'delivered', 'bounced', 'complained', 'dropped');
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS tenants (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  kind tenant_kind NOT NULL DEFAULT 'personal',
  name TEXT NOT NULL,
  slug TEXT NOT NULL UNIQUE,
  status tenant_status NOT NULL DEFAULT 'active',
  security_tier TEXT NOT NULL DEFAULT 'standard',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS users (
  principal_id UUID PRIMARY KEY,
  email VARCHAR(255) NOT NULL UNIQUE,
  name VARCHAR(255) NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS plans (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  code VARCHAR(50) NOT NULL UNIQUE,
  name TEXT NOT NULL,
  included_storage_gb INTEGER NOT NULL DEFAULT 0 CHECK (included_storage_gb >= 0),
  included_users INTEGER NOT NULL DEFAULT 0 CHECK (included_users >= 0),
  retention_days INTEGER NOT NULL DEFAULT 0 CHECK (retention_days >= 0),
  max_share_links INTEGER NOT NULL DEFAULT 0 CHECK (max_share_links >= 0),
  audit_level TEXT NOT NULL DEFAULT 'basic',
  max_share_link_ttl_days INTEGER NOT NULL DEFAULT 30,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO plans (
  code,
  name,
  included_storage_gb,
  included_users,
  retention_days,
  max_share_links,
  audit_level,
  max_share_link_ttl_days
)
VALUES
  ('trial', 'Trial', 10, 1, 30, 5, 'basic', 7),
  ('solo_pro', 'Solo Pro', 100, 1, 90, 25, 'standard', 30),
  ('team', 'Team', 1000, 10, 180, 100, 'standard', 90),
  ('team_plus', 'Team Plus', 5000, 50, 365, 500, 'advanced', 365)
ON CONFLICT (code) DO NOTHING;

CREATE TABLE IF NOT EXISTS workspaces (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  workspace_type workspace_type NOT NULL DEFAULT 'personal',
  plan_code VARCHAR(50) NOT NULL DEFAULT 'solo_pro',
  plan_id UUID REFERENCES plans(id),
  owner_user_id UUID REFERENCES users(principal_id),
  trial_ends_at TIMESTAMPTZ,
  deleted_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS workspace_memberships (
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  principal_id UUID NOT NULL REFERENCES users(principal_id) ON DELETE CASCADE,
  role workspace_member_role NOT NULL DEFAULT 'member',
  status workspace_member_status NOT NULL DEFAULT 'active',
  source membership_source NOT NULL DEFAULT 'manual',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (workspace_id, principal_id)
);

CREATE TABLE IF NOT EXISTS workspace_policies (
  workspace_id UUID PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
  member_can_create_share_links BOOLEAN NOT NULL DEFAULT FALSE,
  require_admin_approval_for_member_share BOOLEAN NOT NULL DEFAULT TRUE,
  default_share_link_ttl_days INTEGER NOT NULL DEFAULT 7,
  max_share_link_ttl_days INTEGER NOT NULL DEFAULT 30,
  required_acr step_up_level NOT NULL DEFAULT 'aal1',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS subscriptions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE RESTRICT,
  status subscription_status NOT NULL,
  billing_provider billing_provider NOT NULL DEFAULT 'stripe',
  billing_customer_id TEXT,
  billing_subscription_id TEXT,
  current_period_start TIMESTAMPTZ,
  current_period_end TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS billing_accounts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  provider billing_provider NOT NULL DEFAULT 'stripe',
  stripe_customer_id TEXT,
  billing_email TEXT,
  country CHAR(2),
  customer_type customer_type NOT NULL DEFAULT 'b2b',
  vat_number TEXT,
  tax_exempt_status TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS stripe_price_mappings (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  plan_id UUID NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
  meter TEXT NOT NULL DEFAULT 'subscription',
  stripe_product_id TEXT NOT NULL,
  stripe_price_id TEXT NOT NULL UNIQUE,
  country_code CHAR(2),
  pricing_region TEXT,
  currency CHAR(3) NOT NULL DEFAULT 'EUR',
  amount_minor BIGINT CHECK (amount_minor IS NULL OR amount_minor >= 0),
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
  valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  valid_until TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CHECK (country_code IS NULL OR country_code = upper(country_code)),
  CHECK (country_code IS NULL OR pricing_region IS NULL),
  CHECK (valid_until IS NULL OR valid_until > valid_from)
);

CREATE TABLE IF NOT EXISTS quota_usage (
  workspace_id UUID PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
  used_storage_bytes BIGINT NOT NULL DEFAULT 0 CHECK (used_storage_bytes >= 0),
  file_count BIGINT NOT NULL DEFAULT 0 CHECK (file_count >= 0),
  bandwidth_out_bytes_month BIGINT NOT NULL DEFAULT 0 CHECK (bandwidth_out_bytes_month >= 0),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS invoice_estimates (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  billing_period_start DATE NOT NULL,
  billing_period_end DATE NOT NULL,
  estimated_amount_cents BIGINT NOT NULL CHECK (estimated_amount_cents >= 0),
  currency CHAR(3) NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS usage_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  meter TEXT NOT NULL,
  quantity BIGINT NOT NULL,
  unit TEXT NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL,
  source TEXT NOT NULL,
  idempotency_key TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS usage_snapshots (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  billing_period_start DATE NOT NULL,
  billing_period_end DATE NOT NULL,
  meter TEXT NOT NULL,
  quantity BIGINT NOT NULL,
  unit TEXT NOT NULL,
  billable_quantity BIGINT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS billing_webhook_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  provider billing_provider NOT NULL DEFAULT 'stripe',
  provider_event_id TEXT NOT NULL UNIQUE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,
  status billing_webhook_status NOT NULL DEFAULT 'received',
  received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  processed_at TIMESTAMPTZ,
  signature_valid BOOLEAN NOT NULL DEFAULT FALSE,
  payload_summary JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS audit_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  workspace_id UUID REFERENCES workspaces(id) ON DELETE SET NULL,
  actor_principal_id UUID REFERENCES users(principal_id) ON DELETE SET NULL,
  action TEXT NOT NULL,
  target_type TEXT NOT NULL,
  target_id UUID,
  ip INET,
  user_agent TEXT,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  previous_event_hash TEXT,
  event_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS email_events (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  provider_event_id TEXT NOT NULL,
  provider_email_id TEXT,
  email TEXT NOT NULL,
  event_type email_event_type NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL,
  details JSONB DEFAULT '{}'::jsonb,
  processed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS suppressed_emails (
  email TEXT PRIMARY KEY,
  reason TEXT NOT NULL,
  suppressed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  details JSONB DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS email_messages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  job_id UUID NOT NULL,
  business_type TEXT NOT NULL,
  recipient_email TEXT NOT NULL,
  recipient_hash TEXT NOT NULL,
  provider_email_id TEXT,
  status email_message_status NOT NULL DEFAULT 'queued',
  sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_accounts_stripe_customer_id_unique
  ON billing_accounts (stripe_customer_id)
  WHERE stripe_customer_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_subscriptions_billing_customer_id_unique
  ON subscriptions (billing_customer_id)
  WHERE billing_customer_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_subscriptions_workspace_id_unique
  ON subscriptions (workspace_id);
CREATE INDEX IF NOT EXISTS idx_billing_webhook_events_workspace_id
  ON billing_webhook_events (workspace_id, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_stripe_price_mappings_plan_meter_status
  ON stripe_price_mappings (plan_id, meter, status);
CREATE INDEX IF NOT EXISTS idx_stripe_price_mappings_plan_market
  ON stripe_price_mappings (plan_id, meter, status, country_code, pricing_region, valid_from DESC);
CREATE INDEX IF NOT EXISTS idx_invoice_estimates_workspace_id
  ON invoice_estimates (workspace_id);
CREATE INDEX IF NOT EXISTS idx_usage_snapshots_workspace_id
  ON usage_snapshots (workspace_id);
CREATE INDEX IF NOT EXISTS idx_usage_events_workspace_meter_occurred_at
  ON usage_events (workspace_id, meter, occurred_at DESC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_email_events_provider_event_id
  ON email_events(provider_event_id);
CREATE INDEX IF NOT EXISTS idx_email_events_provider_email_id
  ON email_events(provider_email_id);
CREATE INDEX IF NOT EXISTS idx_email_messages_provider_email_id
  ON email_messages(provider_email_id);
