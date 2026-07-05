-- Provider subscriptions track the active PSP and local/regional fallbacks per subscription.

CREATE TABLE IF NOT EXISTS billing_provider_subscriptions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  subscription_id UUID NOT NULL REFERENCES billing_subscriptions(id) ON DELETE CASCADE,
  provider_customer_id UUID NOT NULL REFERENCES billing_provider_customers(id) ON DELETE CASCADE,
  provider billing_provider NOT NULL,
  provider_subscription_id TEXT NOT NULL,
  status TEXT NOT NULL,
  primary_for_subscription BOOLEAN NOT NULL DEFAULT FALSE,
  fallback_eligible BOOLEAN NOT NULL DEFAULT FALSE,
  activated_at TIMESTAMPTZ,
  deactivated_at TIMESTAMPTZ,
  metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (provider, provider_subscription_id)
);

ALTER TABLE billing_provider_subscriptions
  ADD COLUMN IF NOT EXISTS primary_for_subscription BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS fallback_eligible BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE billing_provider_subscriptions subscription
SET primary_for_subscription = TRUE
WHERE NOT EXISTS (
  SELECT 1
  FROM billing_provider_subscriptions other
  WHERE other.subscription_id = subscription.subscription_id
    AND other.primary_for_subscription
)
AND subscription.id = (
  SELECT selected.id
  FROM billing_provider_subscriptions selected
  WHERE selected.subscription_id = subscription.subscription_id
  ORDER BY selected.updated_at DESC, selected.created_at DESC
  LIMIT 1
);

UPDATE billing_provider_subscriptions subscription
SET fallback_eligible = TRUE
WHERE subscription.status IN ('active', 'trialing')
  AND subscription.primary_for_subscription = FALSE;

CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_provider_subscriptions_one_primary
  ON billing_provider_subscriptions (subscription_id)
  WHERE primary_for_subscription;

CREATE INDEX IF NOT EXISTS idx_billing_provider_subscriptions_fallback
  ON billing_provider_subscriptions (subscription_id, fallback_eligible, updated_at DESC);
