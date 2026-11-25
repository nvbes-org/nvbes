-- Billing Service V1 Isolated Schema Foundation
-- Dedicated PostgreSQL schema for Billing runtime. Zero shared tables with Identity or Account.

CREATE TABLE IF NOT EXISTS billing_plans (
    plan_code TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    stripe_price_id TEXT NOT NULL,
    currency TEXT NOT NULL DEFAULT 'eur',
    amount_cents BIGINT NOT NULL,
    billing_interval TEXT NOT NULL DEFAULT 'month',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

-- Seed minimal test plans
INSERT INTO billing_plans (plan_code, name, stripe_price_id, currency, amount_cents, billing_interval, is_active)
VALUES
    ('free', 'Free Tier', 'price_test_free', 'eur', 0, 'month', true),
    ('standard_monthly', 'Standard Monthly', 'price_test_standard_monthly', 'eur', 1000, 'month', true),
    ('pro_monthly', 'Pro Monthly', 'price_test_pro_monthly', 'eur', 2500, 'month', true)
ON CONFLICT (plan_code) DO NOTHING;

CREATE TABLE IF NOT EXISTS billing_customers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('principal', 'team')),
    stripe_customer_id TEXT NOT NULL UNIQUE,
    email TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT uq_billing_customers_account UNIQUE (account_id, account_type)
);

CREATE INDEX IF NOT EXISTS idx_billing_customers_account ON billing_customers(account_id);
CREATE INDEX IF NOT EXISTS idx_billing_customers_stripe_id ON billing_customers(stripe_customer_id);

CREATE TABLE IF NOT EXISTS billing_checkout_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    idempotency_key TEXT NOT NULL UNIQUE,
    account_id UUID NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('principal', 'team')),
    plan_code TEXT NOT NULL REFERENCES billing_plans(plan_code),
    stripe_session_id TEXT NOT NULL UNIQUE,
    stripe_customer_id TEXT NOT NULL,
    checkout_url TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'completed', 'expired')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_billing_checkout_account ON billing_checkout_sessions(account_id);

CREATE TABLE IF NOT EXISTS billing_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL,
    account_type TEXT NOT NULL CHECK (account_type IN ('principal', 'team')),
    stripe_subscription_id TEXT NOT NULL UNIQUE,
    stripe_customer_id TEXT NOT NULL,
    plan_code TEXT NOT NULL REFERENCES billing_plans(plan_code),
    status TEXT NOT NULL CHECK (status IN ('incomplete', 'trialing', 'active', 'past_due', 'canceled', 'unpaid', 'suspended')),
    current_period_start TIMESTAMPTZ,
    current_period_end TIMESTAMPTZ,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT false,
    canceled_at TIMESTAMPTZ,
    last_event_id TEXT,
    last_event_created TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    CONSTRAINT uq_billing_subscriptions_account UNIQUE (account_id, account_type)
);

CREATE INDEX IF NOT EXISTS idx_billing_subscriptions_customer ON billing_subscriptions(stripe_customer_id);

CREATE TABLE IF NOT EXISTS billing_webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    stripe_created_at TIMESTAMPTZ NOT NULL,
    payload JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'processed' CHECK (status IN ('processed', 'ignored', 'failed', 'dead_letter')),
    error_message TEXT,
    received_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_billing_webhook_type ON billing_webhook_events(event_type);
CREATE INDEX IF NOT EXISTS idx_billing_webhook_status ON billing_webhook_events(status);

CREATE TABLE IF NOT EXISTS billing_reconciliation_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_event_id TEXT,
    account_id UUID,
    reason TEXT NOT NULL,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'resolved', 'dismissed')),
    resolved_by TEXT,
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX IF NOT EXISTS idx_billing_reconcil_status ON billing_reconciliation_items(status);

CREATE TABLE IF NOT EXISTS billing_audit_events (
    id BIGSERIAL PRIMARY KEY,
    event_uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    details JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX IF NOT EXISTS idx_billing_audit_account ON billing_audit_events(account_id);

CREATE TABLE IF NOT EXISTS billing_outbox (
    id BIGSERIAL PRIMARY KEY,
    event_uuid UUID NOT NULL UNIQUE DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    aggregate_id UUID NOT NULL,
    payload JSONB NOT NULL,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);

CREATE INDEX IF NOT EXISTS idx_billing_outbox_unpublished ON billing_outbox(id) WHERE published_at IS NULL;
