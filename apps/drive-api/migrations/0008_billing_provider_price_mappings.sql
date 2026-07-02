CREATE TABLE IF NOT EXISTS billing_provider_price_mappings (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  legacy_plan_id UUID REFERENCES plans (id) ON DELETE CASCADE,
  provider billing_provider NOT NULL,
  provider_product_id TEXT NOT NULL,
  provider_price_id TEXT NOT NULL,
  country_code CHAR(2),
  pricing_region TEXT,
  currency CHAR(3) NOT NULL DEFAULT 'EUR',
  amount_minor BIGINT CHECK (amount_minor IS NULL OR amount_minor >= 0),
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE (provider, provider_price_id),
  CHECK (country_code IS NULL OR country_code = upper(country_code)),
  CHECK (country_code IS NULL OR pricing_region IS NULL)
);

CREATE INDEX IF NOT EXISTS idx_billing_provider_price_mappings_plan_market
  ON billing_provider_price_mappings (
    legacy_plan_id,
    provider,
    status,
    country_code,
    pricing_region,
    created_at DESC
  );

INSERT INTO billing_provider_price_mappings (
  legacy_plan_id,
  provider,
  provider_product_id,
  provider_price_id,
  country_code,
  pricing_region,
  currency,
  amount_minor,
  status,
  created_at,
  updated_at
)
SELECT plan_id,
       'stripe',
       stripe_product_id,
       stripe_price_id,
       country_code,
       pricing_region,
       currency,
       amount_minor,
       status,
       created_at,
       updated_at
FROM stripe_price_mappings
ON CONFLICT (provider, provider_price_id) DO NOTHING;
